//! Hover-activated checkbox widget for batch-download row selection (#17, #18).
//!
//! Appears on the left side of a result-row when the row is hovered, and stays
//! visible (and filled) when checked. Callers own the boolean state; this widget
//! is a pure view that returns `Some(new_value)` on click, `None` otherwise.

#![allow(dead_code)] // downstream consumer result_row.rs (#17) not yet written

use eframe::egui;
use egui::{Pos2, Rect, Rounding, Sense, Stroke, Vec2};

use crate::ui::theme::theme;

/// Render a hover-activated 20×20 checkbox.
///
/// Returns `Some(new_value)` if the checkbox was toggled this frame, `None`
/// if the state is unchanged.
///
/// # Visibility rules
/// - `!checked && !row_hovered` → zero-sized allocation, nothing painted,
///   returns `None`.
/// - `checked || row_hovered` → 20×20 clickable checkbox painted.
pub fn hover_checkbox(ui: &mut egui::Ui, checked: bool, row_hovered: bool) -> Option<bool> {
    if !checked && !row_hovered {
        // Allocate nothing; callers can still chain on the return value.
        ui.allocate_space(Vec2::ZERO);
        return None;
    }

    let size = Vec2::new(20.0, 20.0);
    let response = ui.allocate_response(size, Sense::click());
    let rect = response.rect;

    paint_checkbox(ui, rect, checked);

    if response.clicked() {
        Some(!checked)
    } else {
        None
    }
}

/// Draw the checkbox visual into `rect`.
fn paint_checkbox(ui: &mut egui::Ui, rect: Rect, checked: bool) {
    let t = theme();
    let painter = ui.painter_at(rect);

    let rounding = Rounding::same(4.0);

    if checked {
        // Filled background with accent color.
        painter.rect_filled(rect, rounding, t.accent);
        // White checkmark.
        paint_check(
            &painter,
            rect.center(),
            rect.width() * 0.5,
            egui::Color32::WHITE,
        );
    } else {
        // Empty outline only (visible because row is hovered).
        painter.rect_stroke(rect, rounding, Stroke::new(2.0, t.ink_3));
    }
}

/// Paint a checkmark glyph centered on `center`, scaled to `radius`.
/// Two line segments: down-left dip then up-right — identical logic to
/// `progress_circle::paint_check` but exposed here for the checkbox size.
fn paint_check(painter: &egui::Painter, center: Pos2, radius: f32, color: egui::Color32) {
    let s = radius * 0.55;
    let stroke = Stroke::new((radius * 0.18).max(1.5), color);
    let p_left = center + Vec2::new(-s, 0.0);
    let p_mid = center + Vec2::new(-s * 0.25, s * 0.55);
    let p_right = center + Vec2::new(s * 0.85, -s * 0.55);
    painter.line_segment([p_left, p_mid], stroke);
    painter.line_segment([p_mid, p_right], stroke);
}
