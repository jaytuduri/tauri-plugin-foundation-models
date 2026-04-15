pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_apple_intelligence::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
