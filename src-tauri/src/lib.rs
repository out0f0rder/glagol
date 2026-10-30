use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

// Optional local TTS; separate from dictation.
pub mod quick_read;
pub mod tts;

// Speech-to-text (STT) client for the Dictation feature (Sprint 6).
pub mod stt;

// Text processing utilities (chunking, future: preprocessing).
pub mod text;

// Audio utilities (WAV chunk concatenation).
pub mod audio;

// Secrets storage via OS-native credential manager.
pub mod secrets;

// Local SQLite database (connection management, migrations, repository).
pub mod db;

// Filesystem path resolution helpers (audio cache root, database path).
pub mod paths;

// File parsers for the Synthesize page file picker (TXT/MD/DOCX/PDF).
pub mod parser;

// Library backup/restore via zip archive (Sprint 5c).
pub mod backup;

// Structured-logging subscriber installation (Sprint 6 PR3.1).
pub mod logging;

// Shared Tauri application state.
pub mod i18n;

pub fn moonshine_worker_entry() -> Option<i32> {
    stt::moonshine::child_entry()
}
pub mod preferences;
pub mod startup;
pub mod state;

// Dictation microphone recorder (Sprint 6 PR2): capture → 16 kHz mono S16LE.
pub mod dictation;

// Tauri commands exposed to the frontend.
pub mod commands;

/// Broadcast event name for microphone RMS levels (D11). Kept in lock-step with
/// `DICTATION_LEVEL_EVENT` in `src/lib/tauri.ts`. Emitted at ~20 Hz while
/// recording; the overlay (PR3) subscribes. kebab-case per project convention
/// (`synthesis-completed`, `backup-progress`).
const DICTATION_LEVEL_EVENT: &str = "dictation-level";

/// Payload of the [`DICTATION_LEVEL_EVENT`] broadcast: one linear RMS value in
/// `0.0..=1.0`. Scaling/smoothing is the overlay's job (D6/D11).
#[derive(Clone, serde::Serialize)]
struct LevelPayload {
    level: f32,
}

/// Emit RMS levels as a Tauri broadcast. `AppHandle` is `Clone + Send + Sync +
/// 'static`, so it satisfies [`dictation::LevelSink`] directly and can be moved
/// onto the recorder thread. We use `app.emit()` (broadcast), not
/// `ipc::Channel`, because the level consumer is a separate overlay window that
/// never `invoke`s — there is no call to bind a channel to (see the PR body).
impl dictation::LevelSink for tauri::AppHandle {
    fn level(&self, rms: f32) {
        let _ = self.emit(DICTATION_LEVEL_EVENT, LevelPayload { level: rms });
    }
}

/// Emit dictation state-machine transitions (D7) as a `dictation-state`
/// broadcast. `AppHandle` is the production [`dictation::pipeline::DictationEmitter`];
/// the overlay window `listen()`s and drives the pill. Broadcast (not
/// `ipc::Channel`) for the same reason as the level meter: the consumer is a
/// separate window that never `invoke`s, so there is no channel to bind.
impl dictation::pipeline::DictationEmitter for tauri::AppHandle {
    fn emit_state(&self, state: dictation::pipeline::DictationState) {
        let state = match state {
            dictation::pipeline::DictationState::Error { message } => {
                dictation::pipeline::DictationState::Error {
                    message: i18n::error(message),
                }
            }
            other => other,
        };
        // The tray and overlay share the same readiness signal. Opening a
        // stream alone is not evidence that the microphone is capturing audio.
        dictation::session::set_tray_recording(
            self,
            matches!(state, dictation::pipeline::DictationState::Recording),
        );
        let _ = self.emit(dictation::pipeline::DICTATION_STATE_EVENT, state);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let startup_metrics = commands::metrics::StartupMetrics::new();
    let launch_mode = startup::LaunchMode::from_args(std::env::args_os());
    tauri::Builder::default()
        .manage(startup::PendingShow::default())
        // Single-instance lock (v0.2.1). MUST be registered FIRST so it runs
        // before `global-shortcut` (and every other plugin) can act: a second
        // launch has to be intercepted *before* it tries to grab the process-
        // global hotkey, or it steals the combo from the copy the user is
        // already using. The callback runs inside the ORIGINAL, already-running
        // process (the second one exits). A normal repeat opens the window;
        // an explicit tray/login repeat leaves visibility and focus untouched.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if startup::LaunchMode::from_args(argv.into_iter().map(Into::into)).show_existing() {
                if app.get_webview_window("main").is_some() {
                    dictation::session::show_main_window(app);
                } else {
                    app.state::<startup::PendingShow>().request();
                }
            }
        }))
        .plugin(tauri_plugin_opener::init())
        // In-app updates (0.6.0): used from Rust commands only, no webview permissions.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::updates::PendingUpdate::default())
        .plugin(tauri_plugin_dialog::init())
        // Global push-to-talk hotkey (Sprint 6 PR3). One handler dispatches the
        // single registered shortcut's Pressed/Released to the dictation session
        // (D3/D4). Registered from Rust in `setup`; the webview binds nothing.
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if quick_read::shortcuts::dispatch(app, shortcut, event.state) {
                        return;
                    }
                    let state = app.state::<state::AppState>();
                    let expected = commands::dictation::startup_hotkey(&state.db.lock().unwrap());
                    if *shortcut == expected {
                        dictation::session::handle_shortcut(app, event.state);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            // Install the tracing subscriber first so every `tracing::*!` call
            // from here on is actually recorded (Sprint 6 PR3.1). In release the
            // returned WorkerGuard must outlive the process or the rolling-file
            // writer stops flushing (D-L4) — it is parked in AppState below.
            let log_guard = crate::logging::init_tracing(app.handle());
            tracing::info!("application setup started");
            app.manage(startup_metrics);

            // Resolve the database path and eagerly initialise the connection.
            // Failure here is fatal: silently continuing with a broken DB would
            // corrupt every subsequent write.
            let db_path = crate::paths::database_path(app.handle())?;
            let existing_database = db_path.exists();
            let conn = crate::db::init_database(&db_path)?;
            preferences::initialize(&conn, existing_database)?;
            preferences::activate(preferences::get(&conn)?.ui_language);
            tracing::info!("startup database ready");

            // Ensure the audio cache directory exists. Synthesis writes
            // straight into it without a per-call mkdir, so the directory
            // must be present before the first synthesis command lands.
            let audio_root = crate::paths::audio_cache_root(app.handle())?;
            std::fs::create_dir_all(&audio_root)
                .map_err(|e| format!("Failed to create audio cache directory: {e}"))?;

            // Spawn the dedicated recorder thread (Sprint 6 PR2). `CpalSource`
            // is built on the recorder thread itself (the factory closure) — its
            // `cpal::Stream` is not `Send` on WASAPI and must never cross a
            // thread boundary. The `AppHandle` is the level sink. No stream is
            // opened until a `Start` arrives, so this is idle (≈0% CPU) at rest.
            let recorder = crate::dictation::recorder::spawn_recorder(
                crate::dictation::recorder::CpalSource::new,
                app.handle().clone(),
            );

            app.manage(state::AppState::new(conn, recorder));
            app.manage(quick_read::QuickRead::default());
            app.manage(std::sync::Arc::new(stt::local::LocalModels::new(
                paths::local_models_root(app.handle())?,
            )));

            let silero = std::sync::Arc::new(tts::silero::Silero::new(paths::tts_models_root(
                app.handle(),
            )?));
            app.manage(tts::silero::models::Models::new(silero.clone()));
            app.manage(silero);
            tts::silero::Silero::verification_task(app.handle().clone());
            tts::silero::Silero::idle_task(app.handle().clone());
            app.state::<std::sync::Arc<stt::local::LocalModels>>()
                .moonshine
                .idle_task();
            let cleanup_marker = db_path.with_file_name("legacy-tts-key-cleaned");
            tauri::async_runtime::spawn_blocking(move || {
                if !cleanup_marker.exists() {
                    if secrets::keyring::cleanup_legacy_tts_key().is_err() {
                        tracing::warn!(
                            "legacy TTS credential cleanup failed; retry on next startup"
                        );
                    } else if std::fs::write(cleanup_marker, b"1").is_err() {
                        tracing::warn!("legacy TTS cleanup marker could not be saved");
                    }
                }
            });

            // Park the log-flush guard for the process lifetime (D-L4).
            app.state::<state::AppState>().set_log_guard(log_guard);
            tracing::info!("startup backend ready");

            #[cfg(windows)]
            {
                let handle = app.handle().clone();
                std::thread::Builder::new()
                    .name("desktop-startup".into())
                    .spawn(move || {
                        if let Err(error) = finish_desktop_setup(&handle, launch_mode) {
                            tracing::error!(%error, "desktop initialization failed");
                            handle.exit(1);
                        }
                    })?;
            }
            #[cfg(not(windows))]
            finish_desktop_setup(app.handle(), launch_mode)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup::get_startup_settings,
            startup::set_launch_at_login,
            startup::set_start_in_tray,
            quick_read::quick_read_action,
            quick_read::quick_read_state,
            quick_read::quick_read_speed,
            quick_read::settings::quick_read_settings,
            quick_read::settings::quick_read_set_shortcut,
            quick_read::settings::quick_read_set_option,
            quick_read::surface::quick_read_region,
            quick_read::surface::quick_read_ocr_cancel,
            quick_read::surface::quick_read_ocr_finish,
            commands::updates::get_update_settings,
            commands::updates::set_update_auto_check,
            commands::updates::mark_update_reminder_shown,
            commands::updates::check_for_update,
            commands::updates::install_update,
            commands::preferences::get_preferences,
            commands::preferences::set_ui_language,
            commands::preferences::migrate_legacy_voice,
            commands::preferences::complete_onboarding,
            commands::preferences::set_tts_preference,
            commands::preferences::set_stt_preference,
            commands::preferences::is_dictating,
            commands::metrics::report_ui_ready,
            commands::synthesize::synthesize_document,
            commands::tts::tts_status,
            commands::tts::prepare_tts,
            commands::tts::install_tts,
            commands::tts::cancel_tts,
            commands::tts::remove_tts,
            commands::tts::preview_tts,
            commands::storage::get_audio_path,
            commands::storage::export_audio,
            commands::storage::list_documents,
            commands::storage::delete_document,
            commands::storage::update_document_title,
            commands::file::read_and_parse_file,
            commands::backup::create_backup,
            commands::backup::validate_backup,
            commands::backup::restore_backup,
            commands::backup::relaunch_app,
            commands::dictation::get_stt_settings,
            commands::speech::get_speech_settings,
            commands::speech::save_speech_settings,
            commands::speech::test_speech_settings,
            commands::speech::delete_speech_key,
            stt::local::local_models_status,
            stt::local::download_local_model,
            stt::local::cancel_model_download,
            stt::local::remove_local_model,
            commands::dictation::save_stt_settings,
            commands::dictation::set_stt_key,
            commands::dictation::delete_stt_key,
            commands::dictation::has_stt_key,
            commands::dictation::test_stt_key,
            commands::dictation::list_audio_input_devices,
            commands::dictation::get_dictation_settings,
            commands::dictation::set_dictation_setting,
            commands::dictation::set_dictation_hotkey,
            commands::dictation::list_dictations,
            commands::dictation::clear_dictation_history,
            commands::dictation::get_recognitions_minutes,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Drain and stop the recorder thread on exit so it is not left as a
            // detached zombie (D2/lib.rs). Best-effort — the process is leaving.
            if let tauri::RunEvent::Exit = event {
                app_handle.state::<quick_read::QuickRead>().shutdown();
                app_handle.state::<state::AppState>().recorder.shutdown();
                let models = app_handle.state::<tts::silero::models::Models>();
                models.ru.stop();
                models.en.stop();
            }
        });
}

// Windows WebView2 creation must run while the UI event loop can dispatch COM
// callbacks. A hidden first webview must not hold the synchronous setup hook.
fn finish_desktop_setup(
    app: &tauri::AppHandle,
    launch_mode: startup::LaunchMode,
) -> Result<(), Box<dyn std::error::Error>> {
    // Construct the ordinary main webview first, while the UI loop is running.
    // It remains hidden/unfocused until native services and surfaces are ready.
    let saved_tray = {
        let state = app.state::<state::AppState>();
        let db = state
            .db
            .lock()
            .map_err(|_| "startup preference lock failed")?;
        startup::start_in_tray(&db)?
    };
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .ok_or("main window configuration missing")?;
    let main = tauri::WebviewWindowBuilder::from_config(app, config)?
        .visible(false)
        .focused(false)
        .build()?;
    let handle = app.clone();
    main.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            dictation::session::hide_main_to_tray(&handle);
        }
    });
    tracing::info!("startup main created");
    tracing::info!("startup surfaces begin");
    // ── Dictation surface (Sprint 6 PR3) ──────────────────────────
    //
    // Create the overlay window once, hidden (D5), and rebuild it after
    // the system resumes from sleep (WebView2 can stop rendering it).
    dictation::session::build_overlay(app, dictation::session::OVERLAY_LABEL)?;
    tracing::info!("startup dictation surface ready");
    dictation::session::spawn_resume_watch(app.clone());

    // System tray with the idle icon + «Показать / Выход» menu (D11).
    dictation::session::build_tray(app)?;
    tracing::info!("startup tray ready");

    // Register the global push-to-talk hotkey (D3/D7). Prefer the user's
    // saved hotkey (falls back to the default if unset or unparseable) so
    // a hotkey changed via Settings persists across restarts. A failure
    // here (some other app already owns the combo) is logged, not fatal —
    // the rest of Glagol (TTS) must still run.
    let hotkey = {
        let state = app.state::<state::AppState>();
        let conn = state
            .db
            .lock()
            .expect("db mutex poisoned during startup hotkey read");
        commands::dictation::startup_hotkey(&conn)
    };
    if let Err(e) = app.global_shortcut().register(hotkey) {
        tracing::warn!("failed to register dictation hotkey {hotkey:?}: {e}");
    }

    tracing::info!("startup dictation shortcut ready");
    quick_read::setup(app)?;
    tracing::info!("startup quick-read surfaces ready");

    let hidden = launch_mode.hidden(saved_tray) && !app.state::<startup::PendingShow>().take();
    if !hidden {
        dictation::session::show_main_window(app);
    }
    tracing::info!(main_hidden = hidden, "startup visibility applied");
    Ok(())
}
