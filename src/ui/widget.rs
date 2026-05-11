//! Shared glass-surface widget container.

use eframe::egui;
use egui::{Color32, Frame, Margin, Rounding, Shadow, Stroke, Vec2};

use crate::ui::theme;

pub struct Widget;

impl Widget {
    pub fn show<R>(
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let t = theme::theme();

        // Glass fill: ~55% white tint over the warm background.
        // Approximates --glass-bg: oklch(1 0 0 / 0.55)
        let glass_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 140);

        // White glass border: oklch(1 0 0 / 0.55)
        let glass_stroke = Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 140));

        // Soft drop shadow
        let shadow = Shadow {
            offset: Vec2::new(0.0, 6.0),
            blur: 20.0,
            spread: 0.0,
            color: t.shadow_md[0].color,
        };

        Frame {
            fill: glass_fill,
            rounding: Rounding::same(t.r_xl),
            stroke: glass_stroke,
            outer_margin: Margin::same(0.0),
            inner_margin: Margin::same(14.0),
            shadow,
        }
        .show(ui, content)
    }
}
