use anyhow::Result;
use colored::Colorize;
use inquire::Select;

use crate::config::Config;
use crate::llm::{GroqClient, groq::Message};
use crate::storage::{Database, StudyStore};

/// Question types parsed from quiz output
enum QuizQuestion {
    MultipleChoice {
        question: String,
        options: Vec<(char, String)>,
        correct: char,
    },
    FillInBlank {
        question: String,
        answer: String,
    },
    ShortAnswer {
        question: String,
        expected: String,
    },
}

pub async fn run() -> Result<()> {
    println!();
    println!(
        "    {}",
        "╭──────────────────────────────────────────────────────╮".magenta()
    );
    println!(
        "    {}           {}           {}",
        "│".magenta(),
        "🎯 INTERACTIVE QUIZ 🎯".bold().white(),
        "│".magenta()
    );
    println!(
        "    {}     {}     {}",
        "│".magenta(),
        "Test your knowledge with active recall!".dimmed(),
        "│".magenta()
    );
    println!(
        "    {}",
        "╰──────────────────────────────────────────────────────╯".magenta()
    );
    println!();

    let db = Database::open()?;
    let store = StudyStore::new(&db);

    let due_count = store.count_due()?;

    let mode_options = if due_count > 0 {
        vec![
            format!("📋  Review due items    │ {} quiz items due", due_count),
            "🆕  Generate fresh quiz │ Create new quiz from materials".to_string(),
            "←   Back".to_string(),
        ]
    } else {
        vec![
            "🆕  Generate fresh quiz │ Create new quiz from materials".to_string(),
            "←   Back".to_string(),
        ]
    };

    let selection = Select::new("Choose quiz mode:", mode_options).prompt()?;

    if selection.contains("Back") {
        return Ok(());
    }

    if selection.contains("Review due") {
        return run_due_quiz(&store).await;
    }

    // Generate fresh quiz
    run_fresh_quiz(&store).await
}

async fn run_due_quiz(store: &StudyStore<'_>) -> Result<()> {
    let items = store.get_due(20)?;

    if items.is_empty() {
        println!("{}", "No items due for review!".dimmed());
        return Ok(());
    }

    let total = items.len();
    let mut correct = 0;
    let mut mc_correct = 0;
    let mut mc_total = 0;
    let mut other_correct = 0;
    let mut other_total = 0;

    for (i, item) in items.iter().enumerate() {
        println!("\n{} [{}/{}]", "Question".bold().cyan(), i + 1, total);
        println!("  {}", item.front);
        println!();

        let answer = inquire::Text::new("  Your answer:")
            .with_help_message("Type your answer or press Enter to skip")
            .prompt()?;

        let answer = answer.trim();

        // Simple scoring: case-insensitive containment
        let expected_lower = item.back.to_lowercase();
        let answer_lower = answer.to_lowercase();
        let is_correct = !answer.is_empty()
            && (answer_lower == expected_lower
                || expected_lower.contains(&answer_lower)
                || answer_lower.contains(&expected_lower));

        if is_correct {
            println!("  {} Correct!", "✓".green().bold());
            correct += 1;
            store.update_after_review(item.id, 4)?;
        } else {
            println!("  {} Incorrect", "✗".red().bold());
            println!("  {} {}", "Expected:".dimmed(), item.back);
            store.update_after_review(item.id, 1)?;
        }

        if item.item_type == "quiz_mc" {
            mc_total += 1;
            if is_correct {
                mc_correct += 1;
            }
        } else {
            other_total += 1;
            if is_correct {
                other_correct += 1;
            }
        }

        println!("{}", "─".repeat(50).dimmed());
    }

    print_quiz_summary(
        correct,
        total,
        mc_correct,
        mc_total,
        other_correct,
        other_total,
    );
    Ok(())
}

async fn run_fresh_quiz(store: &StudyStore<'_>) -> Result<()> {
    let config = Config::load()?;
    let api_key = match config.get_api_key() {
        Some(key) => key,
        None => {
            println!(
                "{} No API key configured. Run {} to set up.",
                "Error:".red().bold(),
                "librarian config".cyan()
            );
            return Ok(());
        }
    };

    let client = GroqClient::new(api_key, config.default_model);

    let topic = inquire::Text::new("Topic (or Enter for all materials):")
        .prompt()
        .unwrap_or_default();

    // Get context
    let context = crate::commands::generate::get_document_context_pub(&topic)?;

    if context.is_empty() {
        println!(
            "{} No documents found. Add materials first.",
            "Error:".red()
        );
        return Ok(());
    }

    println!("{}", "Generating quiz...".dimmed());

    let messages = vec![
        Message {
            role: "system".to_string(),
            content: QUIZ_PROMPT.to_string(),
        },
        Message {
            role: "user".to_string(),
            content: if topic.is_empty() {
                format!(
                    "Create an interactive quiz from these materials:\n\n{}\n\nCover the most important topics.",
                    context,
                )
            } else {
                format!(
                    "Create an interactive quiz from these materials:\n\n{}\n\nFocus on: {}",
                    context, topic,
                )
            },
        },
    ];

    let response = client.chat(&messages).await?;

    // Parse questions from response
    let questions = parse_quiz_questions(&response);

    if questions.is_empty() {
        println!("Could not parse quiz questions. Displaying raw quiz:\n");
        println!("{}", response);
        return Ok(());
    }

    // Run quiz interactively
    let total = questions.len();
    let mut correct = 0;
    let mut mc_correct = 0;
    let mut mc_total = 0;
    let mut other_correct = 0;
    let mut other_total = 0;

    // Items to save for spaced repetition
    let mut items_to_save: Vec<(Option<i64>, &str, &str, &str)> = Vec::new();

    for (i, q) in questions.iter().enumerate() {
        println!("\n{} [{}/{}]", "Question".bold().cyan(), i + 1, total);

        match q {
            QuizQuestion::MultipleChoice {
                question,
                options,
                correct: correct_answer,
            } => {
                mc_total += 1;
                println!("  {}", question);
                for (letter, text) in options {
                    println!("    {}) {}", letter, text);
                }
                println!();

                let answer = inquire::Text::new("  Your answer (a/b/c/d):")
                    .prompt()
                    .unwrap_or_default();

                let user_char = answer.trim().to_lowercase().chars().next().unwrap_or(' ');
                let is_correct = user_char == *correct_answer;

                if is_correct {
                    println!("  {} Correct!", "✓".green().bold());
                    correct += 1;
                    mc_correct += 1;
                } else {
                    println!(
                        "  {} Incorrect. Answer: {})",
                        "✗".red().bold(),
                        correct_answer
                    );
                }
            }
            QuizQuestion::FillInBlank { question, answer } => {
                other_total += 1;
                println!("  {}", question);
                println!();

                let user_answer = inquire::Text::new("  Fill in the blank:")
                    .prompt()
                    .unwrap_or_default();

                let is_correct = user_answer
                    .trim()
                    .to_lowercase()
                    .contains(&answer.to_lowercase());

                if is_correct {
                    println!("  {} Correct!", "✓".green().bold());
                    correct += 1;
                    other_correct += 1;
                } else {
                    println!("  {} Incorrect. Answer: {}", "✗".red().bold(), answer);
                }
            }
            QuizQuestion::ShortAnswer { question, expected } => {
                other_total += 1;
                println!("  {}", question);
                println!();

                let user_answer = inquire::Text::new("  Your answer:")
                    .prompt()
                    .unwrap_or_default();

                // Simple heuristic: check for keyword overlap
                let expected_lower = expected.to_lowercase();
                let expected_words: std::collections::HashSet<&str> =
                    expected_lower.split_whitespace().collect();
                let user_lower = user_answer.to_lowercase();
                let user_words: std::collections::HashSet<&str> =
                    user_lower.split_whitespace().collect();

                let overlap = expected_words.intersection(&user_words).count();
                let is_correct = !user_answer.trim().is_empty()
                    && overlap as f64 / expected_words.len().max(1) as f64 > 0.4;

                if is_correct {
                    println!("  {} Good answer!", "✓".green().bold());
                    correct += 1;
                    other_correct += 1;
                } else {
                    println!("  {} Expected: {}", "✗".red().bold(), expected);
                }
            }
        }

        println!("{}", "─".repeat(50).dimmed());
    }

    print_quiz_summary(
        correct,
        total,
        mc_correct,
        mc_total,
        other_correct,
        other_total,
    );

    // Offer to save for spaced repetition
    let save_opts = vec![
        "💾  Save to spaced repetition │ Review these later",
        "❌  Don't save",
    ];
    let save = Select::new("Save quiz items for spaced repetition?", save_opts).prompt();

    if let Ok(s) = save
        && s.contains("Save")
    {
        // Collect items to save
        for q in &questions {
            match q {
                QuizQuestion::MultipleChoice {
                    question,
                    correct: c,
                    options,
                    ..
                } => {
                    let answer = options
                        .iter()
                        .find(|(l, _)| l == c)
                        .map(|(_, t)| t.as_str())
                        .unwrap_or("?");
                    items_to_save.push((None, "quiz_mc", question, answer));
                }
                QuizQuestion::FillInBlank { question, answer } => {
                    items_to_save.push((None, "quiz_fill", question, answer));
                }
                QuizQuestion::ShortAnswer { question, expected } => {
                    items_to_save.push((None, "quiz_short", question, expected));
                }
            }
        }

        let saved = store.bulk_insert(&items_to_save)?;
        println!(
            "{} Saved {} items for spaced repetition review!",
            "✓".green(),
            saved
        );
    }

    Ok(())
}

fn parse_quiz_questions(text: &str) -> Vec<QuizQuestion> {
    let mut questions = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Try to detect a numbered question
        if let Some(q_text) = extract_question_text(line) {
            // Check if next lines have options (a-d)
            let mut options = Vec::new();
            let mut j = i + 1;

            while j < lines.len() {
                let opt_line = lines[j].trim();
                if let Some((letter, text)) = extract_option(opt_line) {
                    options.push((letter, text));
                    j += 1;
                } else {
                    break;
                }
            }

            if options.len() >= 2 {
                // Multiple choice — find answer
                let correct = find_answer_letter(&lines[j..]);
                let skip = if correct.is_some() { j + 1 } else { j };

                questions.push(QuizQuestion::MultipleChoice {
                    question: q_text,
                    options,
                    correct: correct.unwrap_or('a'),
                });
                i = skip;
                continue;
            }

            // Check for fill-in-blank (contains ___)
            if q_text.contains("___")
                && let Some(answer) = find_answer_text(&lines[j..])
            {
                questions.push(QuizQuestion::FillInBlank {
                    question: q_text,
                    answer,
                });
                i = j + 1;
                continue;
            }

            // Default: short answer
            if let Some(answer) = find_answer_text(&lines[(i + 1)..]) {
                questions.push(QuizQuestion::ShortAnswer {
                    question: q_text,
                    expected: answer,
                });
                i = j + 1;
                continue;
            }
        }

        i += 1;
    }

    questions
}

fn extract_question_text(line: &str) -> Option<String> {
    let line = line.trim();
    // Match patterns like "1.", "1)", "Q:", or "**1.**"
    if line.starts_with("Q:") || line.starts_with("Q.") {
        return Some(line[2..].trim().to_string());
    }

    // Numbered question: strip leading number + punctuation
    let mut chars = line.chars().peekable();
    // Skip markdown bold
    if line.starts_with("**") {
        let inner = line.trim_start_matches("**");
        if let Some(end) = inner.find("**") {
            let num_part = &inner[..end];
            if num_part.chars().any(|c| c.is_ascii_digit()) {
                let rest = inner[end..].trim_start_matches("**").trim();
                if !rest.is_empty() {
                    return Some(rest.to_string());
                }
            }
        }
    }

    // Numbered: "1. question" or "1) question"
    if chars.peek().is_some_and(|c| c.is_ascii_digit()) {
        let num_end = line
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(line.len());
        let rest = line[num_end..].trim_start_matches(['.', ')', ':']).trim();
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }

    None
}

fn extract_option(line: &str) -> Option<(char, String)> {
    let line = line.trim();
    if line.len() < 3 {
        return None;
    }

    let first = line.chars().next()?;
    if !first.is_ascii_lowercase() || !('a'..='d').contains(&first) {
        return None;
    }

    let rest = &line[1..];
    if rest.starts_with(')') || rest.starts_with('.') || rest.starts_with(':') {
        let text = rest[1..].trim().to_string();
        if !text.is_empty() {
            return Some((first, text));
        }
    }

    None
}

fn find_answer_letter(lines: &[&str]) -> Option<char> {
    for line in lines.iter().take(3) {
        let line = line.trim().to_lowercase();
        // Match "**Answer: b)**" or "Answer: b" patterns
        if line.contains("answer") {
            for c in line.chars() {
                if ('a'..='d').contains(&c) {
                    return Some(c);
                }
            }
        }
    }
    None
}

fn find_answer_text(lines: &[&str]) -> Option<String> {
    for line in lines.iter().take(3) {
        let line_trimmed = line.trim();
        let lower = line_trimmed.to_lowercase();

        if lower.starts_with("**answer") || lower.starts_with("answer") {
            let text = line_trimmed
                .trim_start_matches("**")
                .trim_start_matches("Answer")
                .trim_start_matches("answer")
                .trim_start_matches("**")
                .trim_start_matches(':')
                .trim_start_matches("**")
                .trim()
                .trim_end_matches("**")
                .trim();

            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn print_quiz_summary(
    correct: usize,
    total: usize,
    mc_correct: usize,
    mc_total: usize,
    other_correct: usize,
    other_total: usize,
) {
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
        "    {}             {}             {}",
        "│".green(),
        "🎯 QUIZ RESULTS 🎯".bold().white(),
        "│".green()
    );
    println!(
        "    {}  Overall: {}/{} ({:.0}%)                              {}",
        "│".green(),
        correct.to_string().cyan(),
        total,
        pct,
        "│".green()
    );

    if mc_total > 0 {
        println!(
            "    {}  Multiple Choice: {}/{}                              {}",
            "│".green(),
            mc_correct.to_string().cyan(),
            mc_total,
            "│".green()
        );
    }
    if other_total > 0 {
        println!(
            "    {}  Other: {}/{}                                        {}",
            "│".green(),
            other_correct.to_string().cyan(),
            other_total,
            "│".green()
        );
    }

    println!(
        "    {}",
        "╰──────────────────────────────────────────────────────╯".green()
    );
    println!();
}

const QUIZ_PROMPT: &str = r#"You are creating a practice quiz from the provided course materials.

Generate a quiz with mixed question types:

## Multiple Choice
1. Question text
   a) Option A
   b) Option B
   c) Option C
   d) Option D
   **Answer: b)**

## Fill in the Blank
1. The process of _______ is essential for...
   **Answer: [correct answer]**

## Short Answer
1. Explain the concept of...
   **Answer: [brief expected answer]**

Rules:
- Create 10 questions total (mix of types)
- Base questions only on the provided materials
- Include answers after each question
- Progress from easier to harder questions"#;
