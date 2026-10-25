use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const DEFAULT_HOTKEY: &str = "Alt+T";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub hotkey: String,
    /// "auto" for smart zh↔en direction, otherwise a language code.
    pub target_mode: String,
    pub hide_on_blur: bool,
    pub clipboard_prefill: bool,
    pub quick_translate: bool,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            access_key_id: String::new(),
            secret_access_key: String::new(),
            hotkey: DEFAULT_HOTKEY.to_string(),
            target_mode: "auto".to_string(),
            hide_on_blur: true,
            clipboard_prefill: true,
            quick_translate: true,
            autostart: false,
        }
    }
}

impl Settings {
    pub fn has_credentials(&self) -> bool {
        !self.access_key_id.trim().is_empty() && !self.secret_access_key.trim().is_empty()
    }

    pub fn credentials(&self) -> Option<crate::volc::Credentials> {
        if !self.has_credentials() {
            return None;
        }
        Some(crate::volc::Credentials {
            access_key_id: self.access_key_id.trim().to_string(),
            secret_access_key: self.secret_access_key.trim().to_string(),
        })
    }

    pub fn normalized(mut self) -> Self {
        self.access_key_id = self.access_key_id.trim().to_string();
        self.secret_access_key = self.secret_access_key.trim().to_string();
        self.hotkey = self.hotkey.trim().to_string();
        if self.hotkey.is_empty() {
            self.hotkey = DEFAULT_HOTKEY.to_string();
        }
        self.target_mode = self.target_mode.trim().to_string();
        if self.target_mode.is_empty() {
            self.target_mode = "auto".to_string();
        }
        self
    }

    pub fn view(&self) -> SettingsView {
        SettingsView {
            hotkey: self.hotkey.clone(),
            target_mode: self.target_mode.clone(),
            hide_on_blur: self.hide_on_blur,
            clipboard_prefill: self.clipboard_prefill,
            quick_translate: self.quick_translate,
            autostart: self.autostart,
            has_credentials: self.has_credentials(),
            access_key_hint: hint(&self.access_key_id),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub hotkey: String,
    pub target_mode: String,
    pub hide_on_blur: bool,
    pub clipboard_prefill: bool,
    pub quick_translate: bool,
    pub autostart: bool,
    pub has_credentials: bool,
    pub access_key_hint: String,
}

fn hint(key: &str) -> String {
    let key = key.trim();
    let len = key.chars().count();
    if len == 0 {
        return String::new();
    }
    if len <= 8 {
        return "•".repeat(len);
    }
    let head: String = key.chars().take(4).collect();
    let tail: String = key.chars().skip(len - 4).collect();
    format!("{head}…{tail}")
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub hotkey: Option<String>,
    pub target_mode: Option<String>,
    pub hide_on_blur: Option<bool>,
    pub clipboard_prefill: Option<bool>,
    pub quick_translate: Option<bool>,
    pub autostart: Option<bool>,
}

impl Settings {
    pub fn apply(&mut self, patch: SettingsPatch) {
        if let Some(v) = patch.access_key_id {
            self.access_key_id = v.trim().to_string();
        }
        if let Some(v) = patch.secret_access_key {
            self.secret_access_key = v.trim().to_string();
        }
        if let Some(v) = patch.hotkey {
            if !v.trim().is_empty() {
                self.hotkey = v.trim().to_string();
            }
        }
        if let Some(v) = patch.target_mode {
            self.target_mode = v.trim().to_string();
        }
        if let Some(v) = patch.hide_on_blur {
            self.hide_on_blur = v;
        }
        if let Some(v) = patch.clipboard_prefill {
            self.clipboard_prefill = v;
        }
        if let Some(v) = patch.quick_translate {
            self.quick_translate = v;
        }
        if let Some(v) = patch.autostart {
            self.autostart = v;
        }
    }
}

pub fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    let Some(file) = path(app) else {
        return Settings::default();
    };
    let Ok(raw) = std::fs::read_to_string(&file) else {
        return Settings::default();
    };
    match serde_json::from_str::<Settings>(&raw) {
        Ok(settings) => settings.normalized(),
        Err(_) => {
            // Keep the unreadable file for inspection instead of silently losing it.
            let broken = file.with_extension("broken.json");
            let _ = std::fs::rename(&file, broken);
            Settings::default()
        }
    }
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let Some(file) = path(app) else {
        return Err("找不到配置目录。".to_string());
    };
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建配置目录失败：{e}"))?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("保存配置失败：{e}"))?;
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("写入配置失败：{e}"))?;
    std::fs::rename(&tmp, &file).map_err(|e| format!("写入配置失败：{e}"))?;
    Ok(())
}

pub fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| format!("设置开机启动失败：{e}"))
}
