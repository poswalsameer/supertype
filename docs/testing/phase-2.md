# Phase 2 Testing — Local Speech Engine

> Verifies the streaming pipeline: **mic → ring → resample → VAD → Whisper (local) → partial/final transcripts**. No cloud.

**Pass criterion:** `67 cargo tests` green, `bench` finishes on 3 fixtures, Swift FFI harness shows `speech/partial/final` + `metrics`, and manual mic → stop → local transcript cycle works without Dock.

---

## 1. One-command

```sh
./scripts/test-phase-2.sh   # wraps phase-1 + phase-2 checks
# or: ./scripts/test-phase-1.sh && cargo run --bin bench -- 2>&1 | tail -n 20
```

---

## 2. Automated

### Core (67 tests)

```sh
cargo test --manifest-path core/Cargo.toml -- --nocapture
```

Covers:

- **Audio**: `ring_push_pop/overflow/empty`, `resample_48k_to_16k_sine`, `stereo_to_mono`, `same_rate`, `normalize`, `audio_format_conversion_integration`, `pipeline_speech_detection`, `pipeline_resample_stereo_48k`, `pipeline_silence_no_start`
- **VAD**: `vad_speech_onset_and_end`, `short_pause_not_ending`, `silence_never_starts`, `reset`, `config_defaults`
- **Transcription**: `dummy_load_unload`, `dummy_stream`, `whisper_load_and_transcribe`, `corrupted_small_file`, `missing_file`, `unload`, `cancellation`, `stream_partial_final`, `silence_empty`
- **Engine**: `push_audio_requires_recording`, `transcription_flow_with_audio`, `cancellation_discards_transcript`, `resample_and_vad_integration`, `model_load_unload`, `corrupted_model_handling` (+ 12 Phase-1 state tests)
- **Models**: `catalog_has_three`, `discover_marks_downloaded`, `checksum_verify`, `download_url_known`
- **Performance**: `benchmark_summary`, `metrics_backend`, `measure_helper`
- **Storage**: `migrations_idempotent`, `no_audio_table`, etc.

Expected: `67 passed`.

### Benchmark (on synthetic fixtures, no model download required)

```sh
cargo run --manifest-path core/Cargo.toml --bin bench 2>&1 | tail -n 20
# Create synthetic wavs via resources/fixtures/*.wav already present (hello.wav 25K, technical.wav 63K, longer.wav 109K)
# If no model at ~/Library/Application Support/Supertype/models/whisper-tiny.bin,
# bench creates a temp fake model at /tmp/supertype-bench-fake.bin (2 MB) that still simulates timing.

# Real model (on-demand, 75–142 MB):
./scripts/download-model.sh whisper-tiny
cargo run --manifest-path core/Cargo.toml --bin bench -- ~/Library/Application\ Support/Supertype/models/whisper-tiny.bin
```

Expected:

```
Loading model from "..." ...
Model loaded in 70ms (backend: metal)
Benchmark: 3 samples, avg RTF 0.04, max ASR 85ms, backend metal
  technical.wav ...  | the quick brown fox jumps over the lazy dog
  longer.wav    ...  | this is a longer technical utterance ...
  hello.wav     ...  | hello world this is a test
```

Metrics checked: `asr_ms`, `real_time_factor < 1.0` on M1, `vad_ms`, `load_ms`, `eos_to_final_ms`, `peak_memory_mb`, `backend=metal` on arm64.

### Swift FFI (Phase 2 extended)

```sh
cargo build --manifest-path core/Cargo.toml
swift build --package-path macos
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest
```

Expected new lines (beyond Phase 1):

```
✓ push_audio 16k mono (1s)
✓ push_audio_with_format 48k stereo
✓ poll events speech=true partial=true
✓ final_transcript: {"type":"FinalTranscript",...}
✓ metrics: {"asr_ms":0,...,"real_time_factor":0.0,...,"backend":"metal"}
✓ model_info: {"id":"dummy"...}
✓ cancel discards
✓ corrupted/missing model handling rc=16
All FFI checks passed (Phase 1 + 2)
```

### Swift build

```sh
swift build --package-path macos
swift build -c release --package-path macos
otool -L macos/.build/debug/Supertype | grep supertype
```

Must include `AudioCapture.swift` (AVAudioEngine) with no `engine_push_audio` errors.

---

## 3. Manual QA

Requires mic permission (grant once).

```sh
macos/.build/debug/Supertype &
# Allow mic when prompted, then:
# Menu → Start Recording (Test) → speak 2–3 s → Stop Recording
```

Checklist:

- [ ] Start: menu `Recording`, overlay `Listening…` red, `Audio capture started (native 48000 → 16k)` in Console.
- [ ] While speaking: Console shows `partial: ...` every ~1 s, no flicker, overlay stays responsive.
- [ ] Stop: `Processing…` → `✓ Done` → `final: ...` in Console, `metrics: {...}` printed, state `Completed`.
- [ ] No audio indicator after stop; `Audio capture stopped` logged.
- [ ] Cancel discards: Start → speak → Cancel → no `final` event, `last_transcript` cleared.
- [ ] `Settings → Model` shows `whisper-tiny` default; change persists across restart.
- [ ] `~/Library/Application Support/Supertype/models/` — run `download-model.sh whisper-tiny` → 75 MB file appears, `sha256` verify if checksum set.
- [ ] With real model, benchmark RTF < 1.0; with fake model, same pipeline runs (timings ~0.05).

---

## 4. Privacy

```sh
! grep -R "URLSession" macos/Sources --include="*.swift"
grep -R "AUDIO_NEVER_PERSISTED" core/src --include="*.rs" # must hit audio/mod.rs
cargo test --manifest-path core/Cargo.toml no_audio_table -- --nocapture
ls -lh ~/Library/Application\ Support/Supertype/supertype.db
sqlite3 ... "select sql from sqlite_master where name='transcription_history';" # no BLOB audio
```

---

## 5. Troubleshooting

| Symptom | Fix |
|---------|-----|
| `engine_push_audio` not found | `swift build` before `DYLD_LIBRARY_PATH` run; ensure `core/include/supertype_core.h` copied to `macos/Sources/CSupertypeCore/include/` |
| `No model found` bench | `./scripts/download-model.sh whisper-tiny` or bench auto-creates fake at `/tmp/supertype-bench-fake.bin` |
| `VAD never fires` | Check mic permission (`MicrophonePermission`), speak louder (energy threshold 0.02), see `VadConfig::speech_threshold` |
| `RTF >1.0` | Expected with fake model ~0.1; real whisper on M1 ~0.3–0.6. If >1.0 on M1, check Metal is enabled (`metrics.backend == metal`) |
| `AudioEngine start failed` | Grant mic, quit other apps holding mic, check `lsof | grep audio` |
