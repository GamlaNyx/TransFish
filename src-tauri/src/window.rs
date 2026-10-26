use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub const MAIN: &str = "main";

/// Programmatic moves (summon, height clamp, reset) must not be mistaken for a
/// user drag. Ignore `Moved` events for a short while after we place the window.
const PLACE_GRACE: Duration = Duration::from_millis(700);

/// Windows reports a focus loss when the native title-bar move loop starts or
/// ends. Treat blur shortly after a user move as part of the drag, not as
/// "the user clicked another app".
const DRAG_BLUR_GRACE: Duration = Duration::from_millis(1200);

/// True while the left mouse button is held down anywhere on the desktop.
#[cfg(target_os = "windows")]
pub fn primary_button_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    let state = unsafe { GetAsyncKeyState(VK_LBUTTON as i32) };
    (state as u16 & 0x8000) != 0
}

#[cfg(not(target_os = "windows"))]
pub fn primary_button_down() -> bool {
    false
}

/// Blur that must not hide the slip: the drag is still in progress, or it just
/// finished and Windows is still settling focus.
pub fn blur_is_benign(app: &AppHandle, window: &WebviewWindow) -> bool {
    let dragged = app
        .state::<crate::AppState>()
        .last_drag
        .lock()
        .ok()
        .and_then(|value| *value)
        .map(|at| at.elapsed() < DRAG_BLUR_GRACE)
        .unwrap_or(false);
    if dragged {
        return true;
    }
    // A held button only means "drag" while the pointer is still over the slip.
    // Clicking and holding another window must still hide it.
    primary_button_down() && cursor_inside(app, window)
}

fn cursor_inside(app: &AppHandle, window: &WebviewWindow) -> bool {
    let (Ok(cursor), Ok(position), Ok(size)) = (
        app.cursor_position(),
        window.outer_position(),
        window.outer_size(),
    ) else {
        return false;
    };
    cursor.x >= position.x as f64
        && cursor.y >= position.y as f64
        && cursor.x < (position.x + size.width as i32) as f64
        && cursor.y < (position.y + size.height as i32) as f64
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummonPayload {
    /// Text from the clipboard, only present when it changed since the last summon.
    pub clipboard: Option<String>,
}

pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible && window.is_focused().unwrap_or(false) {
        let _ = window.hide();
    } else {
        summon(app, true);
    }
}

pub fn summon(app: &AppHandle, read_clipboard: bool) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };

    place(&window);
    let _ = window.show();
    let _ = window.set_focus();

    let state = app.state::<crate::AppState>();
    if let Ok(mut last) = state.last_summon.lock() {
        *last = Some(std::time::Instant::now());
    }

    // Windows can refuse focus while the window is still being shown.
    let retry = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(70));
        let _ = retry.set_focus();
    });

    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();

    let mut payload = SummonPayload { clipboard: None };
    if read_clipboard && settings.clipboard_prefill {
        if let Ok(text) = app.clipboard().read_text() {
            let trimmed = text.trim();
            let usable = !trimmed.is_empty() && trimmed.chars().count() <= 5000;
            if usable {
                let mut last = state.last_clipboard.lock().unwrap();
                if last.as_deref() != Some(trimmed) {
                    *last = Some(trimmed.to_string());
                    payload.clipboard = Some(trimmed.to_string());
                }
            }
        }
    }

    let _ = app.emit("summon", payload);
}

fn place(window: &WebviewWindow) {
    let app = window.app_handle();
    let size = window
        .outer_size()
        .unwrap_or_else(|_| PhysicalSize::new(480, 300));

    // Respect where the user last dragged the slip to, as long as it is still
    // reachable on a connected monitor.
    if let Some(position) = saved_position(&app) {
        if position_is_usable(&app, position, size) {
            mark_placement(&app);
            let _ = window.set_position(PhysicalPosition::new(position[0], position[1]));
            return;
        }
    }

    place_top_center(&app, window, size);
}

fn saved_position(app: &AppHandle) -> Option<[i32; 2]> {
    app.state::<crate::AppState>()
        .window_position
        .lock()
        .ok()
        .and_then(|position| *position)
}

fn position_is_usable(app: &AppHandle, position: [i32; 2], size: PhysicalSize<u32>) -> bool {
    let [x, y] = position;
    // Probe a point near the title bar rather than the origin, so a partially
    // off-screen window still resolves to the monitor it mostly lives on.
    let Some(monitor) = app
        .monitor_from_point(x as f64 + 40.0, y as f64 + 16.0)
        .ok()
        .flatten()
    else {
        return false;
    };
    let area = monitor.work_area();
    let left = area.position.x;
    let top = area.position.y;
    let right = left + area.size.width as i32;
    let bottom = top + area.size.height as i32;
    let width = size.width as i32;
    let height = size.height as i32;
    // Keep at least the grab area (120x40) of the slip inside the work area.
    x + width > left + 120 && x < right - 120 && y + height > top + 40 && y < bottom - 40
}

fn place_top_center(app: &AppHandle, window: &WebviewWindow, size: PhysicalSize<u32>) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|point| app.monitor_from_point(point.x, point.y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return;
    };

    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let centered_y = area.position.y + (area.size.height as f64 * 0.16) as i32;
    let max_y = area.position.y + area.size.height as i32 - size.height as i32 - 8;
    let y = centered_y.min(max_y).max(area.position.y + 8);

    mark_placement(app);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Remember the position of a programmatic move so the next `Moved` event is
/// not persisted as a user drag.
pub fn mark_placement(app: &AppHandle) {
    if let Ok(mut last) = app.state::<crate::AppState>().last_place.lock() {
        *last = Some(Instant::now());
    }
}

/// Called from the window `Moved` event. Drags are debounced before hitting disk.
pub fn remember_position(app: &AppHandle, position: [i32; 2]) {
    let visible = app
        .get_webview_window(MAIN)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    if !visible {
        return;
    }

    let state = app.state::<crate::AppState>();
    let recently_placed = state
        .last_place
        .lock()
        .ok()
        .and_then(|last| *last)
        .map(|at| at.elapsed() < PLACE_GRACE)
        .unwrap_or(false);
    if recently_placed {
        return;
    }

    // This is a real user move: remember when it happened so the focus loss
    // triggered by the native move loop cannot hide the window mid-drag.
    if let Ok(mut last) = state.last_drag.lock() {
        *last = Some(Instant::now());
    }

    {
        let Ok(mut current) = state.window_position.lock() else {
            return;
        };
        if *current == Some(position) {
            return;
        }
        *current = Some(position);
    }

    let seq = state.position_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(650));
        let state = handle.state::<crate::AppState>();
        if state.position_seq.load(Ordering::SeqCst) != seq {
            return;
        }
        let Some(position) = state.window_position.lock().ok().and_then(|value| *value) else {
            return;
        };
        let Ok(_write) = state.settings_write.lock() else {
            return;
        };
        let Ok(mut settings) = state.settings.lock().map(|value| value.clone()) else {
            return;
        };
        if settings.window_position == Some(position) {
            return;
        }
        settings.window_position = Some(position);
        if crate::settings::save(&handle, &settings).is_ok() {
            if let Ok(mut current) = state.settings.lock() {
                *current = settings;
            }
        }
    });
}

/// Clear the remembered drag position so the next summon returns to the
/// top-center of the monitor under the cursor.
pub fn reset_position(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    let Ok(_write) = state.settings_write.lock() else {
        return;
    };
    state.position_seq.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut current) = state.window_position.lock() {
        *current = None;
    }
    let cloned = state.settings.lock().map(|value| value.clone());
    if let Ok(mut settings) = cloned {
        if settings.window_position.take().is_some() {
            let _ = crate::settings::save(app, &settings);
            if let Ok(mut current) = state.settings.lock() {
                *current = settings;
            }
        }
    }

    if let Some(window) = app.get_webview_window(MAIN) {
        if window.is_visible().unwrap_or(false) {
            let size = window
                .outer_size()
                .unwrap_or_else(|_| PhysicalSize::new(480, 300));
            place_top_center(app, &window, size);
        }
    }
}

/// Flush a pending drag position synchronously, used when quitting.
pub fn persist_position(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    let Some(position) = state.window_position.lock().ok().and_then(|value| *value) else {
        return;
    };
    let Ok(_write) = state.settings_write.lock() else {
        return;
    };
    let cloned = state.settings.lock().map(|value| value.clone());
    if let Ok(mut settings) = cloned {
        if settings.window_position == Some(position) {
            return;
        }
        settings.window_position = Some(position);
        if crate::settings::save(app, &settings).is_ok() {
            if let Ok(mut current) = state.settings.lock() {
                *current = settings;
            }
        }
    }
}

pub fn register_hotkey(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let manager = app.global_shortcut();
    if let Err(e) = manager.unregister_all() {
        return Err(format!("释放旧快捷键失败：{e}"));
    }
    manager
        .register(accelerator)
        .map_err(|e| format!("快捷键「{accelerator}」不可用：{e}"))
}
