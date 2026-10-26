import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { AlertIcon, CheckIcon, ChevronLeftIcon, PinIcon } from "./Icons";
import { TARGET_OPTIONS, langName } from "../lib/lang";
import { backend, toApiError, type SettingsPatch, type SettingsView, type Theme } from "../lib/api";

interface SettingsProps {
  settings: SettingsView;
  version: string;
  onClose: () => void;
  onSaved: (settings: SettingsView) => void;
  onTogglePin: () => void;
  onThemePreview: (theme: Theme | null) => void;
}

const THEME_OPTIONS: { value: Theme; label: string }[] = [
  { value: "light", label: "白" },
  { value: "dark", label: "黑" },
  { value: "translucent", label: "半透明" },
];

type TestState = { ok: boolean; message: string } | null;

function acceleratorKey(event: ReactKeyboardEvent<HTMLInputElement>): string | null {
  const code = event.code;
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) {
    return letter[1];
  }
  const digit = /^Digit([0-9])$/.exec(code);
  if (digit) {
    return digit[1];
  }
  const fn = /^F([1-9]|1[0-9]|2[0-4])$/.exec(code);
  if (fn) {
    return `F${fn[1]}`;
  }
  const map: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Minus: "-",
    Equal: "=",
    BracketLeft: "[",
    BracketRight: "]",
    Backslash: "\\",
    Semicolon: ";",
    Quote: "'",
    Comma: ",",
    Period: ".",
    Slash: "/",
    Backquote: "`",
  };
  return map[code] ?? null;
}

export default function Settings({
  settings,
  version,
  onClose,
  onSaved,
  onTogglePin,
  onThemePreview,
}: SettingsProps) {
  const [accessKeyId, setAccessKeyId] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const [showSecret, setShowSecret] = useState(false);
  const [hotkey, setHotkey] = useState(settings.hotkey);
  const [targetMode, setTargetMode] = useState(settings.targetMode);
  const [theme, setTheme] = useState<Theme>(settings.theme);
  const [hideOnBlur, setHideOnBlur] = useState(settings.hideOnBlur);
  const [clipboardPrefill, setClipboardPrefill] = useState(settings.clipboardPrefill);
  const [quickTranslate, setQuickTranslate] = useState(settings.quickTranslate);
  const [autostart, setAutostart] = useState(settings.autostart);
  const [capturing, setCapturing] = useState(false);
  const [hotkeyError, setHotkeyError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [test, setTest] = useState<TestState>(null);
  const hotkeyRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setHotkey(settings.hotkey);
    setTargetMode(settings.targetMode);
    setTheme(settings.theme);
    setHideOnBlur(settings.hideOnBlur);
    setClipboardPrefill(settings.clipboardPrefill);
    setQuickTranslate(settings.quickTranslate);
    setAutostart(settings.autostart);
  }, [
    settings.hotkey,
    settings.targetMode,
    settings.theme,
    settings.hideOnBlur,
    settings.clipboardPrefill,
    settings.quickTranslate,
    settings.autostart,
  ]);

  const save = async () => {
    if (saving) {
      return;
    }
    setSaving(true);
    setHotkeyError(null);

    const patch: SettingsPatch = {
      hotkey,
      targetMode,
      theme,
      hideOnBlur,
      clipboardPrefill,
      quickTranslate,
      autostart,
    };
    if (accessKeyId.trim()) {
      patch.accessKeyId = accessKeyId.trim();
    }
    if (secretKey.trim()) {
      patch.secretAccessKey = secretKey.trim();
    }

    try {
      const next = await backend.saveSettings(patch);
      setAccessKeyId("");
      setSecretKey("");
      setShowSecret(false);
      setTest(null);
      onSaved(next);
    } catch (error) {
      const failure = toApiError(error);
      if (failure.code === "hotkey") {
        setHotkeyError(failure.message);
      } else {
        setTest({ ok: false, message: failure.message });
      }
    } finally {
      setSaving(false);
    }
  };

  const testConnection = async () => {
    if (testing) {
      return;
    }
    setTesting(true);
    setTest(null);
    try {
      const result = await backend.testCredentials(
        accessKeyId.trim() || undefined,
        secretKey.trim() || undefined,
      );
      const confidence = result.confidence > 0 ? ` · ${Math.round(result.confidence * 100)}%` : "";
      setTest({
        ok: true,
        message: `连接成功 · 检测到${langName(result.language)}${confidence} · ${result.latencyMs}ms`,
      });
    } catch (error) {
      setTest({ ok: false, message: toApiError(error).message });
    } finally {
      setTesting(false);
    }
  };

  const onHotkeyKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Tab") {
      setCapturing(false);
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      setCapturing(false);
      setHotkeyError(null);
      hotkeyRef.current?.blur();
      return;
    }
    const key = acceleratorKey(event);
    if (!key) {
      return;
    }
    const modifiers: string[] = [];
    if (event.ctrlKey) {
      modifiers.push("Ctrl");
    }
    if (event.altKey) {
      modifiers.push("Alt");
    }
    if (event.shiftKey) {
      modifiers.push("Shift");
    }
    if (event.metaKey) {
      modifiers.push("Super");
    }
    if (modifiers.length === 0) {
      setHotkeyError("至少需要一个修饰键：Ctrl、Alt 或 Shift。");
      return;
    }
    setHotkey([...modifiers, key].join("+"));
    setHotkeyError(null);
    setCapturing(false);
    hotkeyRef.current?.blur();
  };

  return (
    <>
      <header className="slip-header" data-tauri-drag-region>
        <button className="icon-btn back-btn" onClick={onClose} title="返回 (Esc)" aria-label="返回">
          <ChevronLeftIcon />
        </button>
        <div className="settings-heading" data-tauri-drag-region>
          <h1>设置</h1>
          <span className="version">v{version}</span>
        </div>
        <div className="header-right">
          <button
            className={`icon-btn pin-btn${settings.alwaysOnTop ? " active" : ""}`}
            onClick={onTogglePin}
            title={settings.alwaysOnTop ? "取消置顶" : "窗口置顶"}
            aria-label={settings.alwaysOnTop ? "取消置顶" : "窗口置顶"}
            aria-pressed={settings.alwaysOnTop}
          >
            <PinIcon size={17} />
          </button>
        </div>
      </header>

      <div className="settings-scroll">
        <div className="section-title">翻译</div>

        <div className="field">
          <div className="field-label">
            <span>目标语言</span>
            <span className="field-hint">自动识别语种</span>
          </div>
          <select
            className="control"
            value={targetMode}
            onChange={(event) => setTargetMode(event.target.value)}
          >
            {TARGET_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <div className="field">
          <div className="field-label">
            <span>呼出快捷键</span>
            <span className="field-hint">点击后按下组合键</span>
          </div>
          <div className="hotkey-control">
            <input
              ref={hotkeyRef}
              className={`control${capturing ? " capturing" : ""}`}
              value={capturing ? "按下组合键…" : hotkey}
              readOnly
              onFocus={() => setCapturing(true)}
              onBlur={() => setCapturing(false)}
              onKeyDown={onHotkeyKeyDown}
              aria-label="呼出快捷键"
            />
            <button
              className="ghost-btn"
              type="button"
              onClick={() => {
                setHotkey("Alt+T");
                setHotkeyError(null);
              }}
            >
              默认
            </button>
          </div>
          {hotkeyError && <div className="credential-state bad">{hotkeyError}</div>}
        </div>

        <div className="section-title">外观</div>

        <div className="field">
          <div className="field-label">
            <span>主题</span>
            <span className="field-hint">
              {theme === "translucent" ? "采样桌面背景" : theme === "dark" ? "纯黑，不透明" : "纯白，不透明"}
            </span>
          </div>
          <div className="segmented" role="radiogroup" aria-label="主题">
            {THEME_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                role="radio"
                aria-checked={theme === option.value}
                className={`segment${theme === option.value ? " selected" : ""}`}
                onClick={() => {
                  setTheme(option.value);
                  onThemePreview(option.value);
                }}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>

        <div className="section-title">火山引擎</div>

        <div className="field">
          <div className="field-label">
            <span>AccessKey ID</span>
            {settings.hasCredentials && settings.accessKeyHint ? (
              <span className="field-hint">已保存 {settings.accessKeyHint}</span>
            ) : (
              <span className="field-hint">火山引擎控制台 → 密钥管理</span>
            )}
          </div>
          <input
            className="control"
            value={accessKeyId}
            onChange={(event) => {
              setAccessKeyId(event.target.value);
              setTest(null);
            }}
            placeholder={settings.hasCredentials ? "留空则继续使用已保存的密钥" : "AKLT…"}
            spellCheck={false}
            autoComplete="off"
          />
        </div>

        <div className="field">
          <div className="field-label">
            <span>Secret Access Key</span>
            <span className="field-hint">只保存在本机</span>
          </div>
          <div className="hotkey-control">
            <input
              className="control"
              type={showSecret ? "text" : "password"}
              value={secretKey}
              onChange={(event) => {
                setSecretKey(event.target.value);
                setTest(null);
              }}
              placeholder={settings.hasCredentials ? "留空则继续使用已保存的密钥" : "粘贴 Secret Key"}
              spellCheck={false}
              autoComplete="off"
            />
            <button className="ghost-btn" type="button" onClick={() => setShowSecret((value) => !value)}>
              {showSecret ? "隐藏" : "显示"}
            </button>
          </div>
        </div>

        <div className="field">
          <button className="btn" type="button" onClick={testConnection} disabled={testing}>
            {testing ? "正在连接…" : "测试连接"}
          </button>
          {test && (
            <div className={`credential-state ${test.ok ? "ok" : "bad"}`}>
              {test.ok ? <CheckIcon size={12} /> : <AlertIcon size={12} />}
              <span>{test.message}</span>
            </div>
          )}
        </div>

        <div className="section-title">习惯</div>

        <label className="check">
          <span className="check-body">
            <span className="check-title">失焦自动收起</span>
            <span className="check-desc">切换到其他窗口时隐藏翻译窗。</span>
          </span>
          <span className="switch">
            <input
              type="checkbox"
              checked={hideOnBlur}
              onChange={(event) => setHideOnBlur(event.target.checked)}
            />
            <span className="switch-ui" aria-hidden />
          </span>
        </label>

        <label className="check">
          <span className="check-body">
            <span className="check-title">用剪贴板预填</span>
            <span className="check-desc">呼出时若剪贴板有新内容，直接填入输入框，按 Enter 翻译。</span>
          </span>
          <span className="switch">
            <input
              type="checkbox"
              checked={clipboardPrefill}
              onChange={(event) => setClipboardPrefill(event.target.checked)}
            />
            <span className="switch-ui" aria-hidden />
          </span>
        </label>

        <label className="check">
          <span className="check-body">
            <span className="check-title">粘贴后立刻翻译</span>
            <span className="check-desc">在输入框粘贴文本时直接翻译，不必按回车。</span>
          </span>
          <span className="switch">
            <input
              type="checkbox"
              checked={quickTranslate}
              onChange={(event) => setQuickTranslate(event.target.checked)}
            />
            <span className="switch-ui" aria-hidden />
          </span>
        </label>

        <label className="check">
          <span className="check-body">
            <span className="check-title">开机自动启动</span>
            <span className="check-desc">登录后在托盘待命，不弹出窗口。</span>
          </span>
          <span className="switch">
            <input
              type="checkbox"
              checked={autostart}
              onChange={(event) => setAutostart(event.target.checked)}
            />
            <span className="switch-ui" aria-hidden />
          </span>
        </label>
      </div>

      <div className="settings-actions">
        <div className="settings-note">AccessKey 只保存在本机配置里。</div>
        <button className="btn" type="button" onClick={onClose} disabled={saving}>
          取消
        </button>
        <button className="btn primary" type="button" onClick={save} disabled={saving}>
          {saving ? "保存中…" : "保存设置"}
        </button>
      </div>
    </>
  );
}
