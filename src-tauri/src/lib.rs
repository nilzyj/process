mod commands;
mod config;
mod db;
mod models;

use std::sync::{Arc, Mutex};
use commands::AppState;
use tauri::webview::PageLoadEvent;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        pool: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            // Pre-connect to DB in background while page loads
            if let Some(config) = config::load_config() {
                let pool_arc = app.state::<AppState>().pool.clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(pool) = db::connect(&config).await {
                        *pool_arc.lock().unwrap() = Some(pool);
                    }
                });
            }
            Ok(())
        })
        .on_page_load(|webview, payload| {
            if webview.label() == "main"
                && matches!(payload.event(), PageLoadEvent::Started)
            {
                let _ = webview.window().show();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_connection,
            commands::get_config,
            commands::save_config,
            commands::init_db,
            commands::list_records,
            commands::get_record,
            commands::add_record,
            commands::update_record,
            commands::delete_record,
            commands::get_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
