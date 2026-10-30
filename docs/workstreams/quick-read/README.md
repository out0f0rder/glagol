# Быстрое чтение Windows

Задача пользователя 2026-10-29: чтение выделения, буфера, UIA под курсором и OCR области без открытия главного окна и сохранения в библиотеку.

Общий проверенный Silero worker, bounded PCM в памяти, cpal output. Общий operation mutex исключает параллельную обычную озвучку. UIA в отдельном процессе для принудительного таймаута чужого COM provider. Ctrl+C fallback не выполняется: безопасное сохранение всех clipboard formats не гарантируется; пользователь явно копирует текст и вызывает чтение буфера. OCR — WinRT, без облака.

A: PCM и последовательное чтение, пауза/отмена, буфер, горячие клавиши. B: изолированный Windows UIA helper. C: OCR overlay, настройки, мини-плеер, документация и NSIS. Реальные GUI/звук/DPI/offline сценарии отмечаются отдельно, автоматическая сборка их не доказывает.

P: по запросу пользователя добавлена приблизительная фонетическая подготовка английских слов на лету тем же RU голосом. Лёгкие правила + исключения в существующем Silero adapter; только Quick Read PCM, по умолчанию включено. Акронимы сохраняют spelling; настройка возвращает прежний режим. Нет смены модели/голоса, новых зависимостей, редактора пользовательского словаря. Проверки и границы качества — в log.md. Offline O отложен пользователем.

V: чтение выделения file editor VS Code 1.143.1 подтверждено пользователем после `editor.accessibilitySupport: on`. До этого UIA TextPattern возвращал collapsed range, после — непустой; исходники приложения не менялись. Настройка описана в RU/EN guides, её изменения minimap/folding и возврат `auto` отмечены. Без автоматической правки профиля, Ctrl+C и чтения всего документа.

L: необязательный HKCU Run автозапуск текущей EXE с --tray; отдельная default-off настройка start_in_tray, --tray/--hidden и --show. Main создаётся hidden/unfocused; обычный repeat открывает окно, hidden repeat ничего с ним не делает. Существующие tray/show/exit и NSIS cleanup переиспользованы; админ, plugin/dependency или автоматическое включение не нужны.

N: Notepad++8.7.5 не предоставлял TextPattern при непустом native selection. Добавлен read-only Scintilla fallback: scalar queries + OS-marshalled WM_GETTEXT, без remote memory/clipboard/keys/focus writes. Один stream selection UTF-8/SBCS, bounded prefix, recheck focus/snapshot; только выделенный UTF-16 slice. Пользователь подтвердил выбранный фрагмент на обновлённой QA копии.

M: крестик скрывает текущий mini-player generation, не меняя PCM, pause, cancellation или постоянный show-player preference. Reporter соблюдает dismissal; новое чтение снова показывает по настройке, явное включение снимает suppression. Stop остаётся отдельным действием. Фактические результаты проверок — в state/log.

T: metadata Telegram Desktop7.3.3: HistoryInner — List без TextPattern/TextPattern2; compose Inner — TextPattern с collapsed range. Переписка и поле ввода различаются. Это диагностика совместимости, не реализация нового fallback; содержимое/подписи сообщений не читались; фокус/буфер не менялись. Явный copy+V/OCR документирован; чтение целого сообщения через pointer E подтверждено пользователем. После ответа пользователя (Desktop/message) foreground metadata подтвердили отсутствие TextPattern у focused message row и HistoryInner. Содержимое не считывалось диагностикой; пользователь отдельно подтвердил слышимое чтение целого сообщения через Ctrl+Alt+E. Приложение не менялось.

U: Gmail/PDF diagnostic root cause — focus-only selection lookup. Gmail ancestor Document and PDF document under pointer expose real nonempty GetSelection ranges while focused row/toolbar does not. Added bounded ancestor lookup, then same-active-window point selection; no whole-page/paragraph fallback or synthetic copy. User confirmed selected-only Ctrl+Alt+R for both Gmail and PDF on rebuilt hidden QA candidate. Source helper budget/protection preserved; image-only PDFs may need OCR.
