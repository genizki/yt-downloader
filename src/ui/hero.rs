//! Hero headline — rendered above the searchbar before any search is submitted.

use eframe::egui;
use egui::FontId;

use crate::ui::theme;

/// Draw the hero headline (eyebrow + large title) centered horizontally.
///
/// Called from [`crate::app::YtDlpApp::update`] when `!searched`.
pub fn draw_hero(ui: &mut egui::Ui) {
    let t = theme::theme();

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add_space(8.0);

        // Eyebrow text — monospace to match design's --font-mono
        ui.label(
            egui::RichText::new("yt-dlp")
                .color(t.ink_3)
                .font(FontId::monospace(13.0)),
        );

        ui.add_space(8.0);

        // Large headline
        ui.label(
            egui::RichText::new("What do you want to download?")
                .color(t.ink)
                .font(FontId::proportional(36.0)),
        );
    });
}
