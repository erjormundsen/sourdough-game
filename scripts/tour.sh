#!/usr/bin/env bash
# Render a screenshot tour of the whole loop under a virtual display (needs Xvfb).
# Usage: scripts/tour.sh [days] [out_dir] [WxH]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DAYS="${1:-1}"
OUT="${2:-$ROOT/out/tour}"
SIZE="${3:-720x1280}"
GODOT="${GODOT:-$ROOT/.tools/godot}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
rm -f "$OUT"/*.png
W="${SIZE%x*}"; H="${SIZE#*x}"
xvfb-run -a -s "-screen 0 ${W}x${H}x24" "$GODOT" --path "$ROOT/godot" --rendering-driver opengl3 \
  --audio-driver Dummy --resolution "${W}x${H}" -- --tour --days="$DAYS" --seed=7 --shots="$OUT"
