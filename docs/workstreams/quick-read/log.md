# Quick Read — журнал

## 2026-10-29T09:16:39.283Z — вход и архитектура

Baseline main 8c59250, дерево чистое; создана feat/quick-read. Прочитаны STATUS, self-update state/README/последние checkpoints, CLAUDE, SECURITY, PROJECT_STRUCTURE, context и ранбуки Windows/TTS. Новый scope разрешает локальные commits и сборку установщика, публикация не запрошена. WSL, инструменты пока обнаруживаются; Windows PowerShell доступен через sandbox escalation, node/cargo отсутствуют в его PATH. Отдельного managed environment tool нет.

A занят Codex. Проверяется реальный worker (дисковый WAV), chunker 280 символов, HTML audio библиотеки, cpal диктовки, общий operation mutex Silero, Rust global-shortcut и close-to-tray. План: расширить текущий worker PCM transport, bounded output, отдельный UIA child mode для killable timeout; интерфейс и OCR в C.

## 2026-10-29T09:27:41.605Z — A проверен, B занят

Добавлен quick_read (консервативная Unicode нормализация/предложения, bounded cpal PCM queue, генерации запросов, pause/stop/skip, clipboard; shared global-shortcut dispatch). Существующий Silero worker получил opt-in binary PCM без WAV, библиотечный WAV-протокол сохранён. Общая TTS operation блокировка сохраняется. UIA selection и pointer входят в B; автоматического Ctrl+C нет.

Windows interop через .scratch/check.cmd, isolated SHA-verified Rust/cargo/clippy/rustfmt 1.99.1 и установленный MSVC. cargo fmt --manifest-path src-tauri/Cargo.toml --check — exit 0; cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings — exit 0; cargo test --manifest-path src-tauri/Cargo.toml — exit 0, 353 passed, 6 ignored (native модели). Первая проверка FAIL: импорт Manager, новые lint; первый тест FAIL: дата ошибочно принималась за инициал; исправлены, проверки повторены. Frontend: Node v24.13.0, pnpm 12.16.1, pnpm exec tsc --noEmit / pnpm build — exit 0, I18N 257 keys, Vite built. Предупреждение bundle >500KB остаётся. Зависимости по frozen lockfile; rust/cargo не установлены глобально.

Новые тесты проверяют очередь/порядок/паузу/очистку/лимит PCM/Unicode/даты/лимит текста. GUI/звук/hotkey manual NOT_RUN. Следующий шаг B — killable helper process с COM MTA и bounded output, selection only GetSelection, pointer RangeFromPoint/Value/Name.

## 2026-10-29T09:32:46.090Z — B проверен, C занят

Добавлен скрытый same-EXE mode до Tauri/logging/single-instance. COM MTA + CUIAutomation8, provider timeouts 200/500 мс, общий hard timeout 3 с, bounded stdout 3 MB, kill-on-drop и watcher родителя. Новый запрос отменяет также извлечение. Selection только GetSelection (до 32 диапазонов, 500k chars); pointer ElementFromPoint + paragraph range/Value/Name/Legacy, до 8 детей и 2 родителей, без обхода всего дерева, password ancestry fail-closed. Clipboard: read-only CF_UNICODETEXT, GlobalSize bound до чтения, без EmptyClipboard/SetClipboardData/SendInput, все исходные форматы остаются у владельца.

Windows: cargo fmt --manifest-path src-tauri/Cargo.toml --check exit 0; cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings exit 0 (после исправления OwnedHandle, lint имён внешних констант); cargo test --manifest-path src-tauri/Cargo.toml exit 0: 354 passed, 6 ignored. Новый тест hung_provider_process_is_killed PASS; UIA/пароль в реальных окнах manual NOT_RUN. Frontend не менялся, A build/tsc остаётся применим; дополнительно запущен повторный build.

Этап A сохранён local commit 9a910a2. Первая попытка commit не выполнена из-за отсутствия identity; повтор с git -c user.name=Codex -c user.email=codex@openai.com, без глобальной настройки. Context gate сначала INCOMPLETE exit 2 из-за отсутствующих release refs; получены шесть публичных тегов, повтор runbook:check exit 0, OK (14 series). Следующий C: OCR WinRT, monitor-local overlay с physical DPI conversion, настройки шести сочетаний и необязательный non-activating mini-player.

## 2026-10-29T09:54:57.704Z — C реализация и автоматические проверки

Добавлены Windows GDI → BGRA memory → Windows.Media.Ocr (явный ru-RU/en-US, IsLanguageSupported, до 15 с), выбор области на мониторе под курсором, physical DPI conversion с отрицательными origin, cancel/Escape. Ограничение первого варианта: один монитор, до 4096 px/side и 16 MP. Неопределённая password ancestry отклоняется, технические Name UUID/HwndWrapper не читаются.

Настройки шести shortcuts используют тот же registrar, что диктовка; общий settings gate исключает гонку, регистрация нового до снятия старого, при конфликте/DB error rollback. Мини-плеер off by default, non-focusable; controls pause/stop/skip/speed, ошибки на 6 с. Во время OCR capture мини-плеер скрыт. Поздняя OCR selection не очищает новую; новое чтение отменяет helper, synthesis и PCM. Исправлен квадратичный разбор длинного токена; тест 100k chars и 10k инициалов. Изменение скорости — resampling, меняет pitch; skip повторно синтезирует целевой фрагмент.

Фактические проверки: pnpm exec tsc --noEmit exit 0; pnpm build exit 0, 278 EN/RU keys, Vite built in 2m55s (warning >500 KB); cargo fmt --manifest-path src-tauri/Cargo.toml --check exit 0; cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings exit 0; cargo test --manifest-path src-tauri/Cargo.toml exit 0, 359 passed/6 ignored до добавления opt-in native PCM smoke. Python3 scripts/silero/test_worker.py MemoryProtocolTests exit 0, 2 tests: настоящий emit_pcm, binary boundary и invalid/oversized frame без модели. Rust fixture проверяет PCM→samples/no WAV/odd length/oversize/truncated stdout; очередь, отмена, DPI и hotkey rollback PASS.

Node child tests в sandbox сначала FAIL exit 1: spawnSync Node выдавал EPERM, пустой stdout. Повтор той же pnpm runbook:test вне sandbox exit 0, 49/49; pnpm i18n:test exit 0, 3/3, включая испорченные CLI inputs с ненулевым кодом. runbook:check после получения отсутствующих tags exit 0 (14 series). Гейты не изменялись.

Первая NSIS сборка exit 0, Finished 1 bundle; после неё добавлены защита UIA ancestry и seam/test hotkey rollback, поэтому артефакт пока не итоговый: нужна повторная сборка. Использованы Windows Rust 1.99.1 и MSVC, Tauri CLI 2.11.1 Windows native binding (SHA-512 из pnpm-lock.yaml). Frontend проверен отдельно Linux pnpm; локальный .scratch/build-context.json отключает повтор beforeBuildCommand в Windows, где pnpm отсутствует. Global settings не менялись, подписывающий release-конфиг не использован.

На хосте найдены установленные RU/EN Silero и runtime. Сейчас копируются только эти pinned компоненты в .scratch/quick-read-native (никаких библиотечных/пользовательских документов, ключей или речи); затем verify hashes и opt-in native PCM/WAV/cancel + EN regression. Реальный звук/GUI, DPI и hotkey latency ещё NOT_RUN. Изменены оба README-раздела, обе USER_GUIDE, CHANGELOG, CLAUDE, PROJECT_STRUCTURE. Сторонний устаревший указатель USER_GUIDE.md записан GL-007, без несвязанного исправления. Источники API: https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nn-uiautomationclient-iuiautomationtextpattern ; https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-understandingperformanceissues ; https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine.recognizeasync .

## 2026-10-29T10:02:54.110Z — C итоговая сборка, настоящий Silero и границы приёмки

Финальный production-код после hotkey registrar refactor и fail-closed UIA ancestry: cargo fmt --manifest-path src-tauri/Cargo.toml --check exit 0; cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings exit 0. cargo test --manifest-path src-tauri/Cargo.toml exit 0: 359 passed, 0 failed, 7 ignored. Исправлен тип Result<usize> SQLite → Result<()> в новом seam; отрицательные тесты конфликта ОС и rollback DB PASS. Новых зависимых версий нет: windows 0.61.3 уже был в lockfile, добавлены target cfg(windows) features.

Изолированный runtime: последовательный cp остановлен из-за медленного копирования множества файлов, Windows robocopy /MT:16 завершился успешно (wrapper exit 0). Скопированы runtime и v5_5_ru.pt/v3_en.pt из установленного каталога, verify полного pinned runtime и модели выполнен тестами до запуска. Установленные модели, библиотека и приложение не менялись.

Windows build 10.0.26200.9711, Rust/cargo 1.99.1, embedded Python из pinned runtime:
- GLAGOL_TTS_SMOKE_ROOT=.scratch/quick-read-native; cargo test --manifest-path src-tauri/Cargo.toml native_silero_pcm_wav_transition_and_cancel -- --ignored --nocapture: exit 0, 1 passed, 41.73 s. Первый RU PCM на загруженном worker — 1146 ms, 88800 samples, ненулевое аудио и без chunk.wav. PCM → прежний WAV → PCM, активная отмена и новый worker после отмены PASS. Это не измерение hotkey-to-sound, аудиоустройство не открывалось.
- Тот же root; cargo test --manifest-path src-tauri/Cargo.toml native_silero_english_shared_runtime -- --ignored --nocapture: exit 0, 1 passed, 43.73 s. Четыре EN голоса, прежний multi-chunk WAV/library pipeline, metadata, recovery и cancel PASS.
- .scratch/quick-read-native/runtime/python.exe -I -B scripts/silero/test_worker.py: exit 0, 7 tests OK (RU/EN normalization и actual PCM binary emitter). Два отдельных MemoryProtocolTests ранее также PASS в Linux Python3; py_compile exit 0.

Финальная NSIS: Windows node .scratch/tauri-windows.cjs build --config .scratch/build-context.json (через .scratch/bundle.cmd), exit 0, release profile finished in 46.31 s; Finished 1 bundle. Артефакт src-tauri/target/release/bundle/nsis/Glagol_0.6.1_x64-setup.exe, время 2026-10-29T09:56:46.088823Z, 10360418 байт, SHA-256 13d70da29057376384a75dd3d90abaf8470c22157719a0287c6ab5e3eb64c827. Копия для передачи: .scratch/quick-read-dist/Glagol_0.6.1_x64-setup.exe + SHA256SUMS.txt + build.json. Это локальный кандидат из исходников 0.6.1, не опубликованный релиз v0.6.1, не подписанный updater и не установленный EXE. Speech модели/runtime в bundle не входят.

Release helper smoke: powershell.exe -NoProfile -ExecutionPolicy Bypass -File .scratch/helper-smoke.ps1, exit 0: Release helper rejects invalid OCR geometry before capture: PASS. Никаких снимков/буфера/пользовательского текста.

Попытка scripted WPF fixture: powershell.exe -NoProfile -STA -ExecutionPolicy Bypass -File .scratch/windows-integration.ps1 — exit 1, Interactive fixture could not become foreground. Защитная проверка остановила сценарий ДО вызова source helper; UIA selection/pointer/password, clipboard sequence и OCR fixture не выполнялись. Это ограничение интерактивной сессии, не PASS и не доказанный дефект приложения. Никаких текстов/снимков/clipboard contents в evidence; фокус/курсор восстанавливались в finally, защиты foreground не обходились. Полная ручная матрица остаётся NOT_RUN.

C завершён по автоматическим гейтам и NSIS. NEXT W: ручная приёмка Windows 11 с настоящим звуком/горячими клавишами/браузерами/overlay/DPI/offline и регрессией диктовки/трея. Claim очищен; установка, push, PR, merge и публикация не выполнялись и не входят в текущую доставку.
