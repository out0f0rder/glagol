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

## Живой аудиовыход (только при участии слушателя)

Этот opt-in тест действительно воспроизводит две публичные фразы, делает паузу
и продолжает через тот же cpal output, что Quick Read. Не включать в unattended CI.
Нужен тот же изолированный pinned runtime/model, что выше.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml native_quick_read_audio_pause_and_drain -- --ignored --nocapture
```

PASS команды подтверждает старт/опустошение cpal очереди и отсутствие ошибки
устройства. Слышимость и разборчивость подтверждает слушатель отдельно (manual).

## Воспроизводимый UIA/OCR fixture

```powershell
powershell.exe -NoProfile -STA -ExecutionPolicy Bypass -File scripts/quick-read/windows-ui-smoke.ps1
```

Окно содержит только публичный тестовый текст. Если Windows не даёт ему фокус,
пользователь обычным кликом нажимает кнопку запуска теста. Кнопка отделяет
активацию окна от автоматического SelectAll и не даёт mouse-up очистить выделение. Никаких AttachThreadInput/UIAccess,
повышения прав, SendInput или обхода foreground-защиты нет. По умолчанию тест
закрывается после проверок; `-KeepOpen` оставляет окно для ручных горячих клавиш.
`-ExePath` выбирает собранный EXE, `-OcrLanguage ru-RU|en-US` — язык OCR.
Не выводится прочитанный текст, буфер не меняется, снимки не сохраняются. Проверки:
selection, pointer без клика, поле пароля, clipboard sequence, OCR публичного поля.
Код 0 — все проверки PASS, 1 — ошибка, 2 — отсутствует OCR language component
(NOT_RUN). Другие приложения могут изменить буфер во время теста; это делает
проверку sequence неопределённой, а не доказывает дефект Glagol.

Для длинного чтения в другом редакторе используется
`scripts/quick-read/public-reading.ru.txt` (публичный RU/EN текст, не пользовательский документ).

## Английские слова русским голосом

`scripts/silero/test_worker.py` проверяет обе RU подготовки: legacy spelling и
приблизительное произношение по правилам/исключениям, смешанный Unicode, CamelCase,
акронимы, версии/единицы, числа и пунктуацию. Native RU PCM smoke использует
смешанную фразу с включённой подготовкой, затем PCM с выключенной и прежний WAV.
Ненулевой PCM не доказывает качество произношения.

Для ручной оценки выберите RU голос и прочитайте публичную фразу:
«Проверка: Quick Read works locally. OpenAI, WebView2, USB и API».
Ожидается один RU голос, приблизительные слова вместо имён всех букв, сохранение
акронимов. Отключите настройку «Читать английские слова русским голосом», запустите
чтение снова и проверьте возврат spelling. Настройка применяется со следующего
чтения, WAV документов/предпрослушивания и EN модель не меняются. Фонетика
приблизительная, особенно для имён/нерегулярных слов; словарь пользователя пока
не редактируется. Никаких новых моделей, сетевых запросов или логирования текста.

## VS Code: выделение в редакторе файла

Если видимое выделение не предоставляется через UIA, пользователь включает
`editor.accessibilitySupport: on` в VS Code, затем возвращается в файл и заново
выделяет текст. Это штатная настройка, не обход Windows security и не изменение
clipboard. Не менять чужой VS Code profile автоматически. Режим может отключать
minimap/folding; значение `auto` возвращает автоматическое определение.
[Официальная инструкция](https://code.visualstudio.com/docs/configure/accessibility/accessibility#_screen-reader-mode).

Для диагностики достаточно metadata: сфокусированный Edit/native-edit-context,
наличие TextPattern и сравнение Start/End GetSelection, без GetText/Name/Value.
Collapsed range не заменять DocumentRange или ValuePattern: иначе можно прочитать
весь документ без выделения. Этот обходной путь проверен пользователем для file
editor VS Code 1.143.1 на Windows 11; terminal, chat/webview и multi-cursor отдельно
не проверены. Если настройка неприемлема, явное Copy + Read clipboard либо OCR.

## Автозапуск и скрытый запуск

`startup` unit tests проверяют CLI policy (default/saved/explicit/repeat), quoting
Unicode/пробелов, UTF-16 лимит Windows Run, default-off preference в SQLite и
изоляцию других настроек. Windows test использует уникальный HKCU Software/Glagol/Tests
ключ, не настоящий Run; проверяет roundtrip, malformed/oversized value, idempotent
removal и сохранность соседнего значения. Он не включает login startup у пользователя.

GUI: закрыть только принадлежащий тесту кандидат; запустить CLI-built EXE с `--tray`,
проверить hidden main и неизменный foreground. Повторный `--tray` не показывает
окно, обычный повтор показывает, `--tray` при видимом окне его не прячет. Отдельно
проверить сохранённую настройку «Запускать в трее» после выхода и `--show` override.
В настройках login toggle создаёт/удаляет только продуктовый HKCU Run, не меняет
StartupApproved. Не включать автозапуск без явного действия пользователя в UI.
Проверка настоящего входа в Windows, задержки ОС, Task Manager disable и
uninstall/update — отдельная ручная приёмка, не результат unit tests.

В generated `target/release/nsis/x64/installer.nsi` убедиться, что value name
`Glagol` совпадает с PRODUCTNAME; штатное удаление Run value ограничено
`$UpdateMode <> 1`. Локальная компиляция NSIS не доказывает реальный uninstall.

## Notepad++ / Scintilla

Диагностика — ClassName=Scintilla, password=false, отсутствие TextPattern, scalar
native selection/count/mode/codepage, без GetText/Value/clipboard/focus writes.
Не выдавать это за проверку правильного текста или звука. Source fallback пробует
только exact focused Unicode HWND после UIA password ancestry. Все SCI_* запросы
скалярные; единственный text buffer идёт в WM_GETTEXT (<WM_USER, OS marshaling).
Никаких VirtualAllocEx/ReadProcessMemory, SCI_GETSELTEXT с remote pointer или
ChangeWindowMessageFilterEx. Перепроверить focus/snapshot после чтения.

Unit tests: selected-only UTF-16 с кириллицей/emoji/CRLF; rejection collapsed,
multiple/rectangle, oversized/end-outside-doc/unsupported-codepage; truncated
buffer/invalid surrogate/blank selection. Native manual: выделить смешанный
публичный пример в Notepad++8.7.5, Ctrl+Alt+R, проверить только выбранный фрагмент,
сохранность выделения/буфера. Empty и rectangular selection не должны читать весь
документ. Большой file prefix: максимум2MB byte offset/500k UTF-16; сообщение
предлагает explicit copy/read clipboard или OCR. UTF-8/SBCS; DBCS отдельно не поддержан.
Все calls150ms, original child hard3s; нет исключений для повышенного процесса.

## Скрытие мини-плеера

Крестик вызывает quick_read_hide_player; suppression хранится по generation, отдельно
от постоянного show-player. Reporter не должен показывать то же поколение снова.
Новый запрос возвращает обычную видимость; явное включение show-player снимает
suppression. Никаких replace/stop/pause из hide. Регрессия проверяет продвижение
PCM, сохранение pause/cancellation generation и видимость при повторных updates.
Manual: длинный текст → × → речь продолжается, окно скрыто; Ctrl+Alt+P и S работают;
новый R показывает окно по настройке без активации. Звук/GUI не выводятся из unit PASS.
