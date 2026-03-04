mod app;
mod config;
mod file_ops;
mod highlighter;
mod parser;
mod theme;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PicoNote",
        native_options,
        Box::new(|cc| Ok(Box::new(app::PicoNoteApp::new(cc)))),
    )
}
