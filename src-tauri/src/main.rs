// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Use mimalloc process-wide instead of the system (glibc) allocator. Deserializing
// query results into serde_json::Value fragments glibc's per-thread arenas so badly
// that freed memory is never reused or returned to the OS — RSS ratchets up on every
// repeated query. mimalloc handles that small-allocation churn without the bloat.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    // std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    ozendb_lib::run()
}
