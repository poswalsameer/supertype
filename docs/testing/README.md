# Testing — Supertype

Phase-gated verification for the local-first voice-to-text stack.

Each phase has its own playbook. **Only the playbook for the current phase is required to pass before moving forward.**

| Phase | Playbook | Focus |
|-------|----------|-------|
| **Phase 1** | [`phase-1.md`](./phase-1.md) | Rust core state machine, SQLite migrations, Swift ↔ Rust FFI, menu-bar shell, permissions, overlay |
| **Phase 2** | [`phase-2.md`](./phase-2.md) | Mic → ring → resample → Silero-like VAD → Whisper (local, quantized, on-demand), streaming partial/final, metrics/bench |
| **Phase 3** | [`phase-3.md`](./phase-3.md) | Global hotkey (hold-to-talk), AX/clipboard injection, ActiveApp, formatter, history, overlay |
| Phase 4 | `phase-4.md` *(not yet)* | Model catalog, downloads, Parakeet integration |
| Phase 5 | `phase-5.md` *(not yet)* | Latency/memory/battery profiling, notarization |

## Quick start (any phase)

From repo root `/`:

```sh
# 1. Automated core + FFI
./scripts/test-phase-1.sh          # Phase 1 one-click (cargo test + swift build + FFI)

# or manually:
cargo test --manifest-path core/Cargo.toml
swift build --package-path macos
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest
```

```sh
# 2. Manual QA for the running app
cat docs/qa-checklist.md            # legacy checklist (Phase 1 only, kept for reference)
cat docs/testing/phase-1.md#manual-qa  # canonical Phase 1 manual steps
```

## Conventions for all phase playbooks

- Every playbook starts with **Prerequisites** (macOS version, CLT/Xcode, Rust).
- Every playbook defines **Automated** (must be green) vs **Manual** (human checklist) sections.
- Every playbook lists **Expected results** and **Troubleshooting** so a fresh machine can verify in <5 min.
- No network is required after initial model download (Phase 4+); Phase 1 is fully offline.
- Privacy regression checks (no `URLSession`, no audio tables) are part of every phase.

## Adding the next phase

Copy `docs/testing/phase-1.md` → `phase-N.md`, keep the same headings (`Prerequisites`, `One-command`, `Automated in detail`, `Manual QA`, `Privacy/Performance`, `Troubleshooting`), and update `scripts/test-phase-N.sh` to call the new automated suite.
