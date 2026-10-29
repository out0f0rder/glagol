//! All COM interfaces stay on this child process's MTA thread and die before uninit.
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{HANDLE, HGLOBAL, POINT},
        System::{Com::*, Threading::*},
        UI::{Accessibility::*, WindowsAndMessaging::GetCursorPos},
    },
};
const LIMIT: i32 = 500_000;
struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: paired with successful initialization on this thread, after interfaces drop.
        unsafe { CoUninitialize() };
    }
}
/// Monitor parent lifetime without UIAccess, elevation or cross-process memory access.
pub fn watch_parent(pid: u32) -> bool {
    // SAFETY: only SYNCHRONIZE access; owned valid handle moved to one watcher.
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }) else {
        return false;
    };
    // SAFETY: transfer the newly opened unique handle into RAII; OwnedHandle is Send.
    let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    std::thread::spawn(move || {
        // SAFETY: handle remains owned by this watcher until closed; parent exit ends helper.
        unsafe {
            WaitForSingleObject(HANDLE(handle.as_raw_handle()), u32::MAX);
        }
        drop(handle);
        std::process::exit(0);
    });
    true
}
fn useful(s: String) -> Option<String> {
    let s = s.trim();
    if s.chars().any(char::is_alphabetic) || s.chars().any(char::is_numeric) {
        Some(s.into())
    } else {
        None
    }
}
fn text_pattern(element: &IUIAutomationElement) -> windows::core::Result<IUIAutomationTextPattern> {
    // SAFETY: element is a live COM interface on its creating thread; requested IDs match types.
    unsafe {
        element.GetCurrentPatternAs(UIA_TextPatternId).or_else(|_| {
            element
                .GetCurrentPatternAs::<IUIAutomationTextPattern2>(UIA_TextPattern2Id)?
                .cast()
        })
    }
}
fn protected(element: &IUIAutomationElement, walker: &IUIAutomationTreeWalker) -> bool {
    let mut node = element.clone();
    for _ in 0..8 {
        // SAFETY: live COM interfaces, no borrowed output pointers retained.
        unsafe {
            if node
                .CurrentIsPassword()
                .map(|v| v.as_bool())
                .unwrap_or(true)
            {
                return true;
            }
            let Ok(parent) = walker.GetParentElement(&node) else {
                break;
            };
            node = parent;
        }
    }
    false
}
/// Read only a selection, never DocumentRange on the focused document.
fn selected(element: &IUIAutomationElement) -> Result<String, String> {
    // SAFETY: COM calls on MTA, bounded range count and character limit; no mutation.
    unsafe {
        let pattern = text_pattern(element).map_err(|_| "unavailable")?;
        let ranges = pattern.GetSelection().map_err(|_| "unavailable")?;
        let count = ranges.Length().map_err(|_| "unavailable")?;
        if count > 32 {
            return Err("limit".into());
        }
        let mut parts = Vec::new();
        let mut remaining = LIMIT;
        for i in 0..count {
            let text = ranges
                .GetElement(i)
                .and_then(|r| r.GetText(remaining + 1))
                .map_err(|_| "unavailable")?
                .to_string();
            remaining -= text.chars().count() as i32;
            if remaining < 0 {
                return Err("limit".into());
            }
            if !text.trim().is_empty() {
                parts.push(text);
            }
        }
        if parts.is_empty() {
            Err("empty".into())
        } else {
            Ok(parts.join("\n\n"))
        }
    }
}
fn pointed(element: &IUIAutomationElement, point: POINT) -> Option<String> {
    // SAFETY: property and pattern getters only; text range is private and bounded.
    unsafe {
        if let Ok(pattern) = text_pattern(element) {
            if let Ok(range) = pattern.RangeFromPoint(point) {
                if range.ExpandToEnclosingUnit(TextUnit_Paragraph).is_ok() {
                    if let Ok(text) = range.GetText(16_000) {
                        if let Some(s) = useful(text.to_string()) {
                            return Some(s);
                        }
                    }
                }
            }
        }
        if let Ok(pattern) =
            element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
        {
            if let Ok(value) = pattern.CurrentValue() {
                if let Some(s) = useful(value.to_string()) {
                    return Some(s);
                }
            }
        }
        if let Ok(name) = element.CurrentName() {
            if let Some(s) = useful(name.to_string()) {
                return Some(s);
            }
        }
        if let Ok(pattern) = element.GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(
            UIA_LegacyIAccessiblePatternId,
        ) {
            if let Ok(value) = pattern.CurrentValue() {
                return useful(value.to_string());
            }
        }
    }
    None
}
pub fn extract(source: &str) -> Result<String, String> {
    if source == "clipboard" {
        return clipboard();
    }
    // SAFETY: fresh child main thread, initialized exactly once as MTA; RAII uninitializes.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|_| "COM")?;
    }
    let _com = Com;
    // SAFETY: standard in-proc Windows UIA class; all calls and returned COM objects stay here.
    unsafe {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).map_err(|_| "UIA")?;
        if let Ok(timeouts) = automation.cast::<IUIAutomation2>() {
            let _ = timeouts.SetConnectionTimeout(200);
            let _ = timeouts.SetTransactionTimeout(500);
        }
        let walker = automation.ControlViewWalker().map_err(|_| "UIA")?;
        let mut point = POINT::default();
        let element = if source == "selection" {
            automation.GetFocusedElement()
        } else {
            GetCursorPos(&mut point).map_err(|_| "cursor")?;
            automation.ElementFromPoint(point)
        }
        .map_err(|_| "element")?;
        if protected(&element, &walker) {
            return Err("protected".into());
        }
        if source == "selection" {
            return selected(&element);
        }
        if let Some(text) = pointed(&element, point) {
            return Ok(text);
        }
        // A bounded shallow child search, then two parents. Never scrape a whole tree.
        if let Ok(mut child) = walker.GetFirstChildElement(&element) {
            for _ in 0..8 {
                if !protected(&child, &walker) {
                    if let Some(text) = pointed(&child, point) {
                        return Ok(text);
                    }
                }
                let Ok(next) = walker.GetNextSiblingElement(&child) else {
                    break;
                };
                child = next;
            }
        }
        let mut parent = element;
        for _ in 0..2 {
            let Ok(next) = walker.GetParentElement(&parent) else {
                break;
            };
            parent = next;
            if protected(&parent, &walker) {
                return Err("protected".into());
            }
            // Do not fall back to an entire window/document from an empty leaf.
            let kind = parent.CurrentControlType().map_err(|_| "element")?;
            if [
                UIA_WindowControlTypeId,
                UIA_DocumentControlTypeId,
                UIA_PaneControlTypeId,
            ]
            .contains(&kind)
            {
                break;
            }
            if let Some(text) = pointed(&parent, point) {
                return Ok(text);
            }
        }
    }
    Err("empty".into())
}

fn clipboard() -> Result<String, String> {
    use windows::Win32::System::{DataExchange::*, Memory::*};
    struct Clipboard;
    impl Drop for Clipboard {
        fn drop(&mut self) {
            // SAFETY: this thread owns one OpenClipboard call.
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }
    struct Memory(HGLOBAL);
    impl Drop for Memory {
        fn drop(&mut self) {
            // SAFETY: one successful GlobalLock is balanced, handle belongs to the clipboard.
            unsafe {
                let _ = GlobalUnlock(self.0);
            }
        }
    }
    // SAFETY: read-only CF_UNICODETEXT while clipboard is locked; bound size before allocation,
    // copy only within GlobalSize and keep handle locked until the slice is no longer used.
    unsafe {
        OpenClipboard(None).map_err(|_| "empty")?;
        let _clipboard = Clipboard;
        let handle = HGLOBAL(GetClipboardData(13).map_err(|_| "empty")?.0);
        let bytes = GlobalSize(handle);
        if bytes > 2_000_004 {
            return Err("limit".into());
        }
        if bytes < 2 || !bytes.is_multiple_of(2) {
            return Err("empty".into());
        }
        let pointer = GlobalLock(handle).cast::<u16>();
        if pointer.is_null() {
            return Err("empty".into());
        }
        let _memory = Memory(handle);
        let units = std::slice::from_raw_parts(pointer, bytes / 2);
        let end = units.iter().position(|c| *c == 0).ok_or("empty")?;
        let text = String::from_utf16(&units[..end]).map_err(|_| "empty")?;
        if text.chars().count() > LIMIT as usize {
            Err("limit".into())
        } else {
            Ok(text)
        }
    }
}
