//! Segmented-control widget — a row of mutually-exclusive option buttons.
#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Rounding, Sense, Vec2};

use crate::ui::theme::theme;

/// Draw a segmented control from a list of `(label, value)` pairs.
///
/// `current` is updated when the user clicks a different option.
/// Returns `true` if the selection changed.
pub fn segmented<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    options: &[(&str, T)],
    current: &mut T,
    disabled: bool,
) -> bool {
    let t = theme();
    let mut changed = false;

    ui.horizontal_wrapped(|ui| {
        for (label, value) in options {
            let selected = current == value;

            let (bg, text_color) = if disabled {
                (
                    Color32::from_rgb(220, 220, 220),
                    Color32::from_rgb(160, 160, 160),
                )
            } else if selected {
                (t.accent, Color32::WHITE)
            } else {
                (t.bg_elev, t.ink_2)
            };

            let text = egui::WidgetText::from(*label);
            let galley = text.into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            );
            let btn_size = Vec2::new(galley.size().x + 16.0, galley.size().y + 8.0);

            let (rect, response) = ui.allocate_exact_size(btn_size, Sense::click());

            if response.clicked() && !disabled && !selected {
                *current = value.clone();
                changed = true;
            }

            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                painter.rect_filled(rect, Rounding::same(8.0), bg);
                painter.galley(
                    egui::pos2(
                        rect.center().x - galley.size().x / 2.0,
                        rect.center().y - galley.size().y / 2.0,
                    ),
                    galley,
                    text_color,
                );
            }
        }
    });

    changed
}
