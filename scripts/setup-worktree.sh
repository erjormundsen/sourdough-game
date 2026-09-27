#!/usr/bin/env bash
# Prepare a fresh checkout / git worktree: build the Rust extension and import the Godot
# project (fonts, extension list). Godot itself is shared from the main checkout's .tools/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source <(sed -n '/^find_godot()/,/^}/p' "$ROOT/scripts/tour.sh")
GODOT="$(find_godot)"
"$ROOT/scripts/build.sh"
"$GODOT" --headless --path "$ROOT/godot" --import >/dev/null 2>&1 || true
echo "Ready: $ROOT (Godot: $GODOT)"
