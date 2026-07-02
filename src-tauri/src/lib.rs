
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
                
                // Set up tray icon
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{TrayIconBuilder};
                
                let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;
                
                let _tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event(|app_handle, event| match event.id.as_ref() {
                        "quit" => {
                            std::process::exit(0);
                        }
                        "settings" => {
                            if let Some(window) = app_handle.get_webview_window("settings") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    })
                    .build(app)?;
                if let Some(settings_window) = app.get_webview_window("settings") {
                    let settings_window_ = settings_window.clone();
                    settings_window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = settings_window_.hide();
                        }
                    });
                }

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
            window_manager::start_drag,
            window_manager::show_ai_result_window,
            window_manager::hide_ai_result_window,
            window_manager::save_history_image,
            window_manager::clear_history_images
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

