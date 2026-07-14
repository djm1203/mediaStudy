//! Worker dispatch (blueprint §1, §3).
//!
//! [`dispatch`] is called from the render loop's `action_rx` arm. It spawns a
//! detached `tokio` task per action, forwards result [`Message`]s over `msg_tx`,
//! and — no matter what — sends exactly one terminal [`Message::ActionDone`] so
//! the loop can decrement its in-flight counter. It never touches `App`.
//!
//! Every [`Action`] has an arm here that calls a per-pane `service::<pane>::*`
//! function. Pane agents implement the *body* of their service function only;
//! this dispatch table is frozen for Phase 2.

use tokio::sync::mpsc::UnboundedSender;

use super::action::{Action, Message, ToastLevel};
use super::service;

/// Spawn the work for one action. Returns immediately; results arrive on `msg_tx`.
pub fn dispatch(action: Action, msg_tx: UnboundedSender<Message>) {
    tokio::spawn(async move {
        run(action, &msg_tx).await;
        let _ = msg_tx.send(Message::ActionDone);
    });
}

async fn run(action: Action, msg_tx: &UnboundedSender<Message>) {
    match action {
        // ---- global / library ----
        Action::LoadLibrary => match service::load_library().await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Failed to load library: {e}")),
        },

        Action::SwitchBucket(name) => match service::switch_bucket(name.clone()).await {
            Ok(msg) => {
                send(msg_tx, msg);
                send(
                    msg_tx,
                    Message::Toast {
                        level: ToastLevel::Success,
                        text: format!("Switched to '{name}'"),
                    },
                );
            }
            Err(e) => toast_err(msg_tx, format!("Failed to switch bucket: {e}")),
        },

        // ---- chat ----
        Action::LoadConversations => match service::load_conversations().await {
            Ok(list) => send(msg_tx, Message::ConversationsLoaded(list)),
            Err(e) => toast_err(msg_tx, format!("Failed to load conversations: {e}")),
        },

        Action::SendChat {
            conversation_id,
            history,
            question,
            is_first,
        } => {
            if let Err(e) =
                service::run_chat(conversation_id, history, question, is_first, msg_tx.clone())
                    .await
            {
                let _ = msg_tx.send(Message::ChatError(e.to_string()));
            }
        }

        // ---- search ----
        Action::RunSearch { query } => match service::search::run_search(query).await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Search failed: {e}")),
        },

        // ---- docs ----
        Action::LoadDocs => match service::docs::load_docs().await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Failed to load documents: {e}")),
        },
        Action::LoadDoc { id } => match service::docs::load_doc(id).await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Failed to load document: {e}")),
        },
        Action::DeleteDoc { id } => match service::docs::delete_doc(id).await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Failed to delete document: {e}")),
        },

        // ---- add ----
        Action::StartIngest { source, is_url } => {
            if let Err(e) = service::add::start_ingest(source, is_url, msg_tx.clone()).await {
                toast_err(msg_tx, format!("Ingest failed: {e}"));
            }
        }

        // ---- study ----
        Action::GenerateStudy {
            kind,
            topic,
            save_items,
        } => {
            if let Err(e) =
                service::study::generate_study(kind, topic, save_items, msg_tx.clone()).await
            {
                toast_err(msg_tx, format!("Study generation failed: {e}"));
            }
        }

        // ---- quiz ----
        Action::StartQuiz { count } => match service::quiz::start_quiz(count).await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Quiz failed: {e}")),
        },
        Action::GradeQuiz { id, quality } => match service::quiz::grade_quiz(id, quality).await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Grading failed: {e}")),
        },

        // ---- review ----
        Action::LoadDue => match service::review::load_due().await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Failed to load due items: {e}")),
        },
        Action::GradeReview { id, quality } => {
            match service::review::grade_review(id, quality).await {
                Ok(msg) => send(msg_tx, msg),
                Err(e) => toast_err(msg_tx, format!("Grading failed: {e}")),
            }
        }

        // ---- config ----
        Action::LoadConfig => match service::config::load_config().await {
            Ok(msg) => send(msg_tx, msg),
            Err(e) => toast_err(msg_tx, format!("Failed to load config: {e}")),
        },
        Action::SaveConfig { api_key, model } => {
            match service::config::save_config(api_key, model).await {
                Ok(msg) => send(msg_tx, msg),
                Err(e) => toast_err(msg_tx, format!("Failed to save config: {e}")),
            }
        }
    }
}

fn send(msg_tx: &UnboundedSender<Message>, msg: Message) {
    let _ = msg_tx.send(msg);
}

fn toast_err(msg_tx: &UnboundedSender<Message>, text: String) {
    let _ = msg_tx.send(Message::Toast {
        level: ToastLevel::Error,
        text,
    });
}
