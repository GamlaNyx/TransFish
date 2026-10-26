import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface SettingsView {
  hotkey: string;
  targetMode: string;
  hideOnBlur: boolean;
  clipboardPrefill: boolean;
  quickTranslate: boolean;
  autostart: boolean;
  alwaysOnTop: boolean;
  hasCredentials: boolean;
  accessKeyHint: string;
}

export interface Bootstrap {
  settings: SettingsView;
  version: string;
  hotkeyError: string | null;
  notice: string | null;
  dev: boolean;
}

export interface Translation {
  text: string;
  source: string;
  target: string;
  detected: boolean;
  latencyMs: number;
  requestId: string | null;
}

export interface DetectResult {
  language: string;
  confidence: number;
  latencyMs: number;
}

export interface ApiError {
  code: string;
  message: string;
  retryable: boolean;
}

export interface SummonPayload {
  clipboard: string | null;
  /// Average desktop colour under the slip, sampled right before it is shown.
  ambient: [number, number, number] | null;
  /// Downscaled desktop snapshot behind the slip, as a BMP data URL.
  backdrop: string | null;
}

export interface SettingsPatch {
  accessKeyId?: string;
  secretAccessKey?: string;
  hotkey?: string;
  targetMode?: string;
  hideOnBlur?: boolean;
  clipboardPrefill?: boolean;
  quickTranslate?: boolean;
  autostart?: boolean;
  alwaysOnTop?: boolean;
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function toApiError(error: unknown): ApiError {
  if (error && typeof error === "object" && "message" in error) {
    const candidate = error as Partial<ApiError>;
    return {
      code: candidate.code ?? "error",
      message: String(candidate.message ?? "出了点问题，请稍后再试。"),
      retryable: Boolean(candidate.retryable),
    };
  }
  if (typeof error === "string" && error.trim()) {
    return { code: "error", message: error, retryable: false };
  }
  return { code: "error", message: "出了点问题，请稍后再试。", retryable: false };
}

export const backend = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  ready: () => invoke<void>("ready"),
  translate: (text: string, target?: string | null) =>
    invoke<Translation>("translate", { text, target: target ?? null }),
  testCredentials: (accessKeyId?: string, secretAccessKey?: string) =>
    invoke<DetectResult>("test_credentials", {
      accessKeyId: accessKeyId ?? null,
      secretAccessKey: secretAccessKey ?? null,
    }),
  saveSettings: (patch: SettingsPatch) =>
    invoke<SettingsView>("save_settings", { patch }),
  setAlwaysOnTop: (enabled: boolean) =>
    invoke<SettingsView>("set_always_on_top", { enabled }),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  setWindowHeight: (height: number) =>
    invoke<void>("set_window_height", { height }),
  hideWindow: () => invoke<void>("hide_window"),
};

export const events = {
  onSummon: (handler: (payload: SummonPayload) => void): Promise<UnlistenFn> =>
    listen<SummonPayload>("summon", (event) => handler(event.payload)),
  onOpenSettings: (handler: () => void): Promise<UnlistenFn> =>
    listen("open-settings", () => handler()),
  onSettingsChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen("settings-changed", () => handler()),
  onToast: (handler: (message: string) => void): Promise<UnlistenFn> =>
    listen<string>("toast", (event) => handler(event.payload)),
};
