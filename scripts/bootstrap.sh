#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== Supertype bootstrap ==="
echo "macOS: $(sw_vers -productVersion) arch: $(uname -m)"
echo "swift: $(swift --version 2>&1 | head -n1)"
if command -v cargo >/dev/null 2>&1; then
  echo "cargo: $(cargo --version)"
else
  echo "cargo missing — installing via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
  rustup component add clippy rustfmt
fi

# XCode check
if xcodebuild -version >/dev/null 2>&1; then
  echo "xcode: $(xcodebuild -version | head -n1)"
else
  echo "note: full Xcode not found — 'swift build' via CLT will still work; install Xcode from App Store for app bundle"
fi

echo "-> building core..."
cargo build --manifest-path "$ROOT/core/Cargo.toml"
cargo test  --manifest-path "$ROOT/core/Cargo.toml" -- --nocapture

echo "-> building macOS app (SPM)..."
swift build --package-path "$ROOT/macos"

echo "bootstrap done. Run ./macos/.build/debug/Supertype to launch (menu-bar)."
