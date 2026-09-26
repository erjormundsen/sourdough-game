#!/usr/bin/env bash
# Build the Rust extension for the host and copy it where proof.gdextension expects it.
# Usage: scripts/build.sh [--release]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE=debug
FLAGS=()
if [[ "${1:-}" == "--release" ]]; then PROFILE=release; FLAGS+=(--release); fi
cd "$ROOT/rust"
cargo build -p proof_gd "${FLAGS[@]}"
case "$(uname -s)" in
  Linux)  SRC="target/$PROFILE/libproof.so";   DST="$ROOT/godot/bin/linux/libproof.so" ;;
  Darwin) SRC="target/$PROFILE/libproof.dylib"; DST="$ROOT/godot/bin/macos/libproof.dylib" ;;
  *)      SRC="target/$PROFILE/proof.dll";      DST="$ROOT/godot/bin/windows/proof.dll" ;;
esac
mkdir -p "$(dirname "$DST")"
cp "$SRC" "$DST"
echo "copied $SRC -> $DST"
