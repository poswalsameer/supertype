# Architecture — Supertype Phase 1

This document answers the seven prompts required for Phase 2 continuity.

## 1. Why SwiftUI

SwiftUI is used for **declarative state-driven UI** (Settings, Permissions) and not for business logic:

- Binds directly to `RustEngine`'s `@Published` state via `ObservableObject`/`Combine`, yielding minimal diffing and low idle cost (no polling views).
- `Settings {}` scene and `TabView` provide native macOS appearance (vibrancy, dark mode, accessibility) for free, without custom drawing.
- Live Previews speed up iteration on `SettingsView`/`OverlayView` even though the menu-bar app itself runs headless.
- Alternative (AppKit forms) would duplicate layout code; SwiftUI is strictly the view layer.

## 2. Why AppKit alongside SwiftUI

SwiftUI alone cannot satisfy native utility requirements:

| Need | AppKit API | Why SwiftUI insufficient |
|------|------------|--------------------------|
| Menu bar, no Dock icon | `NSStatusItem`, `NSApp.setActivationPolicy(.accessory)`, `LSUIElement` | `MenuBarExtra` is SwiftUI 13+ but lacks fine control over menu tagging and click handling |
| Floating overlay | `NSPanel` (`.floating`, `.nonactivatingPanel`, `canJoinAllSpaces`, `orderFrontRegardless`) | SwiftUI `Window` cannot float above all spaces without activation or keep `ignoresMouseEvents` |
| Permissions deep links | `NSWorkspace.open` + `AVCaptureDevice.requestAccess` / `AXIsProcessTrustedWithOptions` | Requires AppKit/Foundation bridging |
| Launch at login | `SMAppService` (ServiceManagement) | Not exposed in SwiftUI |
| Global hotkey (Phase 3) | `CGEventTap`, `NSEvent.addGlobalMonitorForEvents` | Must be AppKit/Carbon |
| Window lifecycle | `NSApplicationDelegate` (`applicationDidFinishLaunching`, `applicationWillTerminate`) | `App` protocol lifecycle is limited for accessory apps |

AppKit is therefore the **integration shell**; SwiftUI is the **content**.

## 3. Why Rust owns the core engine

- **Portability & testability:** State machine, settings validation, SQLite migrations, and future ASR traits are exercised via `cargo test` without booting macOS. Same core will later be reusable for CLI/benchmark tooling.
- **Performance & safety:** Audio/ML hot paths demand zero-copy PCM handling, deterministic allocations, and thread safety (`parking_lot::Mutex`, `OnceLock`). Rust's ownership prevents accidental copies of audio buffers.
- **Privacy by construction:** Rust core never imports `URLSession`/`Network`; the crate boundary makes cloud exfiltration a compile-time absence. The only I/O is local SQLite (`rusqlite` bundled).
- **Concurrency model:** Tokio (Phase 2) runs ASR off the UI thread; Swift calls are synchronous FFI that lock briefly then return. Never blocks the main thread.
- **Error handling:** `Result<T, EngineError>` with explicit codes (10=invalid transition, 11=not initialized, etc.) instead of Swift optionals, ensuring every failure is accounted for.

Swift therefore never contains business logic beyond view glue.

## 4. How Swift communicates with Rust

```
Swift (RustBridge)  ──C-ABI──►  Rust (supertype_core staticlib/dylib)
   engine_new()                    Box<Engine> leaked as *mut Engine
   engine_initialize(path)         Storage::open + load_settings
   engine_start_recording()        Engine::start_recording -> AppState::Recording
   engine_get_state()              AppState as u8
   engine_get_settings() -> *mut c_char  JSON, caller frees via engine_string_free
   engine_update_settings(json)    validate + save to SQLite
   engine_poll_event(out) -> 1/0   pop VecDeque<EngineEvent> as JSON
   engine_free(ptr)                Box::from_raw
   engine_version()                OnceLock<CString>
```

- **Headers:** `core/include/supertype_core.h` is wrapped as a Clang module `CSupertypeCore` (`macos/Sources/CSupertypeCore/module.modulemap`) and imported in Swift. Alternatively `RustBridge.swift` uses the same symbols via header import; `@_silgen_name` is not needed because the Clang module re-exports them.
- **Linking:** `macos/Package.swift` declares `.target(name: "CSupertypeCore")` plus `linkerSettings: [.unsafeFlags(["-L","../core/target/debug","-lsupertype_core"])]`. Debug links the dylib for iteration; release will link the staticlib.
- **Threading:** All Rust entry points lock `parking_lot::Mutex<Inner>` briefly; no async work on the caller's thread. Phase 2 adds a `tokio` runtime internal to `Engine` that never blocks the caller.
- **Events:** Phase 1 uses a `VecDeque<EngineEvent>` drained via `engine_poll_event` polled at 50 ms from a `Timer` publisher. Phase 2 will swap this for `tokio::sync::broadcast` without changing the Swift polling surface.
- **Memory:** Strings cross the boundary as `CString::into_raw` / `engine_string_free` to avoid Swift copying dangling pointers. `OpaquePointer` hides the Rust struct layout.

## 5. Responsibilities per layer

| Layer | Owns | Does NOT own |
|-------|------|--------------|
| **SwiftUI** (`UI/SettingsView`, `UI/OverlayView`) | Layout, bindings, user intent (button taps → Rust commands) | Validation, persistence, state transitions |
| **AppKit** (`App/SupertypeApp`, `Platform/*`, `Permissions/*`) | `NSStatusItem`, `NSPanel`, `SMAppService`, `AVAudio`/`AX` prompts, `NSApplicationDelegate` | Business rules, ASR, formatting |
| **RustBridge** (`Bridge/RustBridge`) | C-ABI glue, `Combine` publishers, JSON serde, `Timer` polling | Audio capture, inference |
| **Rust Core** (`core/src/engine`, `settings`, `storage`) | State machine, invariants, SQLite, event queue | Window management, global hotkeys, text insertion |
| **Future Rust** (`audio`, `transcription`, `models`) | VAD, resampling, `SpeechModel` trait, `ModelManager` | — (remains Rust) |

A file belongs to the macOS layer iff it imports `AppKit`/`SwiftUI`/`ServiceManagement`/`AVFoundation`/`ApplicationServices`. Otherwise it belongs in `core/`.

## 6. How future ASR engines plug into the core

Defined in `core/src/transcription/mod.rs`:

```rust
pub trait SpeechModel: Send + Sync {
    fn model_id(&self) -> &str;
    fn transcribe(&self, pcm: &[f32]) -> Result<TranscriptionOutput, String>;
}
pub struct TranscriptionOutput { pub text: String, pub is_final: bool, pub confidence: Option<f32> }
```

- `DummyModel` proves plumbing; Phase 2 adds `WhisperCppModel` (wrapping `whisper.cpp` via `whisper-rs`), Phase 4 adds `ParakeetModel` (MLX/Core ML).
- `core/src/models/mod.rs` already exposes `ModelInfo` and `builtin_catalog()`; Phase 4 expands `ModelManager` with `discover/download/verify/load/unload/switch/delete`, backed by `~/Library/Application Support/Supertype/models/`.
- `core/src/engine` holds a `Box<dyn SpeechModel>` (or `Arc`) set via `ModelManager::load`. The rest of the engine calls `model.transcribe(pcm)` and maps to `EngineEvent::PartialTranscript/FinalTranscript` without knowing the backend.
- Model switching is a state-guarded operation: only `Idle` → load new model → emit `model_changed`. No inference runs during switch.

## 7. How privacy is enforced architecturally

1. **No network crate.** `core/Cargo.toml` has no `reqwest`/`hyper`; `macos/Package.swift` has no `URLSession` usage outside System Settings deep links. `grep -R URLSession` should be empty post-Phase 1.
2. **Audio never persists.** `core/src/audio/mod.rs` is a marker; `storage/migrations.rs` creates no audio tables — verified by `storage::migrations::tests::no_audio_table` and `audio::AUDIO_NEVER_PERSISTED`.
3. **History opt-in.** `Settings.history_enabled` gates `Storage::push_history`; disabled → transcript not written.
4. **Log redaction.** Rust engine logs only state discriminants and error codes (`eprintln!("[supertype-core] ...")` never interpolates `text`). Production builds strip debug logs.
5. **Local persistence only.** SQLite at `~/Library/Application Support/Supertype/supertype.db` with no sync, encryption optional in Phase 5.
6. **Permissions minimal.** Microphone requested via `AVCaptureDevice.requestAccess`; Accessibility via `AXIsProcessTrustedWithOptions` only when needed (not at every launch — `AppDelegate` only warms `isTrusted()` without prompting).

These invariants are tested (see `cargo test`) and survive refactoring because they are structural, not policy docs.
