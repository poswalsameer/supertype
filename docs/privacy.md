# Privacy Audit — Supertype Phase 4

**Principle:** Inference is local. Network is only for user-initiated model downloads. No telemetry by default.

## Normal Transcription Data Flow

```
Microphone (AVAudioEngine)
  → AudioRingProducer::push_batch (rtrb, lock-free, in-memory)
  → resample_to_mono_16k (16 kHz mono f32, in-memory Vec)
  → VadProcessor (energy, in-memory)
  → WhisperCppModel / ParakeetModel::transcribe (in-memory PCM, warm Vec<u8> model)
  → format_transcript + apply_dictionary (HashMap, in-memory)
  → TextInjector (AX or NSPasteboard transient)
  → discard PCM (pending_pcm.clear on acknowledge)
```

- **Raw audio** stored only in `EngineInner::pending_pcm: Vec<f32>` (prealloc 10s) and `AudioRing` (2s SPSC). Cleared on `stop_recording` (move), `cancel_recording`, `acknowledge`. Never `std::fs::write` — `grep -R "AudioFileWrite\|fs::write.*pcm\|wav"` only in `bench.rs` for fixtures and `downloader` for models.
- **Transcripts** stored only if `Settings.history_enabled == true` (default true, user can disable). `Storage::push_history_detailed` inserts `text` + `bundle_id` only when enabled. No transcript content in logs (`eprintln!` only state/error codes).
- **No network** during inference: `VAD`, `ASR`, `formatter` are pure functions. `cargo` deps have no `reqwest` in hot path except `models/downloader.rs` (only called from `ModelCatalogView` on Download tap). `grep -R URLSession macos/Sources` allowlist only `ModelCatalogView.swift` (download) + `download-model.sh`.
- **SQLite** `~/Library/Application Support/Supertype/supertype.db` holds `settings`, `transcription_history`, `dictionary_entries` only. `migrations.rs:20` asserts no `audio` tables via `no_audio_table` test. `AudioNeverPersisted` marker `audio/mod.rs:15`.

## Network-Capable Code Paths (audit)

| Path | File | Purpose | Guard |
|------|------|---------|-------|
| `ureq::get` | `core/src/models/downloader.rs:45` | Model download (Phase 4) | Only when user taps Download, progress callback, cancelable, temp `.part` not exposed until sha verified + atomic rename via `ModelManager::install_from_temp` |
| `URLSession.downloadTask` | `macos/UI/ModelCatalogView.swift:80` | Alternative Swift download (same guard) | Only on Download tap, `CachePolicy.reloadIgnoringLocalCache`, writes to `FileManager.temporaryDirectory`, then `engine_verify_model` + `moveItem` atomic |
| `curl` fallback | `scripts/download-model.sh:1` | CLI manual download | User-initiated `./scripts/download-model.sh whisper-tiny` only |

All other `macos/Sources` (`AudioCapture`, `RustBridge`, `TextInjector`, `HistoryView`) use `AppKit/AVFoundation/ApplicationServices` only — no `URLSession`, `Network`, `WebKit`. Verified via `scripts/test-phase-4.sh` grep allowlist.

## Controls

- **Settings → Privacy**: `Save transcription history` (toggle, default true), `Save audio recordings` — UI shows `OFF` disabled, never persisted (architecture guarantees discard). Disk free check before download, `history_count` display, `Clear History` + `Reveal DB`.
- **Settings → Vocabulary**: Dictionary stored locally in `dictionary_entries`, applied before history.
- **Telemetry**: None by default. `PerformanceMetrics` stays in-memory, not sent.

## Verification

```sh
# No cloud in inference
! grep -R "URLSession" macos/Sources --include="*.swift" | grep -v ModelCatalogView
cargo test --manifest-path core/Cargo.toml no_audio_table -- --nocapture
sqlite3 ~/Library/Application\ Support/Supertype/supertype.db "SELECT sql FROM sqlite_master WHERE type='table';" # no audio
DYLD_LIBRARY_PATH=core/target/debug cargo run --bin bench -- 2>&1 | grep -v "Loading model"
# Offline after model download
./scripts/download-model.sh whisper-tiny && network_off && macos/.build/debug/Supertype &  # still transcribes
```

## Model Download Privacy

- HTTPS only (`https://huggingface.co/...`), streaming `8192` chunks, `sha256` incremental, `cancel` via `AtomicBool`, `resume` via `Range` if `.part` exists, temp file not exposed until `hex::encode(sha)` matches expected (empty `checksum` skips, but catalog will fill real sha before distribution), `download_progress` callback, `uninstall` deletes file, `disk-space` check via `hardware::probe().disk_free_gb`.
