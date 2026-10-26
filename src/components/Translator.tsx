import type { KeyboardEvent as ReactKeyboardEvent, RefObject } from "react";
import { AlertIcon, CopyIcon, CheckIcon, FishMark, PinIcon, SettingsIcon } from "./Icons";
import { formatLatency, langName, shortLang } from "../lib/lang";
import type { ApiError, SettingsView, Translation } from "../lib/api";

interface TranslatorProps {
  settings: SettingsView | null;
  text: string;
  result: Translation | null;
  dirty: boolean;
  loading: boolean;
  error: ApiError | null;
  copied: boolean;
  pinned: boolean;
  hotkey: string;
  hotkeyError?: string | null;
  inputRef: RefObject<HTMLTextAreaElement | null>;
  onTextChange: (value: string) => void;
  onPasteText?: (value: string) => void;
  onSubmit: () => void;
  onCopy: () => void;
  onRetry: () => void;
  onTogglePin: () => void;
  onOpenSettings: () => void;
}

function Direction({ settings, result }: { settings: SettingsView | null; result: Translation | null }) {
  if (result) {
    return (
      <>
        <span>{shortLang(result.source)}</span>
        <span className="arrow">→</span>
        <span>{shortLang(result.target)}</span>
      </>
    );
  }
  const mode = settings?.targetMode ?? "auto";
  return (
    <>
      <span>自动检测</span>
      <span className="arrow">→</span>
      <span>{mode === "auto" ? "中 / EN" : shortLang(mode)}</span>
    </>
  );
}

export default function Translator({
  settings,
  text,
  result,
  dirty,
  loading,
  error,
  copied,
  pinned,
  hotkey,
  hotkeyError,
  inputRef,
  onTextChange,
  onPasteText,
  onSubmit,
  onCopy,
  onRetry,
  onTogglePin,
  onOpenSettings,
}: TranslatorProps) {
  const onboarding = Boolean(settings && !settings.hasCredentials && !result && !error);
  const idle = !result && !loading && !error && !onboarding;

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) {
      return;
    }
    // Ctrl/Cmd+Enter is handled once by the app-level shortcut listener.
    if (event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }
    event.preventDefault();
    onSubmit();
  };

  return (
    <>
      <header className="slip-header" data-tauri-drag-region>
        <div className="brand" data-tauri-drag-region>
          <FishMark size={18} className="brand-mark" />
          <span className="brand-name">TransFish</span>
        </div>
        <div className="header-right">
          <span className="direction" aria-live="polite">
            <Direction settings={settings} result={result} />
          </span>
          <button
            className={`icon-btn pin-btn${pinned ? " active" : ""}`}
            onClick={onTogglePin}
            title={pinned ? "取消置顶" : "窗口置顶"}
            aria-label={pinned ? "取消置顶" : "窗口置顶"}
            aria-pressed={pinned}
          >
            <PinIcon size={17} />
          </button>
          <button className="icon-btn" onClick={onOpenSettings} title="设置 (Ctrl+,)" aria-label="设置">
            <SettingsIcon size={18} />
          </button>
        </div>
      </header>

      <div className="content">
        {hotkeyError && (
          <div className="notice" role="status">
            <span className="notice-icon">
              <AlertIcon />
            </span>
            <span className="notice-text">{hotkeyError}</span>
            <button className="text-btn" onClick={onOpenSettings}>
              更换
            </button>
          </div>
        )}
        <div className="input-shell">
          <textarea
            ref={inputRef}
            className="source"
            value={text}
            onChange={(event) => onTextChange(event.target.value)}
            onPaste={(event) => {
              const pasted = event.clipboardData.getData("text");
              if (!pasted) {
                return;
              }
              event.preventDefault();
              const element = event.currentTarget;
              const start = element.selectionStart ?? text.length;
              const end = element.selectionEnd ?? text.length;
              const next = `${text.slice(0, start)}${pasted}${text.slice(end)}`;
              onTextChange(next);
              onPasteText?.(next);
            }}
            onKeyDown={handleKeyDown}
            placeholder="粘贴或输入要翻译的文字…"
            spellCheck={false}
            rows={1}
            aria-label="待翻译文本"
          />
        </div>

        {loading && (
          <>
            <div className="progress" role="progressbar" aria-label="正在翻译" />
            <div className="loading-note">正在翻译…</div>
          </>
        )}

        {error && (
          <div className="error" role="alert">
            <span className="error-icon">
              <AlertIcon />
            </span>
            <div className="error-body">
              <div className="error-text">{error.message}</div>
              <div className="error-actions">
                {(error.code === "no_credentials" || error.code === "auth") && (
                  <button className="text-btn" onClick={onOpenSettings}>
                    打开设置
                  </button>
                )}
                {error.retryable && (
                  <button className="text-btn" onClick={onRetry}>
                    重试
                  </button>
                )}
                {error.code === "empty" && (
                  <button className="text-btn" onClick={() => inputRef.current?.focus()}>
                    回到输入
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        {result && !loading && (
          <div className={`result${dirty ? " dirty" : ""}`}>
            <div className="result-meta">
              <span className="label">译文</span>
              <span className="rule" />
            </div>
            <div className="result-text">{result.text}</div>
          </div>
        )}

        {onboarding && (
          <div className="onboarding">
            <div className="onboarding-title">先接上火山翻译</div>
            <div className="onboarding-text">
              TransFish 用火山引擎机器翻译。填入 AccessKey 后，按 {hotkey} 随时呼出，输入或粘贴即可翻译。
            </div>
            <div className="onboarding-actions">
              <button className="btn primary" onClick={onOpenSettings}>
                填写 AccessKey
              </button>
            </div>
          </div>
        )}

        {idle && (
          <div className="empty">
            <div className="empty-copy">
              <div className="empty-title">
                {settings && settings.targetMode !== "auto"
                  ? `自动识别语种，译成${langName(settings.targetMode)}。`
                  : "自动识别语种：中文译为英文，其余译为中文。"}
              </div>
              <div className="empty-keys">
                <span>
                  <kbd>Enter</kbd> 翻译
                </span>
                <span>
                  <kbd>Shift</kbd>
                  <kbd>Enter</kbd> 换行
                </span>
                <span>
                  <kbd>Esc</kbd> 收起
                </span>
              </div>
            </div>
          </div>
        )}
      </div>

      <footer className="slip-footer">
        <div className="meta">
          {result ? (
            <>
              <span>
                {langName(result.source) || "自动"} → {langName(result.target)}
              </span>
              <span className="dot" />
              <span>{formatLatency(result.latencyMs)}</span>
            </>
          ) : (
            <span className="hotkey-hint">
              <kbd>{hotkey}</kbd> 呼出
            </span>
          )}
        </div>
        <div className="footer-right">
          <button
            className={`copy-btn${copied ? " copied" : ""}`}
            onClick={onCopy}
            disabled={!result || loading}
            title={result ? "复制译文" : "翻译后可复制"}
          >
            {copied ? <CheckIcon /> : <CopyIcon />}
            {copied ? "已复制" : "复制译文"}
          </button>
        </div>
      </footer>
    </>
  );
}
