pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_foundation_models::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
