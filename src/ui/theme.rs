//! Design-token theme: OKLCH → linear-sRGB → `egui::Color32`, plus radii and
//! shadow definitions sourced 1:1 from `claude Design/yt-dlp/project/styles.css`.
//!
//! The conversion path is `palette::Oklch` → `palette::Srgb` (linear) →
//! gamma-encoded 8-bit per channel via `palette::Srgb::into_format::<u8>()`,
//! which performs the sRGB transfer-function. Results are cached in a
//! `once_cell::sync::Lazy<Theme>` since the inputs are compile-time constants.
//!
//! NOTE: OKLCH lightness in `styles.css` is already normalized to `0..1`
//! (the CSS spec also accepts that form). Our `oklch()` helper takes
//! `(L_norm, C, H_deg)` directly and feeds them to `palette::Oklch::new`.

#![allow(dead_code)] // tokens consumed by downstream UI tickets (#09–#18)

use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui;
use once_cell::sync::Lazy;
use palette::{FromColor, Oklch, Srgb};

use crate::settings::Theme as SettingsTheme;

/// Pre-computed shadow definition. egui has no native multi-layer shadow, so
/// glass surfaces will draw these as stacked rounded rectangles in `glass/`.
#[derive(Clone, Copy, Debug)]
pub struct ShadowDef {
    pub offset: egui::Vec2,
    pub blur: f32,
    pub color: egui::Color32,
}

/// All design tokens resolved to egui-native types.
#[derive(Clone, Debug)]
pub struct Theme {
    // Surface
    pub bg: egui::Color32,
    pub bg_elev: egui::Color32,
    pub bg_sunk: egui::Color32,

    // Lines / borders
    pub line: egui::Color32,
    pub line_soft: egui::Color32,

    // Ink (text) — primary → quaternary
    pub ink: egui::Color32,
    pub ink_2: egui::Color32,
    pub ink_3: egui::Color32,
    pub ink_4: egui::Color32,

    // Accent
    pub accent: egui::Color32,
    pub accent_soft: egui::Color32,

    // Radii (px)
    pub r_sm: f32,
    pub r_md: f32,
    pub r_lg: f32,
    pub r_xl: f32,

    // Shadows (sm / md-pair / lg-pair) — stacked layers, in paint-order.
    pub shadow_sm: [ShadowDef; 1],
    pub shadow_md: [ShadowDef; 2],
    pub shadow_lg: [ShadowDef; 2],
}

/// Convert an OKLCH triple `(L, C, H)` with `L ∈ [0,1]`, `C ∈ [0,~0.4]`,
/// `H ∈ [0,360)` into a non-premultiplied 8-bit-per-channel sRGB color
/// (alpha = 255). The conversion uses the `palette` crate to go through
/// linear-sRGB and then gamma-encode to the standard sRGB transfer function.
pub fn oklch_to_color32(l: f32, c: f32, h_deg: f32) -> egui::Color32 {
    oklch_to_color32_a(l, c, h_deg, 1.0)
}

/// Same as [`oklch_to_color32`] but with explicit alpha in `[0, 1]`.
pub fn oklch_to_color32_a(l: f32, c: f32, h_deg: f32, alpha: f32) -> egui::Color32 {
    let oklch: Oklch<f32> = Oklch::new(l, c, h_deg);
    // Oklch -> sRGB (gamma-encoded float).
    let encoded_f: Srgb<f32> = Srgb::from_color(oklch);
    // float -> 8-bit per channel.
    let encoded: Srgb<u8> = encoded_f.into_format();
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgba_unmultiplied(encoded.red, encoded.green, encoded.blue, a)
}

/// Convert an sRGB triple with an alpha fraction into a `Color32`.
fn rgba(r: u8, g: u8, b: u8, alpha: f32) -> egui::Color32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// Build the light theme using the design's OKLCH tokens (styles.css).
fn build_light_theme() -> Theme {
    // --bg: oklch(0.985 0.004 80)   — warm off-white
    let bg = oklch_to_color32(0.985, 0.004, 80.0);
    // --bg-elev: oklch(1 0 0)       — pure white for elevated surfaces
    let bg_elev = egui::Color32::from_rgb(255, 255, 255);
    // --bg-sunk: oklch(0.965 0.005 80)
    let bg_sunk = oklch_to_color32(0.965, 0.005, 80.0);

    // --line: oklch(0.92 0.005 80 / 0.7)
    let line = oklch_to_color32_a(0.92, 0.005, 80.0, 0.7);
    // --line-soft: oklch(0.945 0.004 80 / 0.6)
    let line_soft = oklch_to_color32_a(0.945, 0.004, 80.0, 0.6);

    // --ink: oklch(0.18 0.01 60)
    let ink = oklch_to_color32(0.18, 0.01, 60.0);
    // --ink-2: oklch(0.42 0.01 60)
    let ink_2 = oklch_to_color32(0.42, 0.01, 60.0);
    // --ink-3: oklch(0.58 0.008 60)
    let ink_3 = oklch_to_color32(0.58, 0.008, 60.0);
    // --ink-4: oklch(0.72 0.006 60)
    let ink_4 = oklch_to_color32(0.72, 0.006, 60.0);

    // --accent: oklch(0.62 0.14 30)  — warm coral/orange (hue 30°)
    let accent = oklch_to_color32(0.62, 0.14, 30.0);
    // --accent-soft: oklch(0.94 0.04 30)
    let accent_soft = oklch_to_color32(0.94, 0.04, 30.0);

    // Shadow colours from CSS: oklch(0.2 0.01 60 / alpha)
    let shadow_ink = |alpha: f32| oklch_to_color32_a(0.2, 0.01, 60.0, alpha);

    let shadow_sm = [ShadowDef {
        offset: egui::vec2(0.0, 1.0),
        blur: 2.0,
        color: shadow_ink(0.05),
    }];
    let shadow_md = [
        ShadowDef {
            offset: egui::vec2(0.0, 6.0),
            blur: 20.0,
            color: shadow_ink(0.08),
        },
        ShadowDef {
            offset: egui::vec2(0.0, 1.0),
            blur: 3.0,
            color: shadow_ink(0.05),
        },
    ];
    let shadow_lg = [
        ShadowDef {
            offset: egui::vec2(0.0, 30.0),
            blur: 80.0,
            color: shadow_ink(0.14),
        },
        ShadowDef {
            offset: egui::vec2(0.0, 8.0),
            blur: 24.0,
            color: shadow_ink(0.08),
        },
    ];

    Theme {
        bg,
        bg_elev,
        bg_sunk,
        line,
        line_soft,
        ink,
        ink_2,
        ink_3,
        ink_4,
        accent,
        accent_soft,
        r_sm: 10.0,
        r_md: 14.0,
        r_lg: 20.0,
        r_xl: 26.0,
        shadow_sm,
        shadow_md,
        shadow_lg,
    }
}

/// Build the dark theme using Apple system colors. Run once and cached.
fn build_dark_theme() -> Theme {
    // Surface tones (Apple system background).
    let bg = egui::Color32::from_rgb(0, 0, 0);
    let bg_elev = egui::Color32::from_rgb(28, 28, 30);
    let bg_sunk = egui::Color32::from_rgb(44, 44, 46);

    // Lines / separators.
    let line = rgba(58, 58, 60, 0.9);
    let line_soft = rgba(72, 72, 74, 0.75);

    // Ink ramp (label colors).
    let ink = egui::Color32::from_rgb(255, 255, 255);
    let ink_2 = egui::Color32::from_rgb(235, 235, 245);
    let ink_3 = egui::Color32::from_rgb(199, 199, 204);
    let ink_4 = egui::Color32::from_rgb(142, 142, 147);

    // Accent (Apple system blue, dark appearance).
    let accent = egui::Color32::from_rgb(10, 132, 255);
    let accent_soft = rgba(10, 132, 255, 0.25);

    // Shadow layer color (subtle black).
    let shadow_ink = |alpha: f32| rgba(0, 0, 0, alpha);

    let shadow_sm = [ShadowDef {
        offset: egui::vec2(0.0, 1.0),
        blur: 2.0,
        color: shadow_ink(0.2),
    }];
    let shadow_md = [
        ShadowDef {
            offset: egui::vec2(0.0, 6.0),
            blur: 20.0,
            color: shadow_ink(0.28),
        },
        ShadowDef {
            offset: egui::vec2(0.0, 1.0),
            blur: 3.0,
            color: shadow_ink(0.2),
        },
    ];
    let shadow_lg = [
        ShadowDef {
            offset: egui::vec2(0.0, 30.0),
            blur: 80.0,
            color: shadow_ink(0.35),
        },
        ShadowDef {
            offset: egui::vec2(0.0, 8.0),
            blur: 24.0,
            color: shadow_ink(0.28),
        },
    ];

    Theme {
        bg,
        bg_elev,
        bg_sunk,
        line,
        line_soft,
        ink,
        ink_2,
        ink_3,
        ink_4,
        accent,
        accent_soft,
        r_sm: 10.0,
        r_md: 14.0,
        r_lg: 20.0,
        r_xl: 26.0,
        shadow_sm,
        shadow_md,
        shadow_lg,
    }
}

const THEME_LIGHT: usize = 0;
const THEME_DARK: usize = 1;

static LIGHT_THEME: Lazy<Theme> = Lazy::new(build_light_theme);
static DARK_THEME: Lazy<Theme> = Lazy::new(build_dark_theme);
static ACTIVE_THEME: AtomicUsize = AtomicUsize::new(THEME_LIGHT);

fn theme_for(mode: egui::Theme) -> &'static Theme {
    match mode {
        egui::Theme::Dark => &DARK_THEME,
        egui::Theme::Light => &LIGHT_THEME,
    }
}

/// Resolve the user's theme preference to a concrete egui theme.
pub fn resolve_theme(preference: &SettingsTheme, ctx: &egui::Context) -> egui::Theme {
    match preference {
        SettingsTheme::Light => egui::Theme::Light,
        SettingsTheme::Dark => egui::Theme::Dark,
        SettingsTheme::System => ctx.system_theme().unwrap_or(egui::Theme::Light),
    }
}

/// Human-friendly theme label for logs.
pub fn theme_label(mode: egui::Theme) -> &'static str {
    match mode {
        egui::Theme::Dark => "Dark",
        egui::Theme::Light => "Light",
    }
}

fn set_active_theme(mode: egui::Theme) {
    let index = match mode {
        egui::Theme::Dark => THEME_DARK,
        egui::Theme::Light => THEME_LIGHT,
    };
    ACTIVE_THEME.store(index, Ordering::Relaxed);
}

/// Returns a reference to the active application theme.
pub fn theme() -> &'static Theme {
    match ACTIVE_THEME.load(Ordering::Relaxed) {
        THEME_DARK => &DARK_THEME,
        _ => &LIGHT_THEME,
    }
}

/// Apply the theme to an egui context: colors, rounding, spacing, fonts size.
/// Idempotent — safe to call multiple times.
pub fn apply(ctx: &egui::Context, mode: egui::Theme) {
    let t = theme_for(mode);
    set_active_theme(mode);

    ctx.style_mut(|style| {
        let visuals = &mut style.visuals;

        *visuals = match mode {
            egui::Theme::Dark => egui::Visuals::dark(),
            egui::Theme::Light => egui::Visuals::light(),
        };

        visuals.override_text_color = Some(t.ink);
        visuals.panel_fill = t.bg;
        visuals.window_fill = t.bg_elev;
        visuals.window_stroke = egui::Stroke::new(0.5, t.line);
        visuals.extreme_bg_color = t.bg_sunk;
        visuals.faint_bg_color = t.bg_sunk;
        visuals.code_bg_color = t.bg_sunk;
        visuals.hyperlink_color = t.accent;
        visuals.selection.bg_fill = t.accent_soft;
        visuals.selection.stroke = egui::Stroke::new(1.0, t.accent);

        // Widget visual state palette — non-interactive baseline plus interactive.
        let r_md = egui::Rounding::same(t.r_md);
        let r_sm = egui::Rounding::same(t.r_sm);
        let transparent = egui::Color32::TRANSPARENT;

        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = transparent;
        widgets.noninteractive.weak_bg_fill = transparent;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, t.line);
        widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, t.ink);
        widgets.noninteractive.rounding = r_md;

        widgets.inactive.bg_fill = transparent;
        widgets.inactive.weak_bg_fill = transparent;
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0, t.line);
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0, t.ink_2);
        widgets.inactive.rounding = r_sm;

        widgets.hovered.bg_fill = transparent;
        widgets.hovered.weak_bg_fill = transparent;
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0, t.line);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0, t.ink);
        widgets.hovered.rounding = r_sm;

        widgets.active.bg_fill = transparent;
        widgets.active.weak_bg_fill = transparent;
        widgets.active.bg_stroke = egui::Stroke::new(1.0, t.accent);
        widgets.active.fg_stroke = egui::Stroke::new(1.0, t.ink);
        widgets.active.rounding = r_sm;

        widgets.open.bg_fill = transparent;
        widgets.open.weak_bg_fill = transparent;
        widgets.open.bg_stroke = egui::Stroke::new(1.0, t.line);
        widgets.open.fg_stroke = egui::Stroke::new(1.0, t.ink);
        widgets.open.rounding = r_sm;

        // Spacing roughly matches the design's airy, generous gutters.
        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(8.0, 8.0);
        spacing.button_padding = egui::vec2(12.0, 8.0);
        spacing.window_margin = egui::Margin::same(16.0);
        spacing.menu_margin = egui::Margin::same(8.0);

        // Bigger default text sizes — design uses 14px body / large hero.
        let text_styles = &mut style.text_styles;
        text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(28.0, egui::FontFamily::Proportional),
        );
        text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
        );
        text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compute the converted RGB triple for assertions.
    fn rgb(l: f32, c: f32, h: f32) -> (u8, u8, u8) {
        let col = oklch_to_color32(l, c, h);
        (col.r(), col.g(), col.b())
    }

    /// `|a - b| <= tol` for `u8`s.
    fn close(a: u8, b: u8, tol: u8) -> bool {
        (a as i32 - b as i32).unsigned_abs() <= tol as u32
    }

    #[test]
    fn oklch_white_converts_to_white() {
        let (r, g, b) = rgb(1.0, 0.0, 0.0);
        assert!(
            close(r, 255, 2) && close(g, 255, 2) && close(b, 255, 2),
            "expected ~(255,255,255), got ({r},{g},{b})"
        );
    }

    #[test]
    fn oklch_black_converts_to_black() {
        let (r, g, b) = rgb(0.0, 0.0, 0.0);
        assert!(
            close(r, 0, 2) && close(g, 0, 2) && close(b, 0, 2),
            "expected ~(0,0,0), got ({r},{g},{b})"
        );
    }

    /// The accent (warm orange-red) must be a non-grey warm color:
    /// red dominates, green > blue, and red is bright.
    #[test]
    fn oklch_accent_is_warm() {
        let (r, g, b) = rgb(0.62, 0.14, 30.0);
        assert!(r > g, "expected R > G, got R={r} G={g}");
        assert!(g > b, "expected G > B, got G={g} B={b}");
        assert!(r > 150, "expected R > 150 (bright warm), got R={r}");
    }

    #[test]
    fn theme_radii_match_styles_css() {
        let t = theme();
        assert_eq!(t.r_sm, 10.0);
        assert_eq!(t.r_md, 14.0);
        assert_eq!(t.r_lg, 20.0);
        assert_eq!(t.r_xl, 26.0);
    }

    #[test]
    fn theme_caches_consistent_colors() {
        let a = theme();
        let b = theme();
        assert_eq!(a.bg, b.bg);
        assert_eq!(a.accent, b.accent);
    }
}
