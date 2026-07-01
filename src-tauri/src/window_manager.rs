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
