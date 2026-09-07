# Security & Privacy Audit

Last audited: Phase 5 hardening

## Network

- `grep -R URLSession macos --include="*.swift"` → only `ModelCatalogView.swift` and `OnboardingView.swift` (download on user tap) — allowed. `core/src/models/downloader.rs` via `ureq` HTTPS only — allowed.
- No other `import Network` or `reqwest`. `cargo tree | grep ureq` only for downloader.
- All downloads are HTTPS to `huggingface.co` (ggml) or `nvidia` (parakeet), streamed, sha256 verified via `ModelManager::verify_checksum`, atomic `rename` from `.part`, disk-space check before start.
- No telemetry by default. `PerformanceMetrics` stays in memory; diagnostics export is explicit user action (Copy/Export in About → Diagnostics).
- Privacy manifest: model download is user-initiated; no `NSPrivacyAccessedAPICategory`.

## Filesystem

- Writes only:
  - `~/Library/Application Support/Supertype/supertype.db` (SQLite WAL, settings/history/dictionary, 0600 via umask default, no world share)
  - `~/Library/Application Support/Supertype/models/*.bin` (model binaries, sha verified)
  - `/tmp/*.part` transient, removed on success or mismatch
- No writes to `~/Documents`, `~/Downloads` except manual Export diagnostics.
- `Storage::push_history_detailed` prunes to 5000 entries and 30 days.

## Permissions

- Microphone: `AVCaptureDevice.requestAccess` only when user holds shortcut; `Info.plist` `NSMicrophoneUsageDescription` explains local transcription.
- Accessibility: `AXIsProcessTrustedWithOptions(prompt:true)` only on onboarding or insertion failure; fallback to clipboard if denied.
- Input Monitoring: required for global `fn` hotkey via `CGEventTap`/`NSEvent.globalMonitor`; user can use non-fn shortcut to avoid. `Supertype.entitlements` sets `device.audio-input` + `automation.apple-events` + `cs.allow-jit` + `allow-unsigned-executable-memory` + `disable-library-validation` (for whisper Metal/ONNX), sandbox `false` (required for AX/CGEvent).

## Clipboard

- `TextInjector.clipboardInsert` saves previous string, sets new, posts Cmd+V via `CGEvent` `cghidEventTap`, restores after 120 ms on background queue (not main `Thread.sleep`). If user copied new content after paste, restore skips (changeCount check). Never persists audio.

## Logging

- Rust `eprintln!` only for state codes and error codes, never `text` or `pcm`. Swift `print` redacted: `partial len=\(t.count)` not content, except `metrics` which contains no transcript.
- No raw audio in logs, no credentials, no private app content.

## SQLite

- `migrations.rs` verifies `no_audio_table`; `AUDIO_NEVER_PERSISTED` marker. `TranscriptionHistory` stores only formatted text + `model_id` + `duration_ms` + `bundle_id`/`app_name`/`confidence`.
- No audio blobs, no `CREATE TABLE audio`.

## Checklist

- [x] `grep -R "engine_push_audio" --include="*.swift"` only in `AudioCapture` + `RustBridge`
- [x] `grep -R "URLSession" --include="*.swift"` allowlisted
- [x] `cargo audit` no `reqwest` beyond `ureq` downloader
- [x] `strings` on `libsupertype_core.a` shows no hardcoded URLs beyond Hugging Face
- [x] `Info.plist` LSUIElement true (no Dock), `LSMinimumSystemVersion 14.0`
