#!/usr/bin/env bash
# tests/run-corpus.sh — corpus runner for axhub plugin evaluation.
#
# Usage:
#   tests/run-corpus.sh [--mode docs-only|plugin] [--corpus <file>] [--out <file>] [--fixture <file>] [--vs <baseline-file-or-name>] [--score]
#
# Default mode is a deterministic fixture replay runner. It copies the committed
# docs-only or claude-native result fixture for the selected corpus scope, then can
# invoke tests/score.ts (legacy --vs <file>) or tests/routing-score.ts (new
# --vs <name>). This closes the old M1.5 "manual only" gap without requiring
# live Claude/API calls in CI.
#
# Supported committed scopes (Phase 5 — Approach E):
#   - tests/corpus.20.jsonl  → tests/baseline-results.{docs-only,claude-native}.20.json
#   - tests/corpus.100.jsonl → tests/baseline-results.{docs-only,claude-native}.100.json
#
# 331-row tests/corpus.jsonl is advisory-only (no committed reliable baseline).
# `--vs <name>` against the full corpus emits an [ADVISORY] stderr line and exits 0.
#
# `--vs` modes:
#   - file path ending in .json   → forwarded to tests/score.ts as legacy baseline
#   - name (no .json suffix)      → routing-score.ts call against
#                                   tests/baseline-results.{docs-only,<name>}.${tier}.json
#                                   tier suffix auto-detected from corpus filename

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

# Phase 5 — tier detection by corpus filename (post-meta_question expansion the
# 20-row and 100-row corpora hold 24/112 rows; row counts are no longer tier
# discriminators).
TIER_SUFFIX=""
case "$CORPUS" in
  *corpus.20.jsonl) TIER_SUFFIX=".20" ;;
  *corpus.100.jsonl) TIER_SUFFIX=".100" ;;
esac

ADVISORY="0"
if [ -z "$TIER_SUFFIX" ]; then
  echo "[corpus-runner] ADVISORY ONLY: $(basename "$CORPUS") ($CORPUS_ROWS rows) has no committed reliable baseline." >&2
  echo "[corpus-runner] ADVISORY ONLY: results below are manual/advisory, NOT a CI gate." >&2
  echo "[corpus-runner] ADVISORY ONLY: use tests/corpus.20.jsonl or tests/corpus.100.jsonl for CI." >&2
  ADVISORY="1"
fi

fixture_for() {
  local mode="$1"
  local tier="$2"   # ".20", ".100", or ""
  case "$mode$tier" in
    "docs-only.20"|"docs-only.100") echo "$PLUGIN_ROOT/tests/baseline-results.docs-only${tier}.json" ;;
    "plugin.20"|"plugin.100") echo "$PLUGIN_ROOT/tests/baseline-results.claude-native${tier}.json" ;;
    *) return 1 ;;
  esac
}

baseline_for() {
  local tier="$1"
  case "$tier" in
    .20|.100) echo "$PLUGIN_ROOT/tests/baseline-results.docs-only${tier}.json" ;;
    *) return 1 ;;
  esac
}

if [ "$MODE" != "docs-only" ] && [ "$MODE" != "plugin" ]; then
  echo "ERROR: unknown mode '$MODE'. Use docs-only or plugin." >&2
  exit 1
fi

# Phase 5 — advisory tier short-circuits before fixture lookup so the full
# corpus remains manual/advisory whether scoring via routing-score (--vs name)
# or the legacy scorer with no committed fixture.
if [ "$SCORE" = "1" ] && [ "$ADVISORY" = "1" ]; then
  if [ -z "$FIXTURE" ] || { [ -n "$VS_FILE" ] && [[ "$VS_FILE" != *.json ]]; }; then
    echo "[ADVISORY] ${CORPUS_ROWS}-row corpus 의 routing-score 결과는 manual/advisory 입니다 (CI gate X). 호출 안 해요." >&2
    exit 0
  fi
fi

if [ -z "$FIXTURE" ]; then
  if ! FIXTURE=$(fixture_for "$MODE" "$TIER_SUFFIX"); then
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
  # Phase 5 — `--vs <name>` (no .json suffix) routes to routing-score.ts with
  # auto-derived baseline pair (docs-only.${tier}.json against <name>.${tier}.json).
  # The advisory short-circuit fired earlier; this block only handles tier 20/100.
  if [ -n "$VS_FILE" ] && [[ "$VS_FILE" != *.json ]]; then
    ROUTING_BASELINE="$PLUGIN_ROOT/tests/baseline-results.docs-only${TIER_SUFFIX}.json"
    ROUTING_AGAINST="$PLUGIN_ROOT/tests/baseline-results.${VS_FILE}${TIER_SUFFIX}.json"
    if [ ! -f "$ROUTING_BASELINE" ]; then
      echo "ERROR: routing baseline missing: $ROUTING_BASELINE" >&2
      exit 1
    fi
    if [ ! -f "$ROUTING_AGAINST" ]; then
      echo "ERROR: routing 'against' baseline missing: $ROUTING_AGAINST" >&2
      exit 1
    fi
    echo "[corpus-runner] routing-score: $ROUTING_BASELINE  vs  $ROUTING_AGAINST  (threshold 0.95)" >&2
    set +e
    bun "$PLUGIN_ROOT/tests/routing-score.ts" \
      --baseline "$ROUTING_BASELINE" \
      --against "$ROUTING_AGAINST" \
      --threshold 0.95
    SCORE_EXIT=$?
    set -e
    exit "$SCORE_EXIT"
  fi

  if [ -z "$VS_FILE" ] && [ "$MODE" = "plugin" ]; then
    if ! VS_FILE=$(baseline_for "$TIER_SUFFIX"); then
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
    if VS_DEFAULT=$(baseline_for "$TIER_SUFFIX" 2>/dev/null); then
      echo "[corpus-runner]   bun tests/score.ts $RESULT_FILE --corpus $CORPUS --vs $VS_DEFAULT" >&2
    else
      echo "[corpus-runner]   bun tests/score.ts $RESULT_FILE --corpus $CORPUS --vs <baseline.json>" >&2
    fi
  else
    echo "[corpus-runner]   bun tests/score.ts $RESULT_FILE --corpus $CORPUS" >&2
  fi
fi
