//! Main application struct and eframe integration.
//!
//! [`YtDlpApp`] is a thin GUI shell that delegates all business logic to
//! [`crate::service::AppService`]. This separation allows the same logic
//! to be driven from a web frontend or native Swift app.

use crate::download::progress::DownloadPhase;
use crate::service::AppService;
use crate::settings::persistence;
use crate::ui;
use crate::ui::searchbar::{Searchbar, SearchbarEvent, SearchbarState};
use crate::ui::settings_overlay::SettingsOverlay;
use crate::ui::theme::Theme;

use chrono::Utc;
use eframe::egui;
use egui::{Color32, Pos2};

/// Central application state.
pub struct YtDlpApp {
    pub service: AppService,
    pub searchbar: SearchbarState,
    pub settings_overlay: SettingsOverlay,
    pub applied_theme: &'static Theme,
    queue_open: bool,
    start_time: std::time::Instant,
}

impl YtDlpApp {
    pub fn new() -> Self {
        let service = AppService::new();
        Self {
            service,
            searchbar: SearchbarState::default(),
            settings_overlay: SettingsOverlay::new(),
            applied_theme: ui::theme::theme(),
            queue_open: false,
            start_time: std::time::Instant::now(),
        }
    }

    fn active_download_count(&self) -> usize {
        self.service
            .download_phases
            .values()
            .filter(|p| {
                matches!(
                    p,
                    DownloadPhase::Downloading(_)
                        | DownloadPhase::PostProcessing
                        | DownloadPhase::Moving
                )
            })
            .count()
    }

    fn handle_searchbar_event(&mut self, event: SearchbarEvent) {
        match event {
            SearchbarEvent::Submit(q) => {
                tracing::debug!(query = %q, "searchbar submitted");
                self.service.submit_search(q);
            }
            SearchbarEvent::Clear => {
                tracing::debug!("searchbar cleared");
                self.service.clear_search();
            }
            SearchbarEvent::None => {}
        }
    }
}

/// Ambient bloom background — heartbeat pulse (~60 BPM: two bumps + silence).
fn paint_bloom(start_time: std::time::Instant, ui: &mut egui::Ui) {
    let t = start_time.elapsed().as_secs_f32();
    let rect = ui.clip_rect();

    let cycle = t % 1.0;
    let pulse = if cycle < 0.12 {
        (std::f32::consts::PI * cycle / 0.12).sin()
    } else if cycle < 0.18 {
        0.0
    } else if cycle < 0.30 {
        0.7 * (std::f32::consts::PI * (cycle - 0.18) / 0.12).sin()
    } else {
        0.0
    };

    let painter = ui.painter_at(rect);

    let blobs: &[(Pos2, f32, Color32)] = &[
        (Pos2::new(0.5, 0.45), 0.55, Color32::from_rgba_unmultiplied(255, 120, 80, (18.0 + pulse * 22.0) as u8)),
        (Pos2::new(0.18, 0.55), 0.40, Color32::from_rgba_unmultiplied(130, 100, 220, (10.0 + pulse * 14.0) as u8)),
        (Pos2::new(0.82, 0.40), 0.38, Color32::from_rgba_unmultiplied(255, 190, 60, (8.0 + pulse * 12.0) as u8)),
    ];

    for (rel, r_frac, color) in blobs {
        let cx = rect.left() + rel.x * rect.width();
        let cy = rect.top() + rel.y * rect.height();
        let radius = rect.width().min(rect.height()) * r_frac;
        let steps = 12u8;
        for step in 0..steps {
            let frac = step as f32 / steps as f32;
            let r = radius * (1.0 - frac * 0.85);
            let alpha = ((color.a() as f32) * (1.0 - frac)).round() as u8;
            painter.circle_filled(
                Pos2::new(cx, cy),
                r,
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha / steps),
            );
        }
    }
}

impl eframe::App for YtDlpApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.service.poll_progress();
        self.service.poll_search();

        // Sync theme if the system preference changed.
        let desired = ui::theme::resolve_theme(&self.service.settings.theme, ctx);
        let current = ui::theme::theme();
        if !std::ptr::eq(self.applied_theme as *const _, current as *const _) {
            ui::theme::apply(ctx, desired);
            self.applied_theme = ui::theme::theme();
        }

        // ── Top bar (always visible, including during settings) ───────────────
        let theme = ui::theme::theme();
        egui::TopBottomPanel::top("topbar")
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(20.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.service.searched && !self.settings_overlay.open {
                        let brand = ui.add(
                            egui::Label::new(
                                egui::RichText::new("yt-dlp")
                                    .color(theme.ink_3)
                                    .font(egui::FontId::monospace(13.0)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if brand.clicked() {
                            tracing::debug!("brand clicked → back to hero");
                            self.service.clear_search();
                            self.searchbar.query.clear();
                        }
                        ui.add_space(12.0);
                    }

                    let active = self.active_download_count();
                    let queue_label = if active > 0 { format!("↓ {active}") } else { "↓".into() };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(&queue_label)
                                    .color(if active > 0 { theme.accent } else { theme.ink_3 })
                                    .size(13.0),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.queue_open = !self.queue_open;
                        tracing::debug!(open = self.queue_open, active_downloads = active, "queue panel toggled");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("⚙").color(theme.ink_3).size(16.0),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.settings_overlay.open = !self.settings_overlay.open;
                            tracing::debug!(open = self.settings_overlay.open, "settings panel toggled");
                        }
                    });
                });
            });

        // ── Queue popover ─────────────────────────────────────────────────────
        if self.queue_open {
            egui::Window::new("Downloads")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::LEFT_TOP, egui::vec2(20.0, 50.0))
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 220))
                        .rounding(egui::Rounding::same(theme.r_xl))
                        .stroke(egui::Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 180)))
                        .inner_margin(egui::Margin::same(16.0))
                        .shadow(egui::Shadow {
                            offset: egui::Vec2::new(0.0, 8.0),
                            blur: 24.0,
                            spread: 0.0,
                            color: Color32::from_rgba_unmultiplied(0, 0, 0, 30),
                        }),
                )
                .show(ctx, |ui| {
                    ui.set_width(280.0);
                    ui.label(egui::RichText::new("QUEUE").color(theme.ink_4).font(egui::FontId::monospace(10.0)));
                    ui.add_space(8.0);

                    let mut any = false;
                    let video_ids: Vec<_> = self.service.results.iter().map(|v| v.id.clone()).collect();
                    let video_titles: Vec<_> = self.service.results.iter().map(|v| v.title.clone()).collect();
                    for (id, title) in video_ids.iter().zip(video_titles.iter()) {
                        if let Some(phase) = self.service.download_phases.get(id) {
                            any = true;
                            ui.horizontal(|ui| {
                                let short: String = title.chars().take(28).collect();
                                let suffix = if title.len() > 28 { "…" } else { "" };
                                ui.label(egui::RichText::new(format!("{short}{suffix}")).color(theme.ink_2).size(12.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    match phase {
                                        DownloadPhase::Downloading(pct) => {
                                            ui.label(egui::RichText::new(format!("{:.0}%", pct * 100.0)).color(theme.accent).font(egui::FontId::monospace(11.0)));
                                        }
                                        DownloadPhase::PostProcessing | DownloadPhase::Moving => {
                                            ui.label(egui::RichText::new("…").color(theme.ink_3).font(egui::FontId::monospace(11.0)));
                                        }
                                        DownloadPhase::Done => {
                                            ui.label(egui::RichText::new("✓").color(theme.accent).size(12.0));
                                        }
                                        DownloadPhase::Failed(_) => {
                                            ui.label(egui::RichText::new("✗").color(Color32::from_rgb(200, 60, 60)).size(12.0));
                                        }
                                        _ => {}
                                    }
                                });
                            });
                        }
                    }
                    if !any {
                        ui.label(egui::RichText::new("No active downloads").color(theme.ink_4).size(12.0));
                    }
                    ui.add_space(8.0);
                    if ui.add(egui::Button::new(egui::RichText::new("Close").size(12.0)).frame(false)).clicked() {
                        self.queue_open = false;
                    }
                });
        }

        // ── Central panel: bloom background + settings OR main content ─────────
        let panel_fill = ui::theme::theme().bg;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(panel_fill))
            .show(ctx, |ui| {
                let start = self.start_time;
                paint_bloom(start, ui);

                if self.settings_overlay.open {
                    let changed = self.settings_overlay.show(ui, &mut self.service.settings);
                    if changed {
                        tracing::debug!("settings saved to disk after change");
                        let _ = persistence::save(&self.service.settings);
                    }
                } else {
                    ui::layout::centered_content(ui, ui::layout::CONTENT_MAX_WIDTH, |ui| {
                        if !self.service.searched {
                            let available_height = ui.available_height();
                            ui.vertical_centered(|ui| {
                                ui.set_max_width(680.0);
                                ui.add_space(available_height * 0.25);
                                ui::hero::draw_hero(ui);
                                ui.add_space(24.0);
                                let event = Searchbar::new(&mut self.searchbar).show(ui);
                                self.handle_searchbar_event(event);
                            });
                        } else {
                            let event = Searchbar::new(&mut self.searchbar).show(ui);
                            self.handle_searchbar_event(event);

                            ui.add_space(8.0);
                            let header_resp = crate::ui::results_header::results_header(
                                ui,
                                self.service.results.len(),
                                &self.service.last_query,
                                self.service.selected.len(),
                                0,
                            );
                            if header_resp.batch_download_clicked {
                                self.service.download_selected();
                            }

                            ui.add_space(4.0);
                            let now = Utc::now();
                            let mut actions: Vec<(crate::api::types::VideoId, bool, bool)> = Vec::new();
                            egui::ScrollArea::vertical().id_salt("results_scroll").show(ui, |ui| {
                                for video in &self.service.results {
                                    let phase = self.service.download_phases.get(&video.id);
                                    let checked = self.service.selected.contains(&video.id);
                                    let resp = crate::ui::widget::Widget::show(ui, |ui| {
                                        crate::ui::result_row::result_row(ui, video, checked, phase, &now)
                                    })
                                    .inner;

                                    let toggled = resp.checkbox_changed.unwrap_or(checked) != checked;
                                    if toggled || resp.download_clicked {
                                        actions.push((video.id.clone(), resp.checkbox_changed.unwrap_or(checked), resp.download_clicked));
                                    }
                                }
                            });
                            for (id, selected, download) in actions {
                                if selected != self.service.selected.contains(&id) {
                                    self.service.toggle_selected(id.clone(), selected);
                                }
                                if download {
                                    tracing::debug!(id = %id.0, "result card clicked → download single");
                                    self.service.download_single(id);
                                }
                            }
                        }
                    });
                }
            });

        ctx.request_repaint();
    }
}
