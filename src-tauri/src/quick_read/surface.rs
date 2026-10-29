//! Optional non-activating player and intentional OCR area selection.
use super::{message, player::Snapshot, QuickRead};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
#[derive(Clone, Serialize, Deserialize)]
pub struct Region {
    pub generation: u64,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}
#[derive(Clone, Deserialize)]
pub struct Selection {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}
/// Convert logical webview drag coordinates to a bounded physical monitor rectangle.
pub fn physical(region: &Region, selection: &Selection) -> Result<[i32; 4], String> {
    if ![
        selection.x1,
        selection.y1,
        selection.x2,
        selection.y2,
        region.scale,
    ]
    .iter()
    .all(|v| v.is_finite())
        || region.scale <= 0.0
    {
        return Err("Invalid region".into());
    }
    let x1 = (selection.x1.min(selection.x2) * region.scale)
        .floor()
        .clamp(0.0, region.width as f64) as i32;
    let y1 = (selection.y1.min(selection.y2) * region.scale)
        .floor()
        .clamp(0.0, region.height as f64) as i32;
    let x2 = (selection.x1.max(selection.x2) * region.scale)
        .ceil()
        .clamp(0.0, region.width as f64) as i32;
    let y2 = (selection.y1.max(selection.y2) * region.scale)
        .ceil()
        .clamp(0.0, region.height as f64) as i32;
    let width = x2 - x1;
    let height = y2 - y1;
    if width < 3
        || height < 3
        || width > 4096
        || height > 4096
        || width as i64 * height as i64 > 16_000_000
    {
        return Err(message(
            "Select an area between 3 and 4096 pixels per side (up to 16 megapixels).",
            "Выберите область от 3 до 4096 пикселей по стороне (до 16 мегапикселей).",
        ));
    }
    Ok([
        region.x.checked_add(x1).ok_or("Region overflow")?,
        region.y.checked_add(y1).ok_or("Region overflow")?,
        width,
        height,
    ])
}
pub fn setup(app: &tauri::AppHandle) -> Result<(), String> {
    WebviewWindowBuilder::new(app, "quick-player", WebviewUrl::App("index.html".into()))
        .title("Glagol — Quick Read")
        .inner_size(420.0, 110.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .resizable(false)
        .build()
        .map_err(|e| e.to_string())?;
    WebviewWindowBuilder::new(app, "quick-ocr", WebviewUrl::App("index.html".into()))
        .title("Glagol — OCR")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .resizable(false)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}
/// Called by the existing state reporter. Errors remain visible even with the player hidden.
pub fn update(app: &tauri::AppHandle, state: &Snapshot) {
    let q = app.state::<QuickRead>();
    let Some(player) = app.get_webview_window("quick-player") else {
        return;
    };
    let show = q
        .capture_generation
        .load(std::sync::atomic::Ordering::Relaxed)
        == 0
        && ((q.show_player.load(std::sync::atomic::Ordering::Relaxed) && state.phase != "idle")
            || state.error.is_some());
    if show {
        if !player.is_visible().unwrap_or(false) {
            if let Some(monitor) = app
                .cursor_position()
                .ok()
                .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
                .or_else(|| app.primary_monitor().ok().flatten())
            {
                let scale = monitor.scale_factor();
                let origin = monitor.position();
                let size = monitor.size();
                let _ = player.set_position(PhysicalPosition::new(
                    origin.x as f64 + (size.width as f64 - 420.0 * scale) / 2.0,
                    origin.y as f64 + size.height as f64 - 200.0 * scale,
                ));
            }
            // focusable(false) prevents SW_SHOW from activating the mini-player.
            let _ = player.show();
        }
    } else {
        let _ = player.hide();
    }
}
/// Show only the monitor under the cursor. Main window remains hidden.
pub fn start_ocr(app: &tauri::AppHandle) -> Result<(), String> {
    if !cfg!(windows) {
        return Err(message("OCR requires Windows.", "OCR требует Windows."));
    }
    let q = app.state::<QuickRead>();
    let id = q.replace();
    q.capture_generation
        .store(id, std::sync::atomic::Ordering::Relaxed);
    if let Some(player) = app.get_webview_window("quick-player") {
        let _ = player.hide();
    }
    *q.content.lock().unwrap() = std::sync::Arc::new(Vec::new());
    let cursor = app.cursor_position().map_err(|e| e.to_string())?;
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .map_err(|e| e.to_string())?
        .ok_or("Monitor unavailable")?;
    let region = Region {
        generation: id,
        x: monitor.position().x,
        y: monitor.position().y,
        width: monitor.size().width,
        height: monitor.size().height,
        scale: monitor.scale_factor(),
    };
    let window = app
        .get_webview_window("quick-ocr")
        .ok_or("OCR overlay unavailable")?;
    window
        .set_position(PhysicalPosition::new(region.x, region.y))
        .map_err(|e| e.to_string())?;
    window
        .set_size(PhysicalSize::new(region.width, region.height))
        .map_err(|e| e.to_string())?;
    *q.region.lock().unwrap() = Some(region.clone());
    let _ = window.emit("quick-read-region", region);
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn quick_read_region(app: tauri::AppHandle) -> Result<Option<Region>, String> {
    Ok(app.state::<QuickRead>().region.lock().unwrap().clone())
}
#[tauri::command]
pub fn quick_read_ocr_cancel(app: tauri::AppHandle) -> Result<(), String> {
    app.state::<QuickRead>().region.lock().unwrap().take();
    app.state::<QuickRead>()
        .capture_generation
        .store(0, std::sync::atomic::Ordering::Relaxed);
    if let Some(w) = app.get_webview_window("quick-ocr") {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
async fn finish_impl(
    app: tauri::AppHandle,
    generation: u64,
    selection: Selection,
) -> Result<(), String> {
    let q = app.state::<QuickRead>();
    let region = {
        let mut region = q.region.lock().unwrap();
        if region.as_ref().is_none_or(|r| r.generation != generation) {
            return Err("Selection expired".into());
        }
        region.take().ok_or("Selection expired")?
    };
    if let Some(w) = app.get_webview_window("quick-ocr") {
        w.hide().map_err(|e| e.to_string())?;
    }
    if !q.current(generation) {
        return Ok(());
    }
    let rectangle = physical(&region, &selection)?;
    let language = super::settings::quick_read_settings(app.clone())?.ocr_language;
    let mut args: Vec<String> = rectangle.iter().map(ToString::to_string).collect();
    args.push(language);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let q = app.state::<QuickRead>();
        let mut changes = q.changes.subscribe();
        if !q.current(generation) {
            return;
        }
        q.buffer.lock().unwrap().snapshot.phase = "extracting".into();
        // Let the compositor remove the overlay before capturing its pixels.
        let result = tokio::select! {
            biased;
            _=changes.changed()=>return,
            result=async {
                tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                super::sources::run("ocr",&args,std::time::Duration::from_secs(15)).await
            }=>result,
        };
        let _ = q.capture_generation.compare_exchange(
            generation,
            0,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
        match result.and_then(|s| super::text::chunks(&s)) {
            Ok(chunks) => {
                if q.current(generation) {
                    let chunks = std::sync::Arc::new(chunks);
                    *q.content.lock().unwrap() = chunks.clone();
                    super::read(app.clone(), generation, chunks, 0).await;
                }
            }
            Err(e) => super::fail(&app, generation, e),
        }
    });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dpi_negative_monitor_reverse_drag_and_clipping() {
        let r = Region {
            generation: 1,
            x: -2560,
            y: -100,
            width: 2560,
            height: 1440,
            scale: 1.5,
        };
        assert_eq!(
            physical(
                &r,
                &Selection {
                    x1: 100.0,
                    y1: 100.0,
                    x2: 10.0,
                    y2: 20.0
                }
            )
            .unwrap(),
            [-2545, -70, 135, 120]
        );
        assert_eq!(
            physical(
                &r,
                &Selection {
                    x1: -10.0,
                    y1: 0.0,
                    x2: 3000.0,
                    y2: 100.0
                }
            )
            .unwrap(),
            [-2560, -100, 2560, 150]
        );
        assert!(physical(
            &r,
            &Selection {
                x1: f64::NAN,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0
            }
        )
        .is_err());
        assert!(physical(
            &r,
            &Selection {
                x1: 1.0,
                y1: 1.0,
                x2: 1.0,
                y2: 1.0
            }
        )
        .is_err());
    }
}

/// Surface validation/capture errors even when the main window is in the tray.
#[tauri::command]
pub async fn quick_read_ocr_finish(
    app: tauri::AppHandle,
    generation: u64,
    selection: Selection,
) -> Result<(), String> {
    let result = finish_impl(app.clone(), generation, selection).await;
    if let Err(e) = &result {
        super::fail(&app, generation, e.clone());
    }
    result
}
