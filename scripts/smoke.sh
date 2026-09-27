#!/usr/bin/env bash
# Headless smoke test: import the project and autoplay a few days through the real screens.
# Fails on any Rust panic or engine error. Usage: scripts/smoke.sh [days]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
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
DAYS="${1:-3}"
"$GODOT" --headless --path "$ROOT/godot" --import >/dev/null 2>&1 || true
LOG="$(mktemp)"
set +e
"$GODOT" --headless --audio-driver Dummy --path "$ROOT/godot" -- --autoplay="$DAYS" --seed=11 >"$LOG" 2>&1
CODE=$?
set -e
grep -E "Proof" "$LOG" || true
if [[ $CODE -ne 0 ]] || grep -qE "panicked|SCRIPT ERROR|^ERROR" "$LOG"; then
  echo "smoke test FAILED (exit $CODE)"; grep -E "panicked|ERROR" -A3 "$LOG" | head -40; exit 1
fi
echo "smoke test passed"
