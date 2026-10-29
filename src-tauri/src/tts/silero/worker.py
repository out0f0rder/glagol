"""Glagol's offline Silero adapter (MIT); model code remains in its NC package.

One bounded JSON request per line. Metadata plus explicitly requested bounded binary PCM use private stdout.
The parent selects all paths. No hub, HTTP service, pip or runtime downloads.
"""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import threading
import wave

PROTOCOL = sys.stdout
sys.stdout = sys.stderr
VOICES = ("aidar", "baya", "kseniya", "xenia", "eugene")
MODEL_HASH = "50081637b602126ee06cb3bc8a744d25651d2da149ee8864b9a379bfdd934437"
EN_MODEL_HASH = "02b71034d9f13bc4001195017bac9db1c6bb6115e03fea52983e8abcff13b665"
EN_VOICES = ("en_0", "en_1", "en_2", "en_3")

LETTER_NAMES = dict(zip("abcdefghijklmnopqrstuvwxyz", ("эй", "би", "си", "ди", "и", "эф", "джи", "эйч", "ай", "джей", "кей", "эл", "эм", "эн", "оу", "пи", "кью", "ар", "эс", "ти", "ю", "ви", "дабл ю", "экс", "уай", "зэд")))
LEGACY_WORDS = {"windows": "виндоус", "python": "пайтон", "linux": "линукс", "email": "электронная почта", "pdf": "пи ди эф", "usb": "ю эс би", "tts": "ти ти эс", "api": "эй пи ай"}
# A small exception seam for irregular words/names; unknown words use rules below.
# This is pronunciation, not translation or language detection. No external data.
EN_PRONUNCIATIONS = {
    "a": "э", "an": "эн", "and": "энд", "are": "ар", "as": "эз", "at": "эт",
    "be": "би", "been": "бин", "but": "бат", "by": "бай", "do": "ду",
    "does": "даз", "done": "дан", "for": "фор", "from": "фром", "have": "хэв",
    "he": "хи", "here": "хир", "i": "ай", "is": "из", "it": "ит", "me": "ми",
    "my": "май", "no": "ноу", "not": "нот", "of": "ов", "on": "он", "one": "уан",
    "or": "ор", "our": "ауэр", "out": "аут", "she": "ши", "so": "соу",
    "some": "сам", "the": "зэ", "their": "зэр", "there": "зэр", "these": "зиз",
    "they": "зэй", "this": "зис", "that": "зэт", "to": "ту", "two": "ту",
    "us": "ас", "use": "юз", "was": "уоз", "we": "уи", "were": "уёр",
    "what": "уот", "where": "уэр", "who": "ху", "why": "уай", "with": "уиз",
    "you": "ю", "your": "ёр", "all": "ол", "any": "эни", "many": "мэни",
    "hello": "хеллоу", "world": "уорлд", "word": "уёрд", "words": "уёрдз",
    "english": "инглиш", "read": "рид", "quick": "квик", "text": "текст",
    "work": "уёрк", "works": "уёркс", "working": "уёркинг", "locally": "лоукэли",
    "computer": "компьютер", "browser": "браузер", "chrome": "хром",
    "google": "гугл", "microsoft": "майкрософт", "apple": "эпл", "iphone": "айфон",
    "github": "гитхаб", "web": "вэб", "view": "вью", "open": "оупен",
    "code": "коуд", "voice": "войс", "queue": "кью", "file": "файл",
    "save": "сэйв", "settings": "сеттингс", "speed": "спид", "screen": "скрин",
    "speech": "спич", "language": "лэнгвидж", "through": "сру", "thought": "сот",
    "enough": "инаф", "laugh": "лаф", "thread": "срэд", "technology": "текнолоджи",
}
INITIALISMS = frozenset("ai api cpu gpu ui ux usb pdf tts stt ocr http https html css json xml sql url ssh ram rom ssd hdd gb mb kb ghz mhz hz wifi".split())
PHONETIC_RULES = tuple((re.compile(pattern), replacement) for pattern, replacement in (
    (r"^kn", "н"), (r"^wr", "р"), (r"^gn", "н"), (r"mb$", "м"),
    (r"tch", "ч"), (r"sh", "ш"), (r"ch", "ч"), (r"ph", "ф"), (r"th", "с"),
    (r"qu", "кв"), (r"ck", "к"), (r"wh", "у"), (r"dge", "дж"),
    (r"ssion|tion", "шэн"), (r"sion", "жэн"), (r"ture$", "чер"),
    (r"eigh", "эй"), (r"igh", "ай"), (r"ee|ea", "и"), (r"oo", "у"),
    (r"ai|ay|ei|ey", "эй"), (r"oa|ow", "оу"), (r"oi|oy", "ой"),
    (r"ou", "ау"), (r"ew", "ью"), (r"ir|ur", "ёр"),
    (r"c(?=[eiy])", "с"), (r"g(?=[eiy])", "дж"),
    (r"([b-df-hj-np-tv-z])\1", r"\1"), (r"y$", "и"),
))
PHONETIC_LETTERS = str.maketrans(dict(zip("abcdefghijklmnopqrstuvwxyz", ("э", "б", "к", "д", "е", "ф", "г", "х", "и", "дж", "к", "л", "м", "н", "о", "п", "к", "р", "с", "т", "а", "в", "у", "кс", "й", "з"))))


def spell_latin(word):
    return " ".join(LETTER_NAMES[c] for c in word.lower())


def pronounce_latin(word):
    """Bounded, approximate English-to-Cyrillic spelling for a Russian voice."""
    lower = word.lower()
    if lower in LEGACY_WORDS:
        return LEGACY_WORDS[lower]
    if lower in INITIALISMS or (word.isupper() and lower in ("it", "us")):
        return spell_latin(word)
    if lower in EN_PRONUNCIATIONS:
        return EN_PRONUNCIATIONS[lower]
    # Split names such as QuickRead/OpenAI, preserving initialisms in each part.
    parts = re.split(r"(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", word)
    if len(parts) > 1:
        return " ".join(pronounce_latin(part) for part in parts)
    # Do not turn technical initialisms, variables or long identifiers into words.
    if len(word) == 1 or len(word) > 40 or (word.isupper() and len(word) <= 6) or not re.search(r"[aeiouy]", lower):
        return spell_latin(word)
    # Silent final e lengthens the preceding vowel (code, file, name).
    long_vowels = {"a": "эй", "e": "и", "i": "ай", "o": "оу", "u": "ю"}
    lower = re.sub(r"([aeiou])([b-df-hj-np-tv-z])e$", lambda m: long_vowels[m[1]] + {"c": "с", "g": "дж"}.get(m[2], m[2]), lower)
    lower = re.sub(r"e$", "", lower)
    for pattern, replacement in PHONETIC_RULES:
        lower = pattern.sub(replacement, lower)
    return lower.translate(PHONETIC_LETTERS)


def emit(payload):
    PROTOCOL.write(json.dumps(payload) + "\n")
    PROTOCOL.flush()


def emit_pcm(frames):
    # One sentence cannot allocate or emit unbounded audio. Validate before header.
    if len(frames) > 24000 * 120 * 2 or len(frames) % 2:
        raise ValueError("audio_limit")
    emit({"pcm_bytes": len(frames)})
    PROTOCOL.buffer.write(frames)
    PROTOCOL.buffer.flush()


def watch_parent(pid):
    if sys.platform != "win32":
        return
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.OpenProcess.argtypes = (ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong)
    kernel.OpenProcess.restype = ctypes.c_void_p
    kernel.WaitForSingleObject.argtypes = (ctypes.c_void_p, ctypes.c_ulong)
    kernel.CloseHandle.argtypes = (ctypes.c_void_p,)
    handle = kernel.OpenProcess(0x00100000, False, pid)  # SYNCHRONIZE only
    if not handle:
        os._exit(1)
    def wait():
        kernel.WaitForSingleObject(handle, 0xFFFFFFFF)
        kernel.CloseHandle(handle)
        os._exit(0)
    threading.Thread(target=wait, daemon=True).start()


def normalize(text, phonetic_english=False):
    from num2words import num2words
    # Keep punctuation and explicit Silero stress/focus markers. Digits and
    # Latin letters are otherwise silently removed by this Russian model.
    text = re.sub(r"([аеёиоуыэюяАЕЁИОУЫЭЮЯ])\u0301", r"+\1", text)
    if phonetic_english:
        # Keep model versions/device suffixes separated from expanded numbers.
        text = re.sub(r"(?<=[A-Za-z])(?=[0-9])|(?<=[0-9])(?=[A-Za-z])", " ", text)
    def number(match):
        value = match.group()
        if len(value) > 15 or (len(value) > 1 and value.startswith("0")):
            return " ".join(num2words(int(d), lang="ru") for d in value)
        return num2words(int(value), lang="ru")
    # Decimal/composite values retain every component. We don't infer units or
    # grammatical cases that cannot be determined reliably from plain text.
    text = re.sub(r"(?<=\d),(?=\d)", " запятая ", text)
    text = re.sub(r"(?<=\d):(?=\d)", " часов ", text)
    text = re.sub(r"(?<=\d)\.(?=\d)", " точка ", text)
    text = re.sub(r"[0-9]+", number, text)
    text = re.sub(r"[a-zA-Z]+", lambda m: pronounce_latin(m[0]) if phonetic_english else LEGACY_WORDS.get(m[0].lower()) or spell_latin(m[0]), text)
    text = text.replace("%", " процентов ").replace("№", " номер ")
    return re.sub(r"\s+", " ", text).strip()


def normalize_english(text):
    from num2words import num2words
    from decimal import Decimal
    def number(match):
        value = match.group()
        if len(value) > 15 or (len(value) > 1 and value.startswith("0") and not value.startswith("0.")):
            return " ".join("point" if c == "." else num2words(int(c), lang="en") for c in value)
        return num2words(Decimal(value), lang="en")
    # Expand common written forms before the model's symbol filtering.
    text = re.sub(r"(?<=\d),(?=\d{3}(?:\D|$))", "", text)
    text = re.sub(r"\$([0-9]+(?:\.[0-9]+)?)", r"\1 dollars", text)
    text = re.sub(r"£([0-9]+(?:\.[0-9]+)?)", r"\1 pounds", text)
    text = re.sub(r"(?<=\d):(?=\d)", " ", text)
    text = re.sub(r"[0-9]+(?:\.[0-9]+)?", number, text)
    text = re.sub(r"\b[A-Z]{2,6}\b", lambda m: " ".join(m[0]), text)
    text = text.replace("%", " percent ").replace("№", " number ")
    return re.sub(r"\s+", " ", text).strip()


def segments(text, limit=480):
    while len(text) > limit:
        end = max(text.rfind(mark, 0, limit) for mark in (". ", "! ", "? ", "; "))
        if end >= limit // 3:
            end += 1
        else:
            end = text.rfind(" ", 0, limit)
        if end < 1:
            end = limit
        yield text[:end].strip()
        text = text[end:].strip()
    if text:
        yield text


def main():
    model_path, output_dir, parent = sys.argv[1:4]
    language = sys.argv[4] if len(sys.argv) == 5 else "ru"
    if language not in ("en", "ru"):
        raise ValueError("invalid_language")
    voices = EN_VOICES if language == "en" else VOICES
    model_hash = EN_MODEL_HASH if language == "en" else MODEL_HASH
    watch_parent(int(parent))
    with open(model_path, "rb") as source:
        if hashlib.file_digest(source, "sha256").hexdigest() != model_hash:
            raise ValueError("model_hash")
    import torch
    torch.set_num_threads(2)
    torch.set_num_interop_threads(1)
    model = torch.package.PackageImporter(model_path).load_pickle("tts_models", "model")
    model.to(torch.device("cpu"))
    emit({"ready": True})
    output = Path(output_dir) / "chunk.wav"
    while True:
        line = sys.stdin.buffer.readline(4097)
        if not line:
            return
        if len(line) > 4096 or not line.endswith(b"\n"):
            raise ValueError("protocol_limit")
        request = json.loads(line)
        text, voice = request["text"], request["voice"]
        if not isinstance(text, str) or not 1 <= len(text) <= 300 or voice not in voices:
            raise ValueError("invalid_request")
        try:
            memory = request.get("pcm", False)
            phonetic = request.get("phonetic_english", False)
            if not isinstance(memory, bool) or not isinstance(phonetic, bool):
                raise ValueError("invalid_request")
            # Only Quick Read's explicit PCM option changes pronunciation. The
            # document/preview WAV path and independent English model stay stable.
            normalized = normalize_english(text) if language == "en" else normalize(text, memory and phonetic)
            if not re.search("[a-zA-Z]" if language == "en" else "[а-яёА-ЯЁ]", normalized):
                emit({"empty": True})
                continue
            frames = bytearray()
            writer = None if memory else wave.open(str(output), "wb")
            try:
                if writer:
                    writer.setnchannels(1)
                    writer.setsampwidth(2)
                    writer.setframerate(24000)
                for segment in segments(normalized):
                    with torch.inference_mode():
                        options = {} if language == "en" else {"put_accent": True, "put_yo": True}
                        audio = model.apply_tts(text=segment, speaker=voice, sample_rate=24000, **options)
                    if not torch.isfinite(audio).all() or audio.numel() > 24000 * (120 if memory else 300):
                        raise ValueError("audio_limit")
                    pcm = (audio.clamp(-1, 1) * 32767).round().to(torch.int16).cpu().numpy().astype("<i2").tobytes()
                    if memory:
                        if len(frames) + len(pcm) > 24000 * 120 * 2:
                            raise ValueError("audio_limit")
                        frames.extend(pcm)
                    else:
                        writer.writeframes(pcm)
            finally:
                if writer:
                    writer.close()
            if memory:
                emit_pcm(frames)
            else:
                emit({"ok": True})
        except Exception:
            output.unlink(missing_ok=True)
            emit({"error": "synthesis_failed"})


if __name__ == "__main__":
    try:
        main()
    except Exception:
        emit({"error": "worker_failed"})
        raise SystemExit(1)
