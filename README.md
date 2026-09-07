# Supertype — Privacy-First Local Voice-to-Text for macOS

> Phase 5 — Production-hardened local dictation: hold one key, speak, text appears almost immediately.

Hold a global shortcut, speak naturally, release — text appears where you type, entirely on-device via local ASR (whisper.cpp / Parakeet), VAD, deterministic formatting, and AX/clipboard injection.

**Status:** Phase 5 complete — onboarding wizard, 6-tab settings, polished overlay, latency/memory/battery hardening, reliability edge cases, text quality regression, app compatibility matrix, per-model benchmarks, diagnostics, security audit, release-ready (signing docs, DMG, hardened entitlements).

## Architecture at a Glance

```
macOS Shell (SwiftUI + AppKit)
        │  C-ABI / Swift ↔ Rust FFI (staticlib supertype_core, OpaquePointer *mut Engine)
        ▼
Rust Core Engine
  ├─ State (Idle→Recording→Processing→Completed/Error, 30s cap, focus-change guard)
  ├─ Settings (shortcut/behavior, language, punct/caps, mic, history, overlay)
  ├─ Storage (SQLite WAL, settings/history/dictionary, 5000/30d prune, no audio)
  ├─ Audio (AVAudioEngine 1024 tap → AVAudioConverter 48k→16k mono → ring rtrb → resample → VAD 30ms → pending_pcm 30s cap)
  ├─ Transcription (SpeechModel trait → WhisperCppModel q4_0/q5_0, Parakeet 0.6B fp16/CC-BY-4.0, streaming partial every 1s off-lock)
  ├─ Formatting (token-based spoken punct, sorted dictionary, dedup, i→I, triple-space collapse, with_options)
  ├─ Models (catalog 43/75/142/600 MB, downloader ureq stream sha256 atomic, hardware probe, recommend)
  └─ Performance (metrics: capture/VAD/ASR/RTF/eos, BenchmarkReport)
        ▼
 Local State (~/Library/Application Support/Supertype/supertype.db + models/*.bin, no audio persisted)
```

- **macOS** owns: menu bar `NSStatusItem`, `NSPanel` overlay (non-activating, floating, multi-display clamped, async AX, low-CPU), onboarding, settings, permissions, `SMAppService`, `AVAudioEngine`, `CGEventTap/NSEvent`, `AXIsProcessTrusted`, `TextInjector` (AXSelectedText preferred, clipboard fallback background restore, secure-field block).
- **Rust** owns: state, audio, transcription, storage, settings, formatting, input, models, performance, hardware. No network beyond downloader.

## Prerequisites

- macOS 14+, Apple Silicon recommended, full Xcode for app bundle or CLT for `swift build`
- Rust `stable` + `clippy/rustfmt`: `curl ... | sh && source $HOME/.cargo/env && rustup component add clippy rustfmt`

## Build & Run

```sh
cargo build --manifest-path core/Cargo.toml
cargo test  --manifest-path core/Cargo.toml   # 108 tests
swift build --package-path macos              # debug
./macos/.build/debug/Supertype                # menu bar, LSUIElement

# Release + bench
cargo build --release --manifest-path core/Cargo.toml
cargo run --manifest-path core/Cargo.toml --bin bench
swift build -c release --package-path macos
DYLD_LIBRARY_PATH=core/target/release macos/.build/release/SupertypeFFITest

# One-command verifications
./scripts/test-phase-1.sh; ./scripts/test-phase-3.sh; ./scripts/test-phase-4.sh
cargo clippy --manifest-path core/Cargo.toml -- -D warnings
```

## First-Run (no account)

1. Launch → onboarding wizard explains "Your voice is processed locally on this Mac."
2. Grant Microphone → Accessibility → choose hold/toggle shortcut.
3. Download a model (Tiny Q4 43 MB shown, Tiny 75 MB default/recommended, Base 142 MB, Parakeet 600 MB) — size shown before download, SHA verified.
4. Hold shortcut (default `fn`, alternatives `ctrl+space`) in any app (Safari, Chrome, Slack, Discord, VS Code, Cursor, Notion, Notes, TextEdit, Terminal — see `docs/compatibility.md`) → overlay `● Listening` (subtle waveform, non-focus-stealing, never interrupts target, clamped to visibleFrame, hides on sleep) → speak → release → `◐ Processing` → `✓ Done` → text inserted at caret (AXSelectedText preferred, clipboard fallback), audio discarded, optional history saved, app returns to idle (<0.3% CPU).
5. Configure once, forget app exists.

Settings: `General` (launch at login, shortcut, hold/toggle, overlay), `Microphone` (device UID picker, live level test), `Speech` (Models, Vocabulary, Language, punct/caps toggles), `History` (enable, clear, reveal DB), `Privacy` (local inference explainer, network allowlist, pruned SQLite), `About` (version, macOS, licenses, diagnostics copy/export).

## Repository Layout

```
/core                Rust engine + FFI + bench + fixtures
/macos               SwiftUI+AppKit shell (App/Onboarding, Audio, Bridge, Platform, UI, Permissions)
/docs/architecture   design rationale
/docs/testing        phase 1-4 playbooks + regression
/docs/compatibility  matrix (Safari/Chrome/Slack/VS Code/… insertion method, limits)
/docs/benchmarks     report.md (download/load/RAM/RTF/CPU, recommended defaults)
/docs/privacy.md     local-only flow + privacy controls
/docs/security.md    network/filesystem/permissions/clipboard audit
/docs/release.md     versioning/signing/hardened runtime/DMG
/resources/fixtures  hello/technical/longer wav + regression.json
/scripts             bootstrap, download-model.sh, test-phase-*.sh, make-dmg.sh
```

## Privacy Guarantees

- `AUDIO_NEVER_PERSISTED` — ring + pending_pcm cleared on stop/cancel/ack/sleep; no audio SQLite table (migration test asserts).
- No `URLSession` except `ModelCatalogView/Onboarding` download on tap (HTTPS + SHA atomic). No telemetry. Logs redacted (len only).
- History opt-in, 5000/30d pruned, `~/Library/Application Support/Supertype/supertype.db` WAL.

## On-Demand Models

```sh
./scripts/download-model.sh whisper-tiny       # 43-75 MB
./scripts/download-model.sh whisper-base       # 142 MB
./scripts/download-model.sh parakeet-tdt-0.6b  # 600 MB fp16
```

Catalog shows size, quant, runtime, license (MIT/CC-BY-4.0), attribution, capabilities, recommended badge per hardware (8 GB→Tiny, 16 GB+→Parakeet).

## Testing & Benchmarks

```sh
cargo test --manifest-path core/Cargo.toml -- --nocapture   # 108 including formatting regression
cargo run --manifest-path core/Cargo.toml --bin bench -- ~/Library/Application\ Support/Supertype/models/whisper-tiny-q4_0.bin
cat docs/benchmarks/report.md          # RTF 0.02-0.11 metal, peak 40-650 MB
cat docs/testing/phase-4.md; cat docs/compat*md
```

Regression corpus: `resources/fixtures/regression.json` covers conversational/technical/programming/proper/mixed/short/long/dedup.

## Distribution

`Info.plist` `CFBundleShortVersionString 0.2.0 (42)`, `LSUIElement true`, `LSMinimumSystemVersion 14.0`. `Supertype.entitlements` hardened (`allow-jit`, `allow-unsigned-executable-memory`, `disable-library-validation` for Metal/ONNX), `audio-input`, `automation.apple-events`. `scripts/make-dmg.sh` builds DMG; signing/notarization steps documented in `docs/release.md` (requires credentials — project is release-ready, not falsely claiming notarized).

See `docs/architecture/README.md` for 7 rationales, `docs/release.md` for signing, `docs/security.md` for audit.
