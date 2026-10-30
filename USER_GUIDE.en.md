# Glagol user guide — 0.6.1

[Русский](USER_GUIDE.ru.md) · [README](README.md)

Glagol reads English and Russian documents aloud and turns dictation into text.
This guide describes [Glagol 0.6.1](docs/releases/v0.6.1.md).
The GitHub link at the bottom of the sidebar opens the author's profile in your browser.

## Installation

Windows 10/11 x64 is required. Run the locally built `Glagol_0.6.1_x64-setup.exe`.
The installer offers English and Russian, starts with Windows' language and uses
English as fallback. The unsigned installer may trigger SmartScreen: check the
artifact's source and SHA-256 before choosing More info → Run anyway. Installation
and removal do not download speech models or accept Silero's terms.

On first launch choose **English / Русский** using the large buttons. Your choice
is saved immediately. Then choose dictation and synthesis independently on two
speech cards. Install either, both or neither; Continue opens the app without
requiring a download. Closing and reopening resumes the saved setup step. With no
network you can still skip setup and use the library or existing local components.

On a new installation, both speech languages initially match the chosen interface;
dictation starts in local mode. Changing the interface later changes neither speech
language. Switch interface language in the shell or Settings without restarting.

## Updating an existing installation

From 0.6.0 the current version is shown in the sidebar and in Settings → Updates.
Check for updates asks GitHub whether a newer version exists. Check automatically at
startup is off by default; when on, Glagol checks once after it starts and shows a
notification about a new version. While it is off, a reminder with Check now and
Enable auto-check buttons appears at startup at most once a month; a manual check
postpones it. Install update shows what is new in that version (from 0.6.1) and,
after confirmation, downloads the
installer, verifies its digital signature, closes Glagol, installs the new version and
starts it again; documents, settings, keys and speech components are kept. Installation
does not start during dictation or a speech operation. A file with an invalid signature
is never installed. Versions before 0.6.0 update manually as described below.

Back up your library, exit through the tray, run the new installer and retain the
existing directory (choose “Do not uninstall” on the existing-installation page).
The first upgrade from 0.4.1 also asks for interface language. Existing local/server/cloud
modes, models, voices, addresses and credentials are retained, including old implicit
Russian/cloud settings. The speech cards show the saved configuration. A Russian
voice previously kept in the webview is transferred once into saved preferences.

Existing WAVs need no installed TTS engine to play. Old document metadata is preserved;
new synthesis stores its speech language. Restoring an older backup remains supported.
Installation, live-microphone and listening checks of this candidate remain separate
from automated tests; see the [work log](docs/workstreams/english-first/log.md).
[Current 0.5.0 screenshots](docs/screenshots/0.5.0/README.md) show English synthesis, local
dictation and the library, plus completed synthesis in the Russian interface. Older [0.4.0 screenshots](docs/screenshots/windows10-0.4.0/README.md) are historical.

## Optional Silero synthesis

Both Silero **v3_en (English)** and **v5.5 (Russian)** are noncommercial components
under [CC BY-NC-SA 4.0](docs/third-party/Silero-LICENSE.txt), by Silero Team. Glagol
itself is MIT licensed; dictation does not require Silero. Cloud providers have their
own terms. This version has no commercial TTS provider or SaluteSpeech integration.

1. Choose the synthesis language on Synthesize or in Settings.
2. Read and explicitly acknowledge the selected model's license.
3. Download and enable, or select the exact previously downloaded model file.
4. Pick a voice and preview it. English offers EN 0, EN 1, EN 2, EN 3 (default EN 0).
   Russian offers Aidar, Baya, Kseniya, Xenia and Eugene. Numerical English voice
   IDs do not imply a guaranteed gender or US/UK accent.

English model: **57.2 MB** (`v3_en.pt`); Russian: **145.4 MB** (`v5_5_ru.pt`).
Both use **one 249.3 MB runtime**. First English install downloads about 306.5 MB;
first Russian install 394.8 MB. With valid RU Silero installed, adding English needs
only its 57.2 MB model. The app accounts for existing files. Initial setup/staging
needs 1.9 GB free. No system Python, pip, CUDA or SAPI registration is required.

Changing a language does not install anything. Interrupted downloads can resume;
size and SHA-256 must match before activation. Repair checks damaged components.
If the model server is unreachable, import the exact pinned file; arbitrary `.pt`
files are rejected. Removing a language model removes its acknowledgement but keeps
the other model, shared runtime and library. Conditions are acknowledged per model.

A full-verification receipt lasts 30 days. Expiry, changes to key files or a worker
failure trigger full verification. Opening Synthesize preloads the selected model;
status shows preparation. Warm models unload after 15 idle minutes or on exit.
Installed local synthesis works offline. This is process isolation, not an OS sandbox.

## Reading a document

Paste text or choose TXT, Markdown, DOCX or text-based PDF, then select a voice and
Synthesize and save to library. Input files are limited to 10 MB and 500,000 extracted
characters. Scanned PDFs need external OCR. Text stays in the editor when switching
languages. Language/model changes are disabled during the corresponding operation.

Long text is split and synthesized sequentially with progress and cancellation.
Cancellation does not publish a partial recording as a completed library document.
English text uses English number/abbreviation handling; Russian retains its existing
stress and Latin-letter conversion. Dates and unusual notation may be read component
by component: listen to important text. Choosing a language is not translation.

The Library supports play/pause, seek, 0.5–2× playback speed, volume, rename, WAV
export and delete. User titles, document contents and
historical records are not translated when changing the interface.

## Login startup and starting in the tray

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

## Quick Read (local development build)

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
`Ctrl+Alt+O` for an area. For a whole message, try pointing at it and pressing
`Ctrl+Alt+E`; the result depends on Telegram's accessible label. The compose box
is a different control with TextPattern support. These metadata checks did not
read messages or confirm live reading with a foreground selection.

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

## Dictation

Hold `Ctrl+Shift+Space`. Wait for Preparing microphone to change to the red recording
indicator and level bars; then speak and release. You receive one final text, without
partial text or hands-free capture. Early release cancels preparation. The microphone
is closed between attempts. The maximum recording duration is 60 seconds; silence
is filtered. Queue overflow or an engine failure discards the attempt with an error
instead of inserting a truncated transcript.

On Dictation choose language, microphone, hotkey and Auto-paste or Clipboard only.
History is off by default; enabling it retains the last 10 transcripts. Turning it
off stops additions; Clear history removes existing entries. Total dictated is a
lifetime duration counter. UI, dictation and TTS languages are independent; each
speech language remembers its local model and voice.

- **On this computer:** English Moonshine Small Streaming (158.8 MB initial download,
  including 16.5 MB native runtime); Russian GigaAM v3 CTC/RNNT (272.2–273.7 MB model
  plus 20.1 MB runtime). Windows x64 CPU. Models install by explicit choice, work offline
  after verification and survive updates. Other-language installed models remain listed.
  Remove an inactive model to free space; shared runtime is retained. Local speech uses
  explicit EN/RU; mixed-language detection and translation are not included.
- **Organization server:** OpenAI-compatible `/audio/transcriptions`; `/models` is
  optional. Example `http://192.168.1.10:8000/v1`. Your administrator installs the
  server. HTTP is allowed only for localhost/private IPs on a trusted network; audio
  and keys are unencrypted there. Hostnames need trusted HTTPS. System proxies are
  bypassed; an explicit proxy is available.
- **Cloud service:** choose a preset or compatible endpoint, model, key and optional
  dictation-only proxy. The existing remote `auto` language option is retained.

Office/cloud profiles and keys stay separate. Save and use activates an edited profile;
changing endpoints does not send the old key to the new service. Local Moonshine streams
audio internally while you hold the hotkey; GigaAM/server profiles keep their batch path.
Large GigaAM recordings are split at quiet boundaries; punctuation and rare terms may suffer.
Dictation and synthesis can run independently.
After the computer wakes from sleep, Glagol rebuilds the hidden indicator window within
a few seconds. Sleep shorter than about 13 seconds is not detected; if the indicator
ever stays invisible, restart Glagol from the tray.

## Windows limitations

- Auto-paste restores previous text clipboard contents, but cannot restore images/files.
  Clipboard managers changing line endings can prevent restoration. Windows Win+V
  history may retain transcripts independently of Glagol's history setting.
- Elevated windows may need Glagol at the same privilege level. Two identically named
  microphones are hard to distinguish. Background noise can pass the silence filter.
- `Ctrl+Shift+Space` conflicts with Office's nonbreaking space; choose another hotkey,
  for example `Alt+Shift+D`. A failed hotkey change retains the old binding.

## Backup, data and troubleshooting

Data remains in `%LOCALAPPDATA%\app.glagol.desktop\`: SQLite `glagol.db`, WAVs in
`audio_cache`, STT in `speech_models`, Silero in `tts_models`. Settings → Create backup
writes a ZIP of the library. Restore shows a confirmation, makes a protective backup
and restarts the app after replacing the library. Old libraries/backups remain readable.
Models, runtime, OS credentials, previews and Silero consent are excluded; enable speech
components separately on another PC. Credentials are stored in Windows Credential Manager.

If TTS cannot start, check selected language, model installation and acknowledgement;
repair if necessary. If a voice sounds wrong, check the speech language and try another
voice. For recognition errors, retry after checking microphone/model, and verify files
if needed. Close Glagol through the tray before collecting logs so they are flushed;
never attach private speech, documents or credentials.

[Report a bug](https://github.com/dimasiksuleyman-sudo/glagol/issues).
Glagol is independent open source software under MIT, created to listen to long texts.
