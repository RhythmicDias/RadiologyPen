use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn show_canvas_window(app: AppHandle) -> Result<(), String> {
    if let Some(canvas_window) = app.get_webview_window("canvas") {
        // Ensure it's focused and visible
        canvas_window.show().map_err(|e| e.to_string())?;
        canvas_window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_canvas_window(app: AppHandle) -> Result<(), String> {
    if let Some(canvas_window) = app.get_webview_window("canvas") {
        canvas_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn show_mirror_window(app: AppHandle) -> Result<(), String> {
    if let Some(mirror_window) = app.get_webview_window("mirror") {
        // Try to find a secondary monitor to place the parent view on
        if let Ok(monitors) = mirror_window.available_monitors() {
            // If we have at least 2 monitors
            if monitors.len() > 1 {
                // Find a monitor that isn't the primary one
                // Usually monitors[1] is the secondary monitor
                let secondary = &monitors[1];
                let pos = secondary.position();
                mirror_window.set_position(tauri::Position::Physical(*pos)).ok();
                mirror_window.set_fullscreen(true).ok();
            }
        }
        mirror_window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_mirror_window(app: AppHandle) -> Result<(), String> {
    if let Some(mirror_window) = app.get_webview_window("mirror") {
        mirror_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn is_mirror_active(app: AppHandle) -> bool {
    if let Some(mirror_window) = app.get_webview_window("mirror") {
        mirror_window.is_visible().unwrap_or(false)
    } else {
        false
    }
}

#[tauri::command]
pub fn start_drag(window: tauri::WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn show_ai_result_window(app: AppHandle) -> Result<(), String> {
    if let Some(ai_window) = app.get_webview_window("ai-result") {
        ai_window.show().map_err(|e| e.to_string())?;
        ai_window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_ai_result_window(app: AppHandle) -> Result<(), String> {
    if let Some(ai_window) = app.get_webview_window("ai-result") {
        ai_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_history_image(app: AppHandle, id: String, bytes: Vec<u8>) -> Result<String, String> {
    use tauri::Manager;
    let mut path = app.path().app_data_dir().map_err(|e| e.to_string())?;
    path.push("history_images");
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    path.push(format!("{}.jpg", id));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn clear_history_images(app: AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let mut path = app.path().app_data_dir().map_err(|e| e.to_string())?;
    path.push("history_images");
    if path.exists() {
        std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
