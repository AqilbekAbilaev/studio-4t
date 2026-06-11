mod commands;
mod error;
mod menu;
mod pool;
mod storage;
mod uri;

use commands::*;
use pool::ConnectionPool;
use storage::Storage;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            app.manage(Storage::new(data_dir.join("connections.json")));
            app.manage(ConnectionPool::new());

            let native_menu = menu::build(app.handle())?;
            app.set_menu(native_menu)?;
            app.on_menu_event(menu::handle_event);

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            test_connection,
            save_connection,
            list_connections,
            delete_connection,
            disconnect,
            set_connection_tag,
            update_last_accessed,
            open_connect_window,
            list_databases,
            find_documents,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
