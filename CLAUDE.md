# CLAUDE.md

> Start with [AGENTS.md](AGENTS.md) and [docs/STATUS.md](docs/STATUS.md).
> This file contains the current architecture contract. Workflow lives only in AGENTS.md.

## Project overview

**Glagol** is an English-first Windows application with independently selected EN/RU interface, dictation and synthesis languages. Optional offline Silero v5.5 RU / v3 EN share one runtime; English dictation uses native Moonshine Small Streaming and Russian uses GigaAM. Office/cloud profiles remain independent. Tauri 2.x + React 19 + TypeScript; MIT application independent of model/service providers.

**Core value proposition:** local library of synthesized documents with resume-playback, MIT application with optional CC BY-NC-SA 4.0 noncommercial Silero model; dictation remains independent.

**Primary target:** Windows 10/11 x64. macOS/Linux are stretch goals after v1.0.

## Tech stack — non-negotiable choices

| Layer | Choice | Reason |
|---|---|---|
| Desktop framework | **Tauri 2.x** | Small bundle, native performance, Rust security |
| Backend language | **Rust stable** (≥1.77) | Memory safety, performance, ecosystem |
| Frontend framework | **React 19 + TypeScript** | Tauri 2 templates, broad knowledge |
| Styling | **Tailwind CSS + shadcn/ui** | Copy-paste components, no vendor lock-in |
| State (frontend) | **React contexts/hooks** | See src/contexts; no Zustand dependency in the current manifest |
| Build / package manager | **pnpm** | Fast, disk-efficient, lockfile committed |
| HTTP client (Rust) | **reqwest + rustls** | Pure-Rust TLS for STT and artifact downloads |
| Local database | **SQLite via rusqlite + rusqlite_migration** | Battle-tested, embedded, sync (chose over `tauri-plugin-sql` for security/test reasons) |
| Secret storage | **keyring-rs** (NOT Stronghold) | Windows Credential Manager, OS-level encryption |
| PDF parsing | **pdfium-render** | Same lib as Chromium, highest quality. Pdfium shared library downloaded by `build.rs` from `bblanchon/pdfium-binaries` and cached in `OUT_DIR/pdfium/`; path baked in via `PDFIUM_LIBRARY_PATH`. |
| DOCX parsing | **docx-rust** | Parsing-focused fork (the original `docx-rs` is writer-first); correct Cyrillic |
| Markdown | **pulldown-cmark** | Fast CommonMark parser |
| Audio (WAV) | **hound** (synthesis-side) + manual streaming (concat-side) | Simple, predictable; streaming WAV header normalization established in Sprint 1 PR #13 |
| Async runtime | **tokio** | Tauri default, mature |
| Audio playback | Library: HTML5 `<audio>` via Tauri asset protocol; Quick Read: cpal PCM | File streaming for library; bounded in-memory offline speech |

**Do NOT introduce these without discussion:**
- Electron (we chose Tauri specifically for bundle size)
- Yarn / npm as primary package manager (we use pnpm)
- Redux / MobX or another global state library
- Material-UI / Ant Design (we use shadcn/ui)
- Stronghold (this project uses keyring-rs)
- Tesseract / third-party OCR libraries (Quick Read uses built-in Windows.Media.Ocr)
- `tauri-plugin-sql` (was original plan, replaced with rusqlite in Sprint 2 — see PR #15 logical for rationale)

## Repository layout (0.5.0)

See [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md). TTS lives in `tts/silero`,
`commands/tts.rs`, `commands/synthesize.rs`, `TtsContext` and `TtsSection`.
STT remains in `stt`, `dictation`, `commands/speech.rs` and `DictationSection`.
Historical module names in day logs describe previous versions only.

## Architecture invariants

These constraints apply to changes. Resolve required architectural changes before implementation; follow AGENTS.md for the working process.

### Security invariants

1. **No secrets in code, config files, or environment variables.** Authorization Keys live in Windows Credential Manager (`keyring-rs`). Access tokens live only in RAM.
2. **Artifact downloads and public STT endpoints use verified TLS.** Office STT permits explicit private-IP/localhost HTTP under its separate policy. No retired OAuth endpoints or custom Sber root certificate.
3. **No TTS inference network access.** Pin downloadable artifacts; the webview uses IPC, not direct provider calls.
4. **No telemetry by default.** Sentry is opt-in, off by default. No analytics. No tracking pixels. No fingerprinting.
5. **No `unsafe` Rust without a `// SAFETY:` comment** explaining the invariants.
6. **No `dangerouslySetInnerHTML` in React.** Ever.
7. **No eval, no dynamic script loading.** CSP `script-src 'self'`.
8. **All user-supplied paths must be validated** against `app_local_data_dir()` or explicit dialog selection.
9. **Asset protocol scope is scoped, not wildcard.** Use the application audio cache scope; never broaden it to `["**"]`.

### Data invariants

1. **SQLite is the source of truth for application document metadata.** Don't store metadata in JSON files alongside.
2. **Audio files are on disk, NOT in SQLite BLOBs.** SQLite stores **relative paths** (`{uuid}.wav`), resolved through `paths::resolve_audio_path()`.
3. **Audio cache lives under `paths::audio_cache_root()`.** Resolve data paths through the Rust helpers; verify the actual app-local path for the build under test.
4. **One document = one row in `documents`.** Keep document/provider metadata and legacy audio on migrations.
5. **All migrations are versioned via `user_version` pragma** (managed by `rusqlite_migration`). Migrations are append-only; never edit a shipped migration.
6. **Persistence is transaction-wrapped.** Multi-step writes (INSERT row + fs::write file) use rusqlite `Transaction` — drop semantics auto-rollback on early return. Orphan files acceptable (invisible to user); orphan rows never (user-visible breakage).

### API invariants

1. **Tauri commands return `Result<T, String>`.** Errors become strings on the frontend boundary. Use `thiserror` for internal Rust error types, convert to `String` at the boundary.
2. **Long-running operations (>100ms) must report progress** via `tauri::ipc::Channel<T>` (high-frequency) or `app.emit()` (broadcast).
3. **One local TTS operation at a time.** A separate mutex protects installation, inference and removal without blocking dictation.
4. **TTS reuses a full verification for at most 30 days when key-file metadata is unchanged; stale/missing receipts and worker errors force a full check.** The Synthesize screen preloads the worker. No TTS worker or downloads start before the component is installed and acknowledged. The worker unloads after 15 idle minutes and exits with its parent.
5. **Readiness is not an OAuth probe.** TTS checks local installation/consent; STT retains its own validation cache.
6. **TTS limits belong to the backend.** Silero uses 280 input characters per request and segments normalized text below 480.
7. **Audio bytes never leave Rust over IPC.** `synthesize_document` returns `document_id` (UUID string). Frontend uses `get_audio_path` + asset protocol for playback, `export_audio` (server-side `fs::copy`) for disk export. Established Sprint 2 PR #16.

### Documentation invariants

1. **User-observable changes are documented in the same PR.** If a PR changes user-observable behaviour — a new setting, screen, default, known limitation, or an external fact the docs assert (pricing, providers, endpoints) — it updates `USER_GUIDE.ru.md` **and** `USER_GUIDE.en.md` in that same PR. The trigger is "will the user notice a difference," not "was the frontend touched." Internal refactors, test-only changes, and backend plumbing with no user-visible effect need no doc update.

2. **RU and EN are edited as a pair, always.** Every user-facing claim exists in both languages saying the same thing. Editing one language's guide without the other is a defect, not a follow-up. This applies to `README.md` too (its RU and EN halves).

3. **App is free; the API is paid.** Glagol itself is free/MIT. Providers are paid or free-with-limits per their own terms (cloud STT per provider terms; Silero is offline and has a separate noncommercial model license). Every pricing sentence keeps these two facts distinct — never let "the provider became paid" become "Glagol became paid."

4. **Verify external facts before updating current guides.** Keep RU/EN synchronized and use the working process in AGENTS.md. Historical logs are not retro-edited.

### Code style invariants

1. **Rust: `cargo fmt` + `cargo clippy -- -D warnings`** — enforced in CI, no exceptions.
2. **TS/React: TypeScript and Vite build.** ESLint/Vitest are not configured; do not claim these checks passed.
3. **Functional React components only.** No class components. Hooks > HOCs.
4. **Tailwind utility classes preferred over custom CSS.**
5. **Public Rust APIs documented with `///` doc comments.**
6. **No `console.log` in production code.** Use proper logging (`tracing` on Rust side, dev-only `console.*` in TS).

## Speech 0.5.0 — current contract

- UI, STT and TTS languages are independent. First-launch and 0.4.1 upgrade ask
  English/Русский, then optional speech setup. Persist choices immediately; preserve
  old implicit RU/cloud behavior on upgrade. UI switching never changes speech.
- Typed EN/RU dictionaries + React context/Intl; native errors are localized at
  IPC/event boundaries. Never translate user documents, names or historical records.
- Moonshine DLLs/Small are pinned, separate downloads. Hidden same-EXE child starts
  before Tauri and verifies files before loading; bounded queue/protocol, 16 kHz
  streaming outside callback, final insertion once, explicit error on overflow.
- Silero EN is v3_en, four numerical voices; shared RU runtime, independent receipts,
  models and consent. Shared operation lock prevents runtime replacement during TTS.
  Removing one language model retains the other model and runtime. No Kokoro/G2P.

- `tts/silero` owns optional files, consent, worker and download state; STT is separate.
- `TtsBackend` defines provider, voices, chunk limit and on-disk WAV result. Future
  Yandex SpeechKit v3 is out of scope until separately requested.
- Silero v5.5 RU and v3 EN are CC BY-NC-SA 4.0, not MIT. Unchecked acknowledgement before first
  download; backend checks model/license hash. No consent in transferred backups.
- Model/runtime excluded from installer. Pin all artifacts, verify before execution;
  read `docs/local-tts-runtime.md`. No hub, pip, arbitrary model or SAPI installation.
- Hidden owned Python process, bounded stdio JSON, mono PCM 24 kHz; no text/audio
  logging or network inference. Cancel/timeout/parent exit terminate the worker.
- Pipeline streams chunks to a temporary WAV and transactionally publishes one row.
  Keep legacy audio/provider metadata, dictation profiles, keys and usage seconds.
- Append-only migration 6 adds nullable speech_language: old rows stay NULL.
  Providers remain `salutespeech-legacy`, `silero` and new `silero-en`.
- Retired OAuth/client/certificate and quota UI are gone. Only a narrow one-time
  cleanup of `Glagol/salutespeech_auth_key` remains; never touch STT keys.
- Version sources: package.json, Cargo.toml, Cargo.lock, tauri.conf.json; run
  `node scripts/check-version.mjs`. User may request local commits without a PR.

## Implementation conventions

- Extract pure *_impl helpers for Tauri commands when tests need no runtime.
- Keep DB mutex guards block-scoped and release them before filesystem/network IO.
- Use in-memory SQLite for repository tests and isolated temporary directories for files.
- Use spawn_blocking for blocking work; preserve ownership and cleanup on errors/cancel.
- Revalidate paths during archive extraction; reject traversal and links.
- Public Rust APIs have doc comments; explain unsafe blocks with SAFETY comments.
- Do not log user text/audio/secrets. Use tracing and user-facing errors at the IPC boundary.
- Keep dependent manifest/lockfile changes together; audit cross-module users before removing dependencies.
- Prefer discriminated state unions and event-driven refresh; keep IPC types in src/lib/tauri.ts.
- Russian quantities use Intl.NumberFormat and existing plural helpers.
- Application code and adapters remain MIT; optional model licenses remain separate.

## Navigation

- [Build and verification](docs/runbooks/windows-build.md)
- [Context format](docs/context/README.md)
- [Historical workflow snapshot](docs/history/claude-before-context-2026-09-13.md) — history only.

Updated 2026-09-24. Historical sprint roadmaps and PR protocols are not current instructions.

## Quick Read (development tree)

- `quick_read` separates text sources, normalization, control generations, output and surfaces. Shared Silero worker has opt-in bounded binary PCM on private stdout; legacy document/preview WAV transport remains unchanged. No audio crosses webview IPC.
- One shared TTS operation lock, at most two queued 200-character sentence fragments plus an inference in progress; PCM limited to 120 s per fragment. cpal output stream belongs to a dedicated thread. Pause retains PCM; stop/new request clears output and cancels source/inference; an interrupted protocol closes the worker.
- Native same-EXE extraction mode runs before Tauri/single-instance/logging. UIA COM MTA stays in the child, with 3 s hard timeout, bounded stdout and parent watcher. Selection only GetSelection; pointer uses ElementFromPoint/paragraph/Value/Name/Legacy and bounded traversal. Password ancestry fails closed. Clipboard is read-only bounded CF_UNICODETEXT. No automatic Ctrl+C or UIAccess/elevation.
- OCR child uses GDI BGRA memory and Windows.Media.Ocr with explicit installed RU/EN language, 15 s timeout. Overlay covers the monitor under the cursor; logical drag coordinates map to its physical rectangle. No spanning monitors, at most 4096 px per side/16 MP. Mini-player stays hidden during capture, never auto-activates, and is off by default. No screenshots, source text or speech results are persisted/logged.
- Rust global-shortcut plugin dispatches by exact shortcut identity; dictation retains Pressed/Released. Settings detect both internal and OS conflicts. UIA/OCR/capture modules and Windows bindings are cfg-isolated. Interactive acceptance remains separate from build/tests: `docs/workstreams/quick-read/manual-qa.md`.

- Russian Quick Read PCM has default-on approximate English pronunciation (stdlib rules + small exceptions in existing `worker.py`), snapshot per reading and persisted toggle. Known/technical initialisms retain letter names; no voice/model switch, network or new dependency. WAV documents/previews keep legacy normalization; English model is independent. Pronunciation quality requires manual listening.

## Startup / tray

- Main config has create=false/visible=false/focus=false. After backend setup, a Windows startup thread creates main first and then the existing surfaces while the UI loop runs; it shows main only after reading the default-off `start_in_tray` preference. `--tray`/`--hidden` force hidden, `--show` overrides it. A normal second launch shows the existing main; hidden repeats leave visibility/focus untouched. Native source/STT child modes still enter before Tauri.
- `startup` owns an explicit opt-in HKCU Run value `Glagol` with quoted current EXE + `--tray`, bounded UTF-16/REG_SZ; never rewrite it at process start or override StartupApproved. Its value name matches NSIS productName cleanup (normal uninstall removes, update mode preserves). No admin/new dependency; settings report another location/errors without hiding visibility preferences.

- Startup registration snapshots settings and releases DB/shortcut guards before UI-thread global-shortcut calls. Keep this ordering to avoid blocking initial webview IPC. A normal repeat received before main exists is retained by PendingShow.

- Selection tries UIA first. For an exact focused Unicode HWND of class Scintilla, after the same password-ancestry check, native fallback uses scalar SCI queries/CountCodeUnits and system-marshalled WM_GETTEXT (<WM_USER). No pointer-bearing private messages, remote memory, synthetic copy, focus changes or elevation. One stream selection, UTF-8/SBCS only; byte prefix end <=2MB, UTF-16 prefix <=500k; per-call150ms and original source child3s cap. Recheck snapshot/focus, discard nonselected prefix, never persist/log text.
