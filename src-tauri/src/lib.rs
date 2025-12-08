mod commands;
mod downloader;
mod persistence;
mod state;
mod utils;

use persistence::{DownloadHistory, DownloadStatus};
use state::{AppState, Settings};
use std::collections::HashMap;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Load history and settings
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut history = DownloadHistory::load().await;
                let settings = Settings::load().await;

                // Mark any "Downloading" status as "Paused" since app was closed
                let mut needs_save = false;
                for record in history.downloads.values_mut() {
                    if record.status == DownloadStatus::Downloading {
                        record.status = DownloadStatus::Paused;
                        record.updated_at = chrono::Utc::now().timestamp();
                        needs_save = true;
                    }
                }
                if needs_save {
                    let _ = history.save().await;
                }

                let state = handle.state::<AppState>();
                *state.history.write().await = history;
                *state.settings.write().await = settings;
            });

            // Create system tray
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("WDM - Web Download Manager")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Handle window close - minimize to tray instead of quitting
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // Prevent the window from closing, just hide it
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            Ok(())
        })
        .manage(AppState {
            downloads: RwLock::new(HashMap::new()),
            settings: RwLock::new(Settings::default()),
            history: RwLock::new(DownloadHistory::default()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::fetch_url_info,
            commands::check_file_exists,
            commands::start_download,
            commands::resume_interrupted_download,
            commands::cancel_download,
            commands::pause_download,
            commands::resume_download,
            commands::set_connections,
            commands::get_connections,
            commands::get_download_folder,
            commands::set_download_folder,
            commands::reset_download_folder,
            commands::get_speed_limit,
            commands::set_speed_limit,
            commands::get_download_history,
            commands::clear_download_history,
            commands::remove_from_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}