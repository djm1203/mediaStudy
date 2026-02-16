use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Database;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: i64,
    pub title: Option<String>,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoredMessage {
    #[allow(dead_code)]
    pub id: i64,
    #[allow(dead_code)]
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

pub struct ConversationStore<'a> {
    db: &'a Database,
}

impl<'a> ConversationStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new conversation
    pub fn create(&self, title: Option<&str>) -> Result<i64> {
        let now = Utc::now().to_rfc3339();

        self.db
            .conn
            .execute(
                "INSERT INTO conversations (title, created_at, updated_at) VALUES (?1, ?2, ?3)",
                params![title, now, now],
            )
            .context("Failed to create conversation")?;

        Ok(self.db.conn.last_insert_rowid())
    }

    /// Add a message to a conversation
    pub fn add_message(&self, conversation_id: i64, role: &str, content: &str) -> Result<i64> {
        let now = Utc::now().to_rfc3339();

        self.db
            .conn
            .execute(
                "INSERT INTO messages (conversation_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![conversation_id, role, content, now],
            )
            .context("Failed to add message")?;

        // Update conversation timestamp
        self.db.conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now, conversation_id],
        )?;

        Ok(self.db.conn.last_insert_rowid())
    }

    /// Get all messages for a conversation
    pub fn get_messages(&self, conversation_id: i64) -> Result<Vec<StoredMessage>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, conversation_id, role, content, created_at
             FROM messages WHERE conversation_id = ?1 ORDER BY id ASC",
        )?;

        let mut rows = stmt.query(params![conversation_id])?;
        let mut messages = Vec::new();

        while let Some(row) = rows.next()? {
            let created_str: String = row.get(4)?;
            messages.push(StoredMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .context("Invalid timestamp")?
                    .with_timezone(&Utc),
            });
        }

        Ok(messages)
    }

    /// List recent conversations
    pub fn list_recent(&self, limit: usize) -> Result<Vec<Conversation>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, title, created_at, updated_at
             FROM conversations ORDER BY updated_at DESC LIMIT ?1",
        )?;

        let mut rows = stmt.query(params![limit as i64])?;
        let mut conversations = Vec::new();

        while let Some(row) = rows.next()? {
            let created_str: String = row.get(2)?;
            let updated_str: String = row.get(3)?;
            conversations.push(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .context("Invalid timestamp")?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_str)
                    .context("Invalid timestamp")?
                    .with_timezone(&Utc),
            });
        }

        Ok(conversations)
    }

    /// Update conversation title
    pub fn update_title(&self, id: i64, title: &str) -> Result<()> {
        self.db.conn.execute(
            "UPDATE conversations SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    /// Delete a conversation and its messages
    #[allow(dead_code)]
    pub fn delete(&self, id: i64) -> Result<bool> {
        self.db.conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![id],
        )?;
        let affected = self
            .db
            .conn
            .execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}
