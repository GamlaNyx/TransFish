use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub const MAIN: &str = "main";

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

    let _ = window.set_position(PhysicalPosition::new(x, y));
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
