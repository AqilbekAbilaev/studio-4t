mod commands;
mod error;
mod keychain;
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
            let data_dir = match app.path().app_data_dir() {
                Ok(val) => val,
                Err(e) => return Err(e.into()),
            };
            app.manage(Storage::new(data_dir.join("connections.json")));
            app.manage(ConnectionPool::new());

            let native_menu = match menu::build(app.handle()) {
                Ok(val) => val,
                Err(e) => return Err(e.into()),
            };
            match app.set_menu(native_menu) {
                Ok(val) => val,
                Err(e) => return Err(e.into()),
            };
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
            create_collection,
            drop_database,
            find_documents,
            insert_document,
            replace_document,
            delete_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
