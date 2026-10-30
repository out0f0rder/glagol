//! Read-only Scintilla fallback. Only scalar SCI_* queries cross the process
//! boundary; text uses system-marshalled WM_GETTEXT, never a remote pointer.
use windows::Win32::{
    Foundation::{
        GetLastError, SetLastError, ERROR_ACCESS_DENIED, ERROR_SUCCESS, HWND, LPARAM, WPARAM,
    },
    UI::WindowsAndMessaging::*,
};
const PREFIX_BYTES: usize = 2_000_000;
const PREFIX_UNITS: usize = 500_000;
const GET_LENGTH: u32 = 2006;
const GET_CODE_PAGE: u32 = 2137;
const GET_START: u32 = 2143;
const GET_END: u32 = 2145;
const GET_MODE: u32 = 2423;
const GET_SELECTIONS: u32 = 2570;
const COUNT_CODE_UNITS: u32 = 2715;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Snapshot {
    start: usize,
    end: usize,
    length: usize,
    count: usize,
    mode: usize,
    code_page: usize,
}
impl Snapshot {
    fn validate(self) -> Result<(), String> {
        if self.count != 1 || self.mode != 0 {
            return Err("scintilla_selection".into());
        }
        if self.start >= self.end {
            return Err("empty".into());
        }
        if self.end > self.length {
            return Err("changed".into());
        }
        if self.end > PREFIX_BYTES {
            return Err("scintilla_limit".into());
        }
        // SBCS and UTF-8 CountCodeUnits match Scintilla's UTF-16 WM_GETTEXT path.
        if ![0, 65001].contains(&self.code_page) {
            return Err("scintilla_encoding".into());
        }
        Ok(())
    }
}
fn decode(prefix: &[u16], start: usize, end: usize) -> Result<String, String> {
    if start >= end || end > PREFIX_UNITS || prefix.len() != end + 1 || prefix[end] != 0 {
        return Err("changed".into());
    }
    let range = prefix.get(start..end).ok_or("changed")?;
    let text = String::from_utf16(range).map_err(|_| "unavailable")?;
    if text.trim().is_empty() {
        Err("empty".into())
    } else {
        Ok(text)
    }
}
fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> Result<usize, String> {
    let mut result = 0;
    // SAFETY: one validated HWND; SCI_* parameters here are integers only.
    // WM_GETTEXT is the sole pointer-bearing call, marshalled by User32 (<WM_USER).
    // Caller keeps its owned bounded UTF-16 buffer alive until this call returns.
    unsafe {
        SetLastError(ERROR_SUCCESS);
        if SendMessageTimeoutW(
            hwnd,
            message,
            WPARAM(wparam),
            LPARAM(lparam),
            SMTO_ABORTIFHUNG | SMTO_BLOCK | SMTO_ERRORONEXIT,
            150,
            Some(&mut result),
        )
        .0 == 0
        {
            return Err(if GetLastError() == ERROR_ACCESS_DENIED {
                "protected"
            } else {
                "unavailable"
            }
            .into());
        }
    }
    Ok(result)
}
fn snapshot(hwnd: HWND) -> Result<Snapshot, String> {
    Ok(Snapshot {
        start: send(hwnd, GET_START, 0, 0)?,
        end: send(hwnd, GET_END, 0, 0)?,
        length: send(hwnd, GET_LENGTH, 0, 0)?,
        count: send(hwnd, GET_SELECTIONS, 0, 0)?,
        mode: send(hwnd, GET_MODE, 0, 0)?,
        code_page: send(hwnd, GET_CODE_PAGE, 0, 0)?,
    })
}
fn is_scintilla(hwnd: HWND) -> bool {
    let mut class = [0u16; 32];
    // SAFETY: valid borrowed window handle; API writes at most this fixed buffer.
    let len = unsafe { GetClassNameW(hwnd, &mut class) };
    len > 0
        && String::from_utf16_lossy(&class[..len as usize]) == "Scintilla"
        && unsafe { IsWindowUnicode(hwnd).as_bool() }
}
fn read_control(hwnd: HWND) -> Result<String, String> {
    if !is_scintilla(hwnd) {
        return Err("unavailable".into());
    }
    let before = snapshot(hwnd)?;
    before.validate()?;
    // Translate byte offsets to UTF-16 in the control, without copying raw bytes
    // or passing a private SCI_* text pointer into another address space.
    let start = send(hwnd, COUNT_CODE_UNITS, 0, before.start as isize)?;
    let end = send(hwnd, COUNT_CODE_UNITS, 0, before.end as isize)?;
    if start >= end {
        return Err("unavailable".into());
    }
    if end > PREFIX_UNITS {
        return Err("scintilla_limit".into());
    }
    let mut prefix = vec![0u16; end + 1];
    let copied = send(hwnd, WM_GETTEXT, prefix.len(), prefix.as_mut_ptr() as isize)?;
    if copied != end || snapshot(hwnd)? != before {
        return Err("changed".into());
    }
    // The unselected bounded prefix is discarded, never spoken, persisted or logged.
    decode(&prefix, start, end)
}
/// Caller has already checked UIA password ancestry. Require the exact native
/// focused Scintilla HWND; do not inspect another tab/control or move focus.
pub(super) fn selected(hwnd: HWND) -> Result<String, String> {
    if !is_scintilla(hwnd) {
        return Err("unavailable".into());
    }
    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: initialized fixed-size structure; zero requests foreground GUI thread.
    unsafe { GetGUIThreadInfo(0, &mut info) }.map_err(|_| "unavailable")?;
    if hwnd.is_invalid() || hwnd != info.hwndFocus {
        return Err("changed".into());
    }
    let text = read_control(hwnd)?;
    // SAFETY: same read-only focus check after extraction to reject common races.
    unsafe { GetGUIThreadInfo(0, &mut info) }.map_err(|_| "unavailable")?;
    if hwnd != info.hwndFocus {
        return Err("changed".into());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Snapshot {
        Snapshot {
            start: 1,
            end: 9,
            length: 30,
            count: 1,
            mode: 0,
            code_page: 65001,
        }
    }
    #[test]
    fn rejects_empty_multiple_rectangular_and_unbounded_ranges() {
        for s in [
            Snapshot {
                start: 9,
                ..sample()
            },
            Snapshot {
                count: 2,
                ..sample()
            },
            Snapshot {
                mode: 1,
                ..sample()
            },
            Snapshot {
                end: PREFIX_BYTES + 1,
                length: PREFIX_BYTES + 2,
                ..sample()
            },
            Snapshot {
                end: 31,
                ..sample()
            },
            Snapshot {
                code_page: 932,
                ..sample()
            },
        ] {
            assert!(s.validate().is_err());
        }
        assert!(sample().validate().is_ok());
        assert!(Snapshot {
            code_page: 0,
            ..sample()
        }
        .validate()
        .is_ok());
    }
    #[test]
    fn decodes_only_selected_utf16_with_cyrillic_emoji_and_crlf() {
        let prefix = "Не читать: Русский 😀 English\r\n";
        let mut units = prefix.encode_utf16().collect::<Vec<_>>();
        let start = "Не читать: ".encode_utf16().count();
        let end = units.len();
        units.push(0);
        assert_eq!(
            decode(&units, start, end).unwrap(),
            "Русский 😀 English\r\n"
        );
    }
    #[test]
    fn rejects_truncation_invalid_surrogates_and_blank_text() {
        assert!(decode(&[65, 66, 0], 0, 3).is_err());
        assert!(decode(&[65, 66, 67], 0, 2).is_err());
        assert!(decode(&[0xD800, 0], 0, 1).is_err());
        assert!(decode(&[32, 13, 10, 0], 0, 3).is_err());
        assert!(decode(&[65, 0], 2, 1).is_err());
    }
}
