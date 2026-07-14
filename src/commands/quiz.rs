//! Quiz question model and parser shared with the TUI Quiz pane.
//!
//! The interactive quiz loop moved to the ratatui TUI (`src/tui/`) at E3
//! Phase 3. These pure types/parsers turn generated quiz text into structured
//! questions and are reused by `tui::service::quiz`.

/// Question types parsed from quiz output
pub enum QuizQuestion {
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

pub fn parse_quiz_questions(text: &str) -> Vec<QuizQuestion> {
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
