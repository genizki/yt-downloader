//! Boolean switch widget — a sliding pill-shaped toggle.
#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Rounding, Sense, Vec2};

use crate::ui::theme::theme;

/// Desired pixel size of the toggle track.
const TRACK_W: f32 = 36.0;
const TRACK_H: f32 = 20.0;
/// Thumb (circle) radius.
const THUMB_R: f32 = 7.0;

/// Draw a boolean toggle switch with `label` to the right.
///
/// Returns `true` if the value was changed (already mutated in `*value`).
pub fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> bool {
    let t = theme();
    let desired_size = Vec2::new(
        TRACK_W
            + 8.0
            + ui.fonts(|f| {
                f.glyph_width(
                    &egui::FontId::new(14.0, egui::FontFamily::Proportional),
                    'M',
                ) * label.len() as f32
            }),
        TRACK_H.max(20.0),
    );

    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.clicked() {
        *value = !*value;
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Track background.
        let track_rect = egui::Rect::from_min_size(rect.min, Vec2::new(TRACK_W, TRACK_H));
        let track_color = if *value {
            t.accent
        } else {
            Color32::from_rgb(180, 180, 180)
        };
        painter.rect_filled(track_rect, Rounding::same(TRACK_H / 2.0), track_color);

        // Thumb (white circle) — slides from left to right.
        let thumb_x = if *value {
            track_rect.left() + TRACK_W - THUMB_R - 3.0
        } else {
            track_rect.left() + THUMB_R + 3.0
        };
        let thumb_y = track_rect.center().y;
        painter.circle_filled(egui::pos2(thumb_x, thumb_y), THUMB_R, Color32::WHITE);

        // Label text to the right of the track.
        let text_pos = egui::pos2(track_rect.right() + 8.0, rect.center().y - 7.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
            t.ink,
        );
    }

    response.clicked()
}
