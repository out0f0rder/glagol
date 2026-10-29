import importlib.util
from pathlib import Path
import sys
import io
import json
import unittest
spec=importlib.util.spec_from_file_location("worker",Path(__file__).resolve().parents[2]/"src-tauri/src/tts/silero/worker.py")
worker=importlib.util.module_from_spec(spec)
original=sys.stdout
spec.loader.exec_module(worker)
sys.stdout=original

class TextTests(unittest.TestCase):
    def test_english_is_independent_and_keeps_numbers(self):
        text=worker.normalize_english("USB PDF 2026 $12.50 14:30 001 20%")
        self.assertIn("U S B P D F",text)
        self.assertIn("two thousand and twenty-six",text)
        self.assertIn("twelve point five dollars",text)
        self.assertIn("zero zero one",text)
        self.assertIn("twenty percent",text)
        self.assertNotRegex(text,r"[а-яёА-ЯЁ0-9]")
    def test_numbers_and_latin_do_not_disappear(self):
        text=worker.normalize("Windows USB 2026,56 14:30 12.09.2026")
        self.assertIn("виндоус ю эс би",text)
        self.assertIn("две тысячи двадцать шесть",text)
        self.assertIn("запятая пятьдесят шесть",text)
        self.assertIn("четырнадцать часов тридцать",text)
        self.assertFalse(any(c.isdigit() for c in text))
    def test_stress_and_questions_survive(self):
        self.assertEqual(worker.normalize("зам+ок? по+ёт! з+амок. замо́к"),"зам+ок? по+ёт! з+амок. зам+ок")
    def test_long_digits_preserve_every_digit(self):
        self.assertEqual(worker.normalize("001"),"ноль ноль один")
        self.assertEqual(len(worker.normalize("1"*20).split()),20)
    def test_split_is_bounded_and_lossless(self):
        text="Это предложение? Следующее предложение. "*100
        parts=list(worker.segments(text.strip()))
        self.assertTrue(all(0<len(p)<=480 for p in parts))
        self.assertEqual(" ".join(parts),text.strip())

class MemoryProtocolTests(unittest.TestCase):
    def test_exact_binary_frame_and_no_wav(self):
        output = io.BytesIO()
        protocol = io.TextIOWrapper(output, encoding="utf-8", newline="\n")
        previous = worker.PROTOCOL
        worker.PROTOCOL = protocol
        try:
            worker.emit_pcm(bytes([0, 128, 255, 127]))
            header, payload = output.getvalue().split(b"\n", 1)
            self.assertEqual(json.loads(header), {"pcm_bytes": 4})
            self.assertEqual(payload, bytes([0, 128, 255, 127]))
        finally:
            worker.PROTOCOL = previous

    def test_invalid_or_oversized_pcm_emits_nothing(self):
        output = io.BytesIO()
        protocol = io.TextIOWrapper(output, encoding="utf-8", newline="\n")
        previous = worker.PROTOCOL
        worker.PROTOCOL = protocol
        try:
            for frames in (b"x", bytes(5760002)):
                with self.assertRaises(ValueError):
                    worker.emit_pcm(frames)
                self.assertEqual(output.getvalue(), b"")
        finally:
            worker.PROTOCOL = previous

if __name__=="__main__": unittest.main()
