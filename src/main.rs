use eframe::egui;

mod api;
mod app;
mod debug;
mod download;
mod i18n;
mod paths;
mod service;
mod settings;
mod ui;

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // ── Debug mode ────────────────────────────────────────────────────────────
    let is_debug = std::env::args().any(|a| a == "--debug");
    if is_debug {
        debug::enable();
    }

    let filter = if is_debug {
        // Debug output for our crate only; all third-party crates stay at warn/info.
        "yt_dlp_gui=debug,info,wgpu=warn,wgpu_core=warn,wgpu_hal=warn,naga=warn,\
         egui_wgpu=warn,eframe=warn,winit=warn,ehttp=info"
    } else {
        "info,wgpu=warn,wgpu_core=warn,wgpu_hal=warn,naga=warn,egui_wgpu=warn"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)   // cleaner output: no module path prefix
        .init();

    if is_debug {
        tracing::info!("╔══════════════════════════════╗");
        tracing::info!("║  yt-dlp-gui  DEBUG MODE ON   ║");
        tracing::info!("╚══════════════════════════════╝");
    }

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("failed to build tokio runtime: {e}");
            return Err(anyhow::anyhow!("failed to build tokio runtime: {e}"));
        }
    };
    let _enter = runtime.enter();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("yt-dlp"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "yt-dlp",
        native_options,
        Box::new(|cc| {
            // Install image loaders so egui can fetch/decode thumbnails via from_uri().
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // fonts::install(&cc.egui_ctx);
            let mut app = app::YtDlpApp::new();
            let mode = ui::theme::resolve_theme(&app.service.settings.theme, &cc.egui_ctx);
            ui::theme::apply(&cc.egui_ctx, mode);
            app.applied_theme = ui::theme::theme();
            Ok(Box::new(app))
        }),
    ) {
        tracing::error!("eframe::run_native failed: {e}");
        return Err(anyhow::anyhow!("eframe::run_native failed: {e}"));
    }

    Ok(())
}
