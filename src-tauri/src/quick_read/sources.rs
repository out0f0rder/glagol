//! Text extraction never modifies the clipboard or the foreground application.
pub async fn extract(source: &str) -> Result<String, String> {
    if source != "clipboard" {
        return Err(super::message(
            "Selection is unavailable. Copy text and use Read clipboard.",
            "Выделение недоступно. Скопируйте текст и прочитайте буфер.",
        ));
    }
    tokio::task::spawn_blocking(|| {
        arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .map_err(|_| {
                super::message(
                    "The clipboard contains no text.",
                    "В буфере обмена нет текста.",
                )
            })
    })
    .await
    .map_err(|e| e.to_string())?
}
