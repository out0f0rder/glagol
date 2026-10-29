# Быстрое чтение: проверки

Основные гейты — [Windows build](windows-build.md). Команды запускаются из корня.

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm exec tsc --noEmit
pnpm build
pnpm runbook:test
pnpm runbook:check
```

Node entrypoints: `node scripts/context-status.mjs write`,
`node scripts/runbook-check.mjs`, `node --test scripts/runbook-check.test.mjs`.
Не считать ignored native tests пройденными обычным cargo test.

## Настоящий Silero без воспроизведения

Подготовить **изолированную** копию pinned runtime и модели; не запускать smoke
на каталоге работающего приложения: Worker удаляет старые private worker dirs.
Хеши проверяются до запуска модели. Не копировать библиотеку, записи речи, ключи
или пользовательские документы. Модели и runtime не коммитить/не включать в NSIS.

```powershell
$env:GLAGOL_TTS_SMOKE_ROOT = (Resolve-Path '.scratch/quick-read-native').Path
cargo test --manifest-path src-tauri/Cargo.toml native_silero_pcm_wav_transition_and_cancel -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml native_silero_english_shared_runtime -- --ignored --nocapture
& .scratch/quick-read-native/runtime/python.exe -I -B scripts/silero/test_worker.py
```

RU тест проверяет ненулевой PCM без chunk.wav, переход в прежний WAV-протокол и
обратно, отмену и восстановление процесса. EN — прежний WAV/library pipeline,
голоса и recovery/cancel. Метрика первого PCM после загрузки worker не равна
задержке горячая клавиша → слышимый звук. Тесты не открывают аудиоустройство.

## UIA/OCR и ручная приёмка

Использовать публичный fixture или тестовые документы. Не выводить извлечённый
текст и не сохранять скриншоты/буфер в evidence. У source helper есть жёсткий
таймаут: UIA/clipboard 3 с, OCR 15 с. Main window и single-instance не должны
инициализироваться в child mode. Нулевой exit процесса сопровождается Result
JSON; ошибку источника определяет Err, не только exit code.

Проверить GetSelection без DocumentRange, pointer без клика, protected ancestry,
сохранение focus/clipboard sequence, ру/ен язык OCR и сообщение об отсутствии
Windows language component. Не заменять отсутствующий язык молча.

[Полная ручная матрица Windows 11](../workstreams/quick-read/manual-qa.md) остаётся
отдельной: реальные браузеры/редакторы, global hotkeys, звук/паузы, mini-player без
фокуса, overlay/Escape, разные DPI/мониторы, offline, регрессия трея и диктовки.
Сценарий на одном scripted WPF fixture не заменяет эти проверки.
