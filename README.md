# PicoNote

A super lightweight markdown notepad built in Rust to replace the now bloated "Notepad" applications shipped with all modern Operating Systems.

* [PicoNote](#piconote)
  * [Features](#features)
  * [Releases](#releases)
  * [Building](#building)
    * [For Development](#for-development)
      * [Linux dependencies](#linux-dependencies)
    * [For Production](#for-production)
      * [All platforms](#all-platforms)
      * [macOS app bundle](#macos-app-bundle)
      * [Linux .deb package](#linux-deb-package)
      * [Windows](#windows)
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

### For Development

```sh
cargo build
cargo run
```

#### Linux dependencies

Linux requires the following system packages before building:

```sh
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libgtk-3-dev
```

### For Production

#### All platforms

Third-party license texts are embedded in the binary. Regenerate them before
every release build to keep them in sync with current dependencies:

```sh
python3 scripts/generate-licenses.py
cargo build --release
```

The release binary is at `target/release/piconote`. It is fully self-contained
with all license attributions viewable under **Help > Third-Party Licenses**.

#### macOS app bundle

Produces a `.app` package with the correct icon, bundle identifier, and
`Info.plist` for Finder/Dock integration:

```sh
cargo install cargo-bundle
python3 scripts/generate-licenses.py
cargo bundle --release
```

The bundle is at `target/release/bundle/osx/PicoNote.app`.

#### Linux .deb package

Produces a `.deb` package with icons and a `.desktop` file for launcher
integration. Requires `cargo-bundle`:

```sh
cargo install cargo-bundle
python3 scripts/generate-licenses.py
cargo bundle --release
```

The package is at `target/release/bundle/deb/piconote_<version>_amd64.deb`.
Install it with:

```sh
sudo dpkg -i target/release/bundle/deb/piconote_*.deb
```

#### Windows

The `.ico` file is embedded at link time via `build.rs` and `winres` (Windows
only). A standard `cargo build --release` on Windows produces a binary with
the correct taskbar and Explorer icon — no extra steps required.

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
| image | PNG decoding for runtime window icon |
