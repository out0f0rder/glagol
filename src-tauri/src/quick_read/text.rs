//! Conservative Unicode preparation. Number/Latin pronunciation stays in Silero.
use crate::text::{chunker::chunk_text, preprocessor::expand_abbreviations};

/// Normalize layout without replacing URLs, model names, dates or units.
/// This is the seam for a future user pronunciation dictionary.
pub fn normalize(input: &str) -> String {
    let input = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::new();
    let mut blank = false;
    for line in input.lines() {
        let line: String = line
            .chars()
            .filter(|c| !matches!(c, '\u{200b}' | '\u{feff}' | '\u{00ad}'))
            .collect();
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            blank = !out.is_empty();
            continue;
        }
        if !out.is_empty() {
            out.push_str(if blank { "\n\n" } else { " " });
        }
        out.push_str(&line);
        blank = false;
    }
    expand_abbreviations(&out)
}

/// One sentence per chunk when possible; long sentences reuse the shared chunker.
pub fn chunks(input: &str) -> Result<Vec<String>, String> {
    if input.chars().count() > 500_000 {
        return Err(super::message(
            "Text is too long (500,000 characters maximum).",
            "Текст слишком длинный (максимум 500 000 символов).",
        ));
    }
    let clean = normalize(input);
    let mut sentences = Vec::new();
    let mut start = 0;
    for (i, c) in clean.char_indices() {
        let end = i + c.len_utf8();
        let next = clean[end..].chars().next();
        let token = clean[start..end].split_whitespace().last().unwrap_or("");
        let abbreviation = c == '.'
            && (token.chars().filter(|c| c.is_alphabetic()).count() == 1
                || [
                    "г.", "ул.", "стр.", "Mr.", "Dr.", "Mrs.", "e.g.", "т.д.", "т.п.",
                ]
                .contains(&token));
        if matches!(c, '.' | '!' | '?' | '…')
            && next.is_none_or(char::is_whitespace)
            && !abbreviation
        {
            sentences.extend(chunk_text(clean[start..end].trim(), 200));
            start = end;
        }
    }
    sentences.extend(chunk_text(clean[start..].trim(), 200));
    if sentences
        .iter()
        .all(|s| !s.chars().any(char::is_alphanumeric))
    {
        return Err(super::message(
            "No readable text. Copy text and use Read clipboard, or try OCR.",
            "Текст не найден. Скопируйте текст и прочитайте буфер либо используйте OCR.",
        ));
    }
    Ok(sentences)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout_and_technical_content_survive() {
        assert_eq!(
            normalize("  USB-C\u{a0}3.2\r\n  12.5 Вт\n\n\nWindows 11\u{200b}"),
            "USB-C 3.2 12.5 Вт\n\nWindows 11"
        );
        assert_eq!(
            chunks("Дата 29.10.2026. Следующий текст! Hello world.")
                .unwrap()
                .len(),
            3
        );
    }
    #[test]
    fn unicode_long_text_is_bounded_and_ordered() {
        let input = "Я🙂 USB-C 12.5 Вт ".repeat(200);
        let result = chunks(&input).unwrap();
        assert!(result.iter().all(|s| s.chars().count() <= 200));
        assert_eq!(result.join(" "), normalize(&input));
        assert!(chunks(" \n… ").is_err());
        assert!(chunks(&"x".repeat(500001)).is_err());
    }
}
