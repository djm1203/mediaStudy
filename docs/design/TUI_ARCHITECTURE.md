---
title: "TUI ARCHITECTURE (E3 blueprint)"
project: project
classification: high
created: 2026-07-13T00:00:00Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Design
author: Derek Martinez
---

# The Librarian — Ratatui TUI blueprint (Epic E3, full replacement of the `inquire` menu)

> This is the implementation blueprint for B-017/B-018. It is the source of truth for the TUI
> rebuild; implementation agents build against it. Grounded in the actual source (`src/main.rs`,
> `src/commands/*`, `src/llm/groq.rs`, `src/storage/*`, `src/embeddings/mod.rs`, `src/search.rs`).

## 0. Load-bearing constraints (read first)

1. **The existing `commands::*::run()` orchestrators cannot be called from inside the TUI.** They
   `println!`/`print!` directly (banners, `colored`, `indicatif`) and call `inquire` prompts that
   seize the terminal. `GroqClient::chat_stream` (`src/llm/groq.rs:155-212`) prints each token to
   stdout. The TUI must call the **lower-level services directly** (storage stores, `GroqClient`,
   `embeddings`, `ingest`) and reuse only the *pure* helpers — never the interactive `run()` wrappers.
2. **A channel-streaming chat method does not exist yet and is the critical path.** Add
   `chat_stream_tx` beside `chat_stream`: same Groq SSE parse loop (`groq.rs:183-206`) but forwards
   each `delta.content` over an `mpsc::Sender` instead of printing.
3. **Private helpers to expose (`pub`, no behavior change):** chat.rs `build_semantic_context`,
   `build_fts_context`, `truncate_content`; generate.rs `mod prompts`, `parse_qa_pairs`
   (`get_document_context_pub` already exists); quiz.rs `QuizQuestion` + `parse_quiz_questions`.
   `search::enhance_query` / `search::deduplicate_chunks` are already `pub`.
4. **`rusqlite::Connection` is not `Sync`; `Database` is opened per-operation; `embeddings::embed_text`
   is synchronous + global-`Mutex` guarded.** All DB and embedding work must run inside
   `tokio::task::spawn_blocking`, opening its own `Database::open()` in the closure — never held across
   `.await`, never shared between the render loop and a worker.
5. **Subcommands already split "arg given (headless)" vs "arg None (prompt)"** — a clean migration seam.

## 1. Module layout — `src/tui/`

```
src/tui/
  mod.rs      Entry: run() / run_on(Screen). Terminal enter/leave (raw + alt screen + panic hook), owns the loop.
  app.rs      App state + Screen/Focus/InputMode enums + pure reducers update()/on_key().
  action.rs   Action enum (intents -> worker) + Message enum (results -> loop). Freeze early.
  worker.rs   dispatch(action, msg_tx): spawns tokio tasks per Action, streams Message back. No App ref.
  service.rs  TUI-facing service layer over storage/llm/ingest. Home of chat_stream_tx + spawn_blocking DB helpers.
  event.rs    crossterm EventStream -> key/mouse/resize; tick timer.
  theme.rs    Palette + Theme (dark/light + accent), Style helpers.
  keymap.rs   Global + per-screen key tables; help metadata.
  markdown.rs Markdown -> ratatui Text (chat/study).
  ui/
    mod.rs      draw(frame, app): sidebar | main | status layout + overlays.
    sidebar.rs  Bucket list (replaces print_library_shelf).
    statusbar.rs Status bar + toast + spinner.
    home.rs     Dashboard/home (replaces print_dashboard + main menu).
    chat.rs search.rs docs.rs add.rs study.rs quiz.rs review.rs config.rs help.rs
    widgets.rs  Shared: input (tui-textarea wrapper), list state, confirm/prompt modal, spinner, scrollbar.
```

Serial core: `app.rs`/`action.rs`/`worker.rs`/`service.rs`/`ui/mod.rs`. Each `ui/<screen>.rs` is an independent parallel unit.

## 2. App state model

```rust
pub enum Screen { Home, Chat, Search, Docs, Add, Study, Quiz, Review, Config }
pub enum Focus { Sidebar, Main, Input }
pub enum InputMode { Normal, Editing }
pub enum Overlay { None, Help, Confirm(ConfirmState), Prompt(PromptState) }

pub struct App {
    screen: Screen, focus: Focus, input_mode: InputMode, overlay: Overlay, should_quit: bool,
    // library/global
    buckets: Vec<String>, current_bucket: Option<String>, sidebar_sel: usize,
    doc_count: i64, chunk_count: i64, has_api_key: bool, model: String,
    // per-screen sub-state structs
    chat: ChatState, search: SearchState, docs: DocsState, add: AddState,
    study: StudyState, quiz: QuizState, review: ReviewState, config: ConfigState,
    // cross-cutting
    status: Option<Toast>, inflight: u32, theme: Theme, action_tx: UnboundedSender<Action>,
}
```

`ChatState { conversation_id, conversations, history: Vec<groq::Message>, streaming, stream_buf, input: TextArea, scroll }`.
`AddState { source_kind, input, running, total, done, current_file, log, summary }`. Others analogous.

**Nav/focus:** Tab/Shift-Tab cycle Focus; number keys or `g`+letter choose Screen; Esc closes overlay → leaves Editing → returns Home → quit-confirm. Sidebar is a persistent global control.

## 3. Event + render loop (async, non-blocking)

Two `mpsc` channels: `action_tx/rx` (panes→worker), `msg_tx/rx` (worker→loop). Render loop never awaits network/DB.

```rust
while !app.should_quit {
    terminal.draw(|f| ui::draw(f, &app))?;
    tokio::select! {
        Some(Ok(ev)) = events.next() => if let Some(a) = app.handle_event(ev) { action_tx.send(a); }
        Some(msg) = msg_rx.recv()     => app.update(msg),
        Some(action) = action_rx.recv() => { app.inflight += 1; worker::dispatch(action, msg_tx.clone()); }
        _ = tick.tick()               => app.on_tick(),
    }
}
```

Worker spawns tokio tasks; DB/embedding inside `spawn_blocking`. `chat_stream_tx` sends `ChatToken(..)` per delta then `ChatDone(full)`. `App::update` appends tokens to `chat.stream_buf`; on `ChatDone` pushes the assistant message and clears the buffer. Long tasks (chat, generation, quiz gen, ingest, transcription, every embedding pass) run in spawned tasks; ingest emits `IngestProgress`/`IngestFileDone` incrementally.

## 4. Screens (calls grounded in source)

| Screen | Calls into | Replaces inquire at |
|---|---|---|
| Home | `Bucket::list_all`, `bucket::get_current_bucket`, `DocumentStore::count`, `ChunkStore::count`, `Config::has_api_key` | main.rs run_interactive menu |
| Chat | `ConversationStore::{list_recent,create,get_messages,update_title,add_message}`, `search::enhance_query`, `build_semantic_context`/`build_fts_context`, `embeddings::{embed_text,find_similar}`, `ChunkStore::{get_all_with_embeddings,search_content}`, new `chat_stream_tx` | chat.rs:167,286 |
| Search | `DocumentStore::search` | docs.rs search Text |
| Docs | `DocumentStore::{list,get,delete}` (+ confirm modal) | docs.rs:41,115,154,210,229 |
| Add | `ingest::{requires_transcription,extract_from_file_async,fetch_url}`, `chunk_text`/`ChunkConfig`, `embeddings::embed_text`, `DocumentStore::{exists_by_path,insert}`, `ChunkStore::{init_schema,insert}` (reimpl add.rs:110-425 loop emitting progress) | add.rs:47-65 |
| Study | `prompts::*`, `get_document_context_pub`, new `generate_stream_tx`, markdown render, `fs::write` + `ingest_generated_content`, `StudyStore::bulk_insert`, `parse_qa_pairs` | generate.rs menu/topic/save |
| Quiz | `StudyStore::{count_due,get_due,update_after_review,bulk_insert}`, `QuizQuestion`+`parse_quiz_questions`, `get_document_context_pub`, `GroqClient::chat` | quiz.rs prompts |
| Review | `StudyStore::{count_due,get_due,update_after_review}` (SM-2) | review.rs:65,81 |
| Config | `Config::{load,save,has_api_key}`, `GroqClient::MODELS` | config.rs:42,86,109 |
| Help | keymap tables | (new overlay) |

## 5. Keybindings + theme

`keymap.rs`: ordered `(KeyChord, ActionKind, help_label)`; resolution overlay → screen → global. Vim + arrows (`j/k`, `h/l`, `g`+letter, `/`, `?`, `q`/`Ctrl-C`, `Tab`), mouse (click→focus/select, wheel→scroll). Help overlay generated from the same tables. `theme.rs`: `Palette { bg, surface, fg, fg_dim, accent, accent_alt, ok, warn, err, border, border_focus }`, `Theme { mode, accent, p }` with `dark()`/`light()` + style helpers; `Ctrl-T` toggles mode, `Ctrl-A` cycles accent. Every widget draws through `app.theme`.

## 6. Migration (full replace, keep scripting)

- `main.rs:249` `None => run_interactive()` becomes `None => tui::run()`; delete `run_interactive`/`print_banner`/`print_library_shelf`/`print_dashboard`/`print_farewell`/`print_header` at parity.
- Arg-less interactive subcommands open the TUI on a screen: Chat→Chat, Docs→Docs, Bucket{None}→Home, Config→Config, Generate{None}→Study, Review→Review, Quiz→Quiz, Add{None}→Add, Search{None}→Search, Delete{None}→Docs.
- Arg-provided paths stay fully headless (no TUI/inquire): `add <path>`, `search <query>`, `delete <id>`, `bucket create/use/delete <name>`, `generate <kind> <topic>`, `list`, `completions`.
- **Headless inquire residuals to convert to defaults/flags:** docs.rs `delete()` confirm (delete without prompt / `--yes`); generate.rs save + study-items prompts (auto-save to bucket `generated/`, skip items or `--save-items`); bucket.rs `create()` "switch now?" (auto-switch / `--use`).
- **Remove `inquire`** after parity. Call sites: main.rs:404; add.rs:47-65; chat.rs:167,286; docs.rs:41,115,154,210,229; generate.rs:128,147/159/171/183,230,368,380,437; bucket.rs:45,104,119,189,213,227; config.rs:42,86,109; quiz.rs:68,102,168,244,268,290,333; review.rs:65,81.

## 7. Markdown rendering

Use **`tui-markdown`** (markdown → ratatui `Text`), wrapped behind `tui::markdown::render(md, &theme) -> Text`. Fallback: a `pulldown-cmark` mini-renderer behind the same signature if `tui-markdown` lags ratatui 0.30. `termimad` stays only for non-TUI subcommand output (it writes ANSI, can't render into a ratatui buffer). Render streaming `stream_buf` as plain wrapped text; re-render as markdown on `ChatDone`/`GenerateDone`.

## 8. Build sequence (parallelizable)

- **Phase 0 — Scaffold (serial, keep green):** add crates; `tui/mod.rs` terminal setup/teardown + panic-restore, empty `App`, placeholder draw, quit on `q`/`Ctrl-C`; wire `None => tui::run()`. Don't touch inquire yet.
- **Phase 1 — Core plumbing + Chat vertical slice (serial, CRITICAL PATH):** freeze `Action`/`Message`; `worker.rs`+`service.rs`+`chat_stream_tx`; loop/`event.rs`/`theme.rs`/`keymap.rs`/`ui/mod.rs`/`sidebar.rs`/`statusbar.rs`/`home.rs`; build the **Chat** screen end-to-end to validate streaming. Land the helper-visibility refactor (§0.3) before Phase 2.
- **Phase 2 — Panes in parallel (many agents):** Search / Docs / Add(ingest, highest effort) / Study / Quiz / Review / Config / (markdown+help+theme+keymap+shared `widgets.rs`). Land `widgets.rs` interfaces first.
- **Phase 3 — Cutover & inquire removal (serial):** route arg-less subcommands into `run_on`, convert headless residuals, delete `run_interactive`+helpers, remove `inquire`, delete call sites, parity pass + `verify`.

**Risk/serial flags:** async message plumbing (Phase 1) is the single critical path — do not parallelize; ingest progress plumbing is second-trickiest; ratatui 0.30 is new — verify `tui-textarea`/`tui-markdown` compatibility at Phase 0 with hand-rolled fallbacks ready.

## 9. Crate additions

| Crate | Version | Why |
|---|---|---|
| `ratatui` | 0.30 | TUI framework |
| `crossterm` | align with ratatui 0.30's bundled version (use `ratatui::crossterm` to avoid a duplicate); enable `event-stream` | backend + raw mode + async `EventStream` + mouse |
| `tui-textarea` | verify vs 0.30 | multi-line input; fallback: hand-rolled in `widgets.rs` |
| `tui-markdown` | verify vs 0.30 | markdown → `Text`; fallback: `pulldown-cmark` mini-renderer |
| `pulldown-cmark` | 0.12 (only if fallback) | markdown parser for fallback |

Reuse existing `tokio` (full) and `futures-util`. Do **not** add `ratatui-image`. Keep `termimad` for now. Use ratatui's built-in `Scrollbar`.
