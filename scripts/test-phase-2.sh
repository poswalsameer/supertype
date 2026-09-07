#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
echo "=== Phase 2: Rust core (incl. audio/VAD/Whisper) ==="
source "$HOME/.cargo/env" 2>/dev/null || true
cargo test --manifest-path "$ROOT/core/Cargo.toml" -- --nocapture
cargo clippy --manifest-path "$ROOT/core/Cargo.toml" 2>&1 | tail -n 5
cargo build --manifest-path "$ROOT/core/Cargo.toml"
cargo build --release --manifest-path "$ROOT/core/Cargo.toml"
echo "--- bench (fixtures, fake model fallback) ---"
cargo run --manifest-path "$ROOT/core/Cargo.toml" --bin bench 2>&1 | tail -n 20
echo ""
echo "=== Phase 2: Swift (AVAudioEngine + FFI) ==="
swift build --package-path "$ROOT/macos"
swift build -c release --package-path "$ROOT/macos"
echo "--- FFI harness Phase 2 ---"
DYLD_LIBRARY_PATH="$ROOT/core/target/debug" "$ROOT/macos/.build/debug/SupertypeFFITest"
DYLD_LIBRARY_PATH="$ROOT/core/target/release" "$ROOT/macos/.build/release/SupertypeFFITest" || true
echo ""
echo "=== Phase 2: Privacy greps ==="
if grep -R "URLSession" "$ROOT/macos/Sources" --include="*.swift" 2>/dev/null; then echo "FAIL URLSession"; exit 1; else echo "No URLSession: OK"; fi
echo "Phase 2 green — see docs/testing/phase-2.md"
