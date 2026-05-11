//! Results-header widget: summary label + optional batch-download button.
//!
//! Renders a horizontal row above the search-result list showing:
//! - Left: "About **N** results for *"query"*"
//! - Right: elapsed time label ("in 0.21s") and optionally a
//!   "Download N selected" button when `selected_count >= 1`.
//!
//! Public entry point is [`results_header`]. It returns a
//! [`ResultsHeaderResponse`] so the caller can react to a batch-download
//! button click.

#![allow(dead_code)] // downstream consumer (#24) not yet written

use eframe::egui;
use egui::{Layout, RichText};

use crate::ui::theme::theme;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Events produced by a single results-header render call.
pub struct ResultsHeaderResponse {
    /// `true` when the "Download N selected" button was clicked this frame.
    pub batch_download_clicked: bool,
}

// ---------------------------------------------------------------------------
// Public widget function
// ---------------------------------------------------------------------------

/// Render the results-header row and return interaction events.
///
/// # Arguments
/// * `result_count`   – total number of results returned by the search.
/// * `query`          – the search query string (displayed in italics).
/// * `selected_count` – number of rows currently selected for batch download;
///   `0` hides the batch button.
/// * `elapsed_ms`     – search duration in milliseconds, shown as "in X.XXs".
pub fn results_header(
    ui: &mut egui::Ui,
    result_count: usize,
    query: &str,
    selected_count: usize,
    elapsed_ms: u64,
) -> ResultsHeaderResponse {
    let t = theme();
    let mut batch_download_clicked = false;

    ui.horizontal(|ui| {
        // --- Left side: "About N results for "query"" ---
        ui.label(RichText::new("About ").color(t.ink));
        ui.label(
            RichText::new(result_count.to_string())
                .strong()
                .color(t.ink),
        );
        ui.label(RichText::new(" results for ").color(t.ink));
        ui.label(
            RichText::new(format!("\u{201c}{}\u{201d}", query))
                .italics()
                .color(t.ink),
        );

        // --- Right side (right-to-left): elapsed + optional batch button ---
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            // Elapsed time label (always shown, muted).
            let elapsed_s = elapsed_ms as f64 / 1000.0;
            ui.label(RichText::new(format!("in {:.2}s", elapsed_s)).color(t.ink_3));

            // Batch-download button (only when at least one row is selected).
            if selected_count >= 1 {
                let btn_label =
                    RichText::new(format!("Download {} selected", selected_count)).color(t.accent);
                if ui.button(btn_label).clicked() {
                    batch_download_clicked = true;
                }
            }
        });
    });

    ResultsHeaderResponse {
        batch_download_clicked,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the response type fields are accessible and default-false.
    #[test]
    fn response_default_not_clicked() {
        let r = ResultsHeaderResponse {
            batch_download_clicked: false,
        };
        assert!(!r.batch_download_clicked);
    }

    /// Verify the response can carry a true click event.
    #[test]
    fn response_clicked_true() {
        let r = ResultsHeaderResponse {
            batch_download_clicked: true,
        };
        assert!(r.batch_download_clicked);
    }
}
