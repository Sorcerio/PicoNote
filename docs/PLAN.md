# PicoNote — Implementation Plan

## Context

Modern OS-bundled notepad applications have become bloated with features most users never need. PicoNote is a super lightweight markdown notepad built in Rust with **egui/eframe**, focused on extreme simplicity and rapid function. The single distinguishing feature is **markdown syntax highlighting** — not WYSIWYG rendering, but visual styling of raw markdown syntax (headers in accent color + larger font, `*bold*` displayed bold, etc.).

---

## Tech Stack

| Component | Choice | Rationale |
|---|---|---|
| GUI framework | **eframe + egui** (glow backend) | Smallest binary, simplest API, cross-platform, built-in `TextEdit` with custom layouter support |
| File dialogs | **rfd** | Native OS dialogs, well-maintained |
| Config persistence | **confy** | Zero-boilerplate TOML config in OS-appropriate directories |
| Markdown parsing | **Custom (no dep)** | Flat byte-range spans needed for cursor positioning; simpler than adapting pulldown-cmark |
| Serialization | **serde** | Required by confy |

### Release Profile (target: <5 MB binary)

```toml
[profile.release]
opt-level = "s"
lto = true
strip = true
codegen-units = 1
panic = "abort"
```

---

## Final Project Structure

```
piconote/
├── Cargo.toml
├── README.md
├── LICENSE
├── .gitignore
├── docs/
│   └── PLAN.md
└── src/
    ├── main.rs          # Entry point, eframe::run_native
    ├── app.rs           # PicoNoteApp struct, eframe::App impl, UI layout
    ├── parser.rs        # Line-by-line markdown tokenizer → styled spans
    ├── highlighter.rs   # Memoized highlighter → egui LayoutJob
    ├── theme.rs         # Dark/Light color palettes
    ├── config.rs        # Preferences struct + confy load/store
    └── file_ops.rs      # File I/O + native dialogs via rfd
```

---

## Phase 1: Skeleton App — Window + Empty Text Area

**Objective**: Compiling eframe app with a resizable window and multiline `TextEdit`.

**Create**:
- `Cargo.toml` — dependencies: `eframe`, `egui` only
- `.gitignore` — standard Rust (`/target`)
- `src/main.rs` — `eframe::run_native` with 800x600 default, 400x300 min size
- `src/app.rs` — `PicoNoteApp` struct with `content: String`, `eframe::App` impl rendering a `TextEdit::multiline` inside `ScrollArea::vertical` in a `CentralPanel`

**Done when**: `cargo run` opens a resizable window with a full-width scrollable text area. You can type, select, and scroll.

---

## Phase 2: Menu Bar + File Operations

**Objective**: Add File menu (New, Open, Save, Save As, Quit) with native OS dialogs and dirty-state tracking.

**Create**:
- `src/file_ops.rs` — `FileState { path, dirty }`, `open_file_dialog()`, `save_file_dialog()`, `write_file()`

**Modify**:
- `Cargo.toml` — add `rfd`
- `src/app.rs` — add `FileState` field, `TopBottomPanel::top` with `menu::bar`, wire menu actions, track dirty state via `TextEditOutput.response.changed()`, update window title to `"PicoNote - filename.md*"`
- `src/main.rs` — add `mod file_ops`

**Done when**: File > Open shows native dialog, loads file into editor. Save/Save As write to disk. Title bar shows filename + dirty indicator `*`. Quit closes the window.

---

## Phase 3: Markdown Parser (Tokenizer)

**Objective**: Line-by-line markdown tokenizer producing styled spans. Can be built in parallel with Phase 2.

**Create**:
- `src/parser.rs` — types: `MdStyle` and `MdSpan { text, style }`. Function: `parse_markdown(input) -> Vec<MdSpan>`

**`MdStyle` fields**:
```rust
pub struct MdStyle {
    // Basic markdown
    pub heading_level: u8,     // 0 = none, 1-6 = heading level
    pub bold: bool,
    pub italic: bool,
    pub code: bool,            // inline `code`
    pub code_block: bool,      // fenced ``` or ~~~ blocks
    pub blockquote: bool,      // > quoted text
    pub list_bullet: bool,     // the - or * character
    pub link_text: bool,       // [text] part
    pub link_url: bool,        // (url) part

    // Extended markdown
    pub strikethrough: bool,   // ~~deleted~~
    pub task_checkbox: bool,   // - [ ] or - [x]
    pub task_checked: bool,    // specifically - [x] (checked)
    pub table_pipe: bool,      // | column separators
    pub table_align: bool,     // :--- / :---: / ---: alignment row
    pub footnote_ref: bool,    // [^id] reference
    pub footnote_def: bool,    // [^id]: definition
    pub highlight: bool,       // ==highlighted==
    pub heading_id: bool,      // {#custom-id} suffix
    pub emoji_shortcode: bool, // :emoji_name:

    // Meta
    pub syntax_marker: bool,   // the #, *, `, >, ~~, ==, |, etc. characters themselves
}
```

**Parsing logic** (state machine, no regex crate):

*Block-level (line start):*
1. Track `in_code_block` state across lines (toggled by ` ``` ` or `~~~` lines)
2. Headings: `# ` through `###### ` — also detect trailing `{#id}` as `heading_id`
3. Blockquotes: `> ` prefix
4. List items: `- ` or `* ` prefix
5. Task lists: `- [ ] ` (unchecked) or `- [x] ` (checked) — mark checkbox chars as `task_checkbox`
6. Table rows: lines containing `|` — mark `|` chars as `table_pipe`
7. Table alignment: lines matching `|?[\s:]*-{3,}[\s:]*(\|[\s:]*-{3,}[\s:]*)*\|?` — mark as `table_align`
8. Footnote definitions: `[^id]: ` at line start
9. Definition lists: `: ` at line start (term on previous line)

*Inline (within lines):*
1. `**bold**` — markers as `syntax_marker + bold`, inner as `bold`
2. `*italic*` — markers as `syntax_marker + italic`, inner as `italic`
3. `` `code` `` — backticks as `syntax_marker + code`, inner as `code`
4. `~~strikethrough~~` — tildes as `syntax_marker + strikethrough`, inner as `strikethrough`
5. `==highlight==` — equals as `syntax_marker + highlight`, inner as `highlight`
6. `[text](url)` — brackets as `syntax_marker`, text as `link_text`, parens as `syntax_marker`, url as `link_url`
7. `[^id]` — footnote reference as `footnote_ref`
8. `:emoji_name:` — colons + name as `emoji_shortcode`
9. Bare URLs (`https://...`) — detected and colored as `link_url`

**Critical invariant**: Concatenation of all `MdSpan.text` values must equal the original input byte-for-byte (required for egui cursor positioning).

**Unit tests**: All basic syntax (heading, bold, italic, code, code_block, blockquote, link) + all extended syntax (strikethrough, task list, table, footnote, highlight, emoji shortcode, heading ID, bare URL) + concatenation invariant.

**Done when**: `cargo test` passes all parser tests.

---

## Phase 4: Syntax Highlighting via Custom Layouter

**Objective**: Wire parser to egui's `TextEdit` via a custom `layouter` callback. This is the core feature.

**Create**:
- `src/highlighter.rs` — `MemoizedMarkdownHighlighter` (caches previous input/output to avoid re-parsing every frame at 60fps). `build_layout_job()` converts `Vec<MdSpan>` to egui `LayoutJob` via `job.append(text, 0.0, format)`. `md_style_to_text_format()` maps `MdStyle` to `TextFormat` (font size, color, background, italics, underline).

**Style mapping**:

*Basic:*
- Headers: accent color + scaled font size (h1=1.6x, h2=1.4x, h3=1.2x)
- Syntax markers: dimmed gray (all `#`, `*`, `` ` ``, `>`, `~~`, `==`, `|`, etc.)
- Code/code blocks: monospace + `code_bg_color` background
- Links: blue + underline
- Blockquotes: `weak_text_color()`
- Bold: `strong_text_color()` (true bold font is a future enhancement — egui `TextFormat` has no `bold` field)

*Extended:*
- Strikethrough (`~~text~~`): `strikethrough: Stroke` on `TextFormat` + dimmed color
- Highlight (`==text==`): warm background color (yellow-ish tint)
- Task checkboxes (`- [ ]` / `- [x]`): accent color; checked gets additional dimmed/green tint
- Table pipes (`|`): accent color; alignment row (`---`, `:---:`) dimmed
- Footnote refs (`[^id]`): small accent color, similar to links
- Footnote defs (`[^id]:`): accent color for the label portion
- Emoji shortcodes (`:name:`): subtle accent/warm color
- Heading IDs (`{#id}`): dimmed gray (informational, not prominent)
- Bare URLs: link blue + underline (same as explicit links)

**Modify**:
- `src/app.rs` — add `highlighter` field, pass as `layouter` closure to `TextEdit::multiline`
- `src/main.rs` — add `mod highlighter; mod parser`

**Done when**: Typing markdown shows real-time syntax highlighting. Cursor and selection still work correctly.

---

## Phase 5: Theme System + Preferences

**Objective**: Preferences menu with dark/light theme and font size, persisted via confy.

**Create**:
- `src/config.rs` — `Config { theme: ThemeChoice, font_size: f32, word_wrap: bool }` with `load_config()` / `save_config()` using confy
- `src/theme.rs` — `HighlightColors` struct, `colors_for_theme()`, `apply_theme()` (sets `egui::Visuals::dark()` or `light()`)

**Modify**:
- `Cargo.toml` — add `serde`, `confy`
- `src/highlighter.rs` — accept `HighlightColors` + `font_size` instead of hardcoded values; cache-bust on theme/size changes
- `src/app.rs` — add `Config` field, load at startup, add "Preferences" menu with theme radio buttons, font size slider, word wrap checkbox
- `src/main.rs` — add `mod config; mod theme`

**Config file location** (handled automatically by confy):
- macOS: `~/Library/Application Support/piconote/default-config.toml`
- Linux: `~/.config/piconote/default-config.toml`
- Windows: `%APPDATA%\piconote\default-config.toml`

**Done when**: Preferences menu works. Theme and font size changes are immediate and persist across restarts.

---

## Phase 6: Keyboard Shortcuts + Polish

**Objective**: Standard shortcuts, unsaved-changes dialog, status bar.

**Modify** (`src/app.rs`):

**Keyboard shortcuts** (via `ctx.input()`):
- `Cmd/Ctrl+S` — Save
- `Cmd/Ctrl+Shift+S` — Save As
- `Cmd/Ctrl+O` — Open
- `Cmd/Ctrl+N` — New
- `Cmd/Ctrl+Plus` / `Cmd/Ctrl+Minus` — Font size adjust

**Unsaved changes dialog**: When closing/new/open with dirty state, show egui `Window` modal with Save / Don't Save / Cancel. Handle `close_requested()` viewport event with `CancelClose`.

**Status bar**: `TopBottomPanel::bottom` showing line count, character count, file path.

**Menu polish**: Add `.shortcut_text("Ctrl+S")` to menu buttons. Refactor save/open/new into methods on `PicoNoteApp`.

**Done when**: All shortcuts work. Unsaved changes prompt on close. Status bar shows document stats. App feels like a real notepad.

---

## Phase 7: Build Optimization + Cross-Platform Testing

**Objective**: Minimize binary, verify cross-platform, optional CI.

**Tasks**:
- Finalize `Cargo.toml` release profile
- Audit dependency features — drop unused features (e.g., eframe `persistence`, `accesskit`)
- `cargo build --release` — verify binary <5 MB
- Test on macOS, Windows, Linux (native dialogs, config paths, font rendering)
- Optionally create `.github/workflows/ci.yml` with matrix build (ubuntu, macos, windows)
- Linux build deps: `libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev`

**Done when**: Release binary <5 MB on all platforms. CI green (if configured).

---

## Phase 8: Update README.md

**Objective**: Comprehensive README documenting the project.

**Create**:
- `README.md` — project description, features list, screenshots placeholder, installation (from source + pre-built), build instructions, architecture overview (one-line per source file), license
- `LICENSE` — MIT

**Done when**: README accurately describes all implemented features. LICENSE exists.

---

## Phase Dependency Graph

```
Phase 1 (Skeleton)
  ├──→ Phase 2 (File Ops) ──────┐
  └──→ Phase 3 (Parser) ──┐     │
                           ↓     │
                     Phase 4 (Highlighter)
                           ↓
                     Phase 5 (Themes/Prefs)
                           ↓
                     Phase 6 (Shortcuts + Polish)
                           ↓
                     Phase 7 (Build + Cross-Platform)
                           ↓
                     Phase 8 (README)
```

Phases 2 and 3 can be worked in parallel.

---

## Verification

After each phase, verify by running the app (`cargo run`) and testing the new functionality. Specific checks:

- **Phase 3**: `cargo test` — all parser unit tests pass, concatenation invariant holds
- **Phase 4**: Type various markdown in the editor — headers, bold, italic, code blocks, links, `~~strikethrough~~`, `==highlight==`, tables with `|`, task lists `- [x]`, footnotes `[^1]`, `:emoji:` shortcodes — and confirm visual styling appears correctly with working cursor
- **Phase 7**: `cargo build --release && ls -lh target/release/piconote` — confirm <5 MB
- **All phases**: No compiler warnings (`cargo clippy`), code formatted (`cargo fmt`)
