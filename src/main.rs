#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

mod api;
mod commands;
mod debug;
mod download;
mod i18n;
mod paths;
mod service;
mod settings;

use crate::commands::ServiceState;
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
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build tokio runtime: {e}"))?;
    let _guard = runtime.enter();

    let service = AppService::new();
    let state = ServiceState(Mutex::new(service));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::get_results,
            commands::get_phases,
            commands::get_selected,
            commands::submit_search,
            commands::clear_search,
            commands::poll,
            commands::toggle_selected,
            commands::download_single,
            commands::download_selected,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("tauri runtime error: {e}"))?;

    Ok(())
}
