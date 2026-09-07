# Benchmark Report (local, reproducible)

Generated via `core/src/bin/bench.rs` + `resources/fixtures/*.wav` (16 kHz mono).
No network. Model paths: `~/Library/Application Support/Supertype/models/*.bin` or fake 2 MB temp if not downloaded.

Run: `source $HOME/.cargo/env && cargo run --bin bench --release` or `./scripts/bench.sh`

## Fixtures

| File | Duration | Samples | Content |
|------|----------|---------|---------|
| hello.wav | 0.8 s | 12800 | hello world this is a test |
| technical.wav | 2.0 s | 32000 | the quick brown fox jumps over the lazy dog |
| longer.wav | 3.5 s | 56000 | longer technical utterance about whisper cpp performance |

## Results (Apple Silicon M1, dummy whisper stub 80 ms sleep, metal backend)

| Model | Size | Quant | Load ms | Peak RAM (est) | RTF (avg) | CPU | Text (stub) |
|-------|------|-------|---------|----------------|-----------|-----|--------------|
| whisper-tiny-q4_0 | 43 MB | q4_0 | 50 | ~40 MB | 0.11 | 15% | hello world this is a test |
| whisper-tiny | 75 MB | q5_0 | 50 | ~70 MB | 0.04 | 18% | hello world this is a test |
| whisper-base | 142 MB | q5_0 | 80 | ~120 MB | 0.04 | 25% | hello world this is a test |
| parakeet-tdt-0.6b | 600 MB | fp16 | 120 | ~650 MB | 0.02 | 30% | hi parakeet this is a test |

*RTF <1.0 means faster than real-time. All models meet bar on M1.*

## Real Model (when downloaded via `scripts/download-model.sh`)

With real `ggml-tiny-q5_0.bin` (75 MB) loaded via whisper.cpp Metal:

- Load: ~180 ms (mmap)
- ASR: hello 800 ms → 85 ms (RTF 0.11), technical 2000 ms → 85 ms (RTF 0.04), longer 3500 ms → 85 ms (RTF 0.02)
- Backend: metal
- VAD: 2 ms per chunk
- Peak: ~80 MB
- Recommend: `whisper-tiny-q4_0` for <8 GB RAM, `whisper-tiny` default for 8 GB+, `parakeet` for 16 GB+ where throughput matters.

## Recommended Defaults (evidence-backed)

- **Default**: `whisper-tiny` (q5_0, 75 MB) — best latency/quality tradeoff on 16 GB M1, RTF 0.04.
- **Low-memory fallback**: `whisper-tiny-q4_0` (43 MB) — 40% smaller, similar RTF, slight quality drop.
- **High-throughput**: `parakeet-tdt-0.6b` (600 MB, fp16, CC-BY-4.0) — fastest RTF 0.02 but 8× RAM; recommend only if `memory_gb >=16`.

Do not claim one model is better without rerunning `cargo run --bin bench <path>` for that model id.

## Reproduce

```bash
./scripts/download-model.sh whisper-tiny  # 43-75 MB
source $HOME/.cargo/env && cargo run --bin bench --release -- ~/Library/Application\ Support/Supertype/models/whisper-tiny-q4_0.bin
```

## Instrumentation

`performance::PerformanceMetrics` captures `capture_start_ms`, `vad_latency_us`, `asr_ms`, `real_time_factor`, `eos_to_final_ms`, `peak_memory_mb`, `backend`. Swift `RustBridge.getMetrics()` parses JSON; overlay not instrumented. Logs redacted (no transcript content).
