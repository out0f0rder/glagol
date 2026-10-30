//! All COM interfaces stay on this child process's MTA thread and die before uninit.
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{HANDLE, HGLOBAL, HWND, POINT},
        System::{Com::*, Threading::*},
        UI::{
            Accessibility::*,
            HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
            WindowsAndMessaging::{
                GetAncestor, GetForegroundWindow, GetPhysicalCursorPos, WindowFromPoint, GA_ROOT,
            },
        },
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
    if uuid::Uuid::parse_str(s).is_ok()
        || s.starts_with("HwndWrapper[")
        || s.starts_with("AutomationId:")
    {
        return None;
    }
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
    for _ in 0..16 {
        // SAFETY: live COM interfaces on their MTA thread. Unknown protection or an
        // incomplete ancestry is rejected. A checked window ends the ancestry scope.
        unsafe {
            if node
                .CurrentIsPassword()
                .map(|v| v.as_bool())
                .unwrap_or(true)
            {
                return true;
            }
            match node.CurrentControlType() {
                Ok(kind) if kind == UIA_WindowControlTypeId => return false,
                Err(_) => return true,
                _ => {}
            }
            let Ok(parent) = walker.GetParentElement(&node) else {
                return true;
            };
            node = parent;
        }
    }
    true
}
/// Read only a selection, never DocumentRange on the focused document.
fn selected(
    element: &IUIAutomationElement,
    walker: &IUIAutomationTreeWalker,
) -> Result<String, String> {
    // SAFETY: COM calls on MTA, bounded range count and character limit; no mutation.
    unsafe {
        let pattern = text_pattern(element).map_err(|_| "unavailable")?;
        if protected(element, walker) {
            return Err("protected".into());
        }
        let ranges = pattern.GetSelection().map_err(|_| "unavailable")?;
        let count = ranges.Length().map_err(|_| "unavailable")?;
        if count > 32 {
            return Err("limit".into());
        }
        let mut parts = Vec::new();
        let mut remaining = LIMIT;
        for i in 0..count {
            let range = ranges.GetElement(i).map_err(|_| "unavailable")?;
            if range
                .CompareEndpoints(
                    TextPatternRangeEndpoint_Start,
                    &range,
                    TextPatternRangeEndpoint_End,
                )
                .map_err(|_| "unavailable")?
                == 0
            {
                continue;
            }
            let text = range
                .GetText(remaining + 1)
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
fn selected_with_ancestors(
    element: &IUIAutomationElement,
    walker: &IUIAutomationTreeWalker,
) -> Result<String, String> {
    use super::selection::{first_selection, Probe};
    first_selection(
        element.clone(),
        |node| {
            // SAFETY: same MTA interfaces; stop at the containing window. Only
            // GetSelection is read, never DocumentRange, Name, Value or siblings.
            unsafe {
                if node.CurrentControlType().map_err(|_| "unavailable")? == UIA_WindowControlTypeId
                {
                    return Ok(Probe::Boundary);
                }
            }
            match selected(node, walker) {
                Ok(text) => Ok(Probe::Text(text)),
                Err(error) if matches!(error.as_str(), "empty" | "unavailable") => {
                    Ok(Probe::Missing)
                }
                Err(error) => Err(error),
            }
        },
        |node| {
            // SAFETY: read-only parent lookup on this thread; process timeout and
            // a 16-node budget bound hostile or cyclic providers.
            unsafe { walker.GetParentElement(node).ok() }
        },
    )?
    .ok_or_else(|| "empty".into())
}
fn selection_under_pointer(
    automation: &IUIAutomation,
    walker: &IUIAutomationTreeWalker,
    foreground: HWND,
) -> Result<String, String> {
    // SAFETY: physical cursor/window queries only. Never read a selection from a
    // different window behind the pointer or change the user's focus.
    unsafe {
        if GetForegroundWindow() != foreground {
            return Err("changed".into());
        }
        let mut point = POINT::default();
        GetPhysicalCursorPos(&mut point).map_err(|_| "cursor")?;
        if foreground.0.is_null() || GetAncestor(WindowFromPoint(point), GA_ROOT) != foreground {
            return Err("empty".into());
        }
        let element = automation
            .ElementFromPoint(point)
            .map_err(|_| "unavailable")?;
        if protected(&element, walker) {
            return Err("protected".into());
        }
        let text = selected_with_ancestors(&element, walker)?;
        if GetForegroundWindow() != foreground {
            return Err("changed".into());
        }
        Ok(text)
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
    // SAFETY: this helper owns its windowless thread. UIA consumes physical pixels;
    // configure this thread before COM. The child exits before Tauri initialization.
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
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
            let _ = timeouts.SetConnectionTimeout(500);
            let _ = timeouts.SetTransactionTimeout(1000);
        }
        let walker = automation.ControlViewWalker().map_err(|_| "UIA")?;
        let mut point = POINT::default();
        let foreground = GetForegroundWindow();
        let element = if source == "selection" {
            automation.GetFocusedElement()
        } else {
            GetPhysicalCursorPos(&mut point).map_err(|_| "cursor")?;
            automation.ElementFromPoint(point)
        }
        .map_err(|e| format!("element:{:08X}", e.code().0 as u32))?;
        if protected(&element, &walker) {
            return Err("protected".into());
        }
        if source == "selection" {
            let result = selected_with_ancestors(&element, &walker).or_else(|error| {
                if !matches!(error.as_str(), "empty" | "unavailable") {
                    return Err(error);
                }
                if let Ok(hwnd) = element.CurrentNativeWindowHandle() {
                    match super::scintilla::selected(hwnd) {
                        Ok(text) => return Ok(text),
                        Err(fallback) if fallback != "unavailable" => return Err(fallback),
                        Err(_) => {}
                    }
                }
                // Chrome's PDF toolbar may retain UIA focus while the document
                // owns the selection. Still query only GetSelection, never a
                // paragraph/Name/Value fallback as in the separate pointer mode.
                selection_under_pointer(&automation, &walker, foreground)
            });
            return result.and_then(|text| {
                if GetForegroundWindow() == foreground {
                    Ok(text)
                } else {
                    Err("changed".into())
                }
            });
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
