# Glagol

Offline English and Russian dictation and text-to-speech for Windows 10/11 x64.
Диктовка и озвучка на английском и русском для Windows 10/11 x64.

[English guide](USER_GUIDE.en.md) · [Руководство](USER_GUIDE.ru.md) · [MIT application](LICENSE) · [Security](SECURITY.md)

**Download Glagol 0.6.1 / Скачать Глагол 0.6.1:**
[Windows x64 installer](https://github.com/dimasiksuleyman-sudo/glagol/releases/download/v0.6.1/Glagol_0.6.1_x64-setup.exe)
([release notes and SHA-256](docs/releases/v0.6.1.md), 10,263,535 bytes). From 0.6.0, later versions install from Settings → Updates. / С 0.6.0 следующие версии ставятся из «Настройки» → «Обновления».

## English

### See Glagol

**Turn text into a recording.** English interface and English speech, with a
demonstration text ready to synthesize.

![Glagol 0.5.0: English text-to-speech with voice selection](docs/screenshots/0.5.0/synthesize-en.png)

**Dictate offline in English.** Select Moonshine Small Streaming; downloaded
models stay on your computer.

![Glagol 0.5.0: local English dictation with Moonshine Small Streaming](docs/screenshots/0.5.0/dictation-en.png)

**Keep recordings in your library.** Resume playback, adjust the speed and export
audio as WAV.

![Glagol 0.5.0: English library with a demonstration recording and audio player](docs/screenshots/0.5.0/library-en.png)

[View all 0.5.0 screenshots — synthesis, dictation and library, including Russian](docs/screenshots/0.5.0/README.md).

### Languages and speech

Choose **English / Русский** on first launch, including the first upgrade to 0.5.0.
Interface, dictation and synthesis languages are independent. Change the interface
in the app shell or Settings without restarting; change speech languages on their
own pages. Each language remembers its model and voice. New installs start with
local dictation. Upgrades retain modes, server addresses, credentials and voices.

- Hold `Ctrl+Shift+Space`, wait for the recording signal, speak, release, then receive
  the final text once. Auto-paste or clipboard only; optional history is off by default.
- English local dictation: Moonshine Small Streaming, native CPU runtime. Russian:
  GigaAM v3 CTC/RNNT. Independent office-server and cloud profiles remain available.
- Optional offline Silero synthesis: v3_en with EN 0–3 (default EN 0), and v5.5 RU
  with Aidar, Baya, Kseniya, Xenia and Eugene. Voice preview, progress and cancellation.
- Paste text or import TXT/MD/DOCX/PDF. Long documents use a sequential WAV pipeline.
  Library, rename, localized player, 0.5–2× playback, WAV export and backups.
  Existing recordings and their metadata remain usable.

### Login startup and starting in the tray

In the local development build (not the published v0.6.1), Settings → Startup
provides two switches, both off by default:

- **Launch when signing in to Windows** starts this copy of Glagol in the tray
  when you sign in, without the main window. No administrator rights are needed.
- **Start in the tray** hides the main window on the next ordinary launch.
  It does not change the current window; Windows login startup is always hidden.

Open the window through Show Glagol in the tray menu or an ordinary repeat launch.
A shortcut can append `--tray` (also `--hidden`) to the EXE path for hidden startup,
or `--show` to open the window regardless of the preference. Repeating `--tray`
never hides or activates an existing window. Global shortcuts keep working in the tray.

Login registration uses this copy's path: enable it again after moving the app.
Settings indicate an entry pointing to another copy. Windows can delay startup
or disable it in Task Manager → Startup apps; Glagol does not override that choice.
Normal NSIS uninstallation removes the entry; update mode preserves it. Closing
the window keeps Glagol in the tray; Quit exits the application.

### Quick Read (local development build)

Quick Read is new in the source tree and is **not included in the linked v0.6.1 release**.
Install and acknowledge the optional Silero component, choose Russian or English
and its voice in Settings → Local text to speech, then leave Glagol running in the
tray. Settings → Quick Read changes shortcuts, OCR language and the optional
mini-player (off by default).

| Action | Default shortcut |
|---|---|
| Read selection | Ctrl+Alt+R |
| Read clipboard | Ctrl+Alt+V |
| Read element under pointer | Ctrl+Alt+E |
| Select screen area and read with OCR | Ctrl+Alt+O |
| Pause / resume | Ctrl+Alt+P |
| Stop | Ctrl+Alt+S |

Reading starts after the first sentence fragment is synthesized, while later
fragments are prepared in a queue of at most two. PCM stays in memory; no WAV or
library entry is created. A new request replaces the previous reading. Pause
retains prepared audio. The mini-player adds previous/next fragment and 0.5–2×
speed; changing speed also changes pitch. Long sentences can span several
fragments; jumping synthesizes the target again. Errors appear briefly without
opening the main window. A cold Silero worker still needs time to load.

The × button hides the mini-player for the current reading; speech and pause
position are preserved. Use ■ or Ctrl+Alt+S to stop. A new reading shows it again
if enabled in Settings. To restore it during the same reading, turn “Show
mini-player” off and on in Settings; closing it does not change that preference.

Russian Quick Read approximates English words on the fly using the same selected
Russian voice: `Quick Read` → «квик рид». Lightweight rules and built-in exceptions
need no downloads; names and irregular words may sound inaccurate. Abbreviations
such as USB/API keep letter names. Turn off “Read English words with the Russian
voice” in Settings → Quick Read to restore spelling; the change applies to the
next reading. The English model and regular document synthesis keep their
existing text processing.

Selection first uses the application's UI Automation support. No automatic Ctrl+C
fallback is used: when selection is unavailable, copy explicitly and use
Ctrl+Alt+V. Clipboard formats and the user's selection are never modified by
Quick Read. Password controls are excluded; protected/elevated applications may
be inaccessible. Pointer lookup uses physical screen coordinates on scaled displays.
UIA extraction has a three-second timeout; Glagol cannot
provide text that an application does not expose.

Selection checks a chain of up to 16 elements, starting with the focused control
and stopping at the window. If they expose no selection, Glagol also checks the element under
the pointer and its ancestors within the same active window. Only selected
ranges are read, never the whole page. This handles Gmail's focused message row
and a PDF toolbar retaining focus while text is selected in the document. Keep
the pointer over the selected text in such PDFs. An image-only PDF without an
accessible text layer requires OCR; explicit copy + `Ctrl+Alt+V` is another path.

**Notepad++:** an ordinary single selection uses a read-only Scintilla fallback
when UI Automation does not expose text. UTF-8 and single-byte modes are supported;
focus, selection and clipboard stay unchanged. For rectangular/multiple selections,
other encodings or a range far into a large file, copy explicitly and use
`Ctrl+Alt+V` or OCR. Direct reading is limited to a selection end within 2 MB and a
prefix of at most 500,000 UTF-16 units. Only the selected range is spoken; the rest
of the bounded prefix is discarded.

**Telegram Desktop:** message text selection may be unavailable to `Ctrl+Alt+R`.
In 7.3.3 on Windows, the message-history control exposed a list without
TextPattern/TextPattern2; its whole-message selection is not a selected text
range. Copy the desired fragment explicitly, then press `Ctrl+Alt+V`, or use
`Ctrl+Alt+O` for an area. For a whole message, point at it and press `Ctrl+Alt+E`;
a user confirmed this in Telegram Desktop 7.3.3 on Windows 11. The compose box is
a different control with TextPattern support. The missing patterns were also
confirmed with Telegram active, without reading message contents.

**VS Code file editor:** if a visible selection produces “No readable text”,
open VS Code Settings (`Ctrl+,`), find `editor.accessibilitySupport` and select
`on`. Return to the file, select the text again and press `Ctrl+Alt+R`.
A user confirmed this with VS Code 1.143.1 on Windows 11: UIA exposed an empty
selection before the change, and reading worked afterward. Screen reader mode
can disable the minimap and code folding; choose `auto` to restore automatic
mode. Glagol does not change VS Code settings. If this mode is unsuitable, copy
explicitly and use `Ctrl+Alt+V` or OCR.
[VS Code screen reader mode](https://code.visualstudio.com/docs/configure/accessibility/accessibility#_screen-reader-mode).

OCR is local Windows.Media.Ocr. Choose **Русский / English** in Quick Read
settings and install that Windows OCR language component; a missing component
produces an error, with no silent language substitution. Drag a rectangle on
the monitor under the pointer; Escape cancels. DPI is converted to physical
pixels, including monitors with negative origins. A region is limited to one
monitor, 4096 pixels per side and 16 megapixels. OCR takes at most 15 seconds.
No cloud service, screenshot file or text log is created. Silero and its licence
requirements are the same as document synthesis; no new download occurs while
reading. Another active TTS setup/document operation is reported as busy.

Build and unit tests do not establish GUI, OCR accuracy, audio quality or measured
hotkey-to-sound latency. Windows 11 acceptance scenarios are tracked in
[manual QA](docs/workstreams/quick-read/manual-qa.md).

### Install only the components you need

One compact installer, with English/Russian installation and removal. The language
selector starts from Windows' language, with English fallback. No speech weights,
speech ONNX runtime, Python/PyTorch or external pronunciation data are bundled.
First-run speech setup can be skipped. Choosing a language never downloads files
or accepts a model license.

| Optional component | First download | With its shared runtime already installed |
|---|---:|---:|
| English dictation | 158.8 MB | 142.3 MB model |
| Russian dictation | 292.2–293.8 MB | 272.2–273.7 MB model |
| English Silero TTS | 306.5 MB | **57.2 MB model** |
| Russian Silero TTS | 394.8 MB | 145.4 MB model |

Sizes use decimal MB; the app accounts for installed files. Both Silero languages
share one 249.3 MB Python/PyTorch download. Allow 1.9 GB for initial Silero installation
and staging. No system Python, pip, CUDA or SAPI setup. Downloads support cancellation,
resume and integrity checks. Importing the exact model file is available when the
Silero server cannot be reached. Removing one model retains the shared runtime.

Silero retains a full-verification receipt for 30 days; changed key files, an expired
receipt or worker failure trigger a full check. Workers stay warm and unload after
15 idle minutes. Installed local speech works offline. Speech languages are selected
explicitly: mixed-language detection and translation are not provided locally.

### Licenses and data

Glagol is free and MIT licensed. Dictation can be used in organizations, subject to
each model/service's terms. Moonshine English streaming and GigaAM models are MIT.
**Both Russian and English Silero models are optional, noncommercial CC BY-NC-SA 4.0
components**, by Silero Team. Separate downloads do not waive those terms.
[Silero license](docs/third-party/Silero-LICENSE.txt) · [Moonshine notices](docs/third-party/Moonshine-LICENSE.txt).
Glagol is independent of its model and service providers. Cloud services have their
own terms and charges; no commercial TTS provider is integrated in this version.

Data stays in `%LOCALAPPDATA%\app.glagol.desktop\`: `glagol.db`, `audio_cache`,
`speech_models`, `tts_models`. Library backups exclude models, runtime, OS credentials
and Silero consent. User text, document names and old history are never translated
when changing the interface language.

### Development and validation

Tauri 2, Rust, React 19, TypeScript, SQLite. `pnpm install --frozen-lockfile`,
`pnpm tauri build`. [Architecture](PROJECT_STRUCTURE.md), [contributing](CONTRIBUTING.md),
[Windows checks](docs/runbooks/windows-build.md), [TTS runtime](docs/local-tts-runtime.md),
[STT runtime](docs/local-dictation-runtime.md), [changelog](CHANGELOG.md).
Current implementation evidence and unperformed manual checks are recorded in the
[0.5.0 workstream](docs/workstreams/english-first/log.md). Synthetic audio tests are
not a listening-quality result. Direct Silero-origin delivery was unavailable from
the development machine; importing pinned files was exercised separately.

## Русский

### Как выглядит Глагол

**Превратите текст в запись.** Русский интерфейс и русская озвучка:
демонстрационная запись сохранена в библиотеку и доступна для экспорта.

![Глагол 0.5.0: русская озвучка текста с выбором голоса](docs/screenshots/0.5.0/synthesize-ru.png)

**Диктуйте на английском без интернета.** Выберите Moonshine Small Streaming;
скачанные модели остаются на компьютере. **Храните записи в библиотеке:**
продолжайте прослушивание, меняйте скорость и экспортируйте WAV.
Английские экраны диктовки и библиотеки показаны выше.

[Все скриншоты 0.5.0 — озвучка, диктовка и библиотека, включая русский интерфейс](docs/screenshots/0.5.0/README.md).

### Языки и речь

При первом запуске, включая первое обновление до 0.5.0, выберите **English / Русский**.
Языки интерфейса, диктовки и озвучки независимы. Интерфейс переключается в оболочке
или настройках без перезапуска; речь — на своей странице. Для каждого языка
сохраняются модель и голос. Новая установка начинает с локальной диктовки;
обновление сохраняет режимы, адреса серверов, ключи и голоса.

- Удерживайте `Ctrl+Shift+Space`, дождитесь сигнала записи, говорите и отпустите:
  готовый текст вставится один раз. Доступен режим буфера; история изначально выключена.
- Английская локальная диктовка: Moonshine Small Streaming, нативный CPU-runtime.
  Русская: GigaAM v3 CTC/RNNT. Сохраняются отдельные офисный и облачный профили.
- Необязательная офлайн-озвучка Silero: v3_en с EN 0–3 (по умолчанию EN 0) и
  v5.5 RU с Айдаром, Баей, Ксенией, Xenia и Евгением. Есть образец голоса, прогресс и отмена.
- Текст или TXT/MD/DOCX/PDF; последовательная озвучка длинных документов в WAV.
  Библиотека, переименование, локализованный плеер, скорость 0.5–2×, экспорт и бэкапы.
  Старые записи и метаданные остаются доступны.

### Автозапуск и запуск в трей

В локальной сборке разработки (не в опубликованном v0.6.1) в «Настройки» →
«Запуск приложения» доступны два выключенных по умолчанию переключателя:

- **Запускать при входе в Windows** — запускать текущую копию Глагола в трее
  при входе в учётную запись, без главного окна. Права администратора не нужны.
- **Запускать в трее** — скрывать главное окно при следующем обычном запуске.
  Текущее окно не меняется; автозапуск Windows всегда скрытый независимо от этой настройки.

Открыть окно можно через меню трея «Показать Glagol» или обычным повторным запуском.
В ярлыке можно добавить к пути EXE `--tray` (также `--hidden`) для скрытого запуска
или `--show`, чтобы открыть окно независимо от настройки. Повторный `--tray` не
скрывает и не активирует уже запущенное окно. Горячие клавиши работают из трея.

Автозапуск запоминает путь этой копии: после перемещения включите его заново.
Если запись ведёт к другой копии, настройки это показывают. Windows может задержать
запуск или отключить его в «Диспетчер задач → Автозагрузка приложений»; Глагол не
обходит это отключение. Штатное удаление NSIS убирает запись автозапуска, режим
обновления её сохраняет. Закрытие окна оставляет приложение в трее; «Выход» завершает его.

### Быстрое чтение (локальная сборка разработки)

Быстрое чтение добавлено в исходники и **не входит в опубликованный v0.6.1 по ссылке выше**.
Установите необязательный Silero, подтвердите его условия, выберите русский или
английский язык и голос в «Настройки» → «Локальная озвучка», затем оставьте Глагол
в трее. «Настройки» → «Быстрое чтение» меняет сочетания, язык OCR и необязательный
мини-плеер (по умолчанию выключен).

| Действие | По умолчанию |
|---|---|
| Прочитать выделение | Ctrl+Alt+R |
| Прочитать буфер обмена | Ctrl+Alt+V |
| Прочитать элемент под курсором | Ctrl+Alt+E |
| Выделить область и прочитать через OCR | Ctrl+Alt+O |
| Пауза / продолжение | Ctrl+Alt+P |
| Остановка | Ctrl+Alt+S |

Чтение начинается после синтеза первого фрагмента предложения; следующие
готовятся в очереди максимум из двух. PCM остаётся в памяти; WAV и запись в
библиотеке не создаются. Новый запрос заменяет прежнее чтение. Пауза сохраняет
подготовленное аудио. В мини-плеере доступны предыдущий/следующий фрагмент и
скорость 0,5–2×; изменение скорости меняет и высоту голоса. Длинное предложение
может занимать несколько фрагментов; переход синтезирует целевой фрагмент заново.
Ошибки появляются ненадолго без открытия главного окна. Холодному движку Silero
нужно время на загрузку.

Кнопка × скрывает мини-плеер на время текущего чтения; речь и позиция паузы
сохраняются. Для остановки используйте ■ или Ctrl+Alt+S. При новом чтении плеер
снова появляется, если включён в настройках. Чтобы вернуть его во время того же
чтения, выключите и включите «Показывать мини-плеер» в настройках; крестик эту
настройку не меняет.

В русском быстром чтении английские слова по умолчанию получают приблизительное
русское произношение на лету: `Quick Read` → «квик рид», тем же выбранным голосом.
Используются лёгкие правила и встроенные исключения без скачиваний; произношение
имён и нерегулярных слов может быть неточным. Аббревиатуры вроде USB/API читаются
по буквам. В «Настройки» → «Быстрое чтение» выключите «Читать английские слова
русским голосом», чтобы вернуть побуквенное чтение; изменение действует со
следующего запуска чтения. Английская модель и обычная озвучка документов
сохраняют прежнюю обработку текста.

Сначала выделение запрашивается через UI Automation приложения. Автоматического
Ctrl+C нет: при недоступном выделении скопируйте текст явно и нажмите Ctrl+Alt+V.
Быстрое чтение не меняет форматы буфера и выделение пользователя. Поля паролей
исключены; защищённые/повышенные приложения могут быть недоступны. Наведение использует физические
координаты указателя при масштабировании экрана. На извлечение
UIA отведены три секунды; Глагол не может получить текст, который приложение не
предоставляет.

Глагол проверяет цепочку до 16 элементов от элемента в фокусе до границы окна.
Если выделения там нет, проверяет элемент под указателем и его предков в том же
активном окне. Читаются только выделенные диапазоны, а не вся страница. Это
учитывает фокус на строке письма Gmail и на панели PDF при выделении в документе.
В таком PDF оставьте указатель над выделенным текстом. Для скана без доступного
текстового слоя нужен OCR; другой путь — явное копирование + `Ctrl+Alt+V`.

**Notepad++:** обычное одиночное выделение читается через отдельный read-only
адаптер Scintilla, когда UI Automation не предоставляет текст. Поддержаны UTF-8
и однобайтовые режимы; фокус, выделение и буфер не меняются. Для прямоугольного или
множественного выделения, других кодировок и области далеко от начала большого
файла используйте явное копирование + `Ctrl+Alt+V` либо OCR. Прямой путь ограничен
позицией конца до 2 МБ и префиксом до 500 000 UTF-16 единиц. Из ограниченного
префикса озвучивается только выделенный диапазон; остальной текст отбрасывается.

**Telegram Desktop:** выделение текста сообщения может быть недоступно для
`Ctrl+Alt+R`. В 7.3.3 на Windows область переписки предоставляла список без
TextPattern/TextPattern2; выбор целых сообщений не является диапазоном выделенного
текста. Скопируйте нужный фрагмент явно и нажмите `Ctrl+Alt+V` либо используйте
`Ctrl+Alt+O` для области. Для целого сообщения наведите указатель и нажмите `Ctrl+Alt+E`;
это подтвердил пользователь в Telegram Desktop 7.3.3 на Windows 11. Поле ввода —
другой элемент с поддержкой TextPattern. Отсутствие нужных интерфейсов
подтверждено и в активном Telegram, без чтения содержимого сообщений.

**VS Code, редактор файла:** если при видимом выделении появляется «Текст не найден»,
откройте настройки VS Code (`Ctrl+,`), найдите `editor.accessibilitySupport` и
выберите `on`. Вернитесь в файл, заново выделите текст и нажмите `Ctrl+Alt+R`.
В VS Code 1.143.1 на Windows 11 это подтвердил пользователь: в обычном режиме UIA
сообщал пустое выделение, после включения чтение заработало. Режим доступности
может отключать мини-карту и сворачивание блоков; вернуть автоматический режим
можно значением `auto`. Глагол сам настройки VS Code не меняет. Если этот режим
не подходит, скопируйте текст явно и используйте `Ctrl+Alt+V` либо OCR.
[Режим доступности VS Code](https://code.visualstudio.com/docs/configure/accessibility/accessibility#_screen-reader-mode).

OCR локальный, через Windows.Media.Ocr. Выберите **Русский / English** в настройках
быстрого чтения и установите соответствующий компонент OCR Windows; отсутствие
компонента даёт ошибку без молчаливой подмены языка. Выделите прямоугольник на
мониторе под курсором; Escape отменяет. DPI переводится в физические пиксели,
учитываются мониторы с отрицательными координатами. Область ограничена одним
монитором, 4096 пикселями по стороне и 16 мегапикселями. OCR занимает не более
15 секунд. Нет облака, файлов снимков и логирования текста. Silero и его условия
те же, что у озвучки документов; при чтении ничего не скачивается. Другая активная
операция установки/озвучки сообщается как занятая.

Сборка и unit-тесты не доказывают GUI, точность OCR, качество звука и измеренную
задержку от горячей клавиши. Сценарии приёмки Windows 11 — в
[ручной QA](docs/workstreams/quick-read/manual-qa.md).

### Только нужные компоненты

Один компактный установщик, установка и удаление на EN/RU. Начальный язык — по
Windows, резервный — английский. В установщике нет речевых весов, речевого ONNX-runtime,
Python/PyTorch и внешних словарей произношения. Начальную настройку речи можно пропустить.
Выбор языка сам ничего не скачивает и не принимает лицензию.

| Необязательный компонент | Первая загрузка | Общий runtime уже установлен |
|---|---:|---:|
| Английская диктовка | 158,8 МБ | 142,3 МБ модель |
| Русская диктовка | 292,2–293,8 МБ | 272,2–273,7 МБ модель |
| Английская Silero | 306,5 МБ | **57,2 МБ модель** |
| Русская Silero | 394,8 МБ | 145,4 МБ модель |

МБ — десятичные; приложение учитывает установленные файлы. У обоих языков Silero
один runtime Python/PyTorch, 249,3 МБ загрузки. Для первой установки и временных
файлов освободите 1,9 ГБ. Системные Python, pip, CUDA и SAPI не нужны.
Есть отмена, продолжение и проверка загрузки, импорт точного файла модели при
недоступности сервера Silero. Удаление одной модели сохраняет общий runtime.

Отметка полной проверки Silero действует 30 дней; истечение срока, изменение
ключевых файлов или ошибка worker вызывают полный контроль. Прогретый worker
выгружается через 15 минут простоя. Установленная локальная речь работает без сети.
Язык выбирается явно; локального автоопределения смешанной речи и перевода нет.

### Лицензии и данные

Глагол бесплатен, код — MIT. Диктовка доступна организациям с учётом условий
выбранной модели/сервиса. Moonshine English streaming и GigaAM — MIT.
**Обе модели Silero, русская и английская, — необязательные компоненты для
некоммерческого использования под CC BY-NC-SA 4.0**, Silero Team. Раздельная
загрузка не отменяет условий. [Лицензия](docs/third-party/Silero-LICENSE.txt),
[уведомления Moonshine](docs/third-party/Moonshine-LICENSE.txt). Глагол независим
от поставщиков. У облачных сервисов свои условия и тарифы; коммерческого TTS
в этой версии нет.

Данные: `%LOCALAPPDATA%\app.glagol.desktop\`: `glagol.db`, `audio_cache`,
`speech_models`, `tts_models`. Бэкап библиотеки не переносит модели, runtime,
OS-ключи и согласие Silero. При смене интерфейса тексты пользователя, названия
документов и прежняя история не переводятся.

### Разработка и проверка

Текущая версия — **0.6.1**. Установщик по ссылке сверху — 10 263 535 байт.
Стек: Tauri 2, Rust,
React 19, TypeScript, SQLite. `pnpm install --frozen-lockfile`, `pnpm tauri build`.
[Архитектура](PROJECT_STRUCTURE.md), [вклад](CONTRIBUTING.md),
[проверки Windows](docs/runbooks/windows-build.md), [TTS runtime](docs/local-tts-runtime.ru.md),
[STT runtime](docs/local-dictation-runtime.md), [изменения](CHANGELOG.md).
Результаты 0.5.0 и недоступные ручные проверки — в [журнале](docs/workstreams/english-first/log.md).
Синтетические WAV не подтверждают качество на слух. Прямой сервер Silero с
машины разработки был недоступен; импорт закреплённых файлов проверен отдельно.

Available for contract work / Доступен для контрактной работы — Rust/Tauri,
voice/TTS/LLM applications: `kiss2tri@hotmail.com`.
