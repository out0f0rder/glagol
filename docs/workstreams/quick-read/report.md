# Quick Read — локальный результат 2026-10-30

Ветка `feat/quick-read`; фонетика local `91a2d99`, запуск local `aaa6b34`,
Notepad++ local `0135dfd`; скрытие мини-плеера — local `fc44db3`.
Это исходники 0.6.1 + Quick Read, не опубликованный релиз v0.6.1.
Установка и публикация не выполнялись.

## Реализация

| Модули | Изменения |
|---|---|
| `src-tauri/src/quick_read` | Отдельные источники, нормализация/фрагменты, поколения отмены, bounded PCM/cpal, настройки, мини-плеер и OCR overlay |
| `quick_read/sources` | UIA selection и ElementFromPoint, read-only Unicode clipboard, GDI/WinRT OCR; отдельный same-EXE процесс с таймаутом и COM MTA; read-only Scintilla selection для Notepad++ |
| `tts/silero/worker.rs`, `worker.py` | Общий Silero worker с bounded binary PCM без WAV; приблизительное English→Cyrillic для RU Quick Read, настройки и сохранение прежнего WAV/EN пути |
| `startup`, `StartupSection.tsx` | Два opt-in режима запуска, HKCU Run текущей EXE, `--tray` / `--show`, работа single-instance |
| `lib.rs`, `main.rs`, capabilities | Запуск/завершение, exact shortcut dispatch, hidden child и отдельные webview |
| `components/settings/QuickReadSection.tsx`, `components/quick-read`, typed IPC/i18n | Шесть настраиваемых сочетаний, язык OCR, переключатель фонетики, необязательный mini-player без автоматического фокуса |
| README RU/EN, USER_GUIDE RU/EN, CHANGELOG, runbooks, scripts | Инструкции, ограничения, unit/native и attended UIA/OCR/audio проверки |

По умолчанию: Ctrl+Alt+R выделение, V буфер, E наведение, O OCR, P пауза, S стоп.
Чтение идёт после первого готового фрагмента, очередь максимум два, новое чтение
отменяет прежнее. Пауза сохраняет PCM. В библиотеке записей нет. Мини-плеер
выключен по умолчанию, даёт паузу/стоп/переходы и скорость 0,5–2×. Крестик
скрывает окно только текущего чтения; PCM, пауза и настройка показа сохраняются.
Следующее чтение показывает окно по настройке. ■ и Ctrl+Alt+S останавливают звук.

Английские слова внутри RU Quick Read приблизительно читаются тем же русским
голосом на лету, без новых моделей/зависимостей. Правила работают и для неизвестных
словарю слов; исключения помогают частым нерегулярным словам. Акронимы сохраняют
побуквенное чтение. Настройка возвращает legacy spelling со следующего чтения.

## Проверки

Windows 11 build 26200.9711, Rust/MSVC 1.99.1; frontend WSL Node24.13.0/pnpm12.16.1.
Фактические команды/даты/коды и границы — [журнал](log.md).

- Rust fmt/clippy `--all-targets -- -D warnings`: PASS; cargo test: 368 passed,
  8 ignored (ignored не выдаются за выполненные).
- TypeScript, 290 i18n keys и Vite production build (3m53s): PASS. Осталось обычное
  предупреждение размера JS bundle >500kB; гейты не ослаблены.
- Ранее, 2026-10-29 (в M не повторялись): Python adapter 11 tests PASS.
  Native RU mixed PCM, фонетика on/off,
  PCM↔WAV/cancel/recovery: PASS. Native EN four voices/library WAV/recovery: PASS.
- Гейты: 49 runbook tests, 3 i18n tests и runbook-check 14 series: PASS.
- NSIS через Tauri CLI: PASS (release43.64s). Обновлённая direct-run EXE
  скрыто запущена, readiness наблюдалась; это не замер hotkey→звук.
- Пользователь вручную подтвердил на предыдущем CLI кандидате выделение
  Chrome/Notepad, clipboard/pointer/OCR/Escape, pause/stop/replace без наложения,
  mini-player без смены фокуса, конфликт/переназначение, библиотеку и диктовку.
  Слышимость двух публичных RU фраз подтверждена отдельно для настоящего cpal.
  На новом кандидате 91a2d99 пользователь подтвердил хорошее звучание смешанного
  публичного примера тем же RU голосом и возврат чтения по буквам при выключении
  настройки: manual PASS для этого примера и переключения.
- Для M пользователь подтвердил: × скрывает окно, речь продолжается; P/S
  работают, новое чтение показывает по настройке. Два unit regression tests
  отдельно проверяют сохранение PCM/позиции/паузы и отсутствие повторного показа.

## Запуск в трей и автозапуск

Добавлены два default-off переключателя: вход в Windows (всегда в трей) и обычный
запуск в трее. `--tray`/`--hidden` скрывают первый запуск, `--show` открывает окно.
Normal repeat открывает работающую копию, hidden repeat не меняет её видимость.
Автозапуск использует HKCU Run текущей EXE, без администратора; существующий NSIS
сохраняет его в update mode и удаляет при обычном uninstall. Нативный GUI smoke
проверил hidden/no activation, оба repeat-сценария и --show. Пользователь
подтвердил видимость, изменение и сохранение обоих переключателей при повторном
открытии Settings. Настоящий login, OS disable и uninstall остаются NOT_RUN.

## Notepad++

Notepad++8.7.5 не предоставлял UIA TextPattern при непустом выделении. Добавлен
Windows-only fallback для точного focused Unicode Scintilla HWND после password
ancestry: scalar SCI queries и OS-marshalled WM_GETTEXT, затем только selected
UTF-16 slice. Не меняет clipboard/фокус/выделение, не использует remote memory или
повышение прав. Пользователь подтвердил чтение только выбранного фрагмента.
Поддержаны один stream selection и UTF-8/SBCS; rectangle/multiple/DBCS либо prefix
за пределом 2MB bytes/500k UTF-16 требуют явного copy/read clipboard или OCR.

## Артефакт и ограничения

Установщик M: `.scratch/quick-read-dist/Glagol_0.6.1_x64-setup.exe`,
10 404 735 байт; SHA-256 `cf92cc655d3a2719365bcc42bdfa0b6eadbc7154b5cab8421e761a95c2e179d7`.
Собран 2026-10-30T09:27:04.029932+00:00; frontend/NSIS и ручная проверка крестика PASS.
Рядом `SHA256SUMS` и `build.json` с привязкой к исходникам. Сборка не подписана
для updater, не установлена и не опубликована. Тестовая EXE обновлена и запущена.

VS Code file editor 1.143.1 потребовал `editor.accessibilitySupport: on`:
пользователь подтвердил чтение после включения и повторного выделения. Глагол
не меняет профиль редактора. В режиме доступности могут отключаться minimap/folding;
возврат — `auto`. Инструкции добавлены в обе части README и обе USER_GUIDE.

Ctrl+C fallback не эмулируется: безопасное восстановление всех clipboard formats
не гарантируется; при недоступной UIA нужно явное копирование либо OCR. UIA имеет
таймаут 3с и не обходит защищённые/повышенные окна. OCR требует выбранного
установленного RU/EN компонента Windows, максимум 15с, один монитор за выделение,
4096px/сторона и 16MP. Скриншотов и текста в логах нет.

Переходы идут по фрагментам (длинное предложение может делиться), целевой фрагмент
синтезируется заново. Скорость меняет и высоту голоса. Холодной модели нужно время
для загрузки. Английское произношение приблизительно; короткое неизвестное слово
в верхнем регистре может считаться акронимом. Редактора словаря пока нет.

Оставшиеся ручные проверки на настоящем Windows 11: offline (пользователь отложил),
другие DPI/несколько мониторов (сейчас один 125%),
защищённые повышенные процессы, сложные clipboard formats, остальная расширенная
[матрица](manual-qa.md). Hotkey→слышимый звук не измерялся. Positive run переносимого
UIA fixture с новой кнопкой ещё не выполнен; parser и три negative-input проверки
прошли, старый fixture после DPI fix прошёл UIA/password/clipboard/OCR. Это разные результаты.
