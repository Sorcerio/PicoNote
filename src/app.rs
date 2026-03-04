use eframe::egui;

use crate::config::{self, Config, ThemeChoice};
use crate::file_ops::{self, FileState};
use crate::highlighter::MemoizedMarkdownHighlighter;
use crate::theme;

pub struct PicoNoteApp {
    content: String,
    file_state: FileState,
    highlighter: MemoizedMarkdownHighlighter,
    config: Config,
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
}

impl eframe::App for PicoNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_file();
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        self.open();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Save").clicked() {
                        self.save();
                        ui.close_menu();
                    }
                    if ui.button("Save As...").clicked() {
                        self.save_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
                        .add(egui::Slider::new(&mut self.config.font_size, 10.0..=28.0).suffix(" px"))
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
