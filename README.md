# Supertype — Privacy-First Local Voice-to-Text for macOS

> Phase 3 — System-Wide Voice Typing (global hotkey → local transcribe → AX/clipboard inject)

A local-first voice typing utility conceptually similar to Wispr Flow: hold a global shortcut, speak, release, and have text appear in the focused app — entirely on-device.

**Status:** Phase 3 complete — hold-to-talk global hotkey, FocusManager, TextInjector (AX + clipboard fallback), deterministic formatter, transcript lifecycle (raw→formatted), SQLite history (app/bundle), overlay polish.

## Architecture at a Glance

```
macOS Shell (SwiftUI + AppKit)
        │
        │  C-ABI / Swift ↔ Rust FFI
        ▼
Rust Core Engine
  ├─ State Machine (Idle → Preparing → Recording → Processing → Completed/Error)
  ├─ Settings (validated, persisted)
  ├─ Storage (SQLite + migrations)
  ├─ Events (recording_started … error, broadcast → SwiftUI)
  └─ Future seams: SpeechModel trait, ModelManager, VAD, Formatting
        │
        ▼
 Local State (SQLite @ ~/Library/Application Support/Supertype/supertype.db)
```

- **macOS layer** owns: menu bar, settings window, overlay (`NSPanel`), permissions, launch-at-login, lifecycle.
- **Rust core** owns: business logic, audio/transcription abstractions, persistence, state. No business logic leaks into SwiftUI.

## Prerequisites

- macOS 14+ (deployment target), Apple Silicon recommended
- **Full Xcode** (not just CLT) for `Supertype.xcodeproj` — or use SPM via CLT:
  ```sh
  xcode-select --install        # CLT (swift build)
  # For full app bundle, install Xcode from App Store + `sudo xcodebuild -license accept`
  ```
- Rust:
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
  rustup component add clippy rustfmt
  ```

## Build & Run

```sh
# 1. Build Rust core (staticlib + dylib)
cargo build --manifest-path core/Cargo.toml
cargo test  --manifest-path core/Cargo.toml

# 2. Build Swift app (SPM, works with CLT)
swift build --package-path macos

# 3. Run (menu-bar utility, LSUIElement — no Dock icon until Phase 3 hotkey)
./macos/.build/debug/Supertype

# 4. Release
cargo build --release --manifest-path core/Cargo.toml
swift build -c release --package-path macos
```

> Full Xcode build: open `macos/Supertype.xcodeproj` (Phase 1 ships SPM-first; Xcode project is generated on next bootstrap with `xcodegen` if needed).

## Repository Layout

```
/core            Rust engine (state, settings, storage, FFI)
  /src/engine    AppState + Engine + events (streaming partial/final)
  /src/audio     ring (rtrb SPSC) + resample (mono 16k) + VAD (Silero-like) + pipeline
  /src/transcription  SpeechModel trait → WhisperCppModel (load/warm/cancel) + DummyModel
  /src/transcript lifecycle (Raw→Formatted via formatter + dictionary)
  /src/formatting deterministic (punct, new line, caps, dict)
  /src/models    ModelManager (discover/verify/download_url) + builtin catalog
  /src/performance  metrics (RTF, load, VAD, backend metal) + BenchmarkReport
  /src/input     TextInjector trait + InsertionRequest
  /src/bin/bench  local benchmark (fixtures/*.wav, no network)
  /include       C header for Swift (push_audio, metrics, history, format, activeApp)
  /resources/models  on-demand Whisper quantized (download-model.sh)
  /resources/fixtures  hello/technical/longer wav (en)
/macos           SwiftUI + AppKit shell
  /Sources/Supertype/App      SupertypeApp + AppDelegate (hotkey+focus+inject lifecycle)
  /Sources/Supertype/Audio    AudioCapture (AVAudioEngine → Rust, 16k mono, Accelerate)
  /Sources/Supertype/Bridge   RustBridge (FFI + pushAudio + history/format + activeApp)
  /Sources/Supertype/UI       SettingsView (General/Permissions/History/Models/Privacy), OverlayWindow (partial, anchored), HistoryView
  /Sources/Supertype/Permissions  Microphone + Accessibility (+ Input Monitoring note)
  /Sources/Supertype/Platform     LaunchAtLogin, HotkeyManager (InputController CGEventTap/NSEvent), ActiveApp, TextInjector/ClipboardFallback
  /Sources/CSupertypeCore     Clang module re-exporting C header
/docs/architecture  Design rationale
/docs/testing     Phase-wise testing playbooks (phase-1.md canonical, README routing)
/docs/qa-checklist.md  Legacy Phase 1 checkbox (kept, see docs/testing/phase-1.md)
/tests            Integration (see core/tests, docs/testing/)
```

## Privacy Guarantees (enforced in code)

- `core/src/audio/mod.rs` — audio never written to disk; no `BLOB` audio tables (migrations test asserts).
- No `URLSession`, no telemetry, no cloud API. Offline after install.
- `Storage` only holds `settings`, `transcription_history` (text), `dictionary_entries`.
- Logs never include transcript content (engine logs only state codes).

## Settings Persisted (SQLite + UserDefaults mirror)

- `selected_microphone_id` (optional device UID)
- `global_shortcut` (string, Phase 3 will parse)
- `selected_model_id` (e.g. `whisper-tiny`)
- `history_enabled`, `launch_at_login`, `overlay_enabled`

## State Machine

`Idle → Preparing → Recording → Processing → Completed → Idle` (+ `Error` recovery). Invalid transitions return error code (10) — see `core/src/engine/state.rs`.

Events exposed for Phase 2: `recording_started`, `recording_stopped`, `speech_detected`, `speech_ended`, `partial_transcript`, `final_transcript`, `processing_started`, `processing_completed`, `error`, `state_changed`.

## Testing

```sh
./scripts/test-phase-3.sh           # one-command Phase 3 verification (85 tests + bench + FFI + swift)
./scripts/test-phase-2.sh           # Phase 2 still green
./scripts/test-phase-1.sh           # Phase 1 still green
cargo test --manifest-path core/Cargo.toml -- --nocapture
cargo run --manifest-path core/Cargo.toml --bin bench  # 3 fixtures, RTF <1, backend metal
cat docs/testing/phase-3.md         # Phase 3 playbook (hotkey→inject)
cat docs/testing/phase-2.md
cat docs/testing/phase-1.md
```

Full phase routing: [`docs/testing/README.md`](docs/testing/README.md).

## On-demand model (Phase 2, no model bundled)

```sh
./scripts/download-model.sh whisper-tiny   # 75 MB → ~/Library/Application Support/Supertype/models/
./scripts/download-model.sh whisper-base   # 142 MB
# bench works without download (temp fake model) but real transcription needs the file
```

Hold-to-talk: default `fn`, alternatives `ctrl+space` etc. — configurable in Settings → General. Works unfocused via `GlobalInputController` (CGEventTap/NSEvent). Dictation inserts via AX and clipboard fallback; history stored only when enabled.

## Next Phases

- **Phase 2:** ✅ done — AVAudioEngine capture, ring → resample, VAD, whisper.cpp (quantized, on-demand), streaming, metrics/bench
- **Phase 3:** ✅ done — global hotkey (hold/toggle), AX/clipboard TextInjector, ActiveApp, deterministic formatter, transcript lifecycle, SQLite history (bundle), overlay polish
- **Phase 4:** Model catalog, downloads, Parakeet integration
- **Phase 5:** Polish, latency/memory profiling, notarization

See `docs/architecture/README.md` for the seven design rationales Phase 2 needs.
