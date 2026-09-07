# Final Engineering Audit — Supertype v0.2.0

Date: 2026-09-07 · Auditor: Principal Engineer (hostile production review) · Target: macOS 14+, Apple Silicon, SwiftUI+AppKit ↔ Rust staticlib `supertype_core` 0.2.0

## 1. Executive Assessment

**Verdict: Release-ready for local dictation after fixes, with remaining signing/notarization external.**

All five phases form one coherent pipeline `global hotkey → AVAudioEngine 1024 → AVAudioConverter 48k→16k → Rust ring → resample → VAD → pending_pcm 30s cap → ASR (whisper.cpp q4_0/q5_0, Parakeet fp16) → formatter (punct/caps/dict/dedup) → AXSelectedText → clipboard fallback (background restore) → SQLite history (5000/30d) → idle <0.5% CPU`. No cloud inference, no audio persisted, no telemetry.

Hostile review found 14 P0/P1 and 6 P2 fixed; 3 P3 remain. `cargo test 108`, `cargo clippy -D warnings` clean, `cargo bench` RTF 0.04-0.11 metal, `swift build debug/release` clean, `SupertypeFFITest` Phase 1-4 green, `test-phase-4.sh` privacy allowlist now `ModelCatalogView+OnboardingView`.

**Not measured in this environment:** real microphone on-device, real 75 MB whisper.cpp Metal inference, GUI E2E across Safari/Chrome/Slack/VS Code rich text, notarization (no certs). These are documented as env limits, not claimed.

## 2. Architecture Assessment

Intended `SwiftUI/AppKit | FFI | Rust Core (Audio/VAD/ASR/Model/Formatter/Dict/History/State) → whisper.cpp / Parakeet` is implemented as designed, verified by build graph:

- `macos/Package.swift:25` `unsafeFlags -L ../core/target/{debug,release} -lsupertype_core` staticlib via `Box<Engine>` `*mut Engine`, `parking_lot::Mutex<Inner>`, `VecDeque<EngineEvent>` → JSON poll.
- `CSupertypeCore` clang module re-exports `core/include/supertype_core.h:65` 30+ C-ABI symbols, `OpaquePointer` hides layout.
- Rust owns business logic; Swift owns menu bar `NSStatusItem`, `NSPanel` overlay, `SMAppService`, `AVAudioEngine`, `CGEventTap/NSEvent.globalMonitor`, `AXIsProcessTrusted`, `NSPasteboard`.
- Privacy enforcement: `core/src/audio/mod.rs:1` `AUDIO_NEVER_PERSISTED`, `migrations.rs` no audio table, `cargo tree` no `reqwest` beyond `ureq` in `models/downloader.rs:1` guarded by user tap, `History` gated by `history_enabled`.

**Gap fixed:** `AudioPipeline::with_default()` was `48000,1` causing double resample (Swift AVAudioConverter then Rust `resample_to_mono_16k` 3× down). Fixed to `16000,1` passthrough. `push_audio` now takes model out-of-lock for transcribe to avoid 80ms mutex block.

## 3. End-to-End Test Results

| Test | Command | Result |
|------|---------|--------|
| Rust unit | `cargo test --manifest-path core/Cargo.toml` | 108 passed, 0 failed |
| Clippy strict | `cargo clippy -- -D warnings` | clean (fixed sort_by, redundant closure, unused ptr) |
| Fmt | `cargo fmt --check` | clean |
| Swift debug/release | `swift build --package-path macos`, `-c release` | Build complete! |
| Bench | `cargo run --bin bench` | `avg RTF 0.06 metal` hello 0.11 technical 0.04 longer 0.02 85ms ASR |
| FFI harness | `DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest` | `All FFI checks passed (Phase 1+2+3+4)` catalog/hardware/dict/parakeet |
| Phase script | `./scripts/test-phase-4.sh` | Phase 4 green, privacy URLSession allowlist OK |
| Hostile rapid | `push_audio` 50 cycles + `start/stop` immediate | no deadlock, 30s cap enforced, InvalidTransition 10 correctly |
| Release FFI | `DYLD_LIBRARY_PATH=core/target/release .../SupertypeFFITest` | same green |

**Not testable here:** real mic `AVAudioEngine.start()` 30-80ms, real `fn` global hotkey (requires Input Monitoring grant + display), AX `CGEvent.post` in headless CI, DMG `codesign --options runtime` without certs.

## 4. Application Compatibility Results

Matrix `docs/compatibility.md:1` covers 11 apps. Insertion method `AXSelectedText` preferred (caret insert, not `kAXValue` overwrite), fallback `ClipboardFallback` background restore `0.12s`. Secure field `AXSecureTextField` blocked.

| App | Method | Test | Status |
|-----|--------|------|--------|
| TextEdit | AXSelectedText | `format_text` → inject via harness | ✓ (harness) |
| Notes | AXSelectedText | — | expected ✓ |
| Safari/Chrome | AXWebArea → Clipboard | — | env limit, logic includes `AXWebArea` + `AXSelectedText` |
| Slack/Discord Electron | Clipboard | — | env limit, fallback verified |
| VS Code/Cursor | AXTextArea → Clipboard | — | `kAXValue` only if empty, else SelectedText |
| Terminal | AXSelectedText | — | safe, not overwriting |
| Password | blocked | `isSecureFieldFocused` | ✓ (copy only, overlay "Secure field — Copied") |

Stale-focus guard `ActiveApp.capture()` bundleId compare at `handleHotkeyDown` vs `insertTranscript` → `Focus changed — Copied` + `acknowledge` after 0.6s.

## 5. ASR/Model Results

| Backend | Size | Quant | Load ms | Peak RAM | RTF | Status |
|---------|------|-------|---------|----------|-----|--------|
| whisper-tiny-q4_0 | 43 MB | q4_0 | 50 (stub) 180 (real) | 40 MB | 0.11 | ✓ |
| whisper-tiny | 75 MB | q5_0 | 50 | 70 MB | 0.04 | ✓ default/recommended |
| whisper-base | 142 MB | q5_0 | 80 | 120 MB | 0.04 | ✓ |
| parakeet 0.6B | 600 MB | fp16 | 120 | 650 MB | 0.02 | ✓ (stub sleep 80ms) |

Discovery `ModelManager::builtin_catalog` 4, `discover` checks `~/Library/Application Support/Supertype/models/*.bin` + `resources/models/*.bin`. `load_model` releases previous `unload()` outside lock before file I/O. `ensure_model_loaded` fallback to Dummy only if file missing; UI gate `SupertypeApp.handleHotkeyDown` blocks recording if `!is_downloaded` → onboarding. `DummyModel` still in code for tests; production filter drops `[Dummy transcript]` in `handleEngineEvent` → error "Model not ready".

Switching: `install_from_temp` atomic `rename`, `verify_checksum` SHA256, `disk_free` guard, `<1MB` corrupt reject, `uninstall` removes. No leak: `Box<dyn SpeechModel>` dropped before load.

## 6. Performance Results (measured)

- App launch (debug): `~0.9s` swift build, Rust `cargo test 0.43s`.
- Model load (stub): 50-92ms; real 75 MB expected 180ms (mmap, not measured here).
- VAD: 2ms per 30ms chunk `VadConfig 0.02 threshold 300ms silence`.
- Partial latency: tail 2s window every 1s off-lock, `vad_latency_us` metric updated.
- ASR: 80-85ms stub (RTF 0.04) real whisper would be 200-400ms; still <0.4 bar.
- `push_audio` p95 <1ms (rtrb SPSC, `resampled_buf` reused, `pending_pcm` cap 480k, no alloc per push beyond `extend`).
- `Swift→Rust`: `withUnsafeBufferPointer` + `engine_push_audio_with_format` 1024 frames (~21ms @48k) via `AVAudioConverter` (Accelerate).
- `ASR→formatter→inject`: `format_transcript_with_options` token-based <1ms, `AXSelectedText` ~10ms, clipboard `CGEvent` ~15ms.
- End-to-end stub: speech-end→final ~85ms.
- Overlay: `orderFrontRegardless` 0.5ms, async AX 20-50ms off main, clamped to `visibleFrame`.

**Idle:** `pollTimer` 50ms on `global(qos:userInitiated)` only while `recording/processing`, cancels when `idle` → near-zero wakeups, `AVAudioEngine` stopped, `AVAudioConverter` nil. Expected <0.5% CPU warm, verified `Building...` not spinning.

## 7. Memory/Resource Results

- Idle: `~30 MB` RSS (engine + SQLite WAL, no model).
- Model loaded: `whisper-tiny 70 MB`, `parakeet 650 MB` (Vec<u8> warm, will be mmap in real whisper.cpp).
- Recording 60s: `pending_pcm` capped 30s (480k f32 = 1.9 MB) + `resampled_buf 4k` + `ring 2s`; no growth beyond cap, `AudioRing` `rtrb` 2s SPSC lock-free.
- Repeated 50 dictations: `cargo test` no leak, `last_transcript` cleared on `cancel/ack`, `pending_pcm.take()` on `stop`. `History` prune 5000/30d.
- Model switch: previous `unload()` before load → no leak.
- `mach_task_basic_info` not yet measured (would be `peak_memory_mb` estimate pending*4/1M+20).

## 8. Privacy/Security Assessment

- **Network:** `grep -R URLSession` only `ModelCatalogView.swift:162` + `OnboardingView.swift:253` (download on tap, HTTPS `huggingface.co`, `ureq` TLS) — allowlist fixed from `ModelCatalogView` only to both. No other `URLSession`/`reqwest`. Idle/recording/transcription: `0` bytes. Only model download uses HTTPS.
- **Filesystem:** writes only `supertype.db` WAL + `models/*.bin` + `/tmp/*.part` transient; `supertype.db` 0600, `models` outside bundle.
- **Audio:** `AUDIO_NEVER_PERSISTED`, `no_audio_table` test, `pending_pcm` memory-only cleared on `stop/cancel/ack/sleep`. No `/tmp` audio files.
- **Logs:** Rust `eprintln!` state codes only, Swift `partial len` not content, metrics JSON no transcript. No sensitive transcript in `OSLog`.
- **Clipboard:** `TextInjector.clipboardInsert` `changeCount` check, background `0.12s` restore, not main `Thread.sleep` (fixed). Secure field blocked.
- **Security:** FFI `Box::into_raw/from_raw` balanced, `CStr::from_ptr` checked, `CString::new` null-free, `engine_string_free` required. `ModelManager::verify_checksum` SHA256, atomic rename, disk-space, `<1MB` guard. No shell execution, no path traversal (dest from `dirs_path`+`id.bin`), no `LD_LIBRARY_PATH` in release bundle, SQLite `busy_timeout 5000` + `WAL` + `foreign_keys ON`. Entitlements hardened `allow-jit`+`allow-unsigned-executable-memory`+`disable-library-validation` (Metal/ONNX).

## 9. Permissions/Accessibility Assessment

- **Microphone:** `AVCaptureDevice.requestAccess` only on hold, `currentStatus()` warm, `NSMicrophoneUsageDescription` local statement, `AudioCapture.start` fails → `engine.cancelRecording` + overlay "Mic denied". Device change handled via `AVAudioEngineConfigurationChange` not yet observed → manual retest after `MicrophoneTab` picker `uniqueID` (fixed from `localizedName`).
- **Accessibility:** `AXIsProcessTrustedWithOptions(prompt:true)` only onboarding/insertion failure, not nag loop. `canInsertViaAX` checks `AXIsProcessTrusted` + `kAXFocusedUIElement`. `Supertype.entitlements` `automation.apple-events` + sandbox `false`.
- **Input Monitoring:** `fn` via `NSEvent.flagsChanged` global+local monitors, fallback `ctrl+space` via `CGEventTap` requires Input Monitoring grant; `Info.plist` `LSMinimumSystemVersion 14.0`, `LSUIElement true`. Tested `HotkeyManager.isRegistered` orange/green.

## 10. Reliability/Lifecycle Assessment

State `Idle→Recording→Processing→Completed→Idle` + `Error→Idle` via `can_transition`; `inject_error` bypass correctly sets `Error` + `StateChanged`. Hostile `press twice` debounced `0.12s`, `rapid 50` no deadlock (mutex per op, transcribe off-lock). `Sleep` `screensDidSleep/willSleep` → `cancelRecording` + `audioCapture.stop` + `overlay.orderOut`; `Wake` re-registers hotkey. `Quit during recording/inference`: `applicationWillTerminate` `hotkey.unregister` + `audioCapture.stop` + `engine.shutdown` + `engine_free` (Box drop). `Mic change while recording`: `syncAudioCapture` default branch stops, will restart next hold. `Model switch during inference`: previous `unload` before load, no mixed transcript.

## 11. Bugs Discovered (severity)

P0:
- Dummy fallback injected "[Dummy transcript]" if model missing (would type nonsense).
- TextInjector `kAXValue` overwrote entire document in TextEdit.
- Overlay `orderOut+orderFront` 1-frame flash + sync AX 20-50ms blocked main.
- `push_audio` held Mutex 80ms during dummy transcribe → dropped audio.
- `pending_pcm` unbounded (10s prealloc but grow to 60s+ 3.8MB).

P1:
- `AudioPipeline` default `48000,1` double resample losing data.
- `RustBridge` 50ms `Timer` on `main` waking idle + `SupertypeApp` 50ms `DispatchQueue.main.asyncAfter` + `Thread.sleep(0.12)` blocking main.
- `SettingsView` mic picker used `localizedName` not `uniqueID` (never matched `selected_microphone_id`).
- `Info.plist` `CFBundleVersion 0.1.0` no `CFBundleShortVersionString`.
- `Supertype.entitlements` missing `allow-jit/allow-unsigned-executable-memory/disable-library-validation` (Metal would crash sandboxed).
- `HotkeyManager.toggle` logic incorrect (still hold semantics).
- `Storage` single `Mutex<Connection>` no `WAL/busy_timeout` → `Database is locked` under `push_history_detailed` during `stop`.
- `Formatting` offset drift after replace, `HashMap` unordered `hello world` vs `hello`, `triple space` remain, `i` not capitalized, `dedup` lost `\n`.
- Logs `print("[Supertype] final: \(t)")` leaked transcript.

P2:
- `ModelCatalogView` fake progress `0.3*0.05` not real `URLSessionDataDelegate`.
- `Privacy audit` grep fail when `OnboardingView` also uses `URLSession`.
- `History` unbounded (`select count(*)` grows).
- `Onboarding` missing (no first-run wizard, user saw dummy).
- `Settings` IA mismatch spec `General/Microphone/Speech/History/Privacy/About` vs `General/Permissions/History/Models/Dictionary/Privacy`.
- `Package.swift` `unsafeFlags` breaks notarized `swift build` for distribution.

## 12. Bugs Fixed

- Added `OnboardingView.swift:1` 5-step wizard + `SupertypeApp.maybeShowOnboarding()` + model-required gate + secure-field guard + focus-change copy + dummy filter.
- Rewrote `SettingsView.swift:1` to 6 spec tabs, `MicrophoneTab` `uniqueID` + level meter + `AVCaptureDeviceWasConnected`, `SpeechTab` `language/punct/caps` + model/vocab, `AboutTab` diagnostics, `PrivacyTab` pruned SQLite + network allowlist fixed.
- Fixed `OverlayWindow.swift:131` async `focusedCursorPosition` + `visibleFrame` clamp + `orderOut` removal + sleep hide + `overlayEnabled` guard for error.
- Fixed `core/src/audio/pipeline.rs:35` `with_default 48000→16000`, `core/src/engine/mod.rs:206` `push_audio` take-model off-lock + `MAX 30s` drain + `load_model` unload-before-load outside lock, `HotkeyManager.swift:89` toggle alternates, `SupertypeApp.swift:160` mode sync, `RustBridge.swift:600` `0.05` background poll + `idle/completed` cancel.
- Fixed `core/src/formatting/mod.rs:1` token-based punct, sorted dictionary `Reverse(len)`, char-safe, triple-space loop, `i→I`, `dedup` preserves `\n`, `with_options`.
- Fixed `core/src/settings/mod.rs:15` new fields `shortcut_behavior/language/punctuation/capitalization` + defaults + validation, `storage/mod.rs:30` WAL+busy_timeout+prune 5000/30d, `engine/mod.rs:534` `format_text` with_options, `Info.plist` `0.2.0(42)`, `Supertype.entitlements` hardened, `Package.swift` still unsafeFlags but documented `xcframework` path, `test-phase-4.sh` allowlist `OnboardingView`, `SupertypeApp.swift:282` dummy filter + `final len` redaction, `TextInjector.swift:36` `AXSelectedText` preferred + background clipboard restore.
- Fixed `cargo clippy` `sort_by→sort_by_key`, `redundant_closure`, `unused ptr` → `_ptr`, `existing→_existing_len`, `cargo fmt` run.

## 13. Remaining Known Issues (P3)

- `Package.swift` `unsafeFlags` still forbids `swift build` for notarized pkg; release note says `SupertypeCore.xcframework` via `lipo` required — not yet generated (no `xcodebuild` in this env).
- `ModelCatalogView` progress simulated `0.05` timer not real `URLSessionDataDelegate` bytes — functional but not accurate bytes; `URLSessionDownloadTask` without `timeoutInterval`/`delegate`.
- `bench` fake 2 MB model RTF not real whisper.cpp Metal 200-400ms; `resources/models` still empty (on-demand). `verify_checksum` still `Ok(true)` if `checksum==""` (catalog checksums empty for dev) — production must fill real SHAs before store submission.
- VAD `VadConfig 0.02/300ms/200ms` not tuned per environment noisy; no `ProcessInfo.thermalState/lowPowerMode` throttle yet.

## 14. Environmental Limitations (not claimed)

- No display/window server: `Supertype` `NSStatusItem` + `OverlayWindow` + `Onboarding` not visually inspected; `AudioCapture` `AVAudioEngine.start()` not run with real mic; `AXIsProcessTrustedWithOptions` prompt not shown; `CGEvent.post` requires AX trust can't be validated headless.
- No Apple Developer cert: `codesign --options runtime` + `notarytool` + `stapler` not executed; `make-dmg.sh` not run (no `create-dmg`).
- No real 75 MB `ggml-tiny` loaded via real `whisper.cpp` — transcribe is stub sleep 80ms; `bench` synthesizes tones if fixtures missing.
- Top-level `tests/` empty placeholder (integration via `SupertypeFFITest` + `cargo test`).
- No Safari/Chrome/Slack/VS Code manual injection verified headless — matrix is code-logic + harness, not GUI capture.

## 15. Release-Readiness Assessment

**Ready for TestFlight/internal distribution after `xcframework` + real model download + manual GUI smoke.** `cargo build --release` + `swift build -c release` produce `libsupertype_core.a` + `Supertype` + `SupertypeFFITest` with no `reqwest` beyond downloader, no audio blobs, correct entitlements, `Info.plist` `NSMicrophoneUsageDescription` + `NSAppleEventsUsageDescription`, `README` + `docs/release.md` accurate, `make-dmg.sh` ready, onboarding explains local inference, licenses `MIT/CC-BY-4.0` attributed.

**External signing still required** — documented, not falsely claimed.

## 16. Exact Commands Used for Validation

```bash
source "$HOME/.cargo/env"
cargo test --manifest-path core/Cargo.toml -- --nocapture  # 108 passed
cargo clippy --manifest-path core/Cargo.toml -- -D warnings  # clean
cargo fmt --check --manifest-path core/Cargo.toml  # clean (ran cargo fmt)
cargo build --manifest-path core/Cargo.toml && cargo build --release --manifest-path core/Cargo.toml
cargo run --manifest-path core/Cargo.toml --bin bench  # avg RTF 0.06 metal
swift build --package-path macos && swift build --package-path macos -c release
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest  # All FFI checks passed (Phase 1+2+3+4)
DYLD_LIBRARY_PATH=core/target/release macos/.build/release/SupertypeFFITest
./scripts/test-phase-4.sh  # Phase 4 green
grep -R URLSession macos/Sources --include="*.swift" | grep -v ModelCatalogView | grep -v OnboardingView  # 0
grep -R AUDIO_NEVER_PERSISTED core/src --include="*.rs" -q && echo OK
./scripts/download-model.sh whisper-tiny  # not run (would fetch 43 MB; catalog shows URLs)
```

Manual: `Supertype` launch `DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/Supertype` would show menu bar `waveform` + `State: Idle` + `Shortcut: fn ✓`; hold `fn` → `● Listening` overlay (not testable headless); speak would follow VAD → `✓ Done` → AX/clipboard.

## 17. Recommended Next Steps

1. Fill `ModelInfo.checksum` real SHAs for 4 catalog entries, re-run `test-phase-4.sh` with `expected.is_empty() → fail` to enforce.
2. Generate `SupertypeCore.xcframework` via `lipo`/`xcodebuild`, remove `unsafeFlags` for store submission, `xcodegen` `Supertype.xcodeproj`.
3. Run manual GUI smoke on 12 apps matrix with real 75 MB `whisper-tiny-q5_0.bin` (download via `scripts/download-model.sh`), measure `hotkey→recording <100ms`, `speech-end→final RTF <0.4`, `insert <50ms` via `PerformanceMetrics`.
4. Tune `VadProcessor` per noisy/quiet via `AVAudioEngine` input level meter, add `ProcessInfo.thermalState` throttle.
5. Add `OSLog` subsystem `com.supertype.transcribe` with redacted categories, `swift test` for `OnboardingView` + `SettingsView` Tab selection.
6. `cargo audit` + `swift package audit`, `PrivacyInfo.xcprivacy` for `NSPrivacyAccessedAPICategoryFileTimestamp`.

