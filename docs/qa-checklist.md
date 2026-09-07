# QA Checklist — Phase 1

> **Canonical instructions now live in `docs/testing/phase-1.md`.** This file is retained for quick checkbox printing; see `docs/testing/README.md` for phase routing.

Manual verification after `cargo test` + `swift build` succeed.

## 1. Application Startup
- [ ] `./macos/.build/debug/Supertype &` launches without crash; `ps aux | grep Supertype` shows one process.
- [ ] No Dock icon (`LSUIElement` / `.accessory` policy).
- [ ] Menu bar icon (waveform) appears on the right side.

## 2. Menu Bar Presence
- [ ] Clicking the icon shows menu: `State: Idle`, `Settings…`, `Permissions…`, `Start/Stop Recording (Test)`, `Quit`.
- [ ] `State:` updates when triggering Test actions (Idle → Recording → Processing → Completed → Idle).
- [ ] Quitting via menu terminates cleanly (`applicationWillTerminate` logs).

## 3. Settings Window
- [ ] `Settings…` opens a 560×420 TabView (General, Permissions, Models, Privacy).
- [ ] Changing `Global Shortcut` or `Model` persists: quit → relaunch → value retained.
- [ ] `Launch at login` toggles without error (check `SMAppService.mainApp.status`).
- [ ] `Enable recording overlay` off suppresses the floating indicator.

## 4. Permission State Detection
- [ ] Permissions tab reflects `Granted / Denied / Not Determined` for Microphone and `Granted / Not Granted` for Accessibility (via `AVCaptureDevice.authorizationStatus` and `AXIsProcessTrusted`).
- [ ] `Request Microphone Access` prompts once; `Open System Settings` opens `x-apple.systempreferences:...Privacy_Microphone`.
- [ ] `Open System Settings` for Accessibility opens the correct pane; does not re-prompt on every launch (only on button).

## 5. Recording State (mock, no real audio yet)
- [ ] Test `Start Recording` → menu state becomes `Recording`, overlay shows `Listening…` (red dot).
- [ ] `Stop Recording` → `Processing…` then `✓ Done` then auto-hides, state returns to `Idle`.
- [ ] `Cancel` from Recording returns to `Idle` and hides overlay.
- [ ] Invalid transitions (e.g. Stop when Idle) show error without crash (Rust returns code 10).

## 6. Overlay Window Behavior
- [ ] Overlay is an `NSPanel` with level `.floating`, material background, rounded 14, hides when idle (no window ordered in).
- [ ] Overlay appears centered, stays on all spaces, does not activate the app, ignores mouse events.

## 7. Shutdown / Relaunch / Persistence
- [ ] `Quit` then relaunch retains settings (SQLite `@ ~/Library/Application Support/Supertype/supertype.db` exists, `ls -lh` shows file; `sqlite3 … "select * from settings;"` shows JSON).
- [ ] DB has tables `settings`, `transcription_history`, `dictionary_entries` (`sqlite3 … ".tables"`), no audio tables.
- [ ] Second launch with existing DB runs migrations idempotently (no duplicate tables).

## 8. Resource / Performance
- [ ] While Idle, `Activity Monitor` shows < 0.3% CPU, < 40 MB RAM, no microphone indicator.
- [ ] `lsof -p <pid> | grep -i audio` shows no audio device held when idle.
- [ ] No network traffic (`nettop` / `lsof -i`) when using the app offline.

## 9. Build & Tests
- [ ] `cargo test --manifest-path core/Cargo.toml` → 30 passed.
- [ ] `cargo clippy --manifest-path core/Cargo.toml` → no warnings.
- [ ] `swift build --package-path macos` → `Build complete!`, binary links `libsupertype_core` (`otool -L`).
- [ ] `grep -R URLSession macos/` is empty (no cloud).

## 10. Regression Guards
- [ ] Raw audio never written: `grep -R "BLOB\|audio_data\|AudioFile" core/` finds nothing.
- [ ] `cargo test` includes `no_audio_table`, `settings_persistence_roundtrip`, `state_invalid_transitions`, `event_serialization`.
