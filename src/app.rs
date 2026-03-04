use eframe::egui;

use crate::file_ops::{self, FileState};

pub struct PicoNoteApp {
    content: String,
    file_state: FileState,
}

impl PicoNoteApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            content: String::new(),
            file_state: FileState::new(),
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
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let response = ui.add(
                    egui::TextEdit::multiline(&mut self.content)
                        .desired_width(f32::INFINITY)
                        .desired_rows(40)
                        .lock_focus(true)
                        .font(egui::TextStyle::Monospace),
                );
                if response.changed() {
                    self.file_state.dirty = true;
                }
            });
        });
    }
}
