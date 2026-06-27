mod commands;
mod default_queries;
mod error;
mod history;
mod keychain;
mod menu;
mod pool;
mod saved_queries;
mod shell;
mod shell_history;
mod storage;
mod tabs;
mod uri;

use commands::*;
use default_queries::DefaultQueryStorage;
use history::HistoryStorage;
use pool::ConnectionPool;
use saved_queries::SavedQueryStorage;
use shell::ShellEngine;
use shell_history::ShellHistoryStorage;
use storage::Storage;
use tabs::TabStorage;
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
            app.manage(HistoryStorage::new(data_dir.join("history.json")));
            app.manage(SavedQueryStorage::new(data_dir.join("saved_queries.json")));
            app.manage(DefaultQueryStorage::new(data_dir.join("default_queries.json")));
            app.manage(TabStorage::new(data_dir.join("tabs.json")));
            app.manage(ConnectionPool::new());
            app.manage(ShellEngine::new());
            app.manage(ShellHistoryStorage::new(
                data_dir.join("shell_history.json"),
            ));

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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            test_connection,
            save_connection,
            update_connection,
            list_connections,
            delete_connection,
            disconnect,
            set_connection_tag,
            set_connection_open,
            update_last_accessed,
            open_connect_window,
            list_databases,
            create_collection,
            drop_database,
            drop_collection,
            rename_collection,
            create_database,
            find_documents,
            insert_document,
            replace_document,
            delete_document,
            explain_query,
            list_indexes,
            create_index,
            drop_index,
            run_aggregate,
            export_collection,
            import_collection,
            get_default_query,
            set_default_query,
            clear_default_query,
            get_open_tabs,
            set_open_tabs,
            list_saved_queries,
            save_query,
            delete_saved_query,
            get_query_history,
            push_query_history,
            clear_query_history,
            run_shell_command,
            close_shell_session,
            get_shell_history,
            push_shell_command,
            clear_shell_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
