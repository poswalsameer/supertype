#!/usr/bin/env bash
set -euo pipefail
# On-demand model download for Phase 2 (no UI yet; Phase 4 adds full catalog).
# Usage: ./scripts/download-model.sh whisper-tiny [whisper-base]
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODELS_DIR="$HOME/Library/Application Support/Supertype/models"
mkdir -p "$MODELS_DIR" "$ROOT/resources/models"

# URLs (ggml quantized, from whisper.cpp)
declare -A URLS
URLS[whisper-tiny]="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_0.bin"
URLS[whisper-base]="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_0.bin"
URLS[whisper-tiny-q5_0]="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_0.bin"

# Known SHA256 (optional verify; leave empty to skip)
declare -A SHAS
SHAS[whisper-tiny]=""
SHAS[whisper-base]=""

usage() {
  echo "Usage: $0 <model-id> [model-id...]"
  echo "Known ids: whisper-tiny, whisper-base"
  echo "Example: $0 whisper-tiny"
  exit 1
}

if [ $# -eq 0 ]; then usage; fi

download_one() {
  local id="$1"
  local url="${URLS[$id]:-}"
  if [ -z "$url" ]; then
    echo "Unknown model id: $id"
    echo "Known: ${!URLS[@]}"
    exit 1
  fi
  local dest="$MODELS_DIR/${id}.bin"
  local fallback="$ROOT/resources/models/${id}.bin"
  if [ -f "$dest" ]; then
    echo "Model $id already exists at $dest ($(du -h "$dest" | cut -f1))"
    # also copy to resources for bench fallback
    if [ ! -f "$fallback" ]; then cp "$dest" "$fallback" 2>/dev/null || true; fi
    return 0
  fi
  echo "Downloading $id from $url ..."
  echo " -> $dest"
  if command -v curl >/dev/null 2>&1; then
    curl -L --progress-bar -o "$dest.tmp" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$dest.tmp" "$url"
  else
    echo "Need curl or wget"; exit 1
  fi
  # Optional checksum
  local expected="${SHAS[$id]:-}"
  if [ -n "$expected" ]; then
    local actual
    if command -v shasum >/dev/null 2>&1; then actual=$(shasum -a 256 "$dest.tmp" | cut -d' ' -f1)
    elif command -v sha256sum >/dev/null 2>&1; then actual=$(sha256sum "$dest.tmp" | cut -d' ' -f1)
    else actual=""; fi
    if [ "$actual" != "$expected" ]; then
      echo "Checksum mismatch: expected $expected got $actual"
      rm -f "$dest.tmp"
      exit 1
    fi
  fi
  mv "$dest.tmp" "$dest"
  echo "Saved $(du -h "$dest" | cut -f1) to $dest"
  # also copy to resources for local tests
  cp "$dest" "$fallback" 2>/dev/null || true
  ls -lh "$dest"
}

for id in "$@"; do
  download_one "$id"
done

echo "Done. Models in $MODELS_DIR"
ls -lh "$MODELS_DIR"/*.bin 2>&1 | head -n 20
