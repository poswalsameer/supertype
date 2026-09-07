# Phase 1 Testing — Native macOS Foundation

> Tests the skeleton that all later phases build on: **Rust state machine, settings + SQLite, Swift ↔ Rust FFI, menu-bar app, permissions, overlay**. No real transcription yet — that is Phase 2.

**Pass criterion:** every step in *Automated* is green and every checkbox in *Manual QA* can be ticked on a clean macOS 14+ machine with only CLT + Rust.

---

## 1. Prerequisites

| Requirement | Check | Install if missing |
|-------------|-------|--------------------|
| macOS 14+ | `sw_vers` | Upgrade via System Settings |
| Command Line Tools | `xcode-select -p` | `xcode-select --install` (full Xcode optional for Phase 1; App Store → Xcode + `sudo xcodebuild -license accept` only needed for `.xcodeproj` bundle) |
| Rust stable | `cargo --version` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh -s -- -y && source $HOME/.cargo/env && rustup component add clippy rustfmt` |
| Repo at root | `ls core/Cargo.toml macos/Package.swift` | `git clone … && cd supertype` |

All commands below assume **repo root** (`…/supertype`).

---

## 2. One-command verification (copy-paste)

```sh
./scripts/test-phase-1.sh
```

Script does: `cargo fmt --check` → `cargo test` (30 tests) → `cargo clippy` → `cargo build` (debug + release) → `swift build` (debug + release) → `SupertypeFFITest` (Swift→Rust FFI) → privacy greps. Expected exit `0` with `All FFI checks passed`.

If you prefer manual steps, continue below.

---

## 3. Automated tests (must be green)

### 3.1 Rust core — state machine, settings, storage, events

```sh
source "$HOME/.cargo/env"   # if cargo not on PATH
cargo test --manifest-path core/Cargo.toml -- --nocapture
```

**What it covers** (`core/src/**/tests`):

- `engine::state` — `roundtrip_u8`, `valid_transitions` (`Idle→Recording→Processing→Completed→Idle`), `invalid_transitions` (e.g. `Idle→Processing` rejected)
- `engine` — `happy_path_recording_flow`, `cancel_from_recording`, `invalid_double_start`, `invalid_stop_when_idle`, `error_injection` + `acknowledge`, `not_initialized_guard`, `double_init_rejected`, `settings_persistence_via_engine`, `event_emission`
- `settings` — `defaults_are_valid`, `serialization_roundtrip`, `custom_settings_roundtrip`, `empty_shortcut_rejected`
- `storage` — `migrations_idempotent` (re-run safe), `no_audio_table` (privacy), `settings_persistence_roundtrip`, `settings_default_when_missing`, `history_push_and_count`, `dictionary_upsert`, `file_persistence` (reopen DB)
- `transcription` — `dummy_model_transcribes`, `models::catalog_has_three`, `formatting::trims`

**Expected:** `30 passed; 0 failed`.

### 3.2 Formatting & lint

```sh
cargo fmt --manifest-path core/Cargo.toml -- --check   # no diff
cargo clippy --manifest-path core/Cargo.toml -- -D warnings  # 0 errors (only doc-comment warnings allowed)
```

### 3.3 Swift build (SPM, works with CLT)

```sh
swift build --package-path macos                        # debug
swift build -c release --package-path macos             # release

# Verify linking (should show libsupertype_core):
otool -L macos/.build/debug/Supertype | grep supertype
otool -L macos/.build/release/Supertype | grep supertype
nm -g macos/.build/debug/Supertype | grep engine_get_state  # U _engine_get_state
```

**Expected:** `Build complete!`, `otool` shows `…/core/target/debug/libsupertype_core.dylib` (debug) and `…/release/…` (release).

### 3.4 Swift ↔ Rust FFI integration harness

Built as `SupertypeFFITest` (`macos/Sources/SupertypeFFITest/main.swift:1` — uses `engine_new`/`engine_initialize_in_memory`/`engine_get_state`/`engine_start_recording`/`engine_stop_recording`/`engine_acknowledge`/`engine_get_settings`/`engine_update_settings`/`engine_poll_event`).

```sh
# (re)build Rust if you changed core/
cargo build --manifest-path core/Cargo.toml
cargo build --release --manifest-path core/Cargo.toml

# run harness (DYLD path required because we link the dylib)
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest
DYLD_LIBRARY_PATH=core/target/release macos/.build/release/SupertypeFFITest
```

**Expected:**

```
✓ initial state idle
✓ recording
✓ completed
✓ ack to idle
✓ get_settings: {"selected_microphone_id":null,…
✓ update_settings
✓ poll_event: {"type":"StateChanged",…
All FFI checks passed
```

### 3.5 Privacy & architecture greps (regression guards)

```sh
# No cloud networking in app sources
! grep -R "URLSession" macos/Sources --include="*.swift"  # should exit 1 (no matches)

# No audio persistence tables (besides the intentional grep in migrations.rs test)
grep -R "AUDIO_NEVER_PERSISTED" core/src --include="*.rs"   # must find audio/mod.rs

# Migrations test already asserts no table named %audio%
cargo test --manifest-path core/Cargo.toml no_audio_table -- --nocapture
```

---

## 4. Manual QA (human checklist)

Launch the menu-bar app. All steps assume you already ran `cargo build` + `swift build`.

### 4.1 Launch & menu bar

```sh
# Run in background; LSUIElement means no Dock icon
macos/.build/debug/Supertype &
ps aux | grep "[S]upertype"   # one process
```

- [ ] No Dock icon, waveform icon appears on the right side of menu bar.
- [ ] Clicking icon shows `State: Idle`, `Settings…`, `Permissions…`, `Start/Stop Recording (Test)`, `Cancel`, `Quit`.
- [ ] Menu `State:` label updates live when you use the Test items.

Kill with menu `Quit` or `kill %1`. Termination logs `Terminated cleanly.` to stdout.

### 4.2 Settings persistence

1. Open `Settings…` (TabView 560×420: General / Permissions / Models / Privacy).
2. Change `Global Shortcut` → `ctrl+space` and `Model` → `whisper-base`.
3. Quit and relaunch (`macos/.build/debug/Supertype &`).
4. Reopen Settings — values retained.

Verify on disk:

```sh
ls -lh ~/Library/Application\ Support/Supertype/supertype.db
sqlite3 ~/Library/Application\ Support/Supertype/supertype.db \
  "SELECT value FROM settings WHERE key='app_settings';" | python3 -m json.tool
sqlite3 ~/Library/Application\ Support/Supertype/supertype.db ".tables"
# → dictionary_entries  settings  transcription_history
sqlite3 ~/Library/Application\ Support/Supertype/supertype.db \
  "SELECT count(*) FROM sqlite_master WHERE name LIKE '%audio%';"
# → 0
```

### 4.3 Permissions

- [ ] Settings → Permissions tab shows `Granted / Denied / Not Determined` (mic via `AVCaptureDevice.authorizationStatus`, AX via `AXIsProcessTrusted`).
- [ ] `Request Microphone Access` prompts at most once; `Open System Settings` opens `x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone`.
- [ ] Accessibility `Open System Settings` opens `…Privacy_Accessibility`; app does **not** re-prompt on every launch (only when button pressed).

### 4.4 Recording state (mock, no real mic yet)

Via menu `Start/Stop Recording (Test)` (or future hold-to-talk):

- [ ] `Start` → menu `State: Recording`, overlay `Listening…` (red dot) centered, material, rounded 14.
- [ ] `Stop` → overlay `Processing…` → `✓ Done` → auto-hides after ~0.8 s, state returns to `Idle`.
- [ ] `Cancel` from Recording returns to `Idle` and hides overlay.
- [ ] Invalid action (Stop when Idle) does not crash — Rust returns code `10` and menu stays `Idle`.

### 4.5 Overlay window invariants

- [ ] Overlay is `NSPanel` level `.floating`, material, `canJoinAllSpaces`, stays on all spaces, does not activate app, `ignoresMouseEvents = true`.
- [ ] When `Settings → Enable recording overlay` is **off**, no overlay appears even after Start.

### 4.6 Performance / idle

With app idle for 30 s:

- [ ] Activity Monitor: < 0.3% CPU, < 40 MB RAM.
- [ ] No orange mic indicator in menu bar.
- [ ] `lsof -p $(pgrep Supertype) | grep -i audio` shows no audio device held.
- [ ] Offline: disconnect Wi-Fi, relaunch — app starts, settings load, no `nettop`/`lsof -i` traffic.

---

## 5. Troubleshooting

| Symptom | Fix |
|---------|-----|
| `cargo: command not found` | `source $HOME/.cargo/env` or reopen terminal |
| `xcode-select: error: tool 'xcodebuild' requires Xcode` | Phase 1 only needs CLT — `xcode-select --install` is sufficient; install full Xcode only for `.xcodeproj` |
| `swift build` fails `no such module 'CSupertypeCore'` | Ensure `macos/Sources/CSupertypeCore/include/supertype_core.h` exists; `cargo build` first to generate `core/include/` |
| `SupertypeFFITest` fails `Library not loaded: libsupertype_core.dylib` | Prefix with `DYLD_LIBRARY_PATH=core/target/debug` (or `release`) |
| `engine_initialize` returns `-13` (storage) | Remove stale DB: `rm ~/Library/Application\ Support/Supertype/supertype.db` and relaunch |
| `cargo test` fails `no_audio_table` | You added an audio table to migrations — remove it; Phase 1 must not persist audio |
| Overlay never shows | Check `Enable recording overlay` is on; `AppSettings.overlayEnabled` defaults `true` |
| `otool -L` missing supertype | `Package.swift:24` `unsafeFlags` links `../core/target/debug/libsupertype_core` — ensure `cargo build` ran |

---

## 6. What is *not* tested in Phase 1 (out of scope)

- Real microphone capture, resampling, VAD, whisper.cpp/Parakeet inference, partial/final transcripts (Phase 2).
- Global hold-to-talk hotkey, AX text insertion, clipboard fallback (Phase 3).
- Model catalog/downloads, checksum verification, switching (Phase 4).
- Latency benchmarks, battery profiling, `whisper.cpp` quantized vs Parakeet tradeoffs (Phase 5).

Phase 2 playbook will add `cargo bench`, `AVAudioEngine` capture tests, and streaming transcript harness under `docs/testing/phase-2.md`.

---

## 7. CI equivalent (for contributors)

```sh
set -e
source "$HOME/.cargo/env"
cargo fmt --check --manifest-path core/Cargo.toml
cargo test --manifest-path core/Cargo.toml
cargo clippy --manifest-path core/Cargo.toml -- -D warnings
cargo build --manifest-path core/Cargo.toml
cargo build --release --manifest-path core/Cargo.toml
swift build --package-path macos
swift build -c release --package-path macos
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest
! grep -R "URLSession" macos/Sources --include="*.swift"
echo "Phase 1 green"
```

See also: legacy checklist `docs/qa-checklist.md` (kept for reference; canonical steps are here).
