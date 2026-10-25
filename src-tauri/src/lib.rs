use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

mod settings;
mod tray;
mod volc;
mod window;

use settings::{Settings, SettingsPatch, SettingsView};
use volc::ApiError;

pub struct AppState {
    pub http: reqwest::Client,
    pub settings: std::sync::Mutex<Settings>,
    pub last_clipboard: std::sync::Mutex<Option<String>>,
    pub hotkey_error: std::sync::Mutex<Option<String>>,
    pub start_hidden: bool,
    pub quitting: std::sync::atomic::AtomicBool,
    pub last_summon: std::sync::Mutex<Option<std::time::Instant>>,
    pub notice: std::sync::Mutex<Option<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    settings: SettingsView,
    version: String,
    hotkey_error: Option<String>,
    notice: Option<String>,
    dev: bool,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("TransFish/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build HTTP client")
}

#[tauri::command]
fn bootstrap(app: AppHandle) -> Bootstrap {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let hotkey_error = state.hotkey_error.lock().unwrap().clone();
    let notice = state.notice.lock().ok().and_then(|value| value.clone());
    Bootstrap {
        settings: settings.view(),
        version: app.package_info().version.to_string(),
        hotkey_error,
        notice,
        dev: cfg!(debug_assertions),
    }
}

#[tauri::command]
async fn translate(
    app: AppHandle,
    text: String,
    target: Option<String>,
) -> Result<volc::Translation, ApiError> {
    let (client, credentials, mode) = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
        (
            state.http.clone(),
            settings.credentials(),
            settings.target_mode.clone(),
        )
    };

    let Some(credentials) = credentials else {
        return Err(ApiError::new(
            "no_credentials",
            "还没有配置火山翻译的 AccessKey。点击右上角的设置填写后即可翻译。",
            false,
        ));
    };

    volc::translate(&client, &credentials, &text, target.as_deref(), &mode).await
}

#[tauri::command]
async fn test_credentials(
    app: AppHandle,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
) -> Result<volc::DetectResult, ApiError> {
    let (client, saved) = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
        (state.http.clone(), settings)
    };

    let credentials = volc::Credentials {
        access_key_id: access_key_id
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(saved.access_key_id),
        secret_access_key: secret_access_key
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(saved.secret_access_key),
    };

    if credentials.access_key_id.trim().is_empty()
        || credentials.secret_access_key.trim().is_empty()
    {
        return Err(ApiError::new(
            "no_credentials",
            "请先填写 AccessKey ID 与 Secret Access Key。",
            false,
        ));
    }

    volc::detect(
        &client,
        &credentials,
        "Hello there, this is a quick translation test.",
    )
    .await
}

#[tauri::command]
fn save_settings(app: AppHandle, patch: SettingsPatch) -> Result<SettingsView, ApiError> {
    let state = app.state::<AppState>();
    let previous = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let mut next = previous.clone();
    next.apply(patch);
    let next = next.normalized();

    if next.target_mode != "auto" && !valid_language(&next.target_mode) {
        return Err(ApiError::new(
            "target",
            "目标语言代码无效，请使用 auto 或 en / zh / ja 这样的代码。",
            false,
        ));
    }

    if next.hotkey != previous.hotkey {
        if let Err(message) = window::register_hotkey(&app, &next.hotkey) {
            let _ = window::register_hotkey(&app, &previous.hotkey);
            return Err(ApiError::new("hotkey", message, false));
        }
    }

    if next.autostart != previous.autostart {
        settings::set_autostart(&app, next.autostart)
            .map_err(|message| ApiError::new("autostart", message, false))?;
    }

    settings::save(&app, &next).map_err(|message| ApiError::new("io", message, false))?;

    *state.settings.lock().unwrap() = next.clone();
    let _ = tray::refresh(&app);
    Ok(next.view())
}

fn valid_language(code: &str) -> bool {
    (2..=8).contains(&code.len()) && code.chars().all(|c| c.is_ascii_lowercase() || c == '-')
}

#[tauri::command]
fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| format!("复制失败：{e}"))?;
    if let Ok(mut last) = app.state::<AppState>().last_clipboard.lock() {
        *last = Some(text.trim().to_string());
    }
    Ok(())
}

#[tauri::command]
fn set_window_height(app: AppHandle, height: f64) -> Result<(), String> {
    let Some(window) = app.get_webview_window(window::MAIN) else {
        return Ok(());
    };
    let width = window
        .outer_size()
        .map(|size| size.width as f64 / window.scale_factor().unwrap_or(1.0))
        .unwrap_or(480.0);
    let height = height.clamp(180.0, 720.0);
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)))
        .map_err(|e| format!("调整窗口高度失败：{e}"))?;
    // Keep the slip anchored near the top after growing.
    if let Ok(Some(monitor)) = window.current_monitor() {
        let area = monitor.work_area();
        let position = window.outer_position().unwrap_or_default();
        let max_y = area.position.y + area.size.height as i32 - (height as i32) - 8;
        if position.y > max_y {
            let _ = window.set_position(tauri::PhysicalPosition::new(
                position.x,
                max_y.max(area.position.y + 8),
            ));
        }
    }
    Ok(())
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window(window::MAIN) {
        let _ = window.hide();
    }
}

#[tauri::command]
fn ready(app: AppHandle) {
    let state = app.state::<AppState>();
    let start_hidden = state.start_hidden;
    let onboarding = state
        .settings
        .lock()
        .map(|s| !s.has_credentials())
        .unwrap_or(true);
    drop(state);

    if !start_hidden {
        window::summon(&app, true);
        if onboarding {
            let _ = app.emit("open-settings", ());
        }
    }
}

pub fn run() {
    let start_hidden = std::env::args().any(|arg| arg == "--hidden");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            window::summon(app, true);
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        window::toggle(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .setup(move |app| {
            let handle = app.handle().clone();
            let mut settings = settings::load(&handle);
            let mut hotkey_error = None;
            let mut hotkey_note = None;

            // The stock Alt+T shortcut is popular with other tools on Windows. When the
            // untouched default cannot be registered, walk a small list of alternatives
            // instead of leaving the user without a way to summon the window.
            match window::register_hotkey(&handle, &settings.hotkey) {
                Ok(()) => {}
                Err(first) => {
                    if settings.hotkey == settings::DEFAULT_HOTKEY {
                        for candidate in ["Ctrl+Alt+T", "Alt+Shift+T", "Ctrl+Shift+T"] {
                            if window::register_hotkey(&handle, candidate).is_ok() {
                                hotkey_note = Some(format!(
                                    "「{}」已被其他程序占用，已改用「{}」。可在设置里更换。",
                                    settings.hotkey, candidate
                                ));
                                settings.hotkey = candidate.to_string();
                                break;
                            }
                        }
                    }
                    if settings.hotkey == settings::DEFAULT_HOTKEY {
                        hotkey_error = Some(first);
                    }
                }
            }

            if hotkey_note.is_some() {
                let _ = settings::save(&handle, &settings);
            }

            app.manage(AppState {
                http: http_client(),
                settings: std::sync::Mutex::new(settings.clone()),
                last_clipboard: std::sync::Mutex::new(None),
                hotkey_error: std::sync::Mutex::new(hotkey_error),
                start_hidden,
                quitting: std::sync::atomic::AtomicBool::new(false),
                last_summon: std::sync::Mutex::new(None),
                notice: std::sync::Mutex::new(hotkey_note),
            });

            tray::create(&handle)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) => {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                let graceful = state
                    .last_summon
                    .lock()
                    .ok()
                    .and_then(|value| *value)
                    .map(|at| at.elapsed() < std::time::Duration::from_millis(350))
                    .unwrap_or(false);
                if graceful {
                    return;
                }
                let hide = state
                    .settings
                    .lock()
                    .map(|s| s.hide_on_blur)
                    .unwrap_or(true);
                drop(state);
                if hide {
                    let _ = window.hide();
                }
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            translate,
            test_credentials,
            save_settings,
            copy_text,
            set_window_height,
            hide_window,
            ready
        ])
        .build(tauri::generate_context!())
        .expect("error while building TransFish")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                // Stay resident in the tray instead of quitting with the last window.
                if !app
                    .state::<AppState>()
                    .quitting
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    api.prevent_exit();
                }
            }
        });
}
