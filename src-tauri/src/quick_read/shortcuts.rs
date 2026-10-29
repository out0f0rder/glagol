//! Shared global-shortcut plugin with explicit action dispatch.
use super::{message, QuickRead};
use crate::{commands::dictation::startup_hotkey, db::repository, state::AppState};
use std::collections::BTreeMap;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
pub const DEFAULTS: [(&str, &str); 6] = [
    ("selection", "Ctrl+Alt+R"),
    ("clipboard", "Ctrl+Alt+V"),
    ("pointer", "Ctrl+Alt+E"),
    ("ocr", "Ctrl+Alt+O"),
    ("pause", "Ctrl+Alt+P"),
    ("stop", "Ctrl+Alt+S"),
];
#[derive(Default)]
pub struct Bindings {
    pub registered: BTreeMap<String, Shortcut>,
    pub errors: Vec<String>,
}
/// Register independently so a single OS conflict does not disable other actions.
pub fn setup(app: &tauri::AppHandle) -> Result<(), String> {
    let q = app.state::<QuickRead>();
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let dictation = startup_hotkey(&db);
    let mut bindings = q.shortcuts.lock().unwrap();
    for (action, default) in DEFAULTS {
        let raw = repository::get_setting(&db, &format!("quick_read_{action}"))
            .map_err(|e| e.to_string())?
            .unwrap_or(default.into());
        let shortcut = validate(&raw)?;
        if shortcut == dictation
            || bindings.registered.values().any(|s| *s == shortcut)
            || app.global_shortcut().register(shortcut).is_err()
        {
            bindings.errors.push(format!("{action}: {raw}"));
        } else {
            bindings.registered.insert(action.into(), shortcut);
        }
    }
    let speed = repository::get_setting(&db, "quick_read_speed")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|n| n.is_finite() && (0.5..=2.0).contains(n))
        .unwrap_or(1.0);
    q.buffer.lock().unwrap().snapshot.speed = speed;
    Ok(())
}
/// Reject bare keys; global bindings must have a modifier.
pub fn validate(raw: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = raw
        .parse()
        .map_err(|_| message("Invalid shortcut.", "Некорректное сочетание клавиш."))?;
    if shortcut.mods.is_empty() {
        return Err(message(
            "A global shortcut needs a modifier.",
            "Глобальному сочетанию нужен модификатор.",
        ));
    }
    Ok(shortcut)
}
/// Returns true only for Quick Read shortcuts, including release events.
pub fn dispatch(app: &tauri::AppHandle, shortcut: &Shortcut, state: ShortcutState) -> bool {
    let q = app.state::<QuickRead>();
    let action = q
        .shortcuts
        .lock()
        .unwrap()
        .registered
        .iter()
        .find(|(_, s)| *s == shortcut)
        .map(|(a, _)| a.clone());
    if let Some(action) = action {
        if state == ShortcutState::Pressed {
            let _ = super::quick_read_action(app.clone(), action);
        }
        true
    } else {
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_distinct_and_bare_keys_rejected() {
        let shortcuts: Vec<_> = DEFAULTS.iter().map(|(_, s)| validate(s).unwrap()).collect();
        for (i, s) in shortcuts.iter().enumerate() {
            assert!(!shortcuts[..i].contains(s));
        }
        assert!(validate("R").is_err());
        assert!(validate("invalid+bad").is_err());
    }
}
