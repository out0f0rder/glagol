//! Launch policy and opt-in Windows logon registration. Never touches speech data.
#[cfg(windows)]
mod windows;

use crate::{db::repository, preferences::message, state::AppState};
use rusqlite::Connection;
use serde::Serialize;
use std::ffi::OsString;
#[cfg(any(windows, test))]
use std::path::Path;
use tauri::Manager;

const KEY_START_IN_TRAY: &str = "start_in_tray";

/// Retain a normal repeat launch received before the main window exists.
#[derive(Default)]
pub struct PendingShow(std::sync::atomic::AtomicBool);
impl PendingShow {
    /// Defer an explicit request to open the main window until setup creates it.
    pub fn request(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }
    /// Consume a pending request after the main window has been constructed.
    pub fn take(&self) -> bool {
        self.0.swap(false, std::sync::atomic::Ordering::AcqRel)
    }
}

/// Explicit command-line visibility overrides the saved first-launch preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LaunchMode {
    #[default]
    Default,
    Tray,
    Show,
}

impl LaunchMode {
    /// Parse arguments after argv[0]; `--` ends options and `--show` wins conflicts.
    pub fn from_args(args: impl IntoIterator<Item = OsString>) -> Self {
        let mut mode = Self::Default;
        for arg in args.into_iter().skip(1).take_while(|a| a != "--") {
            if arg == "--show" {
                return Self::Show;
            }
            if arg == "--tray" || arg == "--hidden" {
                mode = Self::Tray;
            }
        }
        mode
    }

    /// Only the first process uses the preference. A normal repeat launch opens it.
    pub fn hidden(self, saved: bool) -> bool {
        match self {
            Self::Default => saved,
            Self::Tray => true,
            Self::Show => false,
        }
    }

    /// A login/hidden repeat must not activate or hide the existing main window.
    pub fn show_existing(self) -> bool {
        self != Self::Tray
    }
}

/// Read the default-off visibility preference without changing registration.
pub fn start_in_tray(conn: &Connection) -> rusqlite::Result<bool> {
    Ok(repository::get_setting(conn, KEY_START_IN_TRAY)?.as_deref() == Some("1"))
}

fn save_start_in_tray(conn: &Connection, enabled: bool) -> rusqlite::Result<()> {
    repository::set_setting(
        conn,
        KEY_START_IN_TRAY,
        if enabled { "1" } else { "0" },
        chrono::Utc::now().timestamp_millis(),
    )?;
    Ok(())
}

// Windows Run has a 260 UTF-16 character command-line limit. Always quote the EXE;
// only a constant argument follows it, with no shell or caller-provided arguments.
#[cfg(any(windows, test))]
fn autostart_command(executable: &Path) -> Result<String, String> {
    let Some(path) = executable.to_str() else {
        return Err(message(
            "Cannot use this application path for startup.",
            "Этот путь приложения нельзя использовать для автозапуска.",
        ));
    };
    let command = format!("\"{path}\" --tray");
    if path.is_empty()
        || path.contains(['"', '\0', '\r', '\n'])
        || command.encode_utf16().count() > 260
    {
        return Err(message(
            "The application path is invalid or too long for Windows startup. Move Glagol to a shorter path and try again.",
            "Путь приложения некорректен или слишком длинный для автозапуска Windows. Переместите Глагол в более короткий путь и повторите.",
        ));
    }
    Ok(command)
}

/// Settings describe the current executable's registration, not another copy's.
#[derive(Serialize)]
pub struct StartupSettings {
    pub start_in_tray: bool,
    pub autostart_enabled: bool,
    pub autostart_supported: bool,
    pub autostart_other_copy: bool,
    pub autostart_error: Option<String>,
}

/// Read preferences and the actual Run entry. Windows' own disable policy is respected.
#[tauri::command]
pub fn get_startup_settings(app: tauri::AppHandle) -> Result<StartupSettings, String> {
    let state = app.state::<AppState>();
    let saved = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        start_in_tray(&db).map_err(|e| e.to_string())?
    };
    #[cfg(windows)]
    let registration = || -> Result<(bool, bool), String> {
        let entry = windows::read()?;
        let Some(entry) = entry else {
            return Ok((false, false));
        };
        let expected = autostart_command(&std::env::current_exe().map_err(|_| registry_error())?)?;
        Ok((entry == expected, entry != expected))
    };
    #[cfg(windows)]
    let (enabled, other_copy, error) = match registration() {
        Ok((enabled, other_copy)) => (enabled, other_copy, None),
        Err(error) => (false, false, Some(error)),
    };
    #[cfg(not(windows))]
    let (enabled, other_copy, error) = (false, false, None);
    Ok(StartupSettings {
        start_in_tray: saved,
        autostart_enabled: enabled,
        autostart_supported: cfg!(windows),
        autostart_other_copy: other_copy,
        autostart_error: error,
    })
}

/// Apply only on an explicit settings action; startup itself never rewrites Run.
#[tauri::command]
pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let command = if enabled {
            Some(autostart_command(
                &std::env::current_exe().map_err(|_| registry_error())?,
            )?)
        } else {
            None
        };
        windows::write(command.as_deref())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err(message(
            "Login startup is available on Windows.",
            "Автозапуск доступен в Windows.",
        ))
    }
}

/// Persist visibility for the next first launch; the current window stays as is.
#[tauri::command]
pub fn set_start_in_tray(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    save_start_in_tray(&db, enabled).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn registry_error() -> String {
    message(
        "Cannot read or change Windows startup registration. Check your account permissions.",
        "Не удалось прочитать или изменить автозапуск Windows. Проверьте разрешения учётной записи.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(args: &[&str]) -> LaunchMode {
        LaunchMode::from_args(args.iter().map(OsString::from))
    }

    #[test]
    fn launch_policy_handles_preferences_explicit_flags_and_repeat_launches() {
        for (args, expected) in [
            (vec!["glagol.exe"], LaunchMode::Default),
            (vec!["glagol.exe", "--tray"], LaunchMode::Tray),
            (vec!["glagol.exe", "--hidden"], LaunchMode::Tray),
            (vec!["glagol.exe", "--tray", "--show"], LaunchMode::Show),
            (vec!["glagol.exe", "--show", "--tray"], LaunchMode::Show),
            (vec!["glagol.exe", "--", "--tray"], LaunchMode::Default),
            (vec!["--tray", "--tray-file.txt"], LaunchMode::Default),
        ] {
            assert_eq!(mode(&args), expected);
        }
        assert!(!LaunchMode::Default.hidden(false));
        assert!(LaunchMode::Default.hidden(true));
        assert!(LaunchMode::Tray.hidden(false));
        assert!(!LaunchMode::Show.hidden(true));
        assert!(LaunchMode::Default.show_existing());
        assert!(LaunchMode::Show.show_existing());
        assert!(!LaunchMode::Tray.show_existing());
    }

    #[test]
    fn run_command_quotes_unicode_spaces_and_rejects_ambiguous_or_long_paths() {
        assert_eq!(
            autostart_command(Path::new(r"C:\Программы\Glagol App\glagol.exe")).unwrap(),
            "\"C:\\Программы\\Glagol App\\glagol.exe\" --tray"
        );
        for path in [
            "".into(),
            "bad\"path.exe".into(),
            "bad\npath.exe".into(),
            "a".repeat(252),
            "🦀".repeat(127),
        ] {
            assert!(autostart_command(Path::new(&path)).is_err());
        }
        assert!(autostart_command(Path::new(&"a".repeat(251))).is_ok());
    }

    #[test]
    fn visibility_default_and_saved_choice_do_not_touch_other_settings() {
        let db = crate::db::test_connection();
        assert!(!start_in_tray(&db).unwrap());
        repository::set_setting(&db, "quick_read_player", "1", 0).unwrap();
        save_start_in_tray(&db, true).unwrap();
        assert!(start_in_tray(&db).unwrap());
        save_start_in_tray(&db, false).unwrap();
        assert!(!start_in_tray(&db).unwrap());
        assert_eq!(
            repository::get_setting(&db, "quick_read_player")
                .unwrap()
                .as_deref(),
            Some("1")
        );
    }
}
