#!/usr/bin/env bash
# Render a screenshot tour of the whole loop under a virtual display (needs Xvfb).
# Usage: scripts/tour.sh [days] [out_dir] [WxH]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DAYS="${1:-1}"
OUT="${2:-$ROOT/out/tour}"
SIZE="${3:-720x1280}"
# Find Godot: $GODOT, this checkout's .tools/, or the main checkout's .tools/ (git worktrees).
find_godot() {
  if [[ -n "${GODOT:-}" ]]; then echo "$GODOT"; return; fi
  if [[ -x "$ROOT/.tools/godot" ]]; then echo "$ROOT/.tools/godot"; return; fi
  local common
  common="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)"
  if [[ -n "$common" && -x "$common/../.tools/godot" ]]; then echo "$(cd "$common/.." && pwd)/.tools/godot"; return; fi
  echo "Godot not found: run scripts/fetch-godot.sh or set GODOT" >&2; exit 1
}
GODOT="$(find_godot)"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
rm -f "$OUT"/*.png
W="${SIZE%x*}"; H="${SIZE#*x}"
xvfb-run -a -s "-screen 0 ${W}x${H}x24" "$GODOT" --path "$ROOT/godot" --rendering-driver opengl3 \
  --audio-driver Dummy --resolution "${W}x${H}" -- --tour --days="$DAYS" --seed=7 --shots="$OUT"
