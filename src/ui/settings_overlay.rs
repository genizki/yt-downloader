//! Settings overlay — rendered inside the main CentralPanel so the bloom
//! background stays visible behind the cards.
#![allow(dead_code)]

use eframe::egui::{self, FontId, Key, RichText, ScrollArea, TextEdit};

use crate::settings::{
    AppSettings, AudioBitrate, Codec, Format, MaxSize, Protocol, Quality, Theme,
};
use crate::ui::settings::{
    segmented::segmented,
    toggle::toggle,
    token_input::{token_input, TokenInputState},
};
use crate::ui::theme::theme;
use crate::ui::widget::Widget;

const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Left column: section name (monospace uppercase label).
const SEC_LABEL_W: f32 = 160.0;
/// Left sub-column within rows: row label + hint.
const ROW_LABEL_W: f32 = 180.0;
/// Gap between section label and rows.
const SEC_GAP: f32 = 20.0;
/// Gap between row label and control.
const ROW_GAP: f32 = 16.0;

pub struct SettingsOverlay {
    pub open: bool,
    pub token_state: TokenInputState,
    api_key_visible: bool,
}

impl SettingsOverlay {
    pub fn new() -> Self {
        Self { open: false, token_state: TokenInputState::new(), api_key_visible: false }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, settings: &mut AppSettings) -> bool {
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.open = false;
            return false;
        }
        if !self.token_state.editing && self.token_state.value != settings.auth_token {
            self.token_state.value = settings.auth_token.clone();
        }

        let mut changed = false;
        let t = theme();

        crate::ui::layout::centered_content(ui, crate::ui::layout::CONTENT_MAX_WIDTH, |ui| {
            // ── Header ────────────────────────────────────────────────────────
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Preferences").size(11.0).color(t.ink_3));
                    ui.label(RichText::new("Settings").size(22.0).color(t.ink).strong());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("← Back").color(t.ink_2).size(13.0)).frame(false))
                        .clicked()
                    {
                        self.open = false;
                    }
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ScrollArea::vertical().id_salt("settings_scroll").show(ui, |ui| {
                // ── Downloads ─────────────────────────────────────────────────
                Widget::show(ui, |ui| {
                    let audio_only = settings.format.is_audio_only();
                    sec_layout(ui, "DOWNLOADS", |ui| {
                        sec_row(ui, "Format", "Output container", true, |ui| {
                            if segmented(ui, &[
                                ("MP4", Format::Mp4), ("MKV", Format::Mkv),
                                ("WebM", Format::WebM), ("MOV", Format::Mov),
                                ("MP3", Format::Mp3), ("M4A", Format::M4a),
                                ("AAC", Format::Aac), ("ALAC", Format::Alac),
                                ("AIFF", Format::Aiff), ("FLAC", Format::Flac),
                            ], &mut settings.format, false) { changed = true; }
                        });
                        sec_row(ui, "Video quality", "Max resolution", false, |ui| {
                            if segmented(ui, &[
                                ("360p", Quality::P360), ("720p", Quality::P720),
                                ("1080p", Quality::P1080), ("1440p", Quality::P1440),
                                ("2160p", Quality::P2160),
                            ], &mut settings.quality, audio_only) { changed = true; }
                        });
                        sec_row(ui, "Video codec", "Encoding", false, |ui| {
                            if segmented(ui, &[
                                ("H.264", Codec::H264), ("H.265", Codec::H265),
                                ("VP9", Codec::Vp9), ("AV1", Codec::Av1),
                            ], &mut settings.codec, audio_only) { changed = true; }
                        });
                        sec_row(ui, "Audio quality", "Bitrate", false, |ui| {
                            if segmented(ui, &[
                                ("96 kbps", AudioBitrate::K96), ("128 kbps", AudioBitrate::K128),
                                ("192 kbps", AudioBitrate::K192), ("256 kbps", AudioBitrate::K256),
                                ("320 kbps", AudioBitrate::K320),
                            ], &mut settings.audio_bitrate, false) { changed = true; }
                        });
                        sec_row(ui, "Download path", "Save location", false, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .add(egui::Button::new(RichText::new("📁").size(14.0)).frame(false))
                                    .on_hover_text("Browse…")
                                    .clicked()
                                {
                                    if let Some(folder) = rfd::FileDialog::new()
                                        .set_directory(&settings.download_path)
                                        .pick_folder()
                                    {
                                        settings.download_path = folder;
                                        changed = true;
                                    }
                                }
                                let mut path_str = settings.download_path.to_string_lossy().into_owned();
                                let r = ui.add(TextEdit::singleline(&mut path_str).desired_width(200.0));
                                if r.changed() { settings.download_path = path_str.into(); changed = true; }
                            });
                        });
                        sec_row(ui, "Auto-download", "Start on paste", false, |ui| {
                            if toggle(ui, &mut settings.playlist_auto_download, "") { changed = true; }
                        });
                    });
                });

                // ── Constraints ───────────────────────────────────────────────
                Widget::show(ui, |ui| {
                    sec_layout(ui, "CONSTRAINTS", |ui| {
                        sec_row(ui, "Max file size", "Skip larger files", true, |ui| {
                            if segmented(ui, &[
                                ("No limit", MaxSize::NoLimit), ("50 MB", MaxSize::Mb50),
                                ("100 MB", MaxSize::Mb100), ("500 MB", MaxSize::Mb500),
                                ("1 GB", MaxSize::Gb1),
                            ], &mut settings.max_size, false) { changed = true; }
                        });
                        sec_row(ui, "Protocol", "Network method", false, |ui| {
                            if segmented(ui, &[
                                ("Auto", Protocol::Auto), ("HTTPS", Protocol::Https),
                                ("HTTP", Protocol::Http), ("HLS", Protocol::Hls),
                                ("DASH", Protocol::Dash),
                            ], &mut settings.protocol, false) { changed = true; }
                        });
                    });
                });

                // ── Extras ─────────────────────────────────────────────────────
                Widget::show(ui, |ui| {
                    sec_layout(ui, "EXTRAS", |ui| {
                        let extras = &mut settings.extras;
                        macro_rules! extra_row {
                            ($i:expr, $label:literal, $hint:literal, $field:expr) => {
                                sec_row(ui, $label, $hint, $i == 0, |ui| {
                                    if toggle(ui, &mut $field, "") { changed = true; }
                                });
                            };
                        }
                        extra_row!(0, "Embed thumbnail",  "Cover art in file",    extras.embed_thumbnail);
                        extra_row!(1, "Embed metadata",   "Title, artist, etc.",  extras.embed_metadata);
                        extra_row!(2, "Embed chapters",   "Chapter markers",      extras.embed_chapters);
                        extra_row!(3, "Embed subtitles",  "Subtitles in file",    extras.embed_subtitles);
                        extra_row!(4, "Write subtitles",  "Save .srt/.vtt files", extras.write_subtitles);
                        extra_row!(5, "Skip playlists",   "Single video only",    extras.skip_playlists);
                        extra_row!(6, "ASCII filenames",  "Restrict to ASCII",    extras.restrict_names);
                    });
                });

                // ── Appearance ─────────────────────────────────────────────────
                Widget::show(ui, |ui| {
                    sec_layout(ui, "APPEARANCE", |ui| {
                        sec_row(ui, "Theme", "Color scheme", true, |ui| {
                            if segmented(ui, &[
                                ("Light", Theme::Light), ("Dark", Theme::Dark), ("System", Theme::System),
                            ], &mut settings.theme, false) { changed = true; }
                        });
                        sec_row(ui, "Language", "Interface language", false, |ui| {
                            if segmented(ui, &[
                                ("English", "en".to_owned()), ("Deutsch", "de".to_owned()),
                            ], &mut settings.language, false) { changed = true; }
                        });
                    });
                });

                // ── Authentication ─────────────────────────────────────────────
                Widget::show(ui, |ui| {
                    sec_layout(ui, "AUTH", |ui| {
                        sec_row(ui, "Auth token", "Private content", true, |ui| {
                            if token_input(ui, &mut self.token_state) {
                                settings.auth_token = self.token_state.value.clone();
                                changed = true;
                            }
                            if self.token_state.value != settings.auth_token {
                                settings.auth_token = self.token_state.value.clone();
                                changed = true;
                            }
                            if !self.token_state.value.is_empty()
                                && ui.small_button("Clear").clicked()
                            {
                                self.token_state = TokenInputState::new();
                                settings.auth_token.clear();
                                changed = true;
                            }
                        });
                        sec_row(ui, "YouTube API key", "For search results", false, |ui| {
                            let eye = if self.api_key_visible { "🙈" } else { "👁" };
                            if ui
                                .add(egui::Button::new(RichText::new(eye).size(14.0)).frame(false))
                                .on_hover_text(if self.api_key_visible { "Hide" } else { "Show" })
                                .clicked()
                            {
                                self.api_key_visible = !self.api_key_visible;
                            }
                            let key_resp = ui.add(
                                TextEdit::singleline(&mut settings.youtube_api_key)
                                    .hint_text("AIzaSy…")
                                    .password(!self.api_key_visible)
                                    .desired_width(200.0),
                            );
                            if key_resp.changed() { changed = true; }
                        });
                    });
                });

                // ── Footer ──────────────────────────────────────────────────────
                Widget::show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("yt-dlp-gui v{VERSION}"))
                                .color(theme().ink_4)
                                .size(11.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new(RichText::new("Done")).fill(theme().accent))
                                .clicked()
                            {
                                self.open = false;
                            }
                        });
                    });
                });
            });
        });

        if changed {
            tracing::debug!(
                format = ?settings.format,
                quality = ?settings.quality,
                codec = ?settings.codec,
                audio_bitrate = ?settings.audio_bitrate,
                download_path = %settings.download_path.display(),
                theme = ?settings.theme,
                "settings changed"
            );
            crate::debug::log_ytdlp_command(settings);
        }

        changed
    }
}

impl Default for SettingsOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Two-column card layout matching the design's `.sec`:
/// left = monospace section label (SEC_LABEL_W), right = rows content.
fn sec_layout(ui: &mut egui::Ui, name: &str, rows: impl FnOnce(&mut egui::Ui)) {
    let t = theme();
    ui.horizontal(|ui| {
        // Left: section label
        ui.allocate_ui_with_layout(
            egui::vec2(SEC_LABEL_W, ui.available_height().max(40.0)),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new(name)
                        .font(FontId::monospace(11.0))
                        .color(t.ink_3),
                );
            },
        );
        ui.add_space(SEC_GAP);
        // Right: rows
        ui.vertical(|ui| {
            rows(ui);
        });
    });
}

/// Single settings row: left = label+hint (ROW_LABEL_W), right = control (right-aligned).
/// `first` suppresses the top separator.
fn sec_row(
    ui: &mut egui::Ui,
    label: &str,
    hint: &str,
    first: bool,
    control: impl FnOnce(&mut egui::Ui),
) {
    let t = theme();
    if !first {
        ui.add(egui::Separator::default().spacing(0.0));
    }
    ui.horizontal(|ui| {
        // Label + hint (left column)
        ui.allocate_ui_with_layout(
            egui::vec2(ROW_LABEL_W, 50.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);
                ui.label(RichText::new(label).color(t.ink).size(14.0));
                ui.label(RichText::new(hint).color(t.ink_3).size(12.0));
            },
        );
        ui.add_space(ROW_GAP);
        // Control (right-aligned, takes remaining width)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            control(ui);
        });
    });
    ui.add_space(4.0);
}
