//! Quiz pane service (blueprint §4).
//!
//! Generates questions from the current bucket's materials
//! (`get_document_context_pub` → `GroqClient::chat` → `parse_quiz_questions`),
//! persists them as `study_items` so they gain real ids and feed spaced
//! repetition, and grades answers with `StudyStore::update_after_review` (SM-2).
//!
//! All `rusqlite` work runs inside `tokio::task::spawn_blocking` closures that
//! each open their own [`Database`] — never held across an `.await`.

use anyhow::{Result, anyhow};
use tokio::task;

use crate::commands::generate::get_document_context_pub;
use crate::commands::quiz::{QuizQuestion, parse_quiz_questions};
use crate::config::Config;
use crate::llm::GroqClient;
use crate::llm::groq::Message as LlmMessage;
use crate::storage::{Database, StudyStore};
use crate::tui::action::{Message, StudyCard};

/// System prompt mirroring `commands/quiz.rs`, parameterised on question count.
fn quiz_system_prompt(count: usize) -> String {
    format!(
        r#"You are creating a practice quiz from the provided course materials.

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
- Create exactly {count} questions total (mix of types)
- Base questions only on the provided materials
- Include answers after each question
- Progress from easier to harder questions"#
    )
}

/// Generate a quiz of `count` questions as [`Message::QuizGenerated`].
///
/// Builds document context off-thread, prompts the model (non-streaming),
/// parses the response into questions, persists them as study items, and returns
/// the resulting [`StudyCard`]s.
pub async fn start_quiz(count: usize) -> Result<Message> {
    let count = count.clamp(1, 50);

    let config = Config::load()?;
    let api_key = config
        .get_api_key()
        .ok_or_else(|| anyhow!("No API key configured. Run `librarian config` to set one."))?;
    let client = GroqClient::new(api_key, config.default_model);

    // 1. Build document context on a blocking thread (opens its own DB). No
    //    topic filter — quiz over all of the current bucket's materials.
    let context = task::spawn_blocking(|| get_document_context_pub("")).await??;
    if context.is_empty() {
        return Err(anyhow!("No documents found. Add materials first."));
    }

    // 2. Prompt the model (non-streaming).
    let messages = vec![
        LlmMessage {
            role: "system".to_string(),
            content: quiz_system_prompt(count),
        },
        LlmMessage {
            role: "user".to_string(),
            content: format!(
                "Create an interactive quiz from these materials:\n\n{context}\n\nCover the most important topics."
            ),
        },
    ];
    let response = client.chat(&messages).await?;

    // 3. Parse questions from the model output.
    let questions = parse_quiz_questions(&response);
    if questions.is_empty() {
        return Err(anyhow!(
            "Could not parse any quiz questions from the model response."
        ));
    }

    // 4. Persist as study items (real ids → grading feeds spaced repetition) and
    //    build the cards that cross the message boundary.
    let cards = task::spawn_blocking(move || persist_questions(&questions, count)).await??;
    if cards.is_empty() {
        return Err(anyhow!("The generated quiz produced no usable questions."));
    }

    Ok(Message::QuizGenerated(cards))
}

/// Grade a quiz answer (SM-2 quality 0–5) as [`Message::QuizGraded`].
pub async fn grade_quiz(id: i64, quality: u8) -> Result<Message> {
    task::spawn_blocking(move || {
        let db = Database::open()?;
        let store = StudyStore::new(&db);
        store.update_after_review(id, quality)?;
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(Message::QuizGraded { id })
}

/// Persist parsed questions as `study_items` and return them as [`StudyCard`]s
/// (front = question + any options; back = answer/explanation). Runs on a
/// blocking thread; opens its own [`Database`].
fn persist_questions(questions: &[QuizQuestion], count: usize) -> Result<Vec<StudyCard>> {
    let db = Database::open()?;
    let store = StudyStore::new(&db);

    let mut cards = Vec::new();
    for q in questions.iter().take(count) {
        let (item_type, front, back) = match q {
            QuizQuestion::MultipleChoice {
                question,
                options,
                correct,
            } => {
                let mut front = question.clone();
                for (letter, text) in options {
                    front.push_str(&format!("\n  {letter}) {text}"));
                }
                let answer = options
                    .iter()
                    .find(|(l, _)| l == correct)
                    .map(|(l, t)| format!("{l}) {t}"))
                    .unwrap_or_else(|| correct.to_string());
                ("quiz_mc", front, answer)
            }
            QuizQuestion::FillInBlank { question, answer } => {
                ("quiz_fill", question.clone(), answer.clone())
            }
            QuizQuestion::ShortAnswer { question, expected } => {
                ("quiz_short", question.clone(), expected.clone())
            }
        };

        let id = store.insert(None, item_type, &front, &back)?;
        cards.push(StudyCard {
            id,
            item_type: item_type.to_string(),
            front,
            back,
        });
    }

    Ok(cards)
}
