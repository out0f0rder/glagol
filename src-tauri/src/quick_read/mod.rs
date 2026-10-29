//! Ephemeral speech: extraction, shared Silero inference and bounded PCM output.
pub mod player;
pub mod settings;
pub mod shortcuts;
pub mod sources;
pub mod surface;
pub mod text;
use crate::{db::repository, preferences::Language, state::AppState, tts::silero::models::Models};
use player::{Buffer, Snapshot};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager};
use tokio::sync::watch;

pub(crate) fn message(en: &str, ru: &str) -> String {
    crate::preferences::message(en, ru)
}
/// App-owned state; no document text is persisted or emitted.
pub struct QuickRead {
    pub buffer: Arc<Mutex<Buffer>>,
    changes: watch::Sender<u64>,
    serial: tokio::sync::Mutex<()>,
    content: Mutex<Arc<Vec<String>>>,
    exit: Arc<AtomicBool>,
    output: Mutex<Option<std::thread::JoinHandle<()>>>,
    pub shortcuts: Mutex<shortcuts::Bindings>,
    pub hotkey_gate: tokio::sync::Mutex<()>,
    pub show_player: AtomicBool,
    pub region: Mutex<Option<surface::Region>>,
    pub capture_generation: AtomicU64,
}
impl Default for QuickRead {
    fn default() -> Self {
        let (changes, _) = watch::channel(0);
        Self {
            buffer: Arc::new(Mutex::new(Buffer::default())),
            changes,
            serial: tokio::sync::Mutex::new(()),
            content: Mutex::new(Arc::new(Vec::new())),
            exit: Arc::new(AtomicBool::new(false)),
            output: Mutex::new(None),
            shortcuts: Mutex::new(Default::default()),
            hotkey_gate: tokio::sync::Mutex::new(()),
            show_player: AtomicBool::new(false),
            region: Mutex::new(None),
            capture_generation: AtomicU64::new(0),
        }
    }
}
impl QuickRead {
    fn replace(&self) -> u64 {
        let mut b = self.buffer.lock().unwrap();
        let id = b.snapshot.generation.wrapping_add(1);
        b.reset(id);
        self.changes.send_replace(id);
        id
    }
    fn current(&self, id: u64) -> bool {
        *self.changes.borrow() == id
    }
    pub fn stop(&self) {
        self.replace();
        *self.content.lock().unwrap() = Arc::new(Vec::new());
    }
    pub fn shutdown(&self) {
        self.stop();
        self.exit.store(true, Ordering::Relaxed);
        if let Some(t) = self.output.lock().unwrap().take() {
            let _ = t.join();
        }
    }
}
/// Initialize output owner and hotkeys; works with the main window hidden.
pub fn setup(app: &tauri::AppHandle) -> Result<(), String> {
    let q = app.state::<QuickRead>();
    let buffer = q.buffer.clone();
    let stop = q.exit.clone();
    *q.output.lock().unwrap() = Some(
        std::thread::Builder::new()
            .name("quick-read-audio".into())
            .spawn(move || player::run(buffer, stop))
            .map_err(|e| e.to_string())?,
    );
    shortcuts::setup(app)?;
    surface::setup(app)?;
    q.show_player.store(
        settings::quick_read_settings(app.clone())?.show_player,
        Ordering::Relaxed,
    );
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut error_since = None;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let q = handle.state::<QuickRead>();
            if q.exit.load(Ordering::Relaxed) {
                break;
            }
            let mut state = q.buffer.lock().unwrap().snapshot.clone();
            if state.error.is_some() {
                let (id, since) =
                    error_since.get_or_insert((state.generation, std::time::Instant::now()));
                if *id != state.generation {
                    error_since = Some((state.generation, std::time::Instant::now()));
                } else if since.elapsed() > std::time::Duration::from_secs(6) {
                    let mut buffer = q.buffer.lock().unwrap();
                    if buffer.snapshot.generation != state.generation {
                        continue;
                    }
                    buffer.snapshot.error = None;
                    state.error = None;
                }
            } else {
                error_since = None;
            }
            surface::update(&handle, &state);
            let _ = handle.emit("quick-read-state", state);
        }
    });
    Ok(())
}
fn fail(app: &tauri::AppHandle, id: u64, error: String) {
    let q = app.state::<QuickRead>();
    let mut b = q.buffer.lock().unwrap();
    if b.snapshot.generation == id {
        let _ = q
            .capture_generation
            .compare_exchange(id, 0, Ordering::Relaxed, Ordering::Relaxed);
        b.reset(id);
        b.snapshot.error = Some(crate::i18n::error(error));
        let _ = app.emit("quick-read-state", b.snapshot.clone());
    }
}
/// A new source request immediately supersedes extraction, synthesis and output.
pub fn request(app: &tauri::AppHandle, source: &str) {
    let q = app.state::<QuickRead>();
    let id = q.replace();
    *q.content.lock().unwrap() = Arc::new(Vec::new());
    let _ = surface::quick_read_ocr_cancel(app.clone());
    q.buffer.lock().unwrap().snapshot.phase = "extracting".into();
    let app = app.clone();
    let source = source.to_string();
    tauri::async_runtime::spawn(async move {
        let q = app.state::<QuickRead>();
        let mut changes = q.changes.subscribe();
        if !q.current(id) {
            return;
        }
        let extracted = tokio::select! { biased; _=changes.changed()=>return, result=sources::extract(&source)=>result };
        match extracted.and_then(|s| text::chunks(&s)) {
            Ok(chunks) => {
                let q = app.state::<QuickRead>();
                if !q.current(id) {
                    return;
                }
                let chunks = Arc::new(chunks);
                *q.content.lock().unwrap() = chunks.clone();
                read(app.clone(), id, chunks, 0).await;
            }
            Err(e) => fail(&app, id, e),
        }
    });
}
async fn read(app: tauri::AppHandle, id: u64, chunks: Arc<Vec<String>>, start: usize) {
    let q = app.state::<QuickRead>();
    let mut changes = q.changes.subscribe();
    let _serial = q.serial.lock().await;
    if !q.current(id) {
        return;
    }
    let result = async {
        let (provider, voice) = {
            let state = app.state::<AppState>();
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let prefs = crate::preferences::get(&db).map_err(|e| e.to_string())?;
            if prefs.onboarding_stage != "complete" {
                return Err(message(
                    "Complete speech setup in Settings.",
                    "Завершите настройку речи в настройках.",
                ));
            }
            if prefs.tts_language == Language::Ru {
                ("silero", prefs.tts_voice_ru)
            } else {
                ("silero-en", prefs.tts_voice_en)
            }
        };
        let models = app.state::<Models>();
        let tts = models.select(Some(provider))?;
        let _operation = tts.operation.try_lock().map_err(|_| {
            message(
                "Another speech operation is running.",
                "Другая операция озвучки ещё выполняется.",
            )
        })?;
        if !tts.model.voices.contains(&voice.as_str()) {
            return Err(message("Unknown voice.", "Неизвестный голос."));
        }
        tts.cancel.store(false, Ordering::Relaxed);
        {
            let mut buffer = q.buffer.lock().unwrap();
            if buffer.snapshot.generation != id {
                return Ok(());
            }
            buffer.snapshot.phase = "preparing".into();
            buffer.snapshot.total = chunks.len();
            buffer.snapshot.current = start;
        }
        let synthesis = async {
            tts.prepare(&app).await?;
            let mut audible = false;
            for (index, chunk) in chunks.iter().enumerate().skip(start) {
                loop {
                    {
                        let buffer = q.buffer.lock().unwrap();
                        if let Some(error) = &buffer.snapshot.error {
                            return Err(error.clone());
                        }
                        if buffer.room() {
                            break;
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                let samples = {
                    let mut worker = tts.worker.lock().await;
                    worker
                        .as_mut()
                        .ok_or("Движок не готов")?
                        .synthesize_pcm(chunk, &voice)
                        .await?
                };
                let mut buffer = q.buffer.lock().unwrap();
                if buffer.snapshot.generation != id {
                    return Ok(());
                }
                audible |= !samples.is_empty();
                buffer.push(index, samples)?;
            }
            if !audible {
                return Err("В тексте нет произносимых слов.".into());
            }
            {
                let mut buffer = q.buffer.lock().unwrap();
                if buffer.snapshot.generation != id {
                    return Ok(());
                }
                buffer.complete = true;
            }
            loop {
                {
                    let buffer = q.buffer.lock().unwrap();
                    if let Some(error) = &buffer.snapshot.error {
                        return Err(error.clone());
                    }
                    if buffer.snapshot.phase == "idle" {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            Ok::<_, String>(())
        };
        let result = tokio::select! {
            biased;
            _ = changes.changed() => {
                tts.close_worker().await;
                Ok(())
            },
            result = synthesis => result,
        };
        if result.is_err() {
            tts.close_worker().await;
            tts.invalidate_verification().await;
        }
        tts.finish(&app, &result);
        result
    }
    .await;
    if let Err(e) = result {
        fail(&app, id, e);
    }
}
#[tauri::command]
pub fn quick_read_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    let q = app.state::<QuickRead>();
    match action.as_str() {
        "selection" | "clipboard" | "pointer" => request(&app, &action),
        "stop" => {
            q.stop();
            surface::quick_read_ocr_cancel(app.clone())?;
        }
        "pause" => {
            let mut b = q.buffer.lock().unwrap();
            b.snapshot.paused = !b.snapshot.paused;
        }
        "previous" | "next" => {
            let chunks = q.content.lock().unwrap().clone();
            if chunks.is_empty() {
                return Ok(());
            }
            let current = q.buffer.lock().unwrap().snapshot.current;
            let index = if action == "previous" {
                current.saturating_sub(1)
            } else {
                (current + 1).min(chunks.len() - 1)
            };
            let id = q.replace();
            let app = app.clone();
            tauri::async_runtime::spawn(read(app, id, chunks, index));
        }
        "ocr" => {
            if let Err(e) = surface::start_ocr(&app) {
                let id = q.buffer.lock().unwrap().snapshot.generation;
                fail(&app, id, e);
            }
        }
        _ => {
            return Err(message(
                "Unknown reading action.",
                "Неизвестная команда чтения.",
            ))
        }
    }
    Ok(())
}
#[tauri::command]
pub fn quick_read_state(state: tauri::State<'_, QuickRead>) -> Result<Snapshot, String> {
    Ok(state
        .buffer
        .lock()
        .map_err(|e| e.to_string())?
        .snapshot
        .clone())
}
#[tauri::command]
pub fn quick_read_speed(app: tauri::AppHandle, speed: f64) -> Result<(), String> {
    if !speed.is_finite() || !(0.5..=2.0).contains(&speed) {
        return Err("Speed must be 0.5–2.0".into());
    }
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    repository::set_setting(
        &db,
        "quick_read_speed",
        &speed.to_string(),
        chrono::Utc::now().timestamp_millis(),
    )
    .map_err(|e| e.to_string())?;
    app.state::<QuickRead>()
        .buffer
        .lock()
        .unwrap()
        .snapshot
        .speed = speed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn replacement_wakes_cancellation_and_stop_clears_audio_and_pause() {
        let q = QuickRead::default();
        let first = q.replace();
        let mut old = q.changes.subscribe();
        {
            let mut b = q.buffer.lock().unwrap();
            b.snapshot.phase = "preparing".into();
            b.snapshot.paused = true;
            b.push(0, vec![123; 480]).unwrap();
        }
        let next = q.replace();
        tokio::time::timeout(std::time::Duration::from_millis(50), old.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(!q.current(first));
        assert!(q.current(next));
        {
            let b = q.buffer.lock().unwrap();
            assert!(!b.snapshot.paused);
            assert_eq!(b.snapshot.phase, "idle");
            assert!(b.room());
        }
        q.stop();
        assert!(!q.current(next));
        assert!(q.content.lock().unwrap().is_empty());
    }
}
