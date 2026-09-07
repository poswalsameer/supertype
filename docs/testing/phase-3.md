# Phase 3 Testing — System-Wide Voice Typing

> Validates the dictation product: **global hotkey → immediate capture → VAD/ASR → formatter → injection** into focused app, plus history and overlay.

**Pass:** `85 cargo tests` green, `SupertypeFFITest` Phase 3 (`format/history/activeApp`) green, `bench` still RTF <1, `swift build` green, manual matrix across 8 apps.

---

## 1. One-command

```sh
./scripts/test-phase-3.sh
# or: ./scripts/test-phase-2.sh && DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest 2>&1 | grep -E "format|history|active"
```

---

## 2. Automated

### Core (85 tests)

```sh
cargo test --manifest-path core/Cargo.toml -- --nocapture
```

Additions over Phase 2:

- **Formatter** `formatting::tests` 11: `trims`, `comma_period`, `question_exclamation`, `new_line_paragraph`, `quotes_parens`, `spacing_and_collapse`, `capitalization`, `comma_operator_not_comma`, `dictionary_apply`, `empty`, `complex` + `transcript::tests` `lifecycle_separation`, `lifecycle_with_dict`.
- **History** `storage::tests` `history_detailed_and_fetch`, `history_delete_and_clear`, `history_search`, `migrations::history_has_app_columns`.
- **Input** `input::tests` `insertion_request_empty/valid`.
- **Engine integration** `transcription_flow_with_audio` now verifies formatted output (`Hello, world.` style) via `format_text` before `FinalTranscript`.

### FFI (Phase 3 extended harness)

```sh
cargo build --manifest-path core/Cargo.toml
swift build --package-path macos
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest
```

Expected new lines:

```
✓ format_text: Hello, world.\nThis is a test?
✓ set_active_app
✓ formatted dictation with app context
✓ history_count: 2
✓ get_history: [{"id":2,"text":"[Dummy transcript]",...,"bundle_id":"com.apple.TextEdit"...}]
✓ search_history: [{"id":2,...}]
✓ clear_history
All FFI checks passed (Phase 1 + 2 + 3)
```

### Swift

```sh
swift build --package-path macos -c release
otool -L macos/.build/debug/Supertype | grep supertype
```

Must show `Supertype` links `AVFoundation`, `ApplicationServices`, `AppKit`; `AudioCapture.swift`, `ActiveApp.swift`, `TextInjector.swift`, `HotkeyManager.swift`, `HotkeyShortcut.swift`, `HistoryView.swift` compile (no `engine_push_audio` errors).

---

## 3. Manual QA

### 3.1 Setup

```sh
macos/.build/debug/Supertype &
# Settings → General → Hotkey: “fn” (or “ctrl+space”), check ✓
# Permissions → Mic ✓, Accessibility ✓, Input Monitoring (if shortcut not firing) → grant
```

### 3.2 Global hotkey (hold-to-talk primary)

- [ ] With Supertype not focused (e.g. Safari), hold `fn` (or configured `ctrl+space`) → overlay `Listening…` appears <50 ms, `Audio capture started`.
- [ ] Keep holding → speak → overlay shows `Partial` live (if VAD speech), no flicker.
- [ ] Release → `Processing…` → `✓ Done` → text appears in Safari address bar / Slack.
- [ ] Tap `fn` quickly (<300 ms) does not start (hold threshold) or optionally toggle if mode=toggle.
- [ ] While recording, `FocusManager` captured `bundleId` (e.g. `com.apple.Safari`) and passes to Rust for history.

### 3.3 Text injection matrix (all must insert locally, no cloud)

| App | Method | Result |
|-----|--------|--------|
| Safari (address bar) | AX | ✓ |
| Chrome (omnibox) | AX | ✓ |
| Slack (message) | AX→clipboard | ✓ |
| Discord | clipboard | ✓ |
| VS Code (editor) | AX→clipboard | ✓ |
| Notion | clipboard | ✓ |
| TextEdit | AX | ✓ |
| Notes | AX | ✓ |
| Terminal | clipboard (fallback) | ✓ (or warn) |

Check clipboard preservation: copy “foo”, dictate “hello comma world period”, paste elsewhere → previous “foo” restored after 0.12 s.

### 3.4 Formatter

Dictate: `hello comma world period new line this is a test question mark`
→ Expect: 
```
Hello, world.
This is a test?
```
Check: `open quote hello close quote` → `"Hello"` (no space after opening, no space before closing). Test with dictionary: add `wisp er → Wispr` in `StorageManager` (via Rust), dictate `wisp er` → `Wispr`.

### 3.5 Overlay

- [ ] Idle hidden (no window ordered in).
- [ ] Recording `Listening…` red dot + waveform, stays above windows, does not become key, does not steal focus (`isNonactivatingPanel`).
- [ ] Processing `Processing…` orange.
- [ ] Success `✓ Done` green 0.8 s then hide.
- [ ] Error (mic denied / AX denied / model missing) shows `Mic denied` / `Accessibility required` 1.5 s.
- [ ] Partial text truncates to 40 chars, near cursor if `AXFocusedUIElement` has position, else centered.
- [ ] Across Spaces/full-screen, overlay appears on active space (`canJoinAllSpaces`).

### 3.6 History

- [ ] After dictation, Settings → History shows entry with timestamp, duration, model, app name/bundle (e.g. `TextEdit/com.apple.TextEdit`), text.
- [ ] Search filters, Delete removes one, Clear All empties.
- [ ] When `Settings → Keep transcription history` off, no new entries added (count unchanged).
- [ ] DB at `~/Library/Application Support/Supertype/supertype.db`:
```sh
sqlite3 ~/Library/Application\ Support/Supertype/supertype.db "SELECT text,bundle_id,app_name FROM transcription_history ORDER BY id DESC LIMIT 3;"
```

### 3.7 Failure handling

- [ ] Mic denied → overlay `Mic denied`, request dialog once, cancel state returns to Idle, no crash, menu bar shows error.
- [ ] AX denied → `canInsertViaAX` false → fallback clipboard succeeds, or if clipboard blocked, copy to pasteboard + overlay `Copied` + `Copy Last` menu works.
- [ ] Model missing (`whisper-tiny.bin` not found) → engine falls back to Dummy, overlay still `Done`; explicit `engine_load_model` bad path returns `Model` error code 16, overlay `Model unavailable`.
- [ ] Shortcut conflict (another app uses `ctrl+space`) → `HotkeyManager.lastError` shows, Settings shows ⚠, menu shows `⚠`.
- [ ] Focus change between down and up (user clicked another app) → injection aborted, `Focus changed — Copied` overlay, clipboard has transcript.

### 3.8 Performance / idle

- [ ] Hold → Listening <50 ms from key down.
- [ ] Release → final text <300 ms for 2 s utterance (bench RTF <1).
- [ ] Idle <0.3% CPU, no mic indicator.

---

## 4. Privacy

```sh
! grep -R "URLSession" macos/Sources --include="*.swift"
cargo test --manifest-path core/Cargo.toml no_audio_table -- --nocapture
sqlite3 ~/Library/Application\ Support/Supertype/supertype.db "SELECT sql FROM sqlite_master WHERE name='transcription_history';" | grep -v BLOB
```

Only `text,bundle_id,app_name` collected for history; raw PCM never written.

---

## 5. Troubleshooting

| Symptom | Fix |
|---------|-----|
| Hotkey not firing | Check `System Settings → Privacy → Input Monitoring` for Supertype; try `ctrl+space` instead of `fn` (fn needs `flagsChanged`). |
| `fn` registers but not hold | Use `NSEvent.addGlobalMonitorForEvents` requires app to have `LSUIElement` (we do) and be trusted for `AXIsProcessTrusted`. |
| Text not inserted | Grant Accessibility, check `canInsertViaAX` fallback, see Console `Inserted via AXValue/ClipboardFallback`. |
| History empty | Ensure `Keep transcription history` on; check `engine_get_history_count` >0. |
| Formatter not capitalizing | Check raw contains `period` word boundaries; test `engine_format_text` FFI. |
