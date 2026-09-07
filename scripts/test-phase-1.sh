#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export DYLD_LIBRARY_PATH="${DYLD_LIBRARY_PATH:-}"

echo "=== Phase 1: Rust core ==="
if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then source "$HOME/.cargo/env"; fi
fi
cargo fmt --manifest-path "$ROOT/core/Cargo.toml" -- --check
cargo test --manifest-path "$ROOT/core/Cargo.toml" -- --nocapture
cargo clippy --manifest-path "$ROOT/core/Cargo.toml" -- -D warnings 2>&1 | tail -n 5
cargo build --manifest-path "$ROOT/core/Cargo.toml"
cargo build --release --manifest-path "$ROOT/core/Cargo.toml"
echo "Rust core: OK"

echo ""
echo "=== Phase 1: Swift (SPM, CLT) ==="
swift build --package-path "$ROOT/macos"
swift build -c release --package-path "$ROOT/macos"
echo "Swift build: OK"

echo ""
echo "=== Phase 1: FFI harness ==="
DYLD_LIBRARY_PATH="$ROOT/core/target/debug" "$ROOT/macos/.build/debug/SupertypeFFITest"
DYLD_LIBRARY_PATH="$ROOT/core/target/release" "$ROOT/macos/.build/release/SupertypeFFITest"
echo "FFI: OK"

echo ""
echo "=== Phase 1: Privacy greps ==="
if grep -R "URLSession" "$ROOT/macos/Sources" --include="*.swift" 2>/dev/null; then
  echo "FAIL: URLSession found in macos/Sources"; exit 1
else
  echo "No URLSession in macos/Sources: OK"
fi
if ! grep -R "AUDIO_NEVER_PERSISTED" "$ROOT/core/src" --include="*.rs" -q; then
  echo "FAIL: AUDIO_NEVER_PERSISTED marker missing"; exit 1
else
  echo "Audio never-persisted marker: OK"
fi

echo ""
echo "=== Phase 1 green ==="
echo "Manual QA: open docs/testing/phase-1.md#4 or docs/qa-checklist.md"
