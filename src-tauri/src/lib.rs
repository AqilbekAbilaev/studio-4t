mod commands;
mod default_queries;
mod error;
mod history;
#[cfg(test)]
mod integration_tests;
mod keychain;
mod known_hosts;
mod menu;
mod persist;
mod pool;
mod saved_queries;
mod settings;
mod shell;
mod ssh;
mod shell_history;
mod storage;
mod tabs;
mod uri;

use commands::*;
use default_queries::DefaultQueryStorage;
use history::HistoryStorage;
use known_hosts::KnownHostsStore;
use pool::ConnectionPool;
use std::sync::Arc;
use saved_queries::SavedQueryStorage;
use settings::SettingsStorage;
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
            app.manage(SettingsStorage::new(data_dir.join("settings.json")));
            app.manage(TabStorage::new(data_dir.join("tabs.json")));
            // The host-key trust store is shared between the pool (real connect)
            // and the test_ssh_connection command, so both honor the same TOFU
            // record. Managed as an Arc so the pool can own a clone.
            let known_hosts = Arc::new(KnownHostsStore::new(data_dir.join("known_hosts.json")));
            app.manage(Arc::clone(&known_hosts));
            // Broker for interactive first-contact host-key prompts; shared with
            // the pool so a tunnel established on connect can raise the prompt.
            let host_key_prompts = Arc::new(ssh::HostKeyPrompts::new());
            app.manage(Arc::clone(&host_key_prompts));
            app.manage(ConnectionPool::new(
                Arc::clone(&known_hosts),
                Arc::clone(&host_key_prompts),
                app.handle().clone(),
            ));
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
            test_ssh_connection,
            respond_ssh_host_key,
            forget_ssh_host,
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
            count_documents,
            kill_query,
            server_status,
            connection_uri,
            duplicate_connection,
            export_connections,
            import_connections,
            get_settings,
            update_settings,
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
            analyze_schema,
            translate_sql,
            export_masked_collection,
            collection_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
