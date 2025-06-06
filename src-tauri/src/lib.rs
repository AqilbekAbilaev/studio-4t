// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str, _age: &str) -> String {
    let text = "Hello world!";
    println!("I think this is interesting to see that kind of behavior: {}", text);
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, open_connection])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn open_connection(handle: tauri::AppHandle) {
  let docs_window = tauri::WebviewWindowBuilder::new(
    &handle,
    "external", /* the unique window label */
    tauri::WebviewUrl::External("https://tauri.app/".parse().unwrap())
  ).build().unwrap();
}


