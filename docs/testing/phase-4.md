# Phase 4 Testing — Local Model Platform

> Verifies catalog, verified downloads, Parakeet + Whisper same abstraction, hardware rec, privacy, dictionary, LLM extension.

**Pass:** `103 cargo tests` + `bench` + `FFI Phase 4` + `swift build` + manual catalog download → switch → dictate with two models.

---

## 1. One-command

```sh
./scripts/test-phase-4.sh
# or: cargo test --manifest-path core/Cargo.toml -- --nocapture | grep passed
#     DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest 2>&1 | grep "catalog\|hardware\|dictionary"
```

---

## 2. Automated

### Core (103 tests)

```sh
cargo test --manifest-path core/Cargo.toml -- --nocapture
```

New over Phase 3:

- **Catalog** `catalog_has_three` (now 4, checks tiny/parakeet), `catalog_metadata_valid` (license/runtime/size/url), `download_url_known` (tiny-q4, parakeet), `install_and_uninstall` (2 MB temp → atomic move → delete), `install_corrupted_small`, `install_checksum_mismatch`, `hardware_recommend`.
- **Hardware** `probe_has_fields`, `recommend_tiny_for_low_mem`, `recommend_parakeet_for_high_mem`.
- **Downloader** `download_file_copy_with_progress`, `checksum_mismatch`, `cancelled`, `temp_not_exposed`.
- **Parakeet** `parakeet_load_transcribe` (contains "parakeet"), `parakeet_cancellation`.
- **TextProcessor** `deterministic_processes`, `llm_fallback`, `llm_available`.
- **Input** `insertion_request_empty/valid`.

### FFI (Phase 4 extended)

```sh
cargo build --manifest-path core/Cargo.toml
swift build --package-path macos
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest
```

Expected new:

```
✓ catalog: 4 models
✓ hardware: arch aarch64 metal true
✓ recommended: parakeet-tdt-0.6b (for 32GB) / whisper-tiny (for 6GB)
✓ dictionary upsert/get/delete
✓ install/uninstall atomic
```

Add manual FFI checks:

```sh
# Catalog JSON
DYLD_LIBRARY_PATH=core/target/debug ./target/debug/deps/supertype_core-* 2>&1 | grep catalog || \
python3 -c "import subprocess, json; print('catalog test via cargo test')"
# Or via Swift harness: engine.getCatalog() → 4
```

### Swift

```sh
swift build --package-path macos -c release
otool -L macos/.build/debug/Supertype | grep -E "AVFoundation|ApplicationServices"
# URLSession only in ModelCatalogView
grep -R URLSession macos/Sources --include="*.swift" # should only hit ModelCatalogView.swift
```

---

## 3. Manual QA

### 3.1 Model catalog

```sh
macos/.build/debug/Supertype &
# Settings → Models
```

- [ ] Shows 4 cards: `Whisper Tiny Q4 (43 MB)` `Whisper Tiny (75 MB, Default, Recommended 8 GB)` `Whisper Base (142 MB)` `Parakeet TDT 0.6B (600 MB, Recommended 16 GB+)` each with description, family, runtime, quantization, languages, license, attribution, min memory, capabilities.
- [ ] Recommended badge appears for this Mac (8 GB → Tiny, 32 GB → Parakeet).
- [ ] Size + status `Installed ✓` vs `Download`, languages `en`/`multilingual`, license `MIT`/`CC-BY-4.0` visible.
- [ ] Hardware header shows `aarch64 · 8 cores · 16 GB RAM · Metal · 20 GB free`.

### 3.2 Download → verify → install → load → transcribe → unload

- [ ] Tap `Download` on `Whisper Tiny Q4` → progress bar → `Installed ✓` (verify via `ls -lh ~/Library/Application Support/Supertype/models/`), no `.part` exposed.
- [ ] Kill download mid-progress (Cancel) → `.part` removed, retry succeeds.
- [ ] Corrupt on purpose: `echo tiny > ~/Library/Application Support/Supertype/models/whisper-tiny-q4_0.bin` → `Verify` fails (checksum mismatch) → Delete.
- [ ] `install_from_temp` atomic: `temp.bin` 2 MB → `install` → `temp` gone, `final` appears, size 2 MB.
- [ ] Switch model: Settings → Models → `Set Default` on `Whisper Base` → `engine.getSettings().selectedModelId` changes, `History` new entries show `model_id=whisper-base`, `TextInjector` still works.
- [ ] Dictate with two models: hold hotkey → speak → `Whisper Tiny` → `Hello, world`; switch to `Parakeet` → `hi parakeet fast transcription` (backend difference).

### 3.3 Quantization

- [ ] Tiny Q4 (43 MB) vs Tiny Q5 (75 MB) both appear, Q4 shows `q4_0` badge, Base shows `q5_0`, Parakeet `fp16`. Download Q4 → verify 43 MB, Q5 → 75 MB.

### 3.4 Hardware recommendation

- [ ] On 8 GB Intel, recommended is Tiny; on 32 GB AS, recommended is Parakeet + Base + Tiny-q4. Logic `hardware::recommend_model_ids` matches `ModelManager::recommended_for_hardware`.

### 3.5 Privacy

- [ ] `docs/privacy.md` lists only `ModelCatalogView` + `downloader.rs` as network-capable; `cargo test no_audio_table` passes.
- [ ] Dictate offline after model downloaded (network off) → still transcribes.

### 3.6 Dictionary

- [ ] Settings → Vocabulary → Add `acme corp → Acme Corp`, dictate `acme corp` → `Acme Corp` in final text.
- [ ] Delete phrase → `acme corp` stays as is.

### 3.7 LLM extension

- [ ] Settings → Privacy → `Enhance with local LLM` toggle disabled, shows `DeterministicFormatter active`. No LLM download auto.

### 3.8 License

- [ ] Each card shows `MIT` or `CC-BY-4.0` + attribution `OpenAI Whisper` / `NVIDIA Parakeet`.
- [ ] `Whisper Tiny` MIT, `Parakeet` CC-BY-4.0 correct; `scripts/download-model.sh` comments license.

---

## 4. Troubleshooting

| Symptom | Fix |
|---------|-----|
| `Download 404` | Check `download_urls` in `builtin_catalog`, run `scripts/download-model.sh` fallback |
| `Checksum mismatch` | Delete `.part`, retry; `engine_verify_model` checks sha256 |
| `Not enough disk` | Free space, check `hardware.disk_free_gb` |
| `Parakeet not loading` | Ensure 600 MB file exists, check `parakeet` runtime, fallback to Whisper dummy if <16 GB |

## 5. Integration test

```sh
# download → verify → install → load → transcribe → unload
./scripts/download-model.sh whisper-tiny-q4_0
cargo test --manifest-path core/Cargo.toml install_and_uninstall -- --nocapture
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest 2>&1 | grep -E "catalog|hardware"
```
