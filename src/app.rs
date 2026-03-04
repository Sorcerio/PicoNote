use eframe::egui;

use crate::config::{self, Config, ThemeChoice};
use crate::file_ops::{self, FileState};
use crate::highlighter::MemoizedMarkdownHighlighter;
use crate::theme;

/// What action is pending behind an unsaved-changes dialog.
#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
    Quit,
}

pub struct PicoNoteApp {
    content: String,
    file_state: FileState,
    highlighter: MemoizedMarkdownHighlighter,
    config: Config,
    pending_action: Option<PendingAction>,
}

impl PicoNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = config::load_config();
        theme::apply_theme(&cc.egui_ctx, &config.theme);
        Self {
            content: String::new(),
            file_state: FileState::new(),
            highlighter: MemoizedMarkdownHighlighter::default(),
            config,
            pending_action: None,
        }
    }

    fn window_title(&self) -> String {
        let name = self
            .file_state
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string());
        if self.file_state.dirty {
            format!("PicoNote - {name}*")
        } else {
            format!("PicoNote - {name}")
        }
    }

    fn new_file(&mut self) {
        self.content.clear();
        self.file_state = FileState::new();
    }

    fn open(&mut self) {
        if let Some((content, path)) = file_ops::open_file_dialog() {
            self.content = content;
            self.file_state.path = Some(path);
            self.file_state.dirty = false;
        }
    }

    fn save(&mut self) {
        if let Some(path) = &self.file_state.path {
            let _ = file_ops::write_file(path, &self.content);
            self.file_state.dirty = false;
        } else {
            self.save_as();
        }
    }

    fn save_as(&mut self) {
        if let Some(path) = file_ops::save_file_dialog() {
            let _ = file_ops::write_file(&path, &self.content);
            self.file_state.path = Some(path);
            self.file_state.dirty = false;
        }
    }

    /// Guard an action behind an unsaved-changes check.
    /// Returns true if the action can proceed immediately.
    fn guard_unsaved(&mut self, action: PendingAction) -> bool {
        if self.file_state.dirty {
            self.pending_action = Some(action);
            false
        } else {
            true
        }
    }

    fn execute_pending(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.pending_action.take() {
            match action {
                PendingAction::New => self.new_file(),
                PendingAction::Open => self.open(),
                PendingAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
    }

    fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    fn char_count(&self) -> usize {
        self.content.len()
    }
}

impl eframe::App for PicoNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));

        // --- Keyboard shortcuts ---
        let modifiers = ctx.input(|i| i.modifiers);
        if modifiers.command {
            if ctx.input(|i| i.key_pressed(egui::Key::S)) {
                if modifiers.shift {
                    self.save_as();
                } else {
                    self.save();
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::O)) && self.guard_unsaved(PendingAction::Open)
            {
                self.open();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::N)) && self.guard_unsaved(PendingAction::New)
            {
                self.new_file();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Equals)) {
                self.config.font_size = (self.config.font_size + 1.0).min(28.0);
                config::save_config(&self.config);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Minus)) {
                self.config.font_size = (self.config.font_size - 1.0).max(10.0);
                config::save_config(&self.config);
            }
        }

        // --- Handle window close request ---
        if ctx.input(|i| i.viewport().close_requested()) && self.file_state.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_action = Some(PendingAction::Quit);
        }

        // --- Unsaved changes dialog ---
        if self.pending_action.is_some() {
            let action = self.pending_action;
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("You have unsaved changes.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.save();
                            self.pending_action = action;
                            self.execute_pending(ctx);
                        }
                        if ui.button("Don't Save").clicked() {
                            self.file_state.dirty = false;
                            self.pending_action = action;
                            self.execute_pending(ctx);
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending_action = None;
                        }
                    });
                });
        }

        // --- Menu bar ---
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(egui::Button::new("New").shortcut_text(shortcut_label("N")))
                        .clicked()
                    {
                        if self.guard_unsaved(PendingAction::New) {
                            self.new_file();
                        }
                        ui.close_menu();
                    }
                    if ui
                        .add(egui::Button::new("Open...").shortcut_text(shortcut_label("O")))
                        .clicked()
                    {
                        if self.guard_unsaved(PendingAction::Open) {
                            self.open();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add(egui::Button::new("Save").shortcut_text(shortcut_label("S")))
                        .clicked()
                    {
                        self.save();
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Save As...")
                                .shortcut_text(shortcut_shift_label("S")),
                        )
                        .clicked()
                    {
                        self.save_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        if self.guard_unsaved(PendingAction::Quit) {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Preferences", |ui| {
                    ui.label("Theme");
                    let mut changed = false;
                    changed |= ui
                        .radio_value(&mut self.config.theme, ThemeChoice::Dark, "Dark")
                        .changed();
                    changed |= ui
                        .radio_value(&mut self.config.theme, ThemeChoice::Light, "Light")
                        .changed();
                    if changed {
                        theme::apply_theme(ctx, &self.config.theme);
                        config::save_config(&self.config);
                    }

                    ui.separator();
                    ui.label("Font Size");
                    if ui
                        .add(
                            egui::Slider::new(&mut self.config.font_size, 10.0..=28.0)
                                .suffix(" px"),
                        )
                        .changed()
                    {
                        config::save_config(&self.config);
                    }

                    ui.separator();
                    if ui
                        .checkbox(&mut self.config.word_wrap, "Word Wrap")
                        .changed()
                    {
                        config::save_config(&self.config);
                    }
                });
            });
        });

        // --- Status bar ---
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Lines: {}  |  Chars: {}",
                    self.line_count(),
                    self.char_count()
                ));
                if let Some(path) = &self.file_state.path {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(path.to_string_lossy()).small().weak());
                    });
                }
            });
        });

        // --- Editor ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let font_size = self.config.font_size;
            let highlighter = &mut self.highlighter;
            let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                let mut job = highlighter.highlight(ui.style(), text, font_size);
                job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(job))
            };

            egui::ScrollArea::both().show(ui, |ui| {
                let desired_width = if self.config.word_wrap {
                    f32::INFINITY
                } else {
                    f32::MAX
                };

                let text_edit = egui::TextEdit::multiline(&mut self.content)
                    .desired_width(desired_width)
                    .desired_rows(40)
                    .lock_focus(true)
                    .layouter(&mut layouter);

                let response = ui.add(text_edit);
                if response.changed() {
                    self.file_state.dirty = true;
                }
            });
        });
    }
}

fn shortcut_label(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("\u{2318}{key}")
    } else {
        format!("Ctrl+{key}")
    }
}

fn shortcut_shift_label(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("\u{2318}\u{21E7}{key}")
    } else {
        format!("Ctrl+Shift+{key}")
    }
}
