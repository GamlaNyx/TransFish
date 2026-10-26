import { useCallback, useEffect, useRef, useState } from "react";
import Translator from "./components/Translator";
import Settings from "./components/Settings";
import {
  backend,
  events,
  isTauri,
  toApiError,
  type ApiError,
  type Bootstrap,
  type SettingsView,
  type SummonPayload,
  type Theme,
  type Translation,
} from "./lib/api";

type View = "translate" | "settings";

const DEMO_SETTINGS: SettingsView = {
  hotkey: "Alt+T",
  targetMode: "auto",
  theme: "translucent",
  hideOnBlur: true,
  clipboardPrefill: true,
  quickTranslate: true,
  autostart: false,
  alwaysOnTop: true,
  hasCredentials: false,
  accessKeyHint: "",
};

const DEMO_BOOT: Bootstrap = {
  settings: DEMO_SETTINGS,
  version: "0.1.0",
  hotkeyError: null,
  notice: null,
  dev: true,
};

export default function App() {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [fatal, setFatal] = useState<string | null>(null);
  const [view, setView] = useState<View>("translate");
  const [text, setText] = useState("");
  const [result, setResult] = useState<Translation | null>(null);
  const [lastText, setLastText] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const [backdrop, setBackdrop] = useState<string | null>(null);
  const [summonKey, setSummonKey] = useState(0);
  const [previewTheme, setPreviewTheme] = useState<Theme | null>(null);

  const inputRef = useRef<HTMLTextAreaElement>(null);
  const shellRef = useRef<HTMLDivElement>(null);
  const seqRef = useRef(0);
  const toastTimer = useRef<number | undefined>(undefined);
  const copyTimer = useRef<number | undefined>(undefined);
  const translateRef = useRef<(raw?: string) => void>(() => {});
  const summonRef = useRef<(payload: SummonPayload) => void>(() => {});

  const showToast = useCallback((message: string, duration = 1800) => {
    setToast(message);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), duration);
  }, []);

  const runTranslate = useCallback(
    async (raw?: string) => {
      const value = (raw ?? text).trim();
      if (!value) {
        setError({ code: "empty", message: "先输入一段文字吧。", retryable: false });
        inputRef.current?.focus();
        return;
      }
      const seq = ++seqRef.current;
      setLoading(true);
      setError(null);
      setCopied(false);
      try {
        const next = await backend.translate(value, null);
        if (seq !== seqRef.current) {
          return;
        }
        setResult(next);
        setLastText(value);
      } catch (caught) {
        if (seq !== seqRef.current) {
          return;
        }
        setResult(null);
        setError(toApiError(caught));
      } finally {
        if (seq === seqRef.current) {
          setLoading(false);
        }
      }
    },
    [text],
  );

  translateRef.current = (raw?: string) => {
    void runTranslate(raw);
  };

  const copyResult = useCallback(async () => {
    if (!result) {
      return;
    }
    try {
      await backend.copyText(result.text);
      setCopied(true);
      window.clearTimeout(copyTimer.current);
      copyTimer.current = window.setTimeout(() => setCopied(false), 1500);
      showToast("译文已复制");
    } catch (caught) {
      showToast(toApiError(caught).message);
    }
  }, [result, showToast]);

  const submit = useCallback(() => {
    if (loading) {
      return;
    }
    void runTranslate();
  }, [loading, runTranslate]);

  const handlePasteText = useCallback(
    (value: string) => {
      if (boot?.settings.quickTranslate && value.trim()) {
        void runTranslate(value);
      }
    },
    [boot, runTranslate],
  );

  const handleSummon = useCallback(
    (payload: SummonPayload) => {
      setView("translate");
      setError(null);
      setResult(null);
      setLastText("");
      setCopied(false);
      setSummonKey((key) => key + 1);

      const ambient = payload?.ambient;
      if (ambient && ambient.length === 3) {
        document.documentElement.style.setProperty("--ambient", ambient.join(" "));
        document.documentElement.dataset.ambient = "sampled";
      } else {
        document.documentElement.style.removeProperty("--ambient");
        delete document.documentElement.dataset.ambient;
      }

      setBackdrop(payload?.backdrop ?? null);

      const clipboard = (payload?.clipboard ?? "").trim();
      setText(clipboard);
      window.setTimeout(() => inputRef.current?.focus(), 40);
    },
    [],
  );
  summonRef.current = handleSummon;

  const activeTheme: Theme = previewTheme ?? boot?.settings.theme ?? "translucent";
  useEffect(() => {
    const root = document.documentElement;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    root.dataset.theme = activeTheme;
    const applyTone = () => {
      const tone =
        activeTheme === "dark"
          ? "dark"
          : activeTheme === "light"
            ? "light"
            : media.matches
              ? "dark"
              : "light";
      root.dataset.tone = tone;
      root.style.colorScheme = tone;
    };
    applyTone();
    if (activeTheme !== "translucent") {
      return;
    }
    media.addEventListener("change", applyTone);
    return () => media.removeEventListener("change", applyTone);
  }, [activeTheme]);

  const applyDemo = (demo: string | null) => {
    const credentialed = { ...DEMO_SETTINGS, hasCredentials: true };
    if (demo === "settings") {
      setBoot({ ...DEMO_BOOT, settings: { ...credentialed, accessKeyHint: "AKLT…9f2c" } });
      setView("settings");
      return;
    }
    if (demo === "idle") {
      setBoot({ ...DEMO_BOOT, settings: credentialed });
      return;
    }
    if (demo === "notice") {
      setBoot({
        ...DEMO_BOOT,
        settings: credentialed,
        hotkeyError: "「Alt+T」已被其他程序占用，已改用「Ctrl+Alt+T」。",
      });
      setText("The harbour woke slowly, gulls stitching white threads over the grey water.");
      return;
    }
    if (demo !== "result" && demo !== "error" && demo !== "long") {
      setBoot(DEMO_BOOT);
      return;
    }
    setBoot({ ...DEMO_BOOT, settings: credentialed });
    if (demo === "error") {
      setText("Bonjour, comment allez-vous ?");
      setError({ code: "rate", message: "请求太频繁了，请稍等一秒再试。", retryable: true });
      return;
    }
    if (demo === "long") {
      const source = "春日迟迟，卉木萋萋。仓庚喈喈，采蘩祁祁。执讯获丑，薄言还归。赫赫南仲，玁狁于夷。";
      setText(source);
      setLastText(source);
      setResult({
        text: "Spring comes slowly, the grasses and trees grow lush. The orioles sing, the women gather white mugwort in crowds. We have captured the enemy and taken prisoners; soon we shall return home. Illustrious is the southern expedition; the Xianyun are pacified.",
        source: "zh",
        target: "en",
        detected: true,
        latencyMs: 1210,
        requestId: "demo",
      });
      return;
    }
    const source = "The harbour woke slowly, gulls stitching white threads over the grey water.";
    setText(source);
    setLastText(source);
    setResult({
      text: "港湾缓缓醒来，海鸥在灰色的水面上缝出白色的线。",
      source: "en",
      target: "zh",
      detected: true,
      latencyMs: 846,
      requestId: "demo",
    });
  };

  useEffect(() => {
    let disposed = false;
    const unlisteners: (() => void)[] = [];

    const setup = async () => {
      if (!isTauri()) {
        if (import.meta.env.DEV) {
          applyDemo(new URLSearchParams(window.location.search).get("demo"));
        } else {
          setBoot(DEMO_BOOT);
        }
        return;
      }

      try {
        const initial = await backend.bootstrap();
        if (disposed) {
          return;
        }
        setBoot(initial);
        if (initial.notice) {
          showToast(initial.notice, 6000);
        }

        unlisteners.push(await events.onSummon((payload) => summonRef.current(payload)));
        unlisteners.push(await events.onOpenSettings(() => setView("settings")));
        unlisteners.push(
          await events.onSettingsChanged(async () => {
            try {
              const next = await backend.bootstrap();
              if (!disposed) {
                setBoot(next);
              }
            } catch {
              // Ignore: the tray toggle already reported its own error.
            }
          }),
        );
        unlisteners.push(await events.onToast((message) => showToast(message)));

        if (!disposed) {
          await backend.ready();
        }
      } catch (caught) {
        if (!disposed) {
          setFatal(toApiError(caught).message);
        }
      }
    };

    void setup();
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
      window.clearTimeout(toastTimer.current);
      window.clearTimeout(copyTimer.current);
    };
  }, [showToast]);

  useEffect(() => {
    const element = inputRef.current;
    if (!element) {
      return;
    }
    element.style.height = "auto";
    element.style.height = `${Math.min(element.scrollHeight, 164)}px`;
  }, [text, view]);

  useEffect(() => {
    const element = shellRef.current;
    if (!element) {
      return;
    }
    let frame = 0;
    const report = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const height = Math.ceil(element.getBoundingClientRect().height) + 24;
        if (isTauri()) {
          void backend.setWindowHeight(height).catch(() => undefined);
        }
      });
    };
    const observer = new ResizeObserver(report);
    observer.observe(element);
    report();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [view, summonKey, boot]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        if (view === "settings") {
          setPreviewTheme(null);
          setView("translate");
        } else if (isTauri()) {
          void backend.hideWindow().catch(() => undefined);
        }
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key === ",") {
        event.preventDefault();
        setView((current) => {
          if (current === "settings") {
            setPreviewTheme(null);
            return "translate";
          }
          return "settings";
        });
        return;
      }
      if (event.key === "Enter" && (event.ctrlKey || event.metaKey) && view === "translate") {
        event.preventDefault();
        translateRef.current();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [view]);

  const handleSaved = useCallback(
    (next: SettingsView) => {
      setPreviewTheme(null);
      setBoot((current) => (current ? { ...current, settings: next, hotkeyError: null } : current));
      setError((current) => (current?.code === "no_credentials" || current?.code === "auth" ? null : current));
      setView("translate");
      showToast("设置已保存");
    },
    [showToast],
  );

  const togglePin = useCallback(async () => {
    if (!boot) {
      return;
    }
    const next = !boot.settings.alwaysOnTop;
    try {
      const settings = await backend.setAlwaysOnTop(next);
      setBoot((current) => (current ? { ...current, settings } : current));
      showToast(next ? "窗口已置顶" : "已取消置顶", 1200);
    } catch (caught) {
      showToast(toApiError(caught).message);
    }
  }, [boot, showToast]);

  if (fatal) {
    return (
      <div className="appear">
        <div className="fatal">
          <strong>TransFish 没能启动。</strong>
          <br />
          {fatal}
        </div>
      </div>
    );
  }

  if (!boot) {
    return null;
  }

  const active = boot.settings;
  const dirty = Boolean(result) && text.trim() !== lastText;

  return (
    <div className="appear" key={summonKey} ref={shellRef}>
      <div className="slip" data-variant="regular" data-backdrop={backdrop ? "sampled" : "none"}>
        {backdrop && (
          <div
            className="glass-backdrop"
            style={{ backgroundImage: `url("${backdrop}")` }}
            aria-hidden="true"
          />
        )}
        <div className="glass-tint" aria-hidden="true" />
        {view === "settings" ? (
          <Settings
            settings={active}
            version={boot.version}
            onClose={() => {
              setPreviewTheme(null);
              setView("translate");
            }}
            onSaved={handleSaved}
            onTogglePin={() => void togglePin()}
            onThemePreview={setPreviewTheme}
          />
        ) : (
          <Translator
            settings={active}
            text={text}
            result={result}
            dirty={dirty}
            loading={loading}
            error={error}
            copied={copied}
            pinned={active.alwaysOnTop}
            hotkey={active.hotkey}
            hotkeyError={boot.hotkeyError}
            inputRef={inputRef}
            onTextChange={(value) => {
              setText(value);
              setCopied(false);
              setError(null);
            }}
            onPasteText={handlePasteText}
            onSubmit={submit}
            onCopy={() => void copyResult()}
            onRetry={() => void runTranslate()}
            onTogglePin={() => void togglePin()}
            onOpenSettings={() => setView("settings")}
          />
        )}
        {toast && <div className="toast">{toast}</div>}
      </div>
    </div>
  );
}
