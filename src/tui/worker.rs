//! Worker dispatch (blueprint §1, §3).
//!
//! [`dispatch`] is called from the render loop's `action_rx` arm. It spawns a
//! detached `tokio` task per action, forwards result [`Message`]s over `msg_tx`,
//! and — no matter what — sends exactly one terminal [`Message::ActionDone`] so
//! the loop can decrement its in-flight counter. It never touches `App`.

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
        Action::LoadLibrary => match service::load_library().await {
            Ok(msg) => {
                let _ = msg_tx.send(msg);
            }
            Err(e) => toast_err(msg_tx, format!("Failed to load library: {e}")),
        },

        Action::SwitchBucket(name) => match service::switch_bucket(name.clone()).await {
            Ok(msg) => {
                let _ = msg_tx.send(msg);
                let _ = msg_tx.send(Message::Toast {
                    level: ToastLevel::Success,
                    text: format!("Switched to '{name}'"),
                });
            }
            Err(e) => toast_err(msg_tx, format!("Failed to switch bucket: {e}")),
        },

        Action::LoadConversations => match service::load_conversations().await {
            Ok(list) => {
                let _ = msg_tx.send(Message::ConversationsLoaded(list));
            }
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
    }
}

fn toast_err(msg_tx: &UnboundedSender<Message>, text: String) {
    let _ = msg_tx.send(Message::Toast {
        level: ToastLevel::Error,
        text,
    });
}
