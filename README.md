# PicoNote

A super lightweight markdown notepad built in Rust to replace the now bloated "Notepad" applications shipped with all modern Operating Systems.

* [PicoNote](#piconote)
  * [Features](#features)
  * [Releases](#releases)
  * [Building](#building)
    * [Linux dependencies](#linux-dependencies)
    * [Production build](#production-build)
  * [Architecture](#architecture)
  * [Dependencies](#dependencies)

---

## Features

TODO: Header Image

* **Markdown syntax highlighting** (basic + extended)
  * Headings, bold, italic, inline code, fenced code blocks
  * Blockquotes, lists, links, bare URLs
  * Strikethrough, highlight, tables, task lists
  * Footnotes, heading IDs, emoji shortcodes
* **Dark and Light themes**
* **Configurable font size** (10–28 px)
* **Word wrap toggle**
* **Native file dialogs** (Open, Save, Save As)
* **Unsaved changes protection** with Save / Don't Save / Cancel
* **Keyboard shortcuts**
  * `Cmd/Ctrl+S` — Save
  * `Cmd/Ctrl+Shift+S` — Save As
  * `Cmd/Ctrl+O` — Open
  * `Cmd/Ctrl+N` — New
  * `Cmd/Ctrl+Plus/Minus` — Adjust font size
* **Status bar** with line count, character count, and file path
* **Cross-platform** (macOS, Windows, Linux)
* **Tiny binary** (~3 MB release build)
* **Preferences persist** across sessions

## Releases

TODO: Information on releases.

## Building

Requires [Rust](https://rustup.rs/) 1.85+ (edition 2024) and Python 3.

```sh
# Debug build
cargo build

# Run (debug)
cargo run
```

### Linux dependencies

```sh
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libgtk-3-dev
```

### Production build

Third-party license texts are embedded in the binary. Before a release build,
regenerate them so they stay in sync with your current dependencies:

```sh
python3 scripts/generate-licenses.py
cargo build --release
```

The release binary is at `target/release/piconote`. It is fully self-contained
with all license attributions viewable under **Help > Third-Party Licenses**.

## Architecture

```
src/
├── main.rs          Entry point, eframe window setup
├── app.rs           Application state, UI layout, menu bar, shortcuts
├── parser.rs        Line-by-line markdown tokenizer (no external deps)
├── highlighter.rs   Memoized highlighter producing egui LayoutJob
├── theme.rs         Dark/Light theme application
├── config.rs        Preferences persistence via confy
└── file_ops.rs      File I/O and native dialogs via rfd
```

## Dependencies

| Crate | Purpose |
|---|---|
| eframe/egui | GUI framework (glow backend) |
| rfd | Native file dialogs |
| serde | Serialization for config |
| confy | Config file management |
| font-kit | System font discovery |
