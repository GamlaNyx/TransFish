use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const TRAY_ID: &str = "main-tray";

fn build_menu(
    app: &AppHandle,
    autostart: bool,
    always_on_top: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "打开翻译窗", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let pin = CheckMenuItem::with_id(
        app,
        "always-top",
        "窗口置顶",
        true,
        always_on_top,
        None::<&str>,
    )?;
    let launch = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自动启动",
        true,
        autostart,
        None::<&str>,
    )?;
    let center = MenuItem::with_id(app, "center", "回到顶部居中", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 TransFish", true, None::<&str>)?;
    let first_sep = PredefinedMenuItem::separator(app)?;
    let second_sep = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &open,
            &settings,
            &first_sep,
            &pin,
            &launch,
            &center,
            &second_sep,
            &quit,
        ],
    )
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let enabled = current_settings(app, |settings| settings.autostart);
    let pinned = current_settings(app, |settings| settings.always_on_top);
    let menu = build_menu(app, enabled, pinned)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("TransFish · 快捷翻译")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::window::toggle(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

pub fn refresh(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(
        app,
        current_settings(app, |settings| settings.autostart),
        current_settings(app, |settings| settings.always_on_top),
    )?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

fn current_settings(app: &AppHandle, read: impl Fn(&crate::settings::Settings) -> bool) -> bool {
    app.state::<crate::AppState>()
        .settings
        .lock()
        .map(|settings| read(&settings))
        .unwrap_or(false)
}

fn handle_menu(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "open" => crate::window::summon(app, true),
        "settings" => {
            crate::window::summon(app, false);
            let _ = app.emit("open-settings", ());
        }
        "autostart" => toggle_autostart(app),
        "always-top" => toggle_always_on_top(app),
        "center" => {
            // Bring a hidden slip back before recentering, otherwise the action
            // (and its toast) happens entirely off-screen.
            let visible = app
                .get_webview_window(crate::window::MAIN)
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false);
            if !visible {
                crate::window::summon(app, false);
            }
            crate::window::reset_position(app);
            let _ = app.emit("toast", "窗口已回到顶部居中");
            let _ = app.emit("settings-changed", ());
        }
        "quit" => quit(app),
        _ => {}
    }
}

fn toggle_autostart(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    let mut settings = match state.settings.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return,
    };
    settings.autostart = !settings.autostart;

    if crate::settings::set_autostart(app, settings.autostart).is_err() {
        let _ = app.emit("toast", "无法修改开机启动设置");
        return;
    }
    if crate::settings::save(app, &settings).is_err() {
        let _ = app.emit("toast", "无法保存设置");
        return;
    }

    *state.settings.lock().unwrap() = settings;
    let _ = refresh(app);
    let _ = app.emit("settings-changed", ());
}

fn toggle_always_on_top(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    let mut settings = match state.settings.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return,
    };
    let next = !settings.always_on_top;

    let Some(window) = app.get_webview_window(crate::window::MAIN) else {
        return;
    };
    if window.set_always_on_top(next).is_err() {
        let _ = app.emit("toast", "无法修改窗口置顶");
        return;
    }

    settings.always_on_top = next;
    if crate::settings::save(app, &settings).is_err() {
        let _ = app.emit("toast", "无法保存设置");
        return;
    }

    *state.settings.lock().unwrap() = settings;
    let _ = refresh(app);
    let _ = app.emit("settings-changed", ());
}

fn quit(app: &AppHandle) {
    crate::window::persist_position(app);
    app.state::<crate::AppState>()
        .quitting
        .store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(0);
}
