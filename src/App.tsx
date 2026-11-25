import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Settings = {
  model: string;
  compute_type: string;
  language: string;
  microphone: string;
  shortcut: string;
  mode: string;
  remove_fillers: boolean;
  auto_insert: boolean;
  vad: boolean;
  live_preview: boolean;
  terminology: string;
};
type View = {
  phase: "loading" | "ready" | "recording" | "transcribing" | "error";
  message: string;
  text: string;
  raw_text: string;
  partial: string;
  seconds: number;
  inference_seconds: number | null;
  model_load_seconds: number | null;
  inserted: boolean;
};
const initialView: View = {
  phase: "loading",
  message: "Connecting to the local speech engine…",
  text: "",
  raw_text: "",
  partial: "",
  seconds: 0,
  inference_seconds: null,
  model_load_seconds: null,
  inserted: false,
};

function useEngine() {
  const [view, setView] = useState(initialView);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!isTauri()) {
      setError(
        "Open SONIQ in its Windows desktop app to use the microphone and local speech engine.",
      );
      return;
    }
    let disposed = false;
    let receivedEvent = false;
    let unsubscribe: (() => void) | undefined;
    // Subscribe before taking a snapshot so startup events are not missed.
    void (async () => {
      try {
        unsubscribe = await listen<View>("state", (event) => {
          if (!disposed) {
            receivedEvent = true;
            setView(event.payload);
          }
        });
        if (disposed) {
          unsubscribe();
          return;
        }
        const result = await invoke<{ view: View; settings: Settings }>(
          "snapshot",
        );
        if (!disposed) {
          if (!receivedEvent) setView(result.view);
          setSettings(result.settings);
        }
      } catch (cause) {
        if (!disposed)
          setError(`Could not connect to the desktop engine: ${String(cause)}`);
      }
    })();
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);
  return { view, settings, setSettings, error, setError };
}

function Mic({ size = 22 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      aria-hidden="true"
    >
      <rect x="8" y="2" width="8" height="13" rx="4" />
      <path d="M5 10v2a7 7 0 0 0 14 0v-2M12 19v3m-4 0h8" />
    </svg>
  );
}

export function Indicator() {
  const { view } = useEngine();
  return (
    <div className="indicator" role="status">
      <span
        className={`status-dot ${view.phase === "recording" ? "active" : ""}`}
      />
      <div>
        <strong>
          {view.phase === "recording" ? "Listening" : "Transcribing locally"}
        </strong>
        <small>
          {view.phase === "recording"
            ? `${view.seconds.toFixed(0)}s · Release shortcut to finish`
            : "Your audio stays on this device"}
        </small>
      </div>
      <button
        className="icon-button"
        aria-label="Cancel dictation"
        onClick={() => void invoke("cancel")}
      >
        ×
      </button>
    </div>
  );
}

export function App() {
  const { view, settings, setSettings, error, setError } = useEngine();
  const [page, setPage] = useState<"dictation" | "settings">("dictation");
  const [devices, setDevices] = useState<string[]>([]);
  const [original, setOriginal] = useState(false);
  const [notice, setNotice] = useState("");
  const [pending, setPending] = useState(false);
  const busy =
    view.phase === "recording" ||
    view.phase === "transcribing" ||
    view.phase === "loading";

  async function action(command: string, args?: Record<string, unknown>) {
    setError("");
    setNotice("");
    setPending(true);
    try {
      await invoke(command, args);
      if (command === "copy_text") setNotice("Copied to clipboard.");
    } catch (cause) {
      setError(String(cause));
    } finally {
      setPending(false);
    }
  }
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    function refresh() {
      void invoke<string[]>("microphones")
        .then((result) => {
          if (!disposed) setDevices(result);
        })
        .catch((cause) => {
          if (!disposed) setError(String(cause));
        });
    }
    refresh();
    window.addEventListener("focus", refresh);
    return () => {
      disposed = true;
      window.removeEventListener("focus", refresh);
    };
  }, [page]);
  useEffect(() => {
    function cancel(event: KeyboardEvent) {
      if (
        event.key === "Escape" &&
        (view.phase === "recording" || view.phase === "transcribing")
      ) {
        void invoke("cancel").catch((cause) => setError(String(cause)));
      }
    }
    window.addEventListener("keydown", cancel);
    return () => window.removeEventListener("keydown", cancel);
  }, [view.phase, setError]);

  async function save(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!settings) return;
    setPending(true);
    setError("");
    setNotice("");
    try {
      await invoke("save_settings", { settings });
      setNotice("Preferences saved on this device.");
    } catch (cause) {
      setError(String(cause));
    } finally {
      setPending(false);
    }
  }
  function setting<K extends keyof Settings>(key: K, value: Settings[K]) {
    setSettings((previous) =>
      previous ? { ...previous, [key]: value } : previous,
    );
  }
  const transcript = original ? view.raw_text : view.text;
  const active = view.phase === "recording";

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(event) => {
            event.preventDefault();
            setPage("dictation");
          }}
        >
          <span className="brand-mark">
            <Mic size={21} />
          </span>
          <span>
            SONIQ
          </span>
        </a>
        <div className="workspace-label">YOUR WORKSPACE</div>
        <nav aria-label="Main navigation">
          <button
            className={page === "dictation" ? "nav-item selected" : "nav-item"}
            onClick={() => setPage("dictation")}
          >
            <Mic size={18} />
            Dictation<span className="nav-arrow">↗</span>
          </button>
          <button
            className={page === "settings" ? "nav-item selected" : "nav-item"}
            onClick={() => setPage("settings")}
          >
            <span aria-hidden="true">⚙</span>Preferences
          </button>
        </nav>
        <div className="sidebar-bottom">
          <span className="privacy-glyph" aria-hidden="true">
            ◈
          </span>
          <strong>Made to stay private.</strong>
          <p>
            Local recognition.
            <br />
            Your voice, on your device.
          </p>
          <span className="version">WINDOWS · v0.2.0</span>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>
            Personal workspace <span className="slash">/</span>{" "}
            {page === "dictation" ? "Dictation" : "Preferences"}
          </span>
          <span className="local-badge">
            <span className="status-dot" />
            LOCAL ENGINE
          </span>
        </header>
        <div className="content">
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                {page === "dictation"
                  ? "A LITTLE LESS TYPING"
                  : "MAKE IT YOURS"}
              </div>
              <h1>
                {page === "dictation"
                  ? "Let your thoughts flow."
                  : "Your voice. Your setup."}
              </h1>
              <p>
                {page === "dictation"
                  ? "Speak naturally. Turn your words into something useful."
                  : "Simple controls for a quieter, more personal workflow."}
              </p>
            </div>
            <span className="outline-pill">Offline by default</span>
          </div>
          {error ? (
            <div className="alert error" role="alert">
              {error}
            </div>
          ) : null}
          {notice ? (
            <div className="alert" role="status">
              {notice}
            </div>
          ) : null}
          {page === "dictation" ? (
            <>
              <section
                className={`record-card ${active ? "is-recording" : ""}`}
                aria-label="Voice recording"
              >
                <div className="card-top">
                  <span className="eyebrow">VOICE TO TEXT</span>
                  <span className="engine-name">
                    {settings?.model ?? "…"} <span>· CPU</span>
                  </span>
                </div>
                <div className="record-center">
                  <div className="record-orbit">
                    <button
                      className={`mic-button ${active ? "recording" : ""}`}
                      aria-label={active ? "Stop recording" : "Start recording"}
                      disabled={pending || (!active && view.phase !== "ready")}
                      onClick={() =>
                        void action("recording", { active: !active })
                      }
                    >
                      {active ? (
                        <span className="stop-icon" />
                      ) : (
                        <Mic size={32} />
                      )}
                    </button>
                  </div>
                  <h2>
                    {active
                      ? "Listening to you…"
                      : view.phase === "transcribing"
                        ? "Finding your words…"
                        : view.phase === "loading"
                          ? "Getting ready…"
                          : view.phase === "error"
                            ? "Let’s reconnect."
                            : "Ready when you are."}
                  </h2>
                  <p className="engine-message" role="status">
                    {view.message}
                  </p>
                  {active ? (
                    <div className="record-controls">
                      <span className="record-clock">
                        <span className="status-dot active" />
                        {view.seconds.toFixed(1)}s / 120s
                      </span>
                      <button
                        className="text-button"
                        onClick={() => void action("cancel")}
                      >
                        Cancel
                      </button>
                    </div>
                  ) : null}
                  {view.phase === "transcribing" ? (
                    <button
                      className="text-button"
                      onClick={() => void action("cancel")}
                    >
                      Cancel transcription
                    </button>
                  ) : null}
                  {view.phase === "error" ? (
                    <div className="recovery">
                      <button
                        className="button"
                        disabled={pending}
                        onClick={() =>
                          void action("reload", { download: false })
                        }
                      >
                        Reload engine
                      </button>
                      <button
                        className="button secondary"
                        disabled={pending}
                        onClick={() =>
                          void action("reload", { download: true })
                        }
                      >
                        Download selected model
                      </button>
                    </div>
                  ) : null}
                </div>
                <div className="shortcut-strip">
                  <span>Dictate in another app</span>
                  <kbd>{settings?.shortcut ?? "Ctrl+Space"}</kbd>
                  <span>hold to talk · release to finish</span>
                </div>
              </section>
              <section
                className="transcript-card"
                aria-label="Transcript preview"
              >
                <div className="transcript-header">
                  <div>
                    <h2>Your words</h2>
                    <span>Only this session. No saved history.</span>
                  </div>
                  <div className="transcript-tools">
                    <button
                      className={`tab-button ${!original ? "chosen" : ""}`}
                      onClick={() => setOriginal(false)}
                    >
                      Transcript
                    </button>
                    <button
                      className={`tab-button ${original ? "chosen" : ""}`}
                      onClick={() => setOriginal(true)}
                    >
                      Original
                    </button>
                    <button
                      className="button small secondary"
                      disabled={!transcript || pending}
                      onClick={() => {
                        void action("copy_text", { original });
                      }}
                    >
                      Copy
                    </button>
                  </div>
                </div>
                {view.partial && active ? (
                  <div className="partial">
                    <span>LIVE PREVIEW · MAY CHANGE</span>
                    <p>{view.partial}</p>
                  </div>
                ) : transcript ? (
                  <div className="transcript-text" tabIndex={0}>
                    {transcript}
                  </div>
                ) : (
                  <div className="empty-transcript">
                    <span aria-hidden="true">“</span>
                    <p>A clear space for your next thought.</p>
                    <small>
                      Your transcript will appear here after recording.
                    </small>
                  </div>
                )}
                <div className="transcript-footer">
                  <span>
                    {view.inserted
                      ? "✓ Inserted into original field"
                      : "Preview first. Copy whenever you’re ready."}
                  </span>
                  {view.inference_seconds !== null ? (
                    <span>{view.inference_seconds.toFixed(2)}s inference</span>
                  ) : (
                    <span>EN / हिन्दी / Hinglish</span>
                  )}
                </div>
              </section>
              <div className="privacy-note">
                <span aria-hidden="true">◇</span> No audio uploads. No account.
                No transcript history.
              </div>
            </>
          ) : settings ? (
            <form
              className="settings-card"
              onSubmit={(event) => void save(event)}
            >
              <div className="settings-heading">
                <h2>Recognition</h2>
                <p>Choose the balance that works for your laptop.</p>
              </div>
              <div className="settings-grid">
                <label>
                  Speech model
                  <select
                    value={settings.model}
                    onChange={(e) => setting("model", e.target.value)}
                  >
                    <option value="tiny">Tiny · lighter</option>
                    <option value="base">Base · balanced</option>
                    <option value="small">Small · larger</option>
                  </select>
                </label>
                <label>
                  Language
                  <select
                    value={settings.language}
                    onChange={(e) => setting("language", e.target.value)}
                  >
                    <option value="auto">Auto-detect / Hinglish</option>
                    <option value="en">English</option>
                    <option value="hi">Hindi</option>
                  </select>
                </label>
                <label>
                  Microphone preference
                  <select
                    value={settings.microphone}
                    onChange={(e) => setting("microphone", e.target.value)}
                  >
                    <option value="">Automatic · Windows input</option>
                    {settings.microphone && !devices.includes(settings.microphone) ? (
                      <option value={settings.microphone}>
                        {settings.microphone} · not listed; automatic fallback
                      </option>
                    ) : null}
                    {[...new Set(devices)].map((name) => (
                      <option key={name} value={name}>
                        {name}
                      </option>
                    ))}
                  </select>
                  <small>
                    Uses your preferred microphone when available, otherwise the
                    Windows input. Each recording checks again automatically.
                  </small>
                </label>
                <label>
                  CPU precision
                  <select
                    value={settings.compute_type}
                    onChange={(e) => setting("compute_type", e.target.value)}
                  >
                    <option value="int8">INT8</option>
                    <option value="float32">Float32</option>
                  </select>
                </label>
                <label>
                  Hold-to-talk shortcut
                  <input
                    value={settings.shortcut}
                    maxLength={80}
                    onChange={(e) => setting("shortcut", e.target.value)}
                    spellCheck={false}
                  />
                </label>
                <label>
                  Writing style
                  <select
                    value={settings.mode}
                    onChange={(e) => setting("mode", e.target.value)}
                  >
                    <option value="verbatim">
                      Verbatim · preserve everything
                    </option>
                    <option value="clean">Clean · tidy whitespace</option>
                    <option value="paragraphs">
                      Paragraphs · break at sentences
                    </option>
                  </select>
                </label>
              </div>
              <label className="vocabulary-label">
                Custom terminology
                <textarea
                  value={settings.terminology}
                  maxLength={2000}
                  rows={2}
                  placeholder="Names and technical terms, separated by commas"
                  onChange={(e) => setting("terminology", e.target.value)}
                />
                <small>
                  Hints for recognition, not forced replacements. Always review
                  names and identifiers.
                </small>
              </label>
              <div className="settings-heading divided">
                <h2>Control & privacy</h2>
                <p>Everything below is processed on this device.</p>
              </div>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={settings.auto_insert}
                  onChange={(e) => setting("auto_insert", e.target.checked)}
                />
                <span>
                  <strong>Insert into the original field</strong>
                  <small>
                    Checks focus and password status. Unsupported fields fall
                    back to manual copy. Off by default.
                  </small>
                </span>
              </label>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={settings.vad}
                  onChange={(e) => setting("vad", e.target.checked)}
                />
                <span>
                  <strong>Filter silence</strong>
                  <small>
                    Local Silero voice activity detection. Review quiet speech
                    for omissions.
                  </small>
                </span>
              </label>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={settings.live_preview}
                  onChange={(e) => setting("live_preview", e.target.checked)}
                />
                <span>
                  <strong>Experimental live preview</strong>
                  <small>
                    Refreshes a rolling preview about every 4 seconds. May
                    increase final transcription latency.
                  </small>
                </span>
              </label>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={settings.remove_fillers}
                  disabled={settings.mode === "verbatim"}
                  onChange={(e) => setting("remove_fillers", e.target.checked)}
                />
                <span>
                  <strong>Remove “um” and “uh”</strong>
                  <small>
                    Only in cleanup modes. Original remains available; technical
                    and digit-bearing text is preserved.
                  </small>
                </span>
              </label>
              <div className="settings-actions">
                <button className="button" disabled={pending || busy}>
                  Save preferences
                </button>
                <button
                  type="button"
                  className="button secondary"
                  disabled={busy || pending}
                  onClick={() => void action("reload", { download: true })}
                >
                  Download saved model
                </button>
                <button
                  type="button"
                  className="text-button"
                  disabled={pending}
                  onClick={() =>
                    void invoke<string[]>("microphones")
                      .then(setDevices)
                      .catch((e) => setError(String(e)))
                  }
                >
                  Refresh inputs
                </button>
              </div>
              <p className="settings-note">
                Model downloads contact Hugging Face and can be large. Audio and
                transcripts are never sent. Preferences are stored locally;
                temporary capture files are deleted after processing.
              </p>
            </form>
          ) : (
            <p>Loading preferences…</p>
          )}
        </div>
      </main>
    </div>
  );
}
