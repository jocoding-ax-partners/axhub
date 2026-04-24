#!/usr/bin/env bash
# tests/run-corpus.sh — corpus runner for axhub plugin evaluation.
#
# Usage:
#   tests/run-corpus.sh [--mode docs-only|plugin] [--out <file>]
#
# This is a stub that documents the manual-run protocol for M0.5.
# True automated runner requires Anthropic API headless eval setup (deferred to M1.5+).
#
# Manual protocol:
#   1. For each row in tests/corpus.jsonl, paste utterance into Claude Code session
#      (with plugin enabled or disabled depending on --mode).
#   2. Capture tool calls + exit codes via Claude Code transcript export.
#   3. Format as results.json matching schema in tests/score.ts.
#   4. Run: bun tests/score.ts results.json --vs tests/baseline-results.docs-only.json
#
# Result row schema (one entry per corpus row):
#   {
#     "utterance_id": "T1",           // must match corpus id
#     "fired_skill": "apps",          // null if no skill fired
#     "actual_tool_calls": [          // ordered list of all tool calls made
#       {"cmd": "axhub apps list --json", "exit_code": 0, "ts": "2026-04-23T00:01:00Z"}
#     ],
#     "required_consent_seen": false, // true if Claude showed a consent/preview card before destructive op
#     "ts": "2026-04-23T00:01:00Z",  // when utterance was evaluated
#     "notes": "..."                  // optional human annotation
#   }
#
# M1.5+ automated runner plan:
#   - Use Anthropic API (claude --no-interactive) with frozen model + temp=0
#   - Inject each utterance as user turn
#   - Capture tool_use blocks from API response as actual_tool_calls
#   - Detect required_consent_seen by checking for AskUserQuestion tool call or
#     preview card keyword in assistant text before any destructive tool call
#   - Run 3 times per utterance, take median for stability
#   - Output JSONL trace per case to --out file
#   - Pass --out to bun tests/score.ts for automated M1.5 gate

set -euo pipefail

MODE="docs-only"
OUT_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="$2"
      shift 2
      ;;
    --out)
      OUT_FILE="$2"
      shift 2
      ;;
    --help|-h)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

PLUGIN_ROOT="${PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
CORPUS="${CORPUS:-$PLUGIN_ROOT/tests/corpus.jsonl}"
MODEL="${MODEL:-claude-sonnet-4-6}"

if [ ! -f "$CORPUS" ]; then
  echo "ERROR: corpus not found at $CORPUS" >&2
  exit 1
fi

CORPUS_ROWS=$(wc -l < "$CORPUS" | tr -d ' ')

echo "[corpus-runner] mode=$MODE corpus=$CORPUS rows=$CORPUS_ROWS model=$MODEL" >&2
echo "" >&2

if [ "$MODE" = "docs-only" ]; then
  echo "[corpus-runner] M0.5 docs-only baseline mode." >&2
  echo "[corpus-runner] Pre-scored baseline: tests/baseline-results.docs-only.json" >&2
  echo "[corpus-runner] To score: bun tests/score.ts tests/baseline-results.docs-only.json" >&2
  echo "" >&2
  echo "[corpus-runner] Manual protocol for fresh docs-only measurement:" >&2
  echo "[corpus-runner]   1. Disable this plugin: claude --no-plugin-dir" >&2
  echo "[corpus-runner]   2. Ensure agent-manual.md + CLAUDE.md are in context" >&2
  echo "[corpus-runner]   3. Run each utterance from corpus.jsonl and record results" >&2
  echo "[corpus-runner]   4. Save as results-docs-only.json matching ResultRow schema" >&2
  echo "[corpus-runner]   5. bun tests/score.ts results-docs-only.json" >&2
elif [ "$MODE" = "plugin" ]; then
  echo "[corpus-runner] M1.5 plugin mode — automated runner not yet implemented." >&2
  echo "[corpus-runner] Manual protocol:" >&2
  echo "[corpus-runner]   1. Enable plugin: claude --plugin-dir $PLUGIN_ROOT" >&2
  echo "[corpus-runner]   2. Run each utterance from corpus.jsonl in a fresh session" >&2
  echo "[corpus-runner]   3. Record tool calls + exit codes + consent_seen flag" >&2
  echo "[corpus-runner]   4. Save as results-plugin.json matching ResultRow schema" >&2
  echo "[corpus-runner]   5. bun tests/score.ts results-plugin.json --vs tests/baseline-results.docs-only.json" >&2
else
  echo "ERROR: unknown mode '$MODE'. Use docs-only or plugin." >&2
  exit 1
fi

if [ -n "$OUT_FILE" ]; then
  echo "[corpus-runner] --out $OUT_FILE specified but automated runner not yet implemented." >&2
  echo "[corpus-runner] Placeholder empty results file written." >&2
  echo "[]" > "$OUT_FILE"
fi

echo "" >&2
echo "[corpus-runner] M0.5 stub complete. See tests/README.md for full protocol." >&2
exit 0
