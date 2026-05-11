//! Masked authentication-token text input widget.
#![allow(dead_code)]

use eframe::egui;

use crate::ui::theme::theme;

/// Mask shown in place of the real token when not revealed and not editing.
const TOKEN_MASK: &str = "••••••••••••••••••••••••";

/// Retained state for one token input field.
#[derive(Default)]
pub struct TokenInputState {
    pub value: String,
    pub editing: bool,
    pub revealed: bool,
}

impl TokenInputState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Draw the token input widget.
///
/// Returns `true` if `state.value` was changed.
pub fn token_input(ui: &mut egui::Ui, state: &mut TokenInputState) -> bool {
    let t = theme();
    let mut changed = false;

    if state.editing {
        // Full text-edit — password or plain depending on `revealed`.
        let edit = egui::TextEdit::singleline(&mut state.value)
            .password(!state.revealed)
            .hint_text("Paste token here")
            .desired_width(f32::INFINITY);
        let resp = ui.add(edit);
        // Auto-focus on first frame of editing.
        resp.request_focus();
        if resp.changed() {
            changed = true;
        }
        // Commit on Enter or focus lost.
        if resp.lost_focus() {
            state.editing = false;
        }
    } else if state.value.is_empty() {
        // Empty and not editing — show placeholder, click to start editing.
        let label_resp = ui.add(
            egui::Label::new(egui::RichText::new("Paste token here").color(t.ink_4))
                .sense(egui::Sense::click()),
        );
        if label_resp.clicked() {
            state.editing = true;
        }
    } else {
        // Has value, not editing — show masked or revealed text + controls.
        ui.horizontal(|ui| {
            let display = if state.revealed {
                state.value.clone()
            } else {
                TOKEN_MASK.to_owned()
            };

            let text_resp = ui.add(
                egui::Label::new(egui::RichText::new(&display).color(t.ink_2))
                    .sense(egui::Sense::click()),
            );
            if text_resp.clicked() {
                state.editing = true;
            }

            let eye_label = if state.revealed { "[hide]" } else { "[show]" };
            if ui.small_button(eye_label).clicked() {
                state.revealed = !state.revealed;
            }
        });
    }

    changed
}
