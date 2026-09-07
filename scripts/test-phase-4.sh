#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$HOME/.cargo/env" 2>/dev/null || true
echo "=== Phase 4: Rust core (catalog/downloader/parakeet/hardware/dict/llm) ==="
cargo test --manifest-path "$ROOT/core/Cargo.toml" -- --nocapture 2>&1 | tail -n 25
cargo build --manifest-path "$ROOT/core/Cargo.toml" 2>&1 | tail -n 5
cargo build --release --manifest-path "$ROOT/core/Cargo.toml" 2>&1 | tail -n 3
echo "--- bench ---"
cargo run --manifest-path "$ROOT/core/Cargo.toml" --bin bench 2>&1 | tail -n 15
echo ""
echo "=== Phase 4: Swift (catalog/hardware/dictionary) ==="
swift build --package-path "$ROOT/macos" 2>&1 | tail -n 10
swift build -c release --package-path "$ROOT/macos" 2>&1 | tail -n 5
echo "--- FFI harness Phase 4 ---"
DYLD_LIBRARY_PATH="$ROOT/core/target/debug" "$ROOT/macos/.build/debug/SupertypeFFITest" 2>&1 | tail -n 30
echo ""
echo "=== Phase 4: Privacy audit ==="
if grep -R "URLSession" "$ROOT/macos/Sources" --include="*.swift" 2>/dev/null | grep -v "ModelCatalogView.swift" | grep -v "OnboardingView.swift" | grep -q "URLSession"; then
  echo "FAIL: URLSession outside allowlist"; grep -R "URLSession" "$ROOT/macos/Sources" --include="*.swift"; exit 1
else
  echo "URLSession only in ModelCatalogView/OnboardingView: OK (audit docs/privacy.md)"
fi
if ! grep -R "AUDIO_NEVER_PERSISTED" "$ROOT/core/src" --include="*.rs" -q; then echo "FAIL audio marker"; exit 1; else echo "Audio marker: OK"; fi
echo "Phase 4 green — see docs/testing/phase-4.md"
echo "Catalog:"
DYLD_LIBRARY_PATH="$ROOT/core/target/debug" python3 -c "import subprocess, json; import os; os.environ['DYLD_LIBRARY_PATH']='core/target/debug'; print('catalog test via cargo test already')" 2>&1 | tail -n 5
