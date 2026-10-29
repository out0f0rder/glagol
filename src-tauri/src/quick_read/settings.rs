//! Persist only preferences; speech and screenshots never enter SQLite.
use super::{
    message,
    shortcuts::{self, DEFAULTS},
    QuickRead,
};
use crate::commands::dictation::{AppHotkeyRegistrar, HotkeyRegistrar};
use crate::{db::repository, state::AppState};
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::Manager;
use tauri_plugin_global_shortcut::Shortcut;
#[derive(Serialize)]
pub struct Settings {
    pub shortcuts: BTreeMap<String, String>,
    pub show_player: bool,
    pub phonetic_english: bool,
    pub ocr_language: String,
    pub registration_errors: Vec<String>,
}
#[tauri::command]
pub fn quick_read_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut keys = BTreeMap::new();
    for (action, default) in DEFAULTS {
        keys.insert(
            action.into(),
            repository::get_setting(&db, &format!("quick_read_{action}"))
                .map_err(|e| e.to_string())?
                .unwrap_or(default.into()),
        );
    }
    Ok(Settings {
        shortcuts: keys,
        show_player: repository::get_setting(&db, "quick_read_player")
            .map_err(|e| e.to_string())?
            .as_deref()
            == Some("1"),
        phonetic_english: repository::get_setting(&db, "quick_read_phonetic_english")
            .map_err(|e| e.to_string())?
            .as_deref()
            != Some("0"),
        ocr_language: repository::get_setting(&db, "quick_read_ocr_language")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| {
                crate::preferences::get(&db)
                    .map(|p| {
                        if p.tts_language == crate::preferences::Language::Ru {
                            "ru-RU"
                        } else {
                            "en-US"
                        }
                    })
                    .unwrap_or("ru-RU")
                    .into()
            }),
        registration_errors: app
            .state::<QuickRead>()
            .shortcuts
            .lock()
            .unwrap()
            .errors
            .clone(),
    })
}
#[tauri::command]
pub async fn quick_read_set_shortcut(
    app: tauri::AppHandle,
    action: String,
    shortcut: String,
) -> Result<(), String> {
    if !DEFAULTS.iter().any(|(a, _)| *a == action) {
        return Err("Unknown action".into());
    }
    let q = app.state::<QuickRead>();
    let _gate = q.hotkey_gate.lock().await;
    let new = shortcuts::validate(&shortcut)?;
    let old = {
        let bindings = q.shortcuts.lock().unwrap();
        if bindings
            .registered
            .iter()
            .any(|(a, s)| *a != action && *s == new)
        {
            return Err(message(
                "This shortcut is already used by Quick Read.",
                "Это сочетание уже используется быстрым чтением.",
            ));
        }
        bindings.registered.get(&action).copied()
    };
    let state = app.state::<AppState>();
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        if crate::commands::dictation::startup_hotkey(&db) == new {
            return Err(message(
                "This shortcut is used by dictation.",
                "Это сочетание используется диктовкой.",
            ));
        }
    }
    if old == Some(new) {
        return Ok(());
    }
    let registrar = AppHotkeyRegistrar { app: &app };
    swap_registered(&registrar, old, new, || {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        repository::set_setting(
            &db,
            &format!("quick_read_{action}"),
            shortcut.trim(),
            chrono::Utc::now().timestamp_millis(),
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    })?;
    let mut bindings = q.shortcuts.lock().unwrap();
    bindings.registered.insert(action.clone(), new);
    bindings
        .errors
        .retain(|s| !s.starts_with(&format!("{action}:")));
    Ok(())
}
#[tauri::command]
pub fn quick_read_set_option(
    app: tauri::AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    let persist = match key.as_str() {
        "player" if value == "0" || value == "1" => "quick_read_player",
        "phonetic_english" if value == "0" || value == "1" => "quick_read_phonetic_english",
        "ocr_language" if value == "ru-RU" || value == "en-US" => "quick_read_ocr_language",
        _ => return Err("Invalid reading preference".into()),
    };
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    repository::set_setting(&db, persist, &value, chrono::Utc::now().timestamp_millis())
        .map_err(|e| e.to_string())?;
    if key == "player" {
        app.state::<QuickRead>()
            .show_player
            .store(value == "1", std::sync::atomic::Ordering::Relaxed);
    } else if key == "phonetic_english" {
        app.state::<QuickRead>()
            .phonetic_english
            .store(value == "1", std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

// Register first so an OS conflict leaves the previous binding untouched. Persist
// only after a successful OS swap, restoring the previous binding on DB failure.
fn swap_registered(
    registrar: &impl HotkeyRegistrar,
    old: Option<Shortcut>,
    new: Shortcut,
    save: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    registrar.register(&new).map_err(|_|message("Cannot register shortcut; it may belong to another app. Previous shortcut retained.","Сочетание не удалось зарегистрировать; возможно, оно занято. Прежнее сочетание сохранено."))?;
    if let Some(old) = old {
        if registrar.unregister(&old).is_err() {
            let _ = registrar.unregister(&new);
            return Err(message(
                "Cannot replace the shortcut. Previous shortcut retained.",
                "Не удалось заменить сочетание. Прежнее сочетание сохранено.",
            ));
        }
    }
    if let Err(e) = save() {
        let _ = registrar.unregister(&new);
        if let Some(old) = old {
            registrar.register(&old).map_err(|_| {
                message(
                    "Saving and shortcut recovery failed. Assign the shortcut again.",
                    "Не удалось сохранить и восстановить сочетание. Назначьте его повторно.",
                )
            })?;
        }
        return Err(e);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Fake {
        keys: RefCell<Vec<Shortcut>>,
        occupied: Option<Shortcut>,
    }
    impl HotkeyRegistrar for Fake {
        fn register(&self, key: &Shortcut) -> Result<(), String> {
            if self.occupied == Some(*key) {
                return Err("occupied".into());
            }
            self.keys.borrow_mut().push(*key);
            Ok(())
        }
        fn unregister(&self, key: &Shortcut) -> Result<(), String> {
            self.keys.borrow_mut().retain(|k| k != key);
            Ok(())
        }
    }
    #[test]
    fn conflict_and_persistence_failure_retain_previous_binding() {
        let old = shortcuts::validate("Ctrl+Alt+R").unwrap();
        let new = shortcuts::validate("Ctrl+Alt+Q").unwrap();
        let blocked = Fake {
            keys: RefCell::new(vec![old]),
            occupied: Some(new),
        };
        assert!(swap_registered(&blocked, Some(old), new, || panic!(
            "must not persist on conflict"
        ))
        .is_err());
        assert_eq!(*blocked.keys.borrow(), vec![old]);
        let rollback = Fake {
            keys: RefCell::new(vec![old]),
            occupied: None,
        };
        assert!(swap_registered(&rollback, Some(old), new, || Err("DB failure".into())).is_err());
        assert_eq!(*rollback.keys.borrow(), vec![old]);
        swap_registered(&rollback, Some(old), new, || Ok(())).unwrap();
        assert_eq!(*rollback.keys.borrow(), vec![new]);
    }
}
