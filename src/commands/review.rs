use anyhow::Result;
use colored::Colorize;
use inquire::Select;

use crate::storage::{Database, StudyStore};

pub async fn run() -> Result<()> {
    let db = Database::open()?;
    let store = StudyStore::new(&db);

    let due_count = store.count_due()?;

    if due_count == 0 {
        println!(
            "\n{} No items due for review! Generate some flashcards or quizzes first.",
            "✓".green()
        );
        println!(
            "  Use {} to create study materials.",
            "librarian generate flashcards".cyan()
        );
        return Ok(());
    }

    println!();
    println!(
        "    {}",
        "╭──────────────────────────────────────────────────────╮".blue()
    );
    println!(
        "    {}          {}          {}",
        "│".blue(),
        "🔁 SPACED REPETITION REVIEW 🔁".bold().white(),
        "│".blue()
    );
    println!(
        "    {}   {} items due for review                        {}",
        "│".blue(),
        due_count.to_string().yellow().bold(),
        "│".blue()
    );
    println!(
        "    {}",
        "╰──────────────────────────────────────────────────────╯".blue()
    );
    println!();

    let items = store.get_due(50)?;
    let total = items.len();
    let mut correct = 0;

    for (i, item) in items.iter().enumerate() {
        println!(
            "{} [{}/{}] {}",
            "Card".bold().cyan(),
            i + 1,
            total,
            format!("({})", item.item_type).dimmed()
        );
        println!();
        println!("  {} {}", "Q:".bold().yellow(), item.front);
        println!();

        // Wait for user to reveal answer
        let _ = inquire::Text::new("  Press Enter to reveal answer...")
            .with_default("")
            .prompt();

        println!("  {} {}", "A:".bold().green(), item.back);
        println!();

        // Self-rate
        let options = vec![
            "1 - Did not remember at all",
            "2 - Barely remembered, wrong",
            "3 - Remembered with difficulty",
            "4 - Remembered correctly",
            "5 - Easy, perfect recall",
        ];

        let rating = Select::new("  How well did you recall this?", options).prompt();

        let quality: u8 = match rating {
            Ok(s) => s.chars().next().unwrap_or('3').to_digit(10).unwrap_or(3) as u8,
            Err(inquire::InquireError::OperationCanceled)
            | Err(inquire::InquireError::OperationInterrupted) => {
                println!("\n{}", "Review session ended early.".dimmed());
                print_summary(correct, i);
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        if quality >= 3 {
            correct += 1;
        }

        store.update_after_review(item.id, quality)?;

        println!("{}", "─".repeat(50).dimmed());
    }

    print_summary(correct, total);

    Ok(())
}

fn print_summary(correct: usize, total: usize) {
    let pct = if total > 0 {
        (correct as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!();
    println!(
        "    {}",
        "╭──────────────────────────────────────────────────────╮".green()
    );
    println!(
        "    {}            {}            {}",
        "│".green(),
        "📊 SESSION SUMMARY 📊".bold().white(),
        "│".green()
    );
    println!(
        "    {}  Reviewed: {} │ Correct: {} │ Score: {:.0}%           {}",
        "│".green(),
        total.to_string().cyan(),
        correct.to_string().green(),
        pct,
        "│".green()
    );
    println!(
        "    {}",
        "╰──────────────────────────────────────────────────────╯".green()
    );
    println!();
}
