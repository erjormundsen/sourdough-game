#!/usr/bin/env bash
# Headless smoke test: import the project and autoplay a few days through the real screens.
# Fails on any Rust panic or engine error. Usage: scripts/smoke.sh [days]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="${GODOT:-$ROOT/.tools/godot}"
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
