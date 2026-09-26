#!/usr/bin/env bash
# Cross-compile the Rust extension for Android (arm64) and export an APK.
#
# Needs: Android NDK r28+ (ANDROID_NDK_HOME), `cargo install cargo-ndk`, and Godot 4.7.2 with
# Android export templates + an Android SDK configured in Editor Settings (see README).
# Usage: scripts/build-android.sh [--export]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to your Android NDK (r28 or newer)}"
command -v cargo-ndk >/dev/null || { echo "Install cargo-ndk first: cargo install cargo-ndk"; exit 1; }
rustup target add aarch64-linux-android >/dev/null

cd "$ROOT/rust"
# 16 KB page alignment is required by Google Play for new apps.
RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-z,max-page-size=16384" \
  cargo ndk -t arm64-v8a --platform 24 build -p proof_gd --release
mkdir -p "$ROOT/godot/bin/android/arm64"
cp target/aarch64-linux-android/release/libproof.so "$ROOT/godot/bin/android/arm64/libproof.so"
echo "Built godot/bin/android/arm64/libproof.so"

if [[ "${1:-}" == "--export" ]]; then
  GODOT="${GODOT:-$ROOT/.tools/godot}"
  mkdir -p "$ROOT/build"
  "$GODOT" --headless --path "$ROOT/godot" --export-debug "Android" "$ROOT/build/proof.apk"
  echo "Exported build/proof.apk"
fi
