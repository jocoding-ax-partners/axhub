#!/usr/bin/env bash
# tests/run-corpus.sh — corpus runner for axhub plugin evaluation.
#
# Usage:
#   tests/run-corpus.sh [--mode docs-only|plugin] [--corpus <file>] [--out <file>] [--fixture <file>] [--vs <baseline-file>] [--score]
#
# Default mode is a deterministic fixture replay runner. It copies the committed
# docs-only or plugin-arm result fixture for the selected corpus scope, then can
# invoke tests/score.ts. This closes the old M1.5 "manual only" gap without
# requiring live Claude/API calls in CI.
#
# Supported committed scopes:
#   - tests/corpus.20.jsonl  → tests/{baseline-results.docs-only,plugin-arm-results}.json
#   - tests/corpus.100.jsonl → tests/{baseline-results.docs-only,plugin-arm-results}.100.json
#
# For the full 331-row corpus, pass --fixture explicitly after a fresh manual or
# external eval run. The runner refuses to synthesize fake results.

set -euo pipefail

MODE="docs-only"
OUT_FILE=""
CORPUS=""
FIXTURE=""
VS_FILE=""
SCORE="0"

usage() {
  grep '^#' "$0" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    --corpus)
      CORPUS="${2:-}"
      shift 2
      ;;
    --out)
      OUT_FILE="${2:-}"
      shift 2
      ;;
    --fixture)
      FIXTURE="${2:-}"
      shift 2
      ;;
    --vs)
      VS_FILE="${2:-}"
      shift 2
      ;;
    --score)
      SCORE="1"
      shift
      ;;
    --help|-h)
      usage
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

# Phase 0 sub-task 0.2 (Approach E): 331-row baseline incomplete → manual/advisory only.
# 20-row + 100-row 만 reliable CI gate.
ADVISORY="0"
if [ "$CORPUS_ROWS" != "20" ] && [ "$CORPUS_ROWS" != "100" ]; then
  echo "[corpus-runner] ADVISORY ONLY: $CORPUS_ROWS-row corpus has no committed reliable baseline." >&2
  echo "[corpus-runner] ADVISORY ONLY: results below are manual/advisory, NOT a CI gate." >&2
  echo "[corpus-runner] ADVISORY ONLY: use tests/corpus.20.jsonl or tests/corpus.100.jsonl for CI." >&2
  ADVISORY="1"
fi

fixture_for() {
  local mode="$1"
  local rows="$2"
  case "$mode:$rows" in
    docs-only:20) echo "$PLUGIN_ROOT/tests/baseline-results.docs-only.json" ;;
    docs-only:100) echo "$PLUGIN_ROOT/tests/baseline-results.docs-only.100.json" ;;
    plugin:20) echo "$PLUGIN_ROOT/tests/plugin-arm-results.json" ;;
    plugin:100) echo "$PLUGIN_ROOT/tests/plugin-arm-results.100.json" ;;
    *) return 1 ;;
  esac
}

baseline_for() {
  local rows="$1"
  case "$rows" in
    20) echo "$PLUGIN_ROOT/tests/baseline-results.docs-only.json" ;;
    100) echo "$PLUGIN_ROOT/tests/baseline-results.docs-only.100.json" ;;
    *) return 1 ;;
  esac
}

if [ "$MODE" != "docs-only" ] && [ "$MODE" != "plugin" ]; then
  echo "ERROR: unknown mode '$MODE'. Use docs-only or plugin." >&2
  exit 1
fi

if [ "$SCORE" = "1" ] && [ "$ADVISORY" = "1" ] && [ -z "$FIXTURE" ]; then
  echo "[ADVISORY] ${CORPUS_ROWS}-row corpus has no committed reliable fixture; score is manual/advisory (CI gate X)." >&2
  exit 0
fi

if [ -z "$FIXTURE" ]; then
  if ! FIXTURE=$(fixture_for "$MODE" "$CORPUS_ROWS"); then
    echo "ERROR: no committed $MODE fixture for $CORPUS_ROWS-row corpus." >&2
    echo "ERROR: pass --corpus tests/corpus.20.jsonl, --corpus tests/corpus.100.jsonl, or --fixture <results.json>." >&2
    exit 2
  fi
fi

if [ ! -f "$FIXTURE" ]; then
  echo "ERROR: fixture not found at $FIXTURE" >&2
  exit 1
fi

RESULT_FILE="$FIXTURE"

echo "[corpus-runner] mode=$MODE corpus=$CORPUS rows=$CORPUS_ROWS model=$MODEL" >&2
echo "[corpus-runner] fixture=$FIXTURE" >&2

if [ -n "$OUT_FILE" ]; then
  mkdir -p "$(dirname "$OUT_FILE")"
  cp "$FIXTURE" "$OUT_FILE"
  RESULT_FILE="$OUT_FILE"
  echo "[corpus-runner] wrote replay results to $OUT_FILE" >&2
fi

if [ "$SCORE" = "1" ]; then
  if [ -z "$VS_FILE" ] && [ "$MODE" = "plugin" ]; then
    if ! VS_FILE=$(baseline_for "$CORPUS_ROWS"); then
      echo "ERROR: no committed docs-only baseline for $CORPUS_ROWS-row corpus; pass --vs <baseline.json>." >&2
      exit 2
    fi
  fi

  if [ "$MODE" = "plugin" ]; then
    echo "[corpus-runner] scoring plugin arm against baseline $VS_FILE" >&2
    set +e
    bun "$PLUGIN_ROOT/tests/score.ts" "$RESULT_FILE" --corpus "$CORPUS" --vs "$VS_FILE"
    SCORE_EXIT=$?
    set -e
    # Phase 0 sub-task 0.2: advisory mode forces exit 0 to keep 331-row out of CI gate.
    if [ "$ADVISORY" = "1" ] && [ "$SCORE_EXIT" -ne 0 ]; then
      echo "[corpus-runner] ADVISORY ONLY: score exit $SCORE_EXIT suppressed (not a CI gate)." >&2
      exit 0
    fi
    exit "$SCORE_EXIT"
  else
    echo "[corpus-runner] scoring docs-only baseline (informational; GO/KILL applies to plugin arm)" >&2
    set +e
    bun "$PLUGIN_ROOT/tests/score.ts" "$RESULT_FILE" --corpus "$CORPUS"
    SCORE_EXIT=$?
    set -e
    if [ "$SCORE_EXIT" -ne 0 ]; then
      echo "[corpus-runner] docs-only scorer exited $SCORE_EXIT; treating as baseline signal, not runner failure" >&2
    fi
  fi
else
  echo "[corpus-runner] replay complete. To score:" >&2
  if [ "$MODE" = "plugin" ]; then
    if VS_DEFAULT=$(baseline_for "$CORPUS_ROWS" 2>/dev/null); then
      echo "[corpus-runner]   bun tests/score.ts $RESULT_FILE --corpus $CORPUS --vs $VS_DEFAULT" >&2
    else
      echo "[corpus-runner]   bun tests/score.ts $RESULT_FILE --corpus $CORPUS --vs <baseline.json>" >&2
    fi
  else
    echo "[corpus-runner]   bun tests/score.ts $RESULT_FILE --corpus $CORPUS" >&2
  fi
fi
