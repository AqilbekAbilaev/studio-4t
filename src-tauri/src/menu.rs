use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

// The application menu is a custom in-app component rendered in our own design
// system (src/components/Menubar.vue); there is no native OS menu. What remains
// here is the Connect window helper, invoked by the `open_connect_window`
// command (see commands/connection.rs) from the menu bar's File → Connect item.
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
