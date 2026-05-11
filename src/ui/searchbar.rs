//! Searchbar widget — matches the design from `claude Design/yt-dlp/project/app.jsx`
//! lines 655-684. Self-contained: holds its own state via [`SearchbarState`].
//!
//! Typical usage:
//! ```rust,ignore
//! let event = Searchbar::new(&mut self.searchbar).show(ui);
//! match event {
//!     SearchbarEvent::Submit(q) => { /* kick off search */ }
//!     SearchbarEvent::Clear     => { /* clear results  */ }
//!     SearchbarEvent::None      => {}
//! }
//! ```

#![allow(dead_code)] // downstream consumer (#13) not yet written

use eframe::egui;
use egui::{Color32, FontId, Key, Pos2, Rounding, Sense, Stroke, Vec2};

use crate::ui::theme;
use crate::ui::widget::Widget;

// ── State ─────────────────────────────────────────────────────────────────────

/// Persistent state owned by the caller and passed into [`Searchbar`] each frame.
#[derive(Default)]
pub struct SearchbarState {
    /// The current text in the input box.
    pub query: String,
    /// Whether the text input currently has keyboard focus.
    pub focused: bool,
    /// Recent search history — caller is responsible for updating this list.
    pub recent: Vec<String>,
}

// ── Widget builder ────────────────────────────────────────────────────────────

/// Ephemeral widget handle — create once per frame, call [`Self::show`].
pub struct Searchbar<'a> {
    state: &'a mut SearchbarState,
}

impl<'a> Searchbar<'a> {
    pub fn new(state: &'a mut SearchbarState) -> Self {
        Self { state }
    }

    /// Render the searchbar and return the interaction event for this frame.
    pub fn show(self, ui: &mut egui::Ui) -> SearchbarEvent {
        // The unique ID used to track TextEdit focus.
        let edit_id = ui.id().with("searchbar_textedit");

        // ── Outer widget frame ────────────────────────────────────────────────
        let inner = Widget::show(ui, |ui| {
            let mut inner_event = SearchbarEvent::None;

            ui.horizontal(|ui| {
                let t = theme::theme();

                // ── Search icon (drawn as painter text "🔍" 18 px) ────────────
                let icon_size = Vec2::splat(18.0);
                let (icon_rect, _) = ui.allocate_exact_size(icon_size, Sense::hover());
                ui.painter().text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "🔍",
                    FontId::proportional(14.0),
                    t.ink_3,
                );

                ui.add_space(6.0);

                // ── Text input ────────────────────────────────────────────────
                let available = ui.available_width()
                    - if self.state.query.is_empty() && !self.state.focused {
                        // Reserve room for ⌘K badge (approx 36 px)
                        40.0
                    } else if !self.state.query.is_empty() {
                        // Reserve room for clear button (approx 22 px)
                        26.0
                    } else {
                        0.0
                    };

                let edit_resp = ui.add_sized(
                    Vec2::new(available.max(40.0), 22.0),
                    egui::TextEdit::singleline(&mut self.state.query)
                        .id(edit_id)
                        .hint_text("Start typing for search")
                        .frame(false)
                        .desired_width(available.max(40.0)),
                );

                // Track focus from egui's perspective.
                self.state.focused = edit_resp.has_focus();

                // ── Right-side slot: ⌘K badge or clear button ─────────────────
                if self.state.query.is_empty() && !self.state.focused {
                    // Draw ⌘K pill badge using painter primitives.
                    let badge_size = Vec2::new(34.0, 18.0);
                    let (badge_rect, _) = ui.allocate_exact_size(badge_size, Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(
                        badge_rect,
                        Rounding::same(5.0),
                        Color32::from_rgba_unmultiplied(120, 120, 140, 40),
                    );
                    painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "⌘K",
                        FontId::proportional(11.0),
                        Color32::from_rgba_unmultiplied(80, 80, 100, 200),
                    );
                } else if !self.state.query.is_empty() {
                    // Clear "×" button.
                    let btn_size = Vec2::splat(20.0);
                    let (btn_rect, btn_resp) = ui.allocate_exact_size(btn_size, Sense::click());
                    let t2 = theme::theme();
                    let btn_color = if btn_resp.hovered() {
                        t2.ink_2
                    } else {
                        t2.ink_3
                    };
                    // Draw a small "×" using two line segments.
                    let s = 5.0_f32;
                    let c: Pos2 = btn_rect.center();
                    let stroke = Stroke::new(1.5, btn_color);
                    ui.painter()
                        .line_segment([c + Vec2::new(-s, -s), c + Vec2::new(s, s)], stroke);
                    ui.painter()
                        .line_segment([c + Vec2::new(s, -s), c + Vec2::new(-s, s)], stroke);
                    if btn_resp.clicked() {
                        inner_event = SearchbarEvent::Clear;
                    }
                }

                // ── Keyboard shortcuts ────────────────────────────────────────
                let (cmd_k, enter, _esc) = ui.input(|i| {
                    (
                        i.modifiers.command && i.key_pressed(Key::K),
                        i.key_pressed(Key::Enter),
                        i.key_pressed(Key::Escape),
                    )
                });

                if cmd_k {
                    // Request focus on the text edit.
                    edit_resp.request_focus();
                    self.state.focused = true;
                }

                if enter && !self.state.query.is_empty() {
                    inner_event = SearchbarEvent::Submit(self.state.query.clone());
                }
                // Esc: egui's own focus handling moves focus away; no action needed here.
            });

            inner_event
        });

        let mut event = inner.inner;

        // ── Recent searches dropdown ──────────────────────────────────────────
        if self.state.focused && self.state.query.is_empty() && !self.state.recent.is_empty() {
            let recent_clone = self.state.recent.clone();

            let dropdown = Widget::show(ui, |ui| {
                let mut picked: Option<String> = None;
                ui.vertical(|ui| {
                    let t = theme::theme();
                    for item in &recent_clone {
                        ui.horizontal(|ui| {
                            // Clock icon as text glyph.
                            ui.label(egui::RichText::new("⏱").color(t.ink_3).size(12.0));
                            let resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(item.as_str()).color(t.ink_2).size(13.0),
                                )
                                .sense(Sense::click()),
                            );
                            if resp.clicked() {
                                picked = Some(item.clone());
                            }
                        });
                    }
                });
                picked
            });

            if let Some(picked) = dropdown.inner {
                event = SearchbarEvent::Submit(picked);
            }
        }

        // Apply Clear side-effect: empty the query string.
        if matches!(event, SearchbarEvent::Clear) {
            self.state.query.clear();
        }

        event
    }
}

// ── Event ─────────────────────────────────────────────────────────────────────

/// Interaction event returned by [`Searchbar::show`] for the current frame.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchbarEvent {
    /// Nothing happened this frame.
    None,
    /// The user submitted a query (Enter key or recent item click).
    Submit(String),
    /// The user clicked the clear (×) button.
    Clear,
}
