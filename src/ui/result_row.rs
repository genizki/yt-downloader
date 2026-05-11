//! Result-row widget: renders one search result as a glass card.
//!
//! Layout matches the design's `.result` grid:
//!   [200px thumbnail] [gap 20px] [text column flex]
//! with `14px` padding on all sides.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use eframe::egui;
use egui::{
    Color32, FontId, Frame, Margin, Pos2, Rect, RichText, Rounding, Sense, Shadow, Stroke, Vec2,
};

use crate::api::types::{VideoId, YouTubeVideo};
use crate::download::progress::DownloadPhase;
use crate::ui::hover_checkbox::hover_checkbox;
use crate::ui::progress_circle::{Phase, ProgressCircle};
use crate::ui::theme::theme;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

pub struct ResultRowResponse {
    pub download_clicked: bool,
    pub checkbox_changed: Option<bool>,
}

// ---------------------------------------------------------------------------
// Layout constants (matching the CSS design)
// ---------------------------------------------------------------------------

const THUMB_W: f32 = 200.0;
const THUMB_H: f32 = 112.0; // 16:9
const CARD_PADDING: f32 = 14.0;
const GAP: f32 = 20.0;
const CHECKBOX_W: f32 = 28.0;
const CARD_HEIGHT: f32 = THUMB_H + CARD_PADDING * 2.0;

// ---------------------------------------------------------------------------
// Public widget function
// ---------------------------------------------------------------------------

pub fn result_row(
    ui: &mut egui::Ui,
    video: &YouTubeVideo,
    checked: bool,
    phase: Option<&DownloadPhase>,
    now: &DateTime<Utc>,
) -> ResultRowResponse {
    let t = theme();
    let available_width = ui.available_width();

    // Glass card frame
    let glass_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 140);
    let glass_stroke = Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 140));
    let shadow = Shadow {
        offset: Vec2::new(0.0, 6.0),
        blur: 20.0,
        spread: 0.0,
        color: t.shadow_md[0].color,
    };

    let mut download_clicked = false;
    let mut checkbox_changed: Option<bool> = None;

    let resp = Frame {
        fill: glass_fill,
        rounding: Rounding::same(t.r_xl),
        stroke: glass_stroke,
        outer_margin: Margin::same(0.0),
        inner_margin: Margin::same(CARD_PADDING),
        shadow,
    }
    .show(ui, |ui| {
        let card_width = available_width - CARD_PADDING * 2.0;
        ui.set_min_height(THUMB_H);

        ui.horizontal(|ui| {
            // ── Checkbox (28px, shown on hover) ──────────────────────────────
            let cb_size = Vec2::new(CHECKBOX_W, THUMB_H);
            let (cb_rect, _) = ui.allocate_exact_size(cb_size, Sense::hover());
            let hovered = ui.rect_contains_pointer(ui.min_rect().expand(0.0));
            let center = cb_rect.center();
            let inner = Rect::from_center_size(center, Vec2::new(20.0, 20.0));
            let mut cb_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::centered_and_justified(egui::Direction::TopDown)),
            );
            checkbox_changed = hover_checkbox(&mut cb_ui, checked, hovered);

            // ── Thumbnail (200×112, 16:9) ─────────────────────────────────────
            let thumb_size = Vec2::new(THUMB_W, THUMB_H);
            let (thumb_rect, thumb_resp) = ui.allocate_exact_size(thumb_size, Sense::click());
            paint_thumbnail(ui, thumb_rect, &video.id, video.duration_seconds, &video.thumbnail_url);

            ui.add_space(GAP);

            // ── Text column ───────────────────────────────────────────────────
            let progress_w = if phase.is_some() { 90.0 + GAP } else { 0.0 };
            let text_w = (card_width - CHECKBOX_W - THUMB_W - GAP - progress_w).max(80.0);

            let text_size = Vec2::new(text_w, THUMB_H);
            let (text_rect, _) = ui.allocate_exact_size(text_size, Sense::hover());
            let mut text_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(text_rect)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            text_ui.set_min_height(THUMB_H);
            text_ui.add_space(4.0);
            paint_text(&mut text_ui, video, now);

            // ── Progress circle ───────────────────────────────────────────────
            if let Some(p) = phase {
                ui.add_space(GAP);
                ProgressCircle::new(phase_to_circle_phase(p)).show(ui);
            }

            download_clicked = thumb_resp.clicked();
        });
    });

    // Clicking anywhere on the card (not just thumbnail) triggers download
    if resp.response.clicked() && phase.is_none() {
        download_clicked = true;
    }

    ResultRowResponse {
        download_clicked,
        checkbox_changed,
    }
}

// ---------------------------------------------------------------------------
// Thumbnail painter
// ---------------------------------------------------------------------------

fn paint_thumbnail(
    ui: &mut egui::Ui,
    rect: Rect,
    id: &VideoId,
    duration_secs: u64,
    thumbnail_url: &str,
) {
    let painter = ui.painter_at(rect);

    if !thumbnail_url.is_empty() {
        // ── Real thumbnail via egui_extras HTTP image loader ─────────────────
        // Clip to rounded corners first, then draw the image inside that rect.
        let rounding = Rounding::same(14.0);

        // Draw the image into the pre-allocated rect using a child ui.
        let mut thumb_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::centered_and_justified(egui::Direction::TopDown)),
        );
        thumb_ui.add(
            egui::Image::from_uri(thumbnail_url)
                .fit_to_exact_size(rect.size())
                .rounding(rounding),
        );
    } else {
        // ── Procedural placeholder (no URL yet / loading failed) ─────────────
        let fill = hue_to_color(video_hue(id));
        painter.rect_filled(rect, Rounding::same(14.0), fill);

        let dark = darken(fill, 0.18);
        let spacing = 10.0_f32;
        let total = rect.width() + rect.height();
        let mut offset = 0.0_f32;
        while offset < total {
            let p0 = Pos2::new((rect.left() + offset).min(rect.right()), rect.top());
            let p1 = Pos2::new(rect.left(), (rect.top() + offset).min(rect.bottom()));
            painter.line_segment([p0, p1], Stroke::new(2.0, dark));
            offset += spacing;
        }
    }

    // ── Duration badge (always on top) ────────────────────────────────────────
    let badge_text = fmt_duration(duration_secs);
    let font_id = FontId::monospace(11.0);
    let text_color = Color32::from_rgb(249, 249, 249);
    let padding = Vec2::new(7.0, 3.0);

    let galley = ui.fonts(|f| f.layout_no_wrap(badge_text.clone(), font_id.clone(), text_color));
    let badge_size = galley.size() + padding * 2.0;
    let badge_rect = Rect::from_min_size(
        Pos2::new(rect.right() - badge_size.x - 8.0, rect.bottom() - badge_size.y - 8.0),
        badge_size,
    );

    let painter = ui.painter_at(rect);
    painter.rect_filled(badge_rect, Rounding::same(7.0), Color32::from_rgba_unmultiplied(20, 20, 20, 180));
    painter.text(badge_rect.center(), egui::Align2::CENTER_CENTER, badge_text, font_id, text_color);
}

// ---------------------------------------------------------------------------
// Text painter
// ---------------------------------------------------------------------------

fn paint_text(ui: &mut egui::Ui, video: &YouTubeVideo, now: &DateTime<Utc>) {
    let t = theme();

    // Title — 16px, weight 500, up to 2 lines
    ui.add(
        egui::Label::new(
            RichText::new(&video.title)
                .color(t.ink)
                .font(FontId::proportional(15.0))
                .strong(),
        )
        .truncate(),
    );

    ui.add_space(4.0);

    // Channel / author — 13px ink-2
    ui.add(
        egui::Label::new(
            RichText::new(&video.channel)
                .color(t.ink_2)
                .font(FontId::proportional(13.0)),
        )
        .truncate(),
    );

    // Push meta to bottom
    ui.add_space(ui.available_height().max(4.0) - 22.0);

    // Meta row: duration · views · relative date
    let duration_str = fmt_duration(video.duration_seconds);
    let views_str = fmt_views(video.views);
    let date_str = fmt_date(&video.published_at, now);

    ui.horizontal(|ui| {
        // Duration pill (matching .result-duration style)
        let dur_text = RichText::new(&duration_str)
            .color(t.ink)
            .font(FontId::monospace(11.0));
        let dur_label = egui::Label::new(dur_text);
        let (dur_rect, _) = ui.allocate_exact_size(
            ui.fonts(|f| {
                f.layout_no_wrap(
                    duration_str.clone(),
                    FontId::monospace(11.0),
                    t.ink,
                )
                .size()
                    + Vec2::new(14.0, 4.0)
            }),
            Sense::hover(),
        );
        ui.painter().rect_filled(
            dur_rect,
            Rounding::same(6.0),
            Color32::from_rgba_unmultiplied(255, 255, 255, 128),
        );
        ui.painter().rect_stroke(
            dur_rect,
            Rounding::same(6.0),
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 153)),
        );
        ui.painter().text(
            dur_rect.center(),
            egui::Align2::CENTER_CENTER,
            &duration_str,
            FontId::monospace(11.0),
            t.ink,
        );
        let _ = dur_label;

        ui.label(RichText::new("·").color(t.ink_4).font(FontId::proportional(11.0)));
        ui.label(
            RichText::new(format!("{} views", views_str))
                .color(t.ink_3)
                .font(FontId::monospace(11.0)),
        );
        ui.label(RichText::new("·").color(t.ink_4).font(FontId::proportional(11.0)));
        ui.label(
            RichText::new(date_str)
                .color(t.ink_3)
                .font(FontId::monospace(11.0)),
        );
    });
}

// ---------------------------------------------------------------------------
// Phase mapping
// ---------------------------------------------------------------------------

fn phase_to_circle_phase(phase: &DownloadPhase) -> Phase {
    match phase {
        DownloadPhase::Queued => Phase::Queued,
        DownloadPhase::Downloading(p) => Phase::Downloading(*p),
        DownloadPhase::PostProcessing => Phase::PostProcessing,
        DownloadPhase::Moving => Phase::Moving,
        DownloadPhase::Done => Phase::Done,
        DownloadPhase::Failed(reason) => Phase::Failed(reason.clone()),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fmt_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

fn fmt_views(views: u64) -> String {
    if views >= 1_000_000 {
        let m = views as f64 / 1_000_000.0;
        if m >= 10.0 {
            format!("{:.0}M", m)
        } else {
            format!("{:.1}M", m)
        }
    } else if views >= 1_000 {
        let k = views as f64 / 1_000.0;
        if k >= 10.0 {
            format!("{:.0}K", k)
        } else {
            format!("{:.1}K", k)
        }
    } else {
        format!("{}", views)
    }
}

fn fmt_date(dt: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let diff = now.signed_duration_since(*dt);
    let secs = diff.num_seconds();
    if secs < 0 {
        return "just now".to_string();
    }
    let minutes = secs / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    let weeks = days / 7;
    let months = days / 30;
    let years = days / 365;

    if years >= 2 {
        format!("{} years ago", years)
    } else if years == 1 {
        "1 year ago".to_string()
    } else if months >= 2 {
        format!("{} months ago", months)
    } else if months == 1 {
        "1 month ago".to_string()
    } else if weeks >= 2 {
        format!("{} weeks ago", weeks)
    } else if weeks == 1 {
        "1 week ago".to_string()
    } else if days >= 2 {
        format!("{} days ago", days)
    } else if days == 1 {
        "1 day ago".to_string()
    } else if hours >= 2 {
        format!("{} hours ago", hours)
    } else if hours == 1 {
        "1 hour ago".to_string()
    } else if minutes >= 2 {
        format!("{} minutes ago", minutes)
    } else {
        "just now".to_string()
    }
}

fn video_hue(id: &VideoId) -> f32 {
    let mut hash: u32 = 2166136261;
    for byte in id.0.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    (hash % 360) as f32
}

fn hue_to_color(hue: f32) -> Color32 {
    // HSL with S=30%, L=78% → pastel tones matching the design's SVG colors
    let h = hue / 360.0;
    let s = 0.30_f32;
    let l = 0.78_f32;

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = if h < 1.0 / 6.0 {
        (c, x, 0.0)
    } else if h < 2.0 / 6.0 {
        (x, c, 0.0)
    } else if h < 3.0 / 6.0 {
        (0.0, c, x)
    } else if h < 4.0 / 6.0 {
        (0.0, x, c)
    } else if h < 5.0 / 6.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let to_u8 = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(to_u8(r1), to_u8(g1), to_u8(b1))
}

fn darken(color: Color32, amount: f32) -> Color32 {
    let factor = (1.0 - amount).clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * factor) as u8,
        (color.g() as f32 * factor) as u8,
        (color.b() as f32 * factor) as u8,
        color.a(),
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_duration_under_one_hour() {
        assert_eq!(fmt_duration(0), "0:00");
        assert_eq!(fmt_duration(59), "0:59");
        assert_eq!(fmt_duration(60), "1:00");
        assert_eq!(fmt_duration(1122), "18:42");
    }

    #[test]
    fn fmt_duration_over_one_hour() {
        assert_eq!(fmt_duration(5047), "1:24:07");
        assert_eq!(fmt_duration(10032), "2:47:12");
    }

    #[test]
    fn fmt_views_ranges() {
        assert_eq!(fmt_views(842), "842");
        assert_eq!(fmt_views(1_000), "1.0K");
        assert_eq!(fmt_views(284_000), "284K");
        assert_eq!(fmt_views(1_400_000), "1.4M");
        assert_eq!(fmt_views(8_700_000), "8.7M");
        assert_eq!(fmt_views(10_000_000), "10M");
    }

    #[test]
    fn video_hue_in_range() {
        let id = VideoId::new("dQw4w9WgXcQ");
        let h = video_hue(&id);
        assert!((0.0..360.0).contains(&h), "hue {h} out of range");
    }

    #[test]
    fn video_hue_stable() {
        let id = VideoId::new("abc123");
        assert_eq!(video_hue(&id), video_hue(&id));
    }

    #[test]
    fn hue_to_color_extremes() {
        let c = hue_to_color(0.0);
        assert_eq!(c.a(), 255);
        let c = hue_to_color(359.9);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn phase_to_circle_phase_mapping() {
        assert_eq!(phase_to_circle_phase(&DownloadPhase::Queued), Phase::Queued);
        assert_eq!(
            phase_to_circle_phase(&DownloadPhase::Downloading(0.5)),
            Phase::Downloading(0.5)
        );
        assert_eq!(
            phase_to_circle_phase(&DownloadPhase::PostProcessing),
            Phase::PostProcessing
        );
        assert_eq!(phase_to_circle_phase(&DownloadPhase::Moving), Phase::Moving);
        assert_eq!(phase_to_circle_phase(&DownloadPhase::Done), Phase::Done);
        assert_eq!(
            phase_to_circle_phase(&DownloadPhase::Failed("oops".into())),
            Phase::Failed("oops".into())
        );
    }
}
