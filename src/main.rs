#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod file_ops;
mod highlighter;
mod parser;
mod theme;

use eframe::egui;
use egui::IconData;

fn load_icon() -> IconData {
    let bytes = include_bytes!("../icons/icon_256.png");
    let img = image::load_from_memory(bytes).unwrap().into_rgba8();
    let (w, h) = img.dimensions();
    IconData { rgba: img.into_raw(), width: w, height: h }
}

fn main() -> eframe::Result<()> {
    let arg = std::env::args().nth(1);

    if matches!(arg.as_deref(), Some("-h") | Some("--help")) {
        println!("PicoNote - a lightweight Markdown editor\n");
        println!("Usage: piconote [FILE]\n");
        println!("  FILE    Optional path to a file to open on launch");
        println!("  -h, --help  Show this help message");
        std::process::exit(0);
    }

    let open_path = arg.map(std::path::PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "PicoNote",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::PicoNoteApp::new(cc, open_path)))),
    )
}
