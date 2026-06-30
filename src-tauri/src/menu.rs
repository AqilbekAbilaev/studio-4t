use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Wry,
};

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = match Menu::new(app) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // File
    let connect = match MenuItem::with_id(app, "file:connect", "Connect...", true, None::<&str>) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let separator_file = match PredefinedMenuItem::separator(app) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let quit = match PredefinedMenuItem::quit(app, Some("Exit")) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let file = match Submenu::with_items(app, "File", true, &[&connect, &separator_file, &quit]) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // Edit — predefined items give macOS (and Windows) the standard Cmd/Ctrl+C/V/X/Z
    // shortcuts the webview relies on. On Linux/WebKitGTK the webview already handles
    // these natively in text inputs; adding the predefined items there registers
    // accelerators that swallow Ctrl+C/V/X/A without performing the action, which breaks
    // editing in the query bar and other fields — so on Linux we leave Edit empty.
    let edit = if cfg!(target_os = "linux") {
        match Submenu::new(app, "Edit", true) {
            Ok(val) => val,
            Err(e) => return Err(e),
        }
    } else {
        let undo = match PredefinedMenuItem::undo(app, None) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        let redo = match PredefinedMenuItem::redo(app, None) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        let separator_edit = match PredefinedMenuItem::separator(app) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        let cut = match PredefinedMenuItem::cut(app, None) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        let copy = match PredefinedMenuItem::copy(app, None) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        let paste = match PredefinedMenuItem::paste(app, None) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        let select_all = match PredefinedMenuItem::select_all(app, None) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        match Submenu::with_items(
            app,
            "Edit",
            true,
            &[&undo, &redo, &separator_edit, &cut, &copy, &paste, &select_all],
        ) {
            Ok(val) => val,
            Err(e) => return Err(e),
        }
    };

    // Placeholder menus — items will be wired up as features are built
    let database = match Submenu::new(app, "Database", true) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let collection = match Submenu::new(app, "Collection", true) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let index = match Submenu::new(app, "Index", true) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let grid_fs = match Submenu::new(app, "GridFS", true) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let view = match Submenu::new(app, "View", true) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let shortcuts = match MenuItem::with_id(
        app,
        "help:shortcuts",
        "Keyboard Shortcuts",
        true,
        None::<&str>,
    ) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let help = match Submenu::with_items(app, "Help", true, &[&shortcuts]) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    match menu.append_items(&[
        &file, &edit, &database, &collection, &index, &grid_fs, &view, &help,
    ]) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    Ok(menu)
}

pub fn handle_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "file:connect" => open_connect_window(app),
        "help:shortcuts" => {
            // The frontend owns the shortcuts modal; just nudge it open.
            let _ = app.emit("menu:shortcuts", ());
        }
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
