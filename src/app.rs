use eframe::egui;

use crate::config::{self, Config, ThemeChoice};
use crate::file_ops::{self, FileState};
use crate::highlighter::MemoizedMarkdownHighlighter;
use crate::theme;

/// What action is pending behind an unsaved-changes dialog.
#[derive(Clone)]
enum PendingAction {
    New,
    Open,
    OpenPath(std::path::PathBuf),
    Quit,
}

const THIRD_PARTY_LICENSES: &str = include_str!("THIRD-PARTY-LICENSES.txt");

pub struct PicoNoteApp {
    content: String,
    file_state: FileState,
    highlighter: MemoizedMarkdownHighlighter,
    config: Config,
    pending_action: Option<PendingAction>,
    system_fonts: Vec<String>,
    show_licenses: bool,
}

impl PicoNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open_path: Option<std::path::PathBuf>) -> Self {
        let config = config::load_config();
        theme::apply_theme(&cc.egui_ctx, &config.theme);
        if let Some(ref family) = config.font_family {
            apply_custom_font(&cc.egui_ctx, family);
        }

        #[cfg(target_os = "macos")]
        crate::macos_open::register_open_handler();

        let (content, file_state) = match open_path.and_then(|p| {
            std::fs::read_to_string(&p).ok().map(|c| (c, p))
        }) {
            Some((c, p)) => (c, FileState { path: Some(p), dirty: false }),
            None => (String::new(), FileState::new()),
        };

        Self {
            content,
            file_state,
            highlighter: MemoizedMarkdownHighlighter::default(),
            config,
            pending_action: None,
            system_fonts: enumerate_system_fonts(),
            show_licenses: false,
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

    fn open_path(&mut self, path: std::path::PathBuf) {
        if let Ok(content) = std::fs::read_to_string(&path) {
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
                PendingAction::OpenPath(p) => self.open_path(p),
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

        // --- Handle files opened via macOS "Open with..." ---
        #[cfg(target_os = "macos")]
        if self.pending_action.is_none() {
            if let Ok(mut files) = crate::macos_open::OPEN_FILES.lock() {
                if let Some(path) = files.pop() {
                    files.clear();
                    drop(files);
                    if self.file_state.dirty {
                        self.pending_action = Some(PendingAction::OpenPath(path));
                    } else {
                        self.open_path(path);
                    }
                }
            }
        }

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
            let action = self.pending_action.clone();
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
                            self.pending_action = action.clone();
                            self.execute_pending(ctx);
                        }
                        if ui.button("Don't Save").clicked() {
                            self.file_state.dirty = false;
                            self.pending_action = action.clone();
                            self.execute_pending(ctx);
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending_action = None;
                        }
                    });
                });
        }

        // --- Third-party licenses window ---
        if self.show_licenses {
            let mut open = true;
            egui::Window::new("Third-Party Licenses")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.monospace(THIRD_PARTY_LICENSES);
                    });
                });
            if !open {
                self.show_licenses = false;
            }
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
                    ui.label("Font");
                    let prev_font = self.config.font_family.clone();
                    let selected_text = self
                        .config
                        .font_family
                        .as_deref()
                        .unwrap_or("Default")
                        .to_owned();
                    let font_family = &mut self.config.font_family;
                    let system_fonts = &self.system_fonts;
                    egui::ComboBox::from_id_salt("font_picker")
                        .selected_text(selected_text)
                        .width(200.0)
                        .height(300.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(font_family, None, "Default");
                            for name in system_fonts {
                                ui.selectable_value(font_family, Some(name.clone()), name);
                            }
                        });
                    if self.config.font_family != prev_font {
                        match &self.config.font_family {
                            Some(family) => apply_custom_font(ctx, family),
                            None => reset_default_font(ctx),
                        }
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

                ui.menu_button("Help", |ui| {
                    if ui.button("Third-Party Licenses").clicked() {
                        self.show_licenses = true;
                        ui.close_menu();
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
        let panel_frame = egui::Frame::new()
            .inner_margin(0.0)
            .fill(ctx.style().visuals.panel_fill);
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                let font_size = self.config.font_size;
                let word_wrap = self.config.word_wrap;
                let highlighter = &mut self.highlighter;
                let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                    let mut job = highlighter.highlight(ui.style(), text, font_size);
                    job.wrap.max_width = if word_wrap { wrap_width } else { f32::INFINITY };
                    ui.fonts(|f| f.layout_job(job))
                };

                let scroll = if word_wrap {
                    egui::ScrollArea::vertical()
                } else {
                    egui::ScrollArea::both()
                };
                scroll.show(ui, |ui| {
                    let desired_width = if word_wrap {
                        ui.available_width()
                    } else {
                        f32::MAX
                    };

                    let text_edit = egui::TextEdit::multiline(&mut self.content)
                        .desired_width(desired_width)
                        .desired_rows(40)
                        .lock_focus(true)
                        .frame(false)
                        .margin(egui::Margin::symmetric(6, 4))
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

fn enumerate_system_fonts() -> Vec<String> {
    use font_kit::source::SystemSource;
    let mut families = SystemSource::new().all_families().unwrap_or_default();
    families.sort_unstable_by_key(|a| a.to_lowercase());
    families.dedup();
    families
}

fn apply_custom_font(ctx: &egui::Context, family: &str) {
    use font_kit::family_name::FamilyName;
    use font_kit::handle::Handle;
    use font_kit::properties::Properties;
    use font_kit::source::SystemSource;

    let source = SystemSource::new();
    let handle = match source
        .select_best_match(&[FamilyName::Title(family.to_string())], &Properties::new())
    {
        Ok(h) => h,
        Err(_) => return,
    };

    let (bytes, index) = match handle {
        Handle::Path { path, font_index } => match std::fs::read(&path) {
            Ok(b) => (b, font_index),
            Err(_) => return,
        },
        Handle::Memory { bytes, font_index } => ((*bytes).clone(), font_index),
    };

    let mut font_data = egui::FontData::from_owned(bytes);
    font_data.index = index;

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(family.to_owned(), font_data.into());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, family.to_owned());
    ctx.set_fonts(fonts);
}

fn reset_default_font(ctx: &egui::Context) {
    ctx.set_fonts(egui::FontDefinitions::default());
}
