//! Native screen-to-WinRT OCR, in the same bounded child mode as UIA.
use windows::{
    core::HSTRING,
    Globalization::Language,
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::{
        Graphics::Gdi::*,
        System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
        UI::HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
    },
};
struct Runtime;
impl Drop for Runtime {
    fn drop(&mut self) {
        // SAFETY: one successful RoInitialize on this thread, all WinRT objects dropped first.
        unsafe {
            RoUninitialize();
        }
    }
}
struct Capture {
    screen: HDC,
    memory: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
}
impl Drop for Capture {
    fn drop(&mut self) {
        // SAFETY: each GDI object is owned here; restore the selected bitmap before deletion.
        unsafe {
            if !self.old.is_invalid() {
                SelectObject(self.memory, self.old);
            }
            if !self.bitmap.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            }
            if !self.memory.is_invalid() {
                let _ = DeleteDC(self.memory);
            }
            if !self.screen.is_invalid() {
                ReleaseDC(None, self.screen);
            }
        }
    }
}
fn screenshot(x: i32, y: i32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    // SAFETY: all dimensions have been validated; screen and bitmap handles are RAII owned,
    // DIB buffer matches width*height*4, top-down 32-bit rows need no extra padding.
    unsafe {
        let mut capture = Capture {
            screen: GetDC(None),
            memory: HDC::default(),
            bitmap: HBITMAP::default(),
            old: HGDIOBJ::default(),
        };
        if capture.screen.is_invalid() {
            return Err("capture".into());
        }
        capture.memory = CreateCompatibleDC(Some(capture.screen));
        capture.bitmap = CreateCompatibleBitmap(capture.screen, width, height);
        if capture.memory.is_invalid() || capture.bitmap.is_invalid() {
            return Err("capture".into());
        }
        capture.old = SelectObject(capture.memory, HGDIOBJ(capture.bitmap.0));
        if capture.old.is_invalid() {
            return Err("capture".into());
        }
        BitBlt(
            capture.memory,
            0,
            0,
            width,
            height,
            Some(capture.screen),
            x,
            y,
            SRCCOPY,
        )
        .map_err(|_| "capture")?;
        SelectObject(capture.memory, capture.old);
        capture.old = HGDIOBJ::default();
        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB.0;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        if GetDIBits(
            capture.screen,
            capture.bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        ) != height
        {
            return Err("capture".into());
        }
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
        Ok(pixels)
    }
}
/// Recognize only the explicitly chosen installed language; never silently substitute.
pub fn recognize(args: &[String]) -> Result<String, String> {
    if args.len() != 5 {
        return Err("invalid".into());
    }
    let values: Vec<i32> = args[..4]
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()
        .map_err(|_| "invalid")?;
    let (x, y, w, h) = (values[0], values[1], values[2], values[3]);
    if w < 3
        || h < 3
        || w > 4096
        || h > 4096
        || w as i64 * h as i64 > 16_000_000
        || !matches!(args[4].as_str(), "ru-RU" | "en-US")
    {
        return Err("invalid".into());
    }
    // SAFETY: fresh helper thread. DPI context is local to this process/thread and never shared.
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        RoInitialize(RO_INIT_MULTITHREADED).map_err(|_| "ocr_unavailable")?;
    }
    let _runtime = Runtime;
    let language =
        Language::CreateLanguage(&HSTRING::from(&args[4])).map_err(|_| "ocr_language")?;
    if !OcrEngine::IsLanguageSupported(&language).map_err(|_| "ocr_unavailable")? {
        return Err("ocr_language".into());
    }
    let max = OcrEngine::MaxImageDimension().map_err(|_| "ocr_unavailable")? as i32;
    if w > max || h > max {
        return Err("ocr_size".into());
    }
    let engine = OcrEngine::TryCreateFromLanguage(&language).map_err(|_| "ocr_language")?;
    let pixels = screenshot(x, y, w, h)?;
    let writer = DataWriter::new().map_err(|_| "ocr_unavailable")?;
    writer.WriteBytes(&pixels).map_err(|_| "ocr_unavailable")?;
    let buffer = writer.DetachBuffer().map_err(|_| "ocr_unavailable")?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        w,
        h,
        BitmapAlphaMode::Ignore,
    )
    .map_err(|_| "ocr_unavailable")?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|_| "ocr_unavailable")?
        .get()
        .map_err(|_| "ocr_unavailable")?;
    let lines = result.Lines().map_err(|_| "ocr_unavailable")?;
    let mut text = Vec::new();
    for line in lines {
        text.push(line.Text().map_err(|_| "ocr_unavailable")?.to_string());
    }
    Ok(text.join("\n"))
}
