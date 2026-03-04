use eframe::egui;

use crate::config::ThemeChoice;

pub fn apply_theme(ctx: &egui::Context, theme: &ThemeChoice) {
    match theme {
        ThemeChoice::Dark => ctx.set_visuals(egui::Visuals::dark()),
        ThemeChoice::Light => ctx.set_visuals(egui::Visuals::light()),
    }
}
