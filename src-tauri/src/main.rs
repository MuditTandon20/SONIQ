#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod native;
mod settings;
mod worker;

use serde::Serialize;
use serde_json::{json, Value};
use settings::Settings;
use std::{
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Clone, Serialize)]
struct View {
    phase: String,
    message: String,
    text: String,
    raw_text: String,
    partial: String,
    seconds: f64,
    inference_seconds: Option<f64>,
    model_load_seconds: Option<f64>,
    inserted: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            phase: "loading".into(),
            message: "Starting local speech engine…".into(),
            text: String::new(),
            raw_text: String::new(),
            partial: String::new(),
            seconds: 0.,
            inference_seconds: None,
            model_load_seconds: None,
            inserted: false,
        }
    }
}

struct Runtime {
    settings: Settings,
    view: View,
    worker: Option<worker::Worker>,
    recording: Option<mpsc::Sender<bool>>,
    session: u64,
    target: Option<native::Target>,
    session_dir: Option<tempfile::TempDir>,
    preview_pending: bool,
    worker_generation: u64,
}

type Shared = Arc<Mutex<Runtime>>;

fn emit(app: &tauri::AppHandle, state: &Runtime) {
    let _ = app.emit("state", &state.view);
    let show = ["recording", "transcribing"].contains(&state.view.phase.as_str());
    let handle = app.clone();
    // Window operations must not wait for the UI thread while state is locked.
    let _ = app.run_on_main_thread(move || {
        if let Some(indicator) = handle.get_webview_window("indicator") {
            if show {
                if !indicator.is_visible().unwrap_or(false) {
                    let _ = indicator.show();
                }
            } else {
                let _ = indicator.hide();
            }
        }
    });
}

fn fail(app: &tauri::AppHandle, message: String) {
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    state.view.phase = "error".into();
    state.view.message = message;
    emit(app, &state);
}

fn worker_failed(app: &tauri::AppHandle, generation: u64, error: String) {
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    if generation != state.worker_generation {
        return;
    }
    if let Some(stop) = state.recording.take() {
        let _ = stop.send(true);
    }
    state.view.phase = "error".into();
    state.view.message = error;
    state.target = None;
    state.session_dir = None;
    emit(app, &state);
}

fn receive(app: &tauri::AppHandle, value: Value, generation: u64) {
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    let mut insertion = None;
    if generation != state.worker_generation {
        return;
    }
    if value["event"] == "ready" {
        state.view.phase = "ready".into();
        state.view.message = "Ready. Your microphone is off.".into();
        state.view.model_load_seconds = value["load_seconds"].as_f64();
    } else {
        if value["id"].as_u64() != Some(state.session) {
            return;
        }
        if value["event"] == "partial" {
            state.preview_pending = false;
            if state.view.phase == "recording" {
                state.view.partial = value["text"].as_str().unwrap_or("").into();
            }
        } else if value["event"] == "transcript" {
            state.view.text = value["text"].as_str().unwrap_or("").into();
            state.view.raw_text = value["raw_text"].as_str().unwrap_or("").into();
            state.view.inference_seconds = value["inference_seconds"].as_f64();
            state.view.phase = "ready".into();
            state.view.partial.clear();
            state.session_dir = None;
            state.view.message = if state.view.text.is_empty() {
                "No speech recognized."
            } else {
                "Transcript ready. Copy it, or dictate into a supported field."
            }
            .into();
            if state.settings.auto_insert && !state.view.text.is_empty() {
                insertion = Some((state.target.clone(), state.view.text.clone(), state.session));
                state.view.phase = "transcribing".into();
                state.view.message = "Checking the original field…".into();
            }
            state.target = None;
        } else if value["event"] == "error" {
            state.view.phase = "error".into();
            state.view.message = value["message"]
                .as_str()
                .unwrap_or("Speech engine failed.")
                .into();
            if let Some(stop) = state.recording.take() {
                let _ = stop.send(true);
            }
            state.target = None;
            state.session_dir = None;
        }
    }
    emit(app, &state);
    drop(state);
    // UI Automation can call a window provider: do not hold state across it.
    if let Some((target, text, session)) = insertion {
        let result = target
            .ok_or("No supported target was focused when recording started.".into())
            .and_then(|target| {
                native::insert(&target, &text, || shared.lock().unwrap().session == session)
            });
        let mut state = shared.lock().unwrap();
        if state.session != session {
            return;
        }
        state.view.phase = "ready".into();
        match result {
            Ok(()) => {
                state.view.inserted = true;
                state.view.message = "Text sent to the original field. Review it there.".into();
            }
            Err(error) => state.view.message = format!("Text kept here for manual copy. {error}"),
        }
        emit(app, &state);
    }
}

fn launch_worker(app: &tauri::AppHandle, download: bool) -> Result<(), String> {
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    if state.recording.is_some() {
        return Err("Stop recording before reloading the engine.".into());
    }
    state.worker_generation += 1;
    if let Some(mut old) = state.worker.take() {
        old.stop();
    }
    state.session += 1;
    state.target = None;
    state.session_dir = None;
    state.preview_pending = false;
    state.view.phase = "loading".into();
    state.view.message = if download {
        "Downloading selected model from Hugging Face…"
    } else {
        "Loading cached model locally…"
    }
    .into();
    emit(app, &state);
    let mut process = worker::Worker::start(app.clone(), state.worker_generation)?;
    process.send(
        json!({"command":"load", "id": state.session, "model":state.settings.model,
        "compute_type":state.settings.compute_type,"download":download}),
    )?;
    state.worker = Some(process);
    Ok(())
}

fn start_recording(app: &tauri::AppHandle, external: bool) -> Result<(), String> {
    let target = if external {
        native::focused_target().ok()
    } else {
        None
    };
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    if state.view.phase != "ready" {
        return Err("Wait until the speech engine is ready.".into());
    }
    state.target = target;
    state.session += 1;
    let session = state.session;
    let capture_root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("captures");
    std::fs::create_dir_all(&capture_root).map_err(|e| e.to_string())?;
    let directory = tempfile::Builder::new()
        .prefix("session-")
        .tempdir_in(capture_root)
        .map_err(|e| e.to_string())?;
    let path = directory.path().join("capture.wav");
    state.session_dir = Some(directory);
    state.preview_pending = false;
    state.view = View {
        phase: "recording".into(),
        message: "Recording. Release the shortcut to transcribe.".into(),
        model_load_seconds: state.view.model_load_seconds,
        ..View::default()
    };
    let (stop, commands) = mpsc::channel();
    state.recording = Some(stop);
    let microphone = state.settings.microphone.clone();
    let live = state.settings.live_preview;
    emit(app, &state);
    let app = app.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let (stream, audio) = capture::start(&microphone)?;
            let started = Instant::now();
            let mut last_preview = Instant::now();
            loop {
                match commands.recv_timeout(Duration::from_millis(100)) {
                    Ok(true) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        drop(stream);
                        return Ok(());
                    }
                    Ok(false) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let shared = app.state::<Shared>();
                let mut state = shared.lock().unwrap();
                if state.session != session {
                    return Ok(());
                }
                let audio = audio.lock().unwrap();
                if let Some(error) = &audio.error {
                    return Err(error.clone());
                }
                let full = audio.samples.len() >= audio.rate as usize * capture::MAX_SECONDS;
                let should_preview = live
                    && last_preview.elapsed() >= Duration::from_secs(4)
                    && !state.preview_pending;
                let preview_data = if should_preview {
                    let offset = audio.samples.len().saturating_sub(audio.rate as usize * 12);
                    Some((audio.rate, audio.samples[offset..].to_vec()))
                } else {
                    None
                };
                drop(audio);
                state.view.seconds = started.elapsed().as_secs_f64();
                emit(&app, &state);
                if full {
                    break;
                }
                if let Some((rate, samples)) = preview_data {
                    // One in-flight preview at most; snapshots use the last 12 seconds.
                    let preview = path.with_file_name("preview.wav");
                    capture::save(&preview, rate, &samples)?;
                    let settings = state.settings.clone();
                    let request = json!({"command":"preview","id":session,"audio":preview,
                        "language":if settings.language == "auto" { None } else { Some(settings.language) },
                        "vad":settings.vad,"hotwords":settings.terminology});
                    state
                        .worker
                        .as_mut()
                        .ok_or("Speech engine is unavailable")?
                        .send(request)?;
                    state.preview_pending = true;
                    last_preview = Instant::now();
                }
            }
            drop(stream);
            let audio = audio.lock().unwrap();
            if let Some(error) = &audio.error {
                return Err(error.clone());
            }
            capture::save(&path, audio.rate, &audio.samples)?;
            let shared = app.state::<Shared>();
            let mut state = shared.lock().unwrap();
            if state.session != session {
                return Ok(());
            }
            state.recording = None;
            state.view.phase = "transcribing".into();
            state.view.message = "Transcribing on your computer…".into();
            state.view.seconds = audio.samples.len() as f64 / audio.rate as f64;
            let settings = state.settings.clone();
            state
                .worker
                .as_mut()
                .ok_or("Speech engine unavailable")?
                .send(json!({
                "command":"transcribe","id":session,"audio":path,
                "language":if settings.language == "auto" { None } else { Some(settings.language) },
                "vad":settings.vad,"mode":settings.mode,"remove_fillers":settings.remove_fillers,
                "hotwords":settings.terminology}))?;
            emit(&app, &state);
            Ok(())
        })();
        if let Err(error) = result {
            let shared = app.state::<Shared>();
            let mut state = shared.lock().unwrap();
            if state.session == session {
                state.recording = None;
                state.session_dir = None;
                state.view.phase = "error".into();
                state.view.message = error;
                emit(&app, &state);
            }
        }
    });
    Ok(())
}

fn stop_recording(app: &tauri::AppHandle) -> Result<(), String> {
    let shared = app.state::<Shared>();
    let state = shared.lock().unwrap();
    if let Some(stop) = &state.recording {
        stop.send(false).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn snapshot(state: tauri::State<Shared>) -> Value {
    let state = state.lock().unwrap();
    json!({"view":state.view,"settings":state.settings})
}

#[tauri::command]
fn microphones() -> Result<Vec<String>, String> {
    capture::devices()
}

#[tauri::command]
fn recording(app: tauri::AppHandle, active: bool) -> Result<(), String> {
    if active {
        start_recording(&app, false)
    } else {
        stop_recording(&app)
    }
}

#[tauri::command]
fn cancel(app: tauri::AppHandle) -> Result<(), String> {
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    if let Some(stop) = state.recording.take() {
        let _ = stop.send(true);
    }
    let restart = state.view.phase == "transcribing" || state.preview_pending;
    state.session += 1;
    state.target = None;
    state.view.text.clear();
    state.view.raw_text.clear();
    state.view.partial.clear();
    state.view.phase = "ready".into();
    state.view.message = "Cancelled. Audio and transcript discarded.".into();
    state.session_dir = None;
    emit(&app, &state);
    drop(state);
    if restart {
        launch_worker(&app, false)?;
    }
    Ok(())
}

#[tauri::command]
fn reload(app: tauri::AppHandle, download: bool) -> Result<(), String> {
    launch_worker(&app, download)
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    settings.validate()?;
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    if !["ready", "error"].contains(&state.view.phase.as_str()) {
        return Err("Finish the current recording or model load before changing settings.".into());
    }
    if state.settings.shortcut != settings.shortcut
        || !app
            .global_shortcut()
            .is_registered(settings.shortcut.as_str())
    {
        register_shortcut(&app, &settings.shortcut)?;
        if let Err(error) = settings.save(
            &app.path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .join("settings.json"),
        ) {
            let _ = app.global_shortcut().unregister(settings.shortcut.as_str());
            return Err(error);
        }
        if state.settings.shortcut != settings.shortcut {
            let _ = app
                .global_shortcut()
                .unregister(state.settings.shortcut.as_str());
        }
    } else {
        settings.save(
            &app.path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .join("settings.json"),
        )?;
    }
    let reload = settings.model != state.settings.model
        || settings.compute_type != state.settings.compute_type
        || state.worker.is_none();
    state.settings = settings;
    drop(state);
    if reload {
        launch_worker(&app, false)?;
    }
    Ok(())
}

#[tauri::command]
fn copy_text(app: tauri::AppHandle, original: bool) -> Result<(), String> {
    let shared = app.state::<Shared>();
    let state = shared.lock().unwrap();
    native::copy(if original {
        &state.view.raw_text
    } else {
        &state.view.text
    })
}

fn register_shortcut(app: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            let result = if event.state == ShortcutState::Pressed {
                // Auto-repeat while recording does not start another capture.
                let ready = app.state::<Shared>().lock().unwrap().view.phase == "ready";
                if !ready {
                    return;
                }
                start_recording(app, true)
            } else {
                stop_recording(app)
            };
            if let Err(error) = result {
                fail(app, error);
            }
        })
        .map_err(|e| format!("Shortcut could not be registered: {e}"))
}

fn main() {
    let _instance = match native::single_instance() {
        Ok(Some(handle)) => handle,
        Ok(None) => return,
        Err(error) => {
            eprintln!("Cannot initialize desktop instance: {error}");
            return;
        }
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                // The hidden indicator must not keep the worker and shortcuts alive.
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            // The named mutex prevents deleting another live instance's audio.
            let root = app.path().app_local_data_dir()?.join("captures");
            std::fs::create_dir_all(&root)?;
            let root = root.canonicalize()?;
            for entry in std::fs::read_dir(&root)?.flatten() {
                if entry.file_name().to_string_lossy().starts_with("session-")
                    && entry
                        .file_type()
                        .map(|kind| kind.is_dir() && !kind.is_symlink())
                        .unwrap_or(false)
                {
                    if let Ok(path) = entry.path().canonicalize() {
                        if path.parent() == Some(root.as_path()) {
                            let _ = std::fs::remove_dir_all(path);
                        }
                    }
                }
            }
            let path = app.path().app_config_dir()?.join("settings.json");
            let (settings, error) = match Settings::read(&path) {
                Ok(settings) => (settings, None),
                Err(error) => (Settings::default(), Some(error)),
            };
            let shared: Shared = Arc::new(Mutex::new(Runtime {
                settings: settings.clone(),
                view: View::default(),
                worker: None,
                recording: None,
                session: 0,
                target: None,
                session_dir: None,
                preview_pending: false,
                worker_generation: 0,
            }));
            app.manage(shared);
            // Configured WebViews can execute IPC while they are being created.
            // Register the complete runtime first so their initial snapshot is valid.
            for config in &app.config().app.windows {
                tauri::WebviewWindowBuilder::from_config(app.handle(), config)?.build()?;
            }
            let handle = app.handle().clone();
            let shortcut_error = register_shortcut(&handle, &settings.shortcut).err();
            if let Some(error) = error.or(shortcut_error) {
                fail(&handle, error);
            } else {
                let handle = handle.clone();
                std::thread::spawn(move || {
                    if let Err(error) = launch_worker(&handle, false) {
                        fail(&handle, error);
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            microphones,
            recording,
            cancel,
            reload,
            save_settings,
            copy_text
        ])
        .build(tauri::generate_context!())
        .expect("Could not initialize FlowForge AI")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let shared = app.state::<Shared>();
                let mut state = shared.lock().unwrap();
                if let Some(stop) = state.recording.take() {
                    let _ = stop.send(true);
                }
                if let Some(mut worker) = state.worker.take() {
                    worker.stop();
                }
            }
        });
}
