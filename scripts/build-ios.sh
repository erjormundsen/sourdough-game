#!/usr/bin/env bash
# Build the Rust extension for iOS devices and wrap it as a framework (macOS + Xcode only).
#
# gdext's iOS support is community-maintained; see README for caveats.
# Usage: scripts/build-ios.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
[[ "$(uname -s)" == "Darwin" ]] || { echo "iOS builds need macOS with Xcode."; exit 1; }
rustup target add aarch64-apple-ios >/dev/null
cd "$ROOT/rust"
cargo build -p proof_gd --release --target aarch64-apple-ios
FW="$ROOT/godot/bin/ios/libproof.ios.framework"
mkdir -p "$FW"
cp target/aarch64-apple-ios/release/libproof.dylib "$FW/libproof.ios"
install_name_tool -id "@rpath/libproof.ios.framework/libproof.ios" "$FW/libproof.ios"
cat > "$FW/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>libproof.ios</string>
  <key>CFBundleIdentifier</key><string>org.proofbakery.libproof</string>
  <key>CFBundleName</key><string>libproof</string>
  <key>CFBundlePackageType</key><string>FMWK</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>MinimumOSVersion</key><string>13.0</string>
</dict></plist>
PLIST
echo "Built $FW — now export the iOS preset from Godot and open the Xcode project."
