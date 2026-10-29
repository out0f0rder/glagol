//! Killable extraction process: an unresponsive external COM/clipboard owner cannot
//! strand a thread in the app. No synthetic keys, clipboard writes or focus changes.
#[cfg(windows)]
mod ocr;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use std::process::Stdio;
use std::time::Duration;
#[cfg(windows)]
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};
#[cfg(windows)]
const OUTPUT_LIMIT: u64 = 3_000_000;

pub async fn extract(source: &str) -> Result<String, String> {
    if !matches!(source, "clipboard" | "selection" | "pointer") {
        return Err("Unknown source".into());
    }
    run(source, &[], Duration::from_secs(3)).await
}
/// Private stdout only; source arguments contain geometry/language, never speech text.
pub async fn run(source: &str, args: &[String], timeout: Duration) -> Result<String, String> {
    #[cfg(not(windows))]
    {
        let _ = (source, args, timeout);
        Err(super::message(
            "Quick Read extraction requires Windows.",
            "Извлечение текста Quick Read требует Windows.",
        ))
    }
    #[cfg(windows)]
    {
        let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command
            .args([
                "--glagol-text-source",
                source,
                &std::process::id().to_string(),
            ])
            .args(args)
            .creation_flags(0x08000000);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(|_| {
            super::message(
                "Cannot start text extraction.",
                "Не удалось запустить извлечение текста.",
            )
        })?;
        let result = bounded_output(&mut child, timeout).await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        let bytes = result?;
        let result: Result<String, String> = serde_json::from_slice(&bytes)
            .map_err(|_| super::message("Text extraction failed.", "Не удалось извлечь текст."))?;
        result.map_err(|code| match code.as_str() {
            "ocr_language"=>super::message("The selected OCR language is not installed. Install its Windows OCR language component or select another language in Quick Read settings.","Выбранный язык OCR не установлен. Установите языковой компонент OCR Windows либо выберите другой язык в настройках быстрого чтения."),
            "ocr_size"=>super::message("The OCR region is too large. Select a smaller region.","Область OCR слишком большая. Выберите меньшую область."),
            "ocr_unavailable"|"capture"=>super::message("Windows OCR or screen capture is unavailable.","Windows OCR или захват экрана недоступен."),
            "protected" => super::message(
                "Protected controls cannot be read.",
                "Защищённые поля не читаются.",
            ),
            "limit" => super::message(
                "Text is too long (500,000 characters maximum).",
                "Текст слишком длинный (максимум 500 000 символов).",
            ),
            _ => super::message(
                "No readable text. Copy text and use Read clipboard, or try OCR.",
                "Текст не найден. Скопируйте текст и прочитайте буфер либо используйте OCR.",
            ),
        })
    }
}
#[cfg(windows)]
async fn bounded_output(child: &mut Child, timeout: Duration) -> Result<Vec<u8>, String> {
    let output = child.stdout.take().ok_or("Missing extraction output")?;
    let mut bytes = Vec::new();
    let result = tokio::time::timeout(
        timeout,
        output.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes),
    )
    .await;
    match result {
        Ok(Ok(_)) if bytes.len() as u64 <= OUTPUT_LIMIT => Ok(bytes),
        _ => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(super::message(
                "Text extraction timed out or exceeded its limit.",
                "Извлечение текста превысило время ожидания или лимит.",
            ))
        }
    }
}
/// Hidden same-EXE mode, entered before Tauri/single-instance/logging initialization.
pub fn child_entry() -> Option<i32> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--glagol-text-source") {
        return None;
    }
    #[cfg(windows)]
    {
        if args.len() < 4 {
            return Some(2);
        }
        let Ok(parent) = args[3].parse::<u32>() else {
            return Some(2);
        };
        if !windows::watch_parent(parent) {
            return Some(2);
        }
        let result = if args[2] == "ocr" {
            ocr::recognize(&args[4..])
        } else {
            windows::extract(&args[2])
        };
        use std::io::Write;
        let bytes = serde_json::to_vec(&result).ok()?;
        let _ = std::io::stdout().write_all(&bytes);
        Some(0)
    }
    #[cfg(not(windows))]
    {
        Some(2)
    }
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[tokio::test]
    async fn hung_provider_process_is_killed() {
        let mut child = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        assert!(bounded_output(&mut child, Duration::from_millis(50))
            .await
            .is_err());
        assert!(child.try_wait().unwrap().is_some());
    }
}
