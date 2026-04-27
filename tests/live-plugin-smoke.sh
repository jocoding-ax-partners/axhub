#!/usr/bin/env bash
# Phase 12 US-1201/1202 — subprocess claude live plugin smoke harness.
# Spawns `claude -p '<slash command>'` for each axhub command, captures
# stdout/stderr/exit, writes per-command evidence files + summary table.
set -u

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EVIDENCE_DIR="$REPO_ROOT/.omc/evidence"
SUMMARY="$EVIDENCE_DIR/live-plugin-smoke-summary.txt"
mkdir -p "$EVIDENCE_DIR"

COMMANDS=(
  "/axhub:help"
  "/axhub:status"
  "/axhub:doctor"
  "/axhub:apps"
  "/axhub:apis"
  "/axhub:login"
  "/axhub:logs"
  "/axhub:update"
  "/axhub:deploy --dry-run"
)

{
  echo "Live plugin smoke — subprocess claude -p"
  echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Plugin: axhub@axhub (directory mode → /Users/wongil/Desktop/work/jocoding/axhub)"
  echo ""
  printf "%-30s %-10s %s\n" "command" "exit" "evidence file"
  echo "------------------------------------------------------------------------"
} > "$SUMMARY"

for cmd in "${COMMANDS[@]}"; do
  slug=$(echo "$cmd" | tr -dc 'a-zA-Z0-9' | head -c 30)
  evidence="$EVIDENCE_DIR/live-plugin-smoke-$slug.txt"
  echo "[run] claude -p '$cmd' → $evidence" >&2
  {
    echo "=== command: $cmd ==="
    echo "=== timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
    echo ""
  } > "$evidence"
  # 60s budget per command (some skills make API calls)
  claude -p "$cmd" >> "$evidence" 2>&1 &
  pid=$!
  for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24; do
    sleep 5
    if ! kill -0 $pid 2>/dev/null; then break; fi
  done
  if kill -0 $pid 2>/dev/null; then kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null; exit_code="TIMEOUT"; else wait $pid; exit_code=$?; fi
  printf "%-30s %-10s %s\n" "$cmd" "$exit_code" "$(basename "$evidence")" >> "$SUMMARY"
done

echo "" >> "$SUMMARY"
echo "Summary written to $SUMMARY"
cat "$SUMMARY"
