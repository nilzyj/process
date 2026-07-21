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
        cached_records: Arc::new(Mutex::new(None)),
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
                let cached_arc = app.state::<AppState>().cached_records.clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(pool) = db::connect(&config).await {
                        *pool_arc.lock().unwrap() = Some(pool.clone());
                        // Pre-fetch default query (status="进行中") into cache
                        let filter = models::RecordFilter {
                            search: None,
                            media_type: None,
                            status: Some("进行中".to_string()),
                            tag: None,
                            page: Some(1),
                            page_size: Some(200),
                        };
                        if let Ok(result) = db::list_records(&pool, filter).await {
                            *cached_arc.lock().unwrap() = Some(result);
                        }
                    }
                });
            }
            Ok(())
        })
        .on_page_load(|webview, payload| {
            if webview.label() == "main"
                && matches!(payload.event(), PageLoadEvent::Finished)
            {
                if let Some(splash) = webview.app_handle().get_webview_window("splashscreen") {
                    let _ = splash.close();
                }
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
            commands::get_cached_records,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
