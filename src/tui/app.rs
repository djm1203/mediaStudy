//! Application state + pure reducers (blueprint §2).
//!
//! [`App`] owns all UI state, including one `…State` value per screen.
//! `handle_event` translates a crossterm event into an optional [`Action`]
//! (side-effect intent) while mutating local UI state; `update` folds a worker
//! [`Message`] into state; `on_tick` drives toast expiry and the spinner. No
//! network or DB work happens here — that is the worker's job, reached via
//! `action_tx` or the `Action` returned from `handle_event`.
//!
//! Home + Chat are handled bespoke; the seven Phase-2 panes (Search, Docs, Add,
//! Study, Quiz, Review, Config) are driven **generically** through the
//! [`Pane`] trait, so a pane agent only touches its own `ui/<pane>.rs`.

use std::time::Instant;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::llm::groq::Message as LlmMessage;

use super::action::{Action, ConversationMeta, Message, ToastLevel};
use super::service;
use super::theme::Theme;
use super::ui::{
    AddState, ConfigState, Ctx, DocsState, Pane, QuizState, ReviewState, SearchState, StudyState,
};

const TOAST_TTL: std::time::Duration = std::time::Duration::from_secs(4);
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Chat,
    Search,
    Docs,
    Add,
    Study,
    Quiz,
    Review,
    Config,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Screen::Home => "Home",
            Screen::Chat => "Chat",
            Screen::Search => "Search",
            Screen::Docs => "Docs",
            Screen::Add => "Add",
            Screen::Study => "Study",
            Screen::Quiz => "Quiz",
            Screen::Review => "Review",
            Screen::Config => "Config",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Help,
}

/// A transient status message with an expiry.
#[derive(Debug, Clone)]
pub struct Toast {
    pub level: ToastLevel,
    pub text: String,
    pub created: Instant,
}

/// Chat screen sub-state.
#[derive(Debug, Default)]
pub struct ChatState {
    pub conversation_id: Option<i64>,
    pub conversations: Vec<ConversationMeta>,
    /// Full message list: system prompt at [0], then plain user/assistant turns.
    pub history: Vec<LlmMessage>,
    pub streaming: bool,
    pub stream_buf: String,
    pub input: String,
    /// Transcript scroll-back in wrapped lines from the bottom (0 = follow tail).
    pub scroll: u16,
}

/// Outcome of testing a key against the global keymap.
enum KeyResult {
    /// The key was a global binding; carries an optional [`Action`] to dispatch.
    Consumed(Option<Action>),
    /// Not a global key — the active screen should handle it.
    Pass,
}

pub struct App {
    pub screen: Screen,
    pub focus: Focus,
    pub input_mode: InputMode,
    pub overlay: Overlay,
    pub should_quit: bool,

    // Library / global.
    pub buckets: Vec<String>,
    pub current_bucket: Option<String>,
    pub sidebar_sel: usize,
    pub doc_count: i64,
    pub chunk_count: i64,
    pub has_api_key: bool,
    pub model: String,

    // Per-screen sub-state.
    pub chat: ChatState,
    pub search: SearchState,
    pub docs: DocsState,
    pub add: AddState,
    pub study: StudyState,
    pub quiz: QuizState,
    pub review: ReviewState,
    pub config: ConfigState,

    // Cross-cutting.
    pub status: Option<Toast>,
    pub inflight: u32,
    pub spinner: usize,
    pub theme: Theme,
    pub action_tx: UnboundedSender<Action>,
}

impl App {
    pub fn new(action_tx: UnboundedSender<Action>, screen: Screen) -> Self {
        Self {
            screen,
            focus: Focus::Main,
            input_mode: InputMode::Normal,
            overlay: Overlay::None,
            should_quit: false,
            buckets: Vec::new(),
            current_bucket: None,
            sidebar_sel: 0,
            doc_count: 0,
            chunk_count: 0,
            has_api_key: false,
            model: String::new(),
            chat: ChatState::default(),
            search: SearchState::new(),
            docs: DocsState::new(),
            add: AddState::new(),
            study: StudyState::new(),
            quiz: QuizState::new(),
            review: ReviewState::new(),
            config: ConfigState::new(),
            status: None,
            inflight: 0,
            spinner: 0,
            theme: Theme::default(),
            action_tx,
        }
    }

    fn set_toast(&mut self, level: ToastLevel, text: impl Into<String>) {
        self.status = Some(Toast {
            level,
            text: text.into(),
            created: Instant::now(),
        });
    }

    /// Build the read-only context handed to a pane's `on_key`.
    fn ctx(&self) -> Ctx {
        Ctx {
            current_bucket: self.current_bucket.clone(),
            has_api_key: self.has_api_key,
            model: self.model.clone(),
            doc_count: self.doc_count,
            chunk_count: self.chunk_count,
            action_tx: self.action_tx.clone(),
        }
    }

    /// The active screen as a [`Pane`], for the seven generic panes only
    /// (Home + Chat are bespoke and return `None`).
    pub fn active_pane(&self) -> Option<&dyn Pane> {
        match self.screen {
            Screen::Search => Some(&self.search),
            Screen::Docs => Some(&self.docs),
            Screen::Add => Some(&self.add),
            Screen::Study => Some(&self.study),
            Screen::Quiz => Some(&self.quiz),
            Screen::Review => Some(&self.review),
            Screen::Config => Some(&self.config),
            _ => None,
        }
    }

    fn active_pane_mut(&mut self) -> Option<&mut dyn Pane> {
        match self.screen {
            Screen::Search => Some(&mut self.search),
            Screen::Docs => Some(&mut self.docs),
            Screen::Add => Some(&mut self.add),
            Screen::Study => Some(&mut self.study),
            Screen::Quiz => Some(&mut self.quiz),
            Screen::Review => Some(&mut self.review),
            Screen::Config => Some(&mut self.config),
            _ => None,
        }
    }

    /// Whether the active screen is currently capturing free text (so global
    /// nav keys must be suppressed and every key routed to the screen).
    fn capturing_input(&self) -> bool {
        match self.screen {
            Screen::Chat => self.input_mode == InputMode::Editing,
            Screen::Home => false,
            _ => self.active_pane().map(|p| p.wants_input()).unwrap_or(false),
        }
    }

    // ---- event handling -------------------------------------------------

    /// Translate an event into an optional [`Action`], mutating local UI state.
    pub fn handle_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key),
            _ => None,
        }
    }

    fn on_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl-C always quits, even mid-compose.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return None;
        }

        // Overlay swallows input.
        if self.overlay == Overlay::Help {
            if matches!(
                key.code,
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q')
            ) {
                self.overlay = Overlay::None;
            }
            return None;
        }

        // Unless the active screen is capturing text, global keys win first.
        if !self.capturing_input() {
            match self.handle_global_key(key) {
                KeyResult::Consumed(action) => return action,
                KeyResult::Pass => {}
            }
            if self.focus == Focus::Sidebar {
                return self.on_key_sidebar(key);
            }
        }

        // Delegate to the active screen.
        match self.screen {
            Screen::Home => None,
            Screen::Chat => self.on_key_chat(key),
            _ => {
                let ctx = self.ctx();
                self.active_pane_mut().and_then(|p| p.on_key(key, &ctx))
            }
        }
    }

    /// Resolve a key against the always-on global keymap.
    fn handle_global_key(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.theme.toggle();
                KeyResult::Consumed(None)
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
                KeyResult::Consumed(None)
            }
            KeyCode::Char('?') => {
                self.overlay = Overlay::Help;
                KeyResult::Consumed(None)
            }
            KeyCode::Tab => {
                self.cycle_focus();
                KeyResult::Consumed(None)
            }
            KeyCode::Esc => {
                if self.screen != Screen::Home {
                    self.goto(Screen::Home);
                }
                KeyResult::Consumed(None)
            }
            KeyCode::Char(c @ '1'..='9') => KeyResult::Consumed(self.nav_to(c)),
            _ => KeyResult::Pass,
        }
    }

    /// Map a number key to its screen and switch to it.
    fn nav_to(&mut self, c: char) -> Option<Action> {
        let screen = match c {
            '1' => Screen::Home,
            '2' => Screen::Chat,
            '3' => Screen::Search,
            '4' => Screen::Docs,
            '5' => Screen::Add,
            '6' => Screen::Study,
            '7' => Screen::Quiz,
            '8' => Screen::Review,
            '9' => Screen::Config,
            _ => return None,
        };
        self.goto(screen)
    }

    /// Switch to `screen`, resetting focus/input, and return the action that
    /// screen wants fired on entry (e.g. loading its data), if any.
    fn goto(&mut self, screen: Screen) -> Option<Action> {
        self.screen = screen;
        self.focus = Focus::Main;
        self.input_mode = InputMode::Normal;
        match screen {
            Screen::Chat => Some(Action::LoadConversations),
            Screen::Docs => Some(Action::LoadDocs),
            Screen::Review => Some(Action::LoadDue),
            Screen::Config => Some(Action::LoadConfig),
            _ => None,
        }
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sidebar => Focus::Main,
            Focus::Main | Focus::Input => Focus::Sidebar,
        };
    }

    fn on_key_sidebar(&mut self, key: KeyEvent) -> Option<Action> {
        let len = self.buckets.len();
        if len == 0 {
            return None;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.sidebar_sel = (self.sidebar_sel + 1) % len;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sidebar_sel = (self.sidebar_sel + len - 1) % len;
            }
            KeyCode::Enter => {
                if let Some(name) = self.buckets.get(self.sidebar_sel) {
                    return Some(Action::SwitchBucket(name.clone()));
                }
            }
            _ => {}
        }
        None
    }

    // ---- chat screen ----------------------------------------------------

    fn on_key_chat(&mut self, key: KeyEvent) -> Option<Action> {
        if self.input_mode == InputMode::Editing {
            self.on_key_editing(key)
        } else {
            self.on_key_chat_normal(key)
        }
    }

    fn on_key_chat_normal(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('i') | KeyCode::Enter => {
                self.start_editing();
            }
            KeyCode::Char('n') => {
                self.new_conversation();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.chat.scroll = self.chat.scroll.saturating_sub(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.chat.scroll = self.chat.scroll.saturating_add(1);
            }
            _ => {}
        }
        None
    }

    fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing;
        self.focus = Focus::Input;
    }

    fn new_conversation(&mut self) {
        self.chat = ChatState::default();
        self.set_toast(ToastLevel::Info, "Started a new conversation");
    }

    fn on_key_editing(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.focus = Focus::Main;
                None
            }
            KeyCode::Enter => self.submit_chat(),
            KeyCode::Backspace => {
                self.chat.input.pop();
                None
            }
            KeyCode::Char(c) => {
                self.chat.input.push(c);
                None
            }
            _ => None,
        }
    }

    fn submit_chat(&mut self) -> Option<Action> {
        let text = self.chat.input.trim().to_string();
        if text.is_empty() || self.chat.streaming {
            return None;
        }

        if !self.has_api_key {
            self.set_toast(
                ToastLevel::Warn,
                "No API key configured. Run `librarian config` to set one.",
            );
            return None;
        }

        let is_first = !self.chat.history.iter().any(|m| m.role == "user");

        // Seed the system prompt for a brand-new conversation.
        if self.chat.history.is_empty() {
            let prompt = service::system_prompt(self.doc_count > 0);
            self.chat.history.push(LlmMessage {
                role: "system".to_string(),
                content: prompt.to_string(),
            });
        }

        // Record the plain user turn (for display + as the last message the
        // worker will inject context into).
        self.chat.history.push(LlmMessage {
            role: "user".to_string(),
            content: text.clone(),
        });

        self.chat.input.clear();
        self.chat.stream_buf.clear();
        self.chat.streaming = true;
        self.chat.scroll = 0;

        Some(Action::SendChat {
            conversation_id: self.chat.conversation_id,
            history: self.chat.history.clone(),
            question: text,
            is_first,
        })
    }

    // ---- reducer --------------------------------------------------------

    /// Fold a worker message into state. Pane-addressed results are routed to
    /// the owning pane's [`Pane::apply`] regardless of the active screen.
    pub fn update(&mut self, msg: Message) {
        match msg {
            // ---- global / library ----
            Message::LibraryLoaded {
                buckets,
                current,
                doc_count,
                chunk_count,
                has_api_key,
                model,
            } => {
                self.buckets = buckets;
                self.current_bucket = current;
                self.doc_count = doc_count;
                self.chunk_count = chunk_count;
                self.has_api_key = has_api_key;
                self.model = model;
                if self.sidebar_sel >= self.buckets.len() {
                    self.sidebar_sel = self.buckets.len().saturating_sub(1);
                }
                if !self.has_api_key {
                    self.set_toast(
                        ToastLevel::Warn,
                        "No API key configured — run `librarian config`.",
                    );
                }
            }

            // ---- chat ----
            Message::ConversationsLoaded(list) => {
                self.chat.conversations = list;
            }
            Message::ChatToken(token) => {
                self.chat.stream_buf.push_str(&token);
                self.chat.scroll = 0;
            }
            Message::ChatDone {
                conversation_id,
                response,
            } => {
                self.chat.conversation_id = Some(conversation_id);
                self.chat.history.push(LlmMessage {
                    role: "assistant".to_string(),
                    content: response,
                });
                self.chat.stream_buf.clear();
                self.chat.streaming = false;
                self.chat.scroll = 0;
                // Refresh the recent-conversation list (title may have changed).
                let _ = self.action_tx.send(Action::LoadConversations);
            }
            Message::ChatError(text) => {
                self.chat.streaming = false;
                self.chat.stream_buf.clear();
                // Drop the dangling user turn so history stays consistent.
                if self.chat.history.last().is_some_and(|m| m.role == "user") {
                    self.chat.history.pop();
                }
                self.set_toast(ToastLevel::Error, text);
            }

            // ---- pane-addressed results ----
            Message::SearchResults { .. } => self.search.apply(&msg),
            Message::DocsLoaded(_) | Message::DocLoaded(_) | Message::DocDeleted { .. } => {
                self.docs.apply(&msg)
            }
            Message::IngestProgress { .. }
            | Message::IngestFileDone { .. }
            | Message::IngestComplete { .. } => self.add.apply(&msg),
            Message::StudyToken(_) | Message::StudyDone { .. } => self.study.apply(&msg),
            Message::QuizGenerated(_) | Message::QuizGraded { .. } => self.quiz.apply(&msg),
            Message::DueLoaded(_) | Message::ReviewGraded { .. } => self.review.apply(&msg),
            Message::ConfigLoaded(_) | Message::ConfigSaved => self.config.apply(&msg),

            // ---- cross-cutting ----
            Message::Toast { level, text } => {
                self.set_toast(level, text);
            }
            Message::ActionDone => {
                self.inflight = self.inflight.saturating_sub(1);
            }
        }
    }

    // ---- tick -----------------------------------------------------------

    pub fn on_tick(&mut self) {
        if self.inflight > 0 {
            self.spinner = (self.spinner + 1) % SPINNER_FRAMES.len();
        }
        if let Some(toast) = &self.status
            && toast.created.elapsed() > TOAST_TTL
        {
            self.status = None;
        }
    }

    /// Current spinner glyph (empty when idle).
    pub fn spinner_glyph(&self) -> &'static str {
        if self.inflight > 0 {
            SPINNER_FRAMES[self.spinner]
        } else {
            ""
        }
    }
}
