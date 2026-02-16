use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Database;

#[derive(Debug, Clone)]
pub struct StudyItem {
    pub id: i64,
    #[allow(dead_code)]
    pub document_id: Option<i64>,
    pub item_type: String,
    pub front: String,
    pub back: String,
    #[allow(dead_code)]
    pub next_review_date: DateTime<Utc>,
    #[allow(dead_code)]
    pub interval_days: f64,
    #[allow(dead_code)]
    pub ease_factor: f64,
    #[allow(dead_code)]
    pub review_count: i64,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
}

pub struct StudyStore<'a> {
    db: &'a Database,
}

impl<'a> StudyStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert a new study item
    #[allow(dead_code)]
    pub fn insert(
        &self,
        document_id: Option<i64>,
        item_type: &str,
        front: &str,
        back: &str,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();

        self.db
            .conn
            .execute(
                "INSERT INTO study_items (document_id, item_type, front, back, next_review_date, interval_days, ease_factor, review_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1.0, 2.5, 0, ?6, ?7)",
                params![document_id, item_type, front, back, now, now, now],
            )
            .context("Failed to insert study item")?;

        Ok(self.db.conn.last_insert_rowid())
    }

    /// Get items due for review
    pub fn get_due(&self, limit: usize) -> Result<Vec<StudyItem>> {
        let now = Utc::now().to_rfc3339();

        let mut stmt = self.db.conn.prepare(
            "SELECT id, document_id, item_type, front, back, next_review_date, interval_days, ease_factor, review_count, created_at, updated_at
             FROM study_items WHERE next_review_date <= ?1 ORDER BY next_review_date ASC LIMIT ?2",
        )?;

        let mut rows = stmt.query(params![now, limit as i64])?;
        let mut items = Vec::new();

        while let Some(row) = rows.next()? {
            items.push(Self::row_to_item(row)?);
        }

        Ok(items)
    }

    /// Count items due for review
    pub fn count_due(&self) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM study_items WHERE next_review_date <= ?1",
            params![now],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Update item after review using SM-2 algorithm
    /// quality: 0-5 (0-2 = fail/hard, 3 = okay, 4 = good, 5 = easy)
    pub fn update_after_review(&self, id: i64, quality: u8) -> Result<()> {
        // Get current item
        let mut stmt = self.db.conn.prepare(
            "SELECT interval_days, ease_factor, review_count FROM study_items WHERE id = ?1",
        )?;

        let (interval, ease, count): (f64, f64, i64) = stmt.query_row(params![id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        let quality = quality.min(5) as f64;

        // SM-2 algorithm
        let new_ease = (ease + 0.1 - (5.0 - quality) * (0.08 + (5.0 - quality) * 0.02)).max(1.3);

        let (new_interval, new_count) = if quality < 3.0 {
            // Failed — reset interval
            (1.0, 0_i64)
        } else {
            let ni = match count {
                0 => 1.0,
                1 => 6.0,
                _ => interval * new_ease,
            };
            (ni, count + 1)
        };

        let next_review = Utc::now() + chrono::Duration::seconds((new_interval * 86400.0) as i64);
        let now = Utc::now().to_rfc3339();
        let next_str = next_review.to_rfc3339();

        self.db.conn.execute(
            "UPDATE study_items SET interval_days = ?1, ease_factor = ?2, review_count = ?3, next_review_date = ?4, updated_at = ?5 WHERE id = ?6",
            params![new_interval, new_ease, new_count, next_str, now, id],
        )?;

        Ok(())
    }

    /// Bulk insert study items, returns count inserted
    pub fn bulk_insert(
        &self,
        items: &[(Option<i64>, &str, &str, &str)], // (document_id, item_type, front, back)
    ) -> Result<usize> {
        let now = Utc::now().to_rfc3339();
        let mut count = 0;

        for (doc_id, item_type, front, back) in items {
            self.db.conn.execute(
                "INSERT INTO study_items (document_id, item_type, front, back, next_review_date, interval_days, ease_factor, review_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1.0, 2.5, 0, ?6, ?7)",
                params![doc_id, item_type, front, back, now, now, now],
            )?;
            count += 1;
        }

        Ok(count)
    }

    fn row_to_item(row: &rusqlite::Row) -> Result<StudyItem> {
        let review_str: String = row.get(5)?;
        let created_str: String = row.get(9)?;
        let updated_str: String = row.get(10)?;

        Ok(StudyItem {
            id: row.get(0)?,
            document_id: row.get(1)?,
            item_type: row.get(2)?,
            front: row.get(3)?,
            back: row.get(4)?,
            next_review_date: DateTime::parse_from_rfc3339(&review_str)
                .context("Invalid timestamp")?
                .with_timezone(&Utc),
            interval_days: row.get(6)?,
            ease_factor: row.get(7)?,
            review_count: row.get(8)?,
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .context("Invalid timestamp")?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_str)
                .context("Invalid timestamp")?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use std::path::PathBuf;

    fn test_db() -> Database {
        let path = PathBuf::from(format!("/tmp/librarian_test_{}.db", std::process::id()));
        // Clean up any previous test DB
        let _ = std::fs::remove_file(&path);
        Database::open_at_path(path).unwrap()
    }

    #[test]
    fn test_sm2_easy_increases_interval() {
        let db = test_db();
        let store = StudyStore::new(&db);
        let id = store.insert(None, "flashcard", "Q", "A").unwrap();

        // First review — quality 5 (easy)
        store.update_after_review(id, 5).unwrap();

        let mut stmt = db
            .conn
            .prepare(
                "SELECT interval_days, ease_factor, review_count FROM study_items WHERE id = ?1",
            )
            .unwrap();
        let (interval, ease, count): (f64, f64, i64) = stmt
            .query_row(params![id], |row| {
                Ok((
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    row.get(2).unwrap(),
                ))
            })
            .unwrap();

        assert_eq!(count, 1);
        assert!(interval >= 1.0);
        assert!(ease >= 2.5);

        // Clean up
        let _ = std::fs::remove_file(db.path.as_path());
    }

    #[test]
    fn test_sm2_fail_resets() {
        let path = PathBuf::from(format!(
            "/tmp/librarian_test_fail_{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let db = Database::open_at_path(path).unwrap();
        let store = StudyStore::new(&db);
        let id = store.insert(None, "flashcard", "Q", "A").unwrap();

        // Good review first
        store.update_after_review(id, 4).unwrap();
        // Then fail
        store.update_after_review(id, 1).unwrap();

        let mut stmt = db
            .conn
            .prepare("SELECT interval_days, review_count FROM study_items WHERE id = ?1")
            .unwrap();
        let (interval, count): (f64, i64) = stmt
            .query_row(params![id], |row| {
                Ok((row.get(0).unwrap(), row.get(1).unwrap()))
            })
            .unwrap();

        assert_eq!(count, 0);
        assert!((interval - 1.0).abs() < f64::EPSILON);

        // Clean up
        let _ = std::fs::remove_file(db.path.as_path());
    }
}
