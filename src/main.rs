#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod commands;
mod debug;
mod download;
mod i18n;
mod paths;
mod service;
mod settings;

use crate::commands::ServiceState;
use crate::service::events::bridge_to_tauri;
use crate::service::AppService;

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let is_debug = std::env::args().any(|a| a == "--debug");
    if is_debug {
        debug::enable();
    }

    let filter = if is_debug {
        "yt_dlp_gui=debug,info,ehttp=info"
    } else {
        "info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    if is_debug {
        tracing::info!("yt-dlp-gui DEBUG MODE ON");
        debug::log_bin_paths();
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build tokio runtime: {e}"))?;
    let _guard = runtime.enter();

    let service = AppService::new();
    let event_rx = service.subscribe();
    let state = ServiceState(service);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(move |app| {
            let handle = app.handle().clone();
            tokio::spawn(bridge_to_tauri(handle, event_rx));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::get_results,
            commands::get_phases,
            commands::get_search_status,
            commands::submit_search,
            commands::clear_search,
            commands::download_single,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("tauri runtime error: {e}"))?;

    Ok(())
}
