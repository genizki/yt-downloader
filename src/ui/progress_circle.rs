//! Per-result-row progress indicator: a small ring + status text.
//!
//! Used by `result_row.rs` (#17) to visualize each download's lifecycle:
//! `Idle` → `Queued` → `Downloading(p)` → `PostProcessing` / `Moving` → `Done`
//! or `Failed`. The widget is purely a *view* over a [`Phase`] value — the
//! caller owns the actual phase state (typically a `HashMap<VideoId, Phase>`
//! kept in app state and updated from the download workers via mpsc).
//!
//! Geometry is testable in isolation via [`arc_endpoint`] and [`pulse_value`]
//! (no painter / context required). The arc origin is *top-of-circle*
//! (i.e. `-π/2`) so progress sweeps clockwise from 12 o'clock.
//!
//! `dead_code` is allowed module-wide because `result_row.rs` (#17) is
//! downstream and not yet written; the widget compiles and tests on its own.

#![allow(dead_code)]

use eframe::egui;
use egui::epaint::PathShape;
use egui::{Color32, FontId, Pos2, Sense, Stroke, Vec2};

use crate::ui::theme;

/// Lifecycle phase of a single download — drives both ring fill and label.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// Nothing to show; the widget allocates zero space.
    Idle,
    /// Waiting in a worker queue (no progress yet).
    Queued,
    /// Actively downloading. `progress` is clamped to `[0.0, 1.0]`.
    Downloading(f32),
    /// yt-dlp post-processing (mux/transcode/embed).
    PostProcessing,
    /// Moving the finished file from temp to the configured download dir.
    Moving,
    /// Finished successfully.
    Done,
    /// Failed. The string is the human-readable reason shown as a hover tooltip.
    Failed(String),
}

/// Builder/widget for the progress circle. Construct with [`Self::new`],
/// optionally tweak [`Self::diameter`] / [`Self::thickness`], then `show(ui)`.
pub struct ProgressCircle {
    phase: Phase,
    diameter: f32,
    track_thickness: f32,
}

impl ProgressCircle {
    /// Create a new circle widget for the given phase. Default diameter
    /// is `28px`, default track thickness is `3px`.
    pub fn new(phase: Phase) -> Self {
        Self {
            phase,
            diameter: 28.0,
            track_thickness: 3.0,
        }
    }

    /// Override the outer diameter in points.
    pub fn diameter(mut self, d: f32) -> Self {
        self.diameter = d;
        self
    }

    /// Override the ring stroke thickness in points.
    pub fn thickness(mut self, t: f32) -> Self {
        self.track_thickness = t;
        self
    }

    /// Render the circle plus its status text (right of the circle), and
    /// return the allocated [`egui::Response`].
    ///
    /// `Idle` returns an empty zero-sized response (no allocation, no paint).
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        if matches!(self.phase, Phase::Idle) {
            // Allocate a zero-sized response so callers can chain without
            // special-casing. `Sense::hover()` keeps it cheap.
            let (_id, rect) = ui.allocate_space(Vec2::ZERO);
            return ui.interact(rect, ui.id().with("progress_circle_idle"), Sense::hover());
        }

        let t = theme::theme();
        let label = phase_label(&self.phase);
        let font_id = FontId::proportional(12.0);
        let text_color = t.ink_2;

        // Measure the text so we can allocate exact width.
        let text_width = ui.fonts(|f| {
            f.layout_no_wrap(label.to_string(), font_id.clone(), text_color)
                .size()
                .x
        });

        // Layout: [circle] [8px gap] [text].
        const GAP: f32 = 8.0;
        let total = Vec2::new(self.diameter + GAP + text_width, self.diameter);
        let response = ui.allocate_response(total, Sense::hover());
        let rect = response.rect;

        let painter = ui.painter_at(rect);
        let radius = self.diameter * 0.5;
        let center = rect.left_center() + Vec2::new(radius, 0.0);

        match &self.phase {
            Phase::Idle => unreachable!(),

            Phase::Queued => {
                paint_track(&painter, center, radius, self.track_thickness, t.ink_4);
            }

            Phase::Downloading(p) => {
                let p = p.clamp(0.0, 1.0);
                paint_track(&painter, center, radius, self.track_thickness, t.ink_4);
                paint_arc(
                    &painter,
                    center,
                    radius,
                    self.track_thickness,
                    0.0,
                    p,
                    t.accent,
                );
                ui.ctx().request_repaint();
            }

            Phase::PostProcessing | Phase::Moving => {
                // Pulsing outer halo with full accent ring.
                let time = ui.input(|i| i.time);
                let pulse = pulse_value(time);
                let halo_alpha = lerp_u8(0.3, 0.6, pulse);
                let halo_color = with_alpha(t.accent, halo_alpha);
                // Halo: a slightly thicker stroke just outside the ring.
                painter.circle_stroke(
                    center,
                    radius + self.track_thickness * 0.5,
                    Stroke::new(self.track_thickness * 1.6, halo_color),
                );
                paint_track(&painter, center, radius, self.track_thickness, t.accent);
                ui.ctx().request_repaint();
            }

            Phase::Done => {
                let done_color = Color32::from_rgb(50, 180, 80);
                paint_track(&painter, center, radius, self.track_thickness, done_color);
                paint_check(&painter, center, radius, done_color);
            }

            Phase::Failed(_) => {
                let fail_color = Color32::from_rgb(220, 60, 60);
                paint_track(&painter, center, radius, self.track_thickness, fail_color);
                paint_cross(&painter, center, radius, fail_color);
            }
        }

        // Status text right of the circle.
        let text_pos = Pos2::new(rect.left() + self.diameter + GAP, rect.center().y);
        painter.text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            label,
            font_id,
            text_color,
        );

        if let Phase::Failed(ref reason) = self.phase {
            response.on_hover_text(reason.clone())
        } else {
            response
        }
    }
}

/// Map a [`Phase`] to its short status label.
fn phase_label(phase: &Phase) -> &'static str {
    match phase {
        Phase::Idle => "",
        Phase::Queued => "queued",
        Phase::Downloading(_) => "downloading…",
        Phase::PostProcessing => "formatting…",
        Phase::Moving => "moving…",
        Phase::Done => "done",
        Phase::Failed(_) => "failed",
    }
}

/// Paint the full background ring (track).
fn paint_track(painter: &egui::Painter, center: Pos2, radius: f32, thickness: f32, color: Color32) {
    painter.circle_stroke(center, radius, Stroke::new(thickness, color));
}

/// Paint a circular *arc* from `start` to `start + sweep` (in turns ∈ [0,1]),
/// clockwise, with `0.0 turns` = top-of-circle (12 o'clock).
fn paint_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    thickness: f32,
    start: f32,
    sweep: f32,
    color: Color32,
) {
    let sweep = sweep.clamp(0.0, 1.0);
    if sweep <= 0.0 {
        return;
    }
    const N: usize = 64;
    let mut points = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let p = start + sweep * (i as f32 / N as f32);
        points.push(arc_endpoint(center, radius, p));
    }
    painter.add(PathShape::line(points, Stroke::new(thickness, color)));
}

/// Paint a checkmark glyph centered on `center`, scaled to the inner area.
/// Drawn as 2 line segments: down-left dip, then up-right.
fn paint_check(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let s = radius * 0.55; // half-extent of the glyph
    let stroke = Stroke::new((radius * 0.18).max(1.5), color);
    let p_left = center + Vec2::new(-s, 0.0);
    let p_mid = center + Vec2::new(-s * 0.25, s * 0.55);
    let p_right = center + Vec2::new(s * 0.85, -s * 0.55);
    painter.line_segment([p_left, p_mid], stroke);
    painter.line_segment([p_mid, p_right], stroke);
}

/// Paint a small `X` glyph centered on `center`.
fn paint_cross(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let s = radius * 0.45;
    let stroke = Stroke::new((radius * 0.18).max(1.5), color);
    let tl = center + Vec2::new(-s, -s);
    let br = center + Vec2::new(s, s);
    let tr = center + Vec2::new(s, -s);
    let bl = center + Vec2::new(-s, s);
    painter.line_segment([tl, br], stroke);
    painter.line_segment([tr, bl], stroke);
}

/// Compute a point on the circle for `progress ∈ [0,1]` *turns clockwise*
/// starting at 12 o'clock. Pure helper, used both by the arc painter and
/// the geometry unit tests.
///
/// In math:
/// `angle = -π/2 + 2π·progress`,
/// `(x, y) = (cx + r·cos(angle), cy + r·sin(angle))`.
pub fn arc_endpoint(center: Pos2, radius: f32, progress: f32) -> Pos2 {
    let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * progress;
    Pos2::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

/// Pulse value in `[0, 1]` derived from a time in seconds. Frequency is
/// `1.5 Hz` so pulse_value(t) = 0.5 + 0.5·sin(t·2π·1.5).
pub fn pulse_value(time_seconds: f64) -> f32 {
    let phase = (time_seconds as f32) * std::f32::consts::TAU * 1.5;
    0.5 + 0.5 * phase.sin()
}

/// Linear interpolation `[a, b]` by `t ∈ [0,1]`, returning a `u8 ∈ [0, 255]`.
fn lerp_u8(a: f32, b: f32, t: f32) -> u8 {
    let v = a + (b - a) * t.clamp(0.0, 1.0);
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Return `color` with its alpha replaced by `alpha`.
fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolerance for floating-point comparisons of pixel positions.
    const EPS: f32 = 1e-3;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= EPS
    }

    #[test]
    fn arc_endpoint_at_progress_zero_is_top() {
        let c = Pos2::new(50.0, 50.0);
        let r = 10.0;
        let p = arc_endpoint(c, r, 0.0);
        assert!(
            close(p.x, c.x) && close(p.y, c.y - r),
            "expected ~({}, {}), got {:?}",
            c.x,
            c.y - r,
            p
        );
    }

    #[test]
    fn arc_endpoint_at_progress_half_is_bottom() {
        let c = Pos2::new(0.0, 0.0);
        let r = 5.0;
        let p = arc_endpoint(c, r, 0.5);
        assert!(
            close(p.x, c.x) && close(p.y, c.y + r),
            "expected ~(0, {}), got {:?}",
            r,
            p
        );
    }

    #[test]
    fn arc_endpoint_at_progress_quarter_is_right() {
        let c = Pos2::new(100.0, 200.0);
        let r = 20.0;
        let p = arc_endpoint(c, r, 0.25);
        assert!(
            close(p.x, c.x + r) && close(p.y, c.y),
            "expected ~({}, {}), got {:?}",
            c.x + r,
            c.y,
            p
        );
    }

    #[test]
    fn arc_endpoint_at_progress_three_quarter_is_left() {
        let c = Pos2::new(0.0, 0.0);
        let r = 10.0;
        let p = arc_endpoint(c, r, 0.75);
        assert!(
            close(p.x, c.x - r) && close(p.y, c.y),
            "expected ~({}, 0), got {:?}",
            -r,
            p
        );
    }

    #[test]
    fn arc_endpoint_at_progress_one_wraps_to_top() {
        let c = Pos2::new(0.0, 0.0);
        let r = 7.0;
        let p = arc_endpoint(c, r, 1.0);
        assert!(
            close(p.x, c.x) && close(p.y, c.y - r),
            "expected wrap to top (0, {}), got {:?}",
            -r,
            p
        );
    }

    #[test]
    fn pulse_value_in_range() {
        // Sample many times; pulse must stay within [0, 1] inclusive.
        for k in 0..200 {
            let t = k as f64 * 0.0173; // irregular sampling to hit varied phases
            let v = pulse_value(t);
            assert!((0.0..=1.0).contains(&v), "pulse({t}) = {v} out of [0, 1]");
        }
    }

    #[test]
    fn pulse_value_at_zero_is_half() {
        // sin(0) = 0 → 0.5 + 0.5·0 = 0.5
        let v = pulse_value(0.0);
        assert!(close(v, 0.5), "expected 0.5 at t=0, got {v}");
    }

    #[test]
    fn phase_label_is_stable() {
        assert_eq!(phase_label(&Phase::Idle), "");
        assert_eq!(phase_label(&Phase::Queued), "queued");
        assert_eq!(phase_label(&Phase::Downloading(0.0)), "downloading…");
        assert_eq!(phase_label(&Phase::PostProcessing), "formatting…");
        assert_eq!(phase_label(&Phase::Moving), "moving…");
        assert_eq!(phase_label(&Phase::Done), "done");
        assert_eq!(phase_label(&Phase::Failed("err".into())), "failed");
    }
}
