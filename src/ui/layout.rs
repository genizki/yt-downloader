//! Shared centered layout helpers for all views.

use eframe::egui;

pub const CONTENT_MAX_WIDTH: f32 = 760.0;

pub fn centered_content<R>(
    ui: &mut egui::Ui,
    max_width: f32,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let full_height = ui.available_height();
    ui.horizontal(|ui| {
        let full = ui.available_width();
        let width = full.min(max_width);
        let gutter = ((full - width) * 0.5).max(0.0);

        ui.add_space(gutter);
        let inner = ui
            .allocate_ui_with_layout(
                egui::vec2(width, full_height),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.set_width(width);
                    ui.set_max_width(width);
                    ui.set_min_height(full_height);
                    content(ui)
                },
            )
            .inner;
        ui.add_space(gutter);
        inner
    })
    .inner
}
