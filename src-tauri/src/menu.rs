use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry,
};

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;

    // File
    let connect = MenuItem::with_id(app, "file:connect", "Connect...", true, None::<&str>)?;
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &connect,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("Exit"))?,
        ],
    )?;

    // Edit — predefined items give macOS the standard Cmd+C/V/Z shortcuts
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    // Placeholder menus — items will be wired up as features are built
    let database = Submenu::new(app, "Database", true)?;
    let collection = Submenu::new(app, "Collection", true)?;
    let index = Submenu::new(app, "Index", true)?;
    let grid_fs = Submenu::new(app, "GridFS", true)?;
    let view = Submenu::new(app, "View", true)?;
    let help = Submenu::new(app, "Help", true)?;

    menu.append_items(&[
        &file, &edit, &database, &collection, &index, &grid_fs, &view, &help,
    ])?;

    Ok(menu)
}

pub fn handle_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "file:connect" => open_connect_window(app),
        _ => {}
    }
}

pub fn open_connect_window(app: &AppHandle) {
    // If the window already exists, just focus it instead of opening a duplicate.
    if let Some(w) = app.get_webview_window("connect-window") {
        w.set_focus().ok();
        return;
    }

    WebviewWindowBuilder::new(
        app,
        "connect-window",
        WebviewUrl::App("src/pages/connect.html".into()),
    )
    .title("New Connection")
    .inner_size(480.0, 460.0)
    .resizable(false)
    .center()
    .build()
    .ok();
}
