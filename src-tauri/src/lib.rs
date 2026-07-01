
mod capture;
mod window_manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // On Windows, WebView2 does not honour `transparent: true` from tauri.conf.json
            // unless we also explicitly set the webview background color to fully transparent.
            #[cfg(target_os = "windows")]
            {
                use tauri::Manager;
                if let Some(main_window) = app.get_webview_window("main") {
                    main_window
                        .set_background_color(Some(tauri::window::Color(0, 0, 0, 0)))
                        .ok();
                }
                if let Some(canvas_window) = app.get_webview_window("canvas") {
                    canvas_window
                        .set_background_color(Some(tauri::window::Color(0, 0, 0, 0)))
                        .ok();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            capture::capture_screen,
            window_manager::show_canvas_window,
            window_manager::hide_canvas_window,
            window_manager::show_mirror_window,
            window_manager::hide_mirror_window,
            window_manager::is_mirror_active,
            window_manager::start_drag
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

