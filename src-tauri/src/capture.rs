use screenshots::Screen;
use std::io::Cursor;

#[tauri::command]
pub fn capture_screen() -> Result<Vec<u8>, String> {
    let screens = Screen::all().map_err(|e| e.to_string())?;
    // Grab the first screen (primary screen)
    if let Some(screen) = screens.first() {
        let image = screen.capture().map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        image.write_to(&mut Cursor::new(&mut buffer), screenshots::image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(buffer)
    } else {
        Err("No screen detected".to_string())
    }
}
