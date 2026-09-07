#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$HOME/.cargo/env" 2>/dev/null || true
echo "=== Phase 3: Rust core (formatter/history/lifecycle) ==="
cargo test --manifest-path "$ROOT/core/Cargo.toml" -- --nocapture 2>&1 | tail -n 20
cargo build --manifest-path "$ROOT/core/Cargo.toml"
cargo build --release --manifest-path "$ROOT/core/Cargo.toml" 2>&1 | tail -n 3
echo "--- bench ---"
cargo run --manifest-path "$ROOT/core/Cargo.toml" --bin bench 2>&1 | tail -n 20
echo ""
echo "=== Phase 3: Swift (hotkey/AX/formatter) ==="
swift build --package-path "$ROOT/macos" 2>&1 | tail -n 10
swift build -c release --package-path "$ROOT/macos" 2>&1 | tail -n 5
echo "--- FFI harness Phase 3 ---"
DYLD_LIBRARY_PATH="$ROOT/core/target/debug" "$ROOT/macos/.build/debug/SupertypeFFITest" 2>&1 | tail -n 30
echo ""
echo "=== Phase 3: Privacy greps ==="
if grep -R "URLSession" "$ROOT/macos/Sources" --include="*.swift" 2>/dev/null; then echo "FAIL URLSession"; exit 1; else echo "No URLSession: OK"; fi
if ! grep -R "AUDIO_NEVER_PERSISTED" "$ROOT/core/src" --include="*.rs" -q; then echo "FAIL audio marker"; exit 1; else echo "Audio marker: OK"; fi
echo "Phase 3 green — manual matrix in docs/testing/phase-3.md"
