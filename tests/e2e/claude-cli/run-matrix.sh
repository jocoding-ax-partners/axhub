#!/usr/bin/env bash
# Phase 22.1 — matrix runner. 33-case orchestrator.
# Usage:
#   bash run-matrix.sh                  # all PR-blocking (T1+T2)
#   bash run-matrix.sh --tier t1        # T1 only
#   bash run-matrix.sh --tier nightly   # full T3
#   bash run-matrix.sh --only 01        # single case
#   bash run-matrix.sh --only 01 09     # multiple cases
#   bash run-matrix.sh --persist-session # keep Claude/session sandbox between selected case steps

set -u

HERE="$(cd "$(dirname "$0")" && pwd)"
export HARNESS_LIB="${HERE}/lib"
export CLAUDE_PLUGIN_ROOT="$(cd "${HERE}/../../.." && pwd)"
export OUTPUT_DIR="${HERE}/output"
export SANDBOX_ROOT="${OUTPUT_DIR}/sandboxes"

mkdir -p "$OUTPUT_DIR" "$SANDBOX_ROOT"

# Args
TIER="pr"   # pr | t1 | t2 | nightly
ONLY=()
PERSIST_SESSION_GLOBAL=0

while [ $# -gt 0 ]; do
  case "$1" in
    --tier)
      TIER="$2"
      shift 2
      ;;
    --only)
      shift
      while [ $# -gt 0 ] && [ "${1#--}" = "$1" ]; do
        ONLY+=("$1")
        shift
      done
      ;;
    --persist-session)
      PERSIST_SESSION_GLOBAL=1
      shift
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

# Matrix selection
MATRIX="${HERE}/matrix.jsonl"
if [ ! -f "$MATRIX" ]; then
  echo "matrix.jsonl not found: $MATRIX" >&2
  exit 2
fi

PICK_IDS=()
if [ ${#ONLY[@]} -gt 0 ]; then
  PICK_IDS=("${ONLY[@]}")
else
  case "$TIER" in
    t1)
      while IFS= read -r line; do
        PICK_IDS+=("$(echo "$line" | jq -r '.id')")
      done < <(jq -c 'select(.tier=="T1")' "$MATRIX")
      ;;
    t2)
      while IFS= read -r line; do
        PICK_IDS+=("$(echo "$line" | jq -r '.id')")
      done < <(jq -c 'select(.tier=="T2")' "$MATRIX")
      ;;
    pr)
      while IFS= read -r line; do
        PICK_IDS+=("$(echo "$line" | jq -r '.id')")
      done < <(jq -c 'select(.tier=="T1" or .tier=="T2")' "$MATRIX")
      ;;
    nightly|all)
      while IFS= read -r line; do
        PICK_IDS+=("$(echo "$line" | jq -r '.id')")
      done < <(jq -c '.' "$MATRIX")
      ;;
    *)
      echo "unknown tier: $TIER" >&2
      exit 2
      ;;
  esac
fi

if [ ${#PICK_IDS[@]} -eq 0 ]; then
  echo "no cases selected (tier=${TIER})" >&2
  exit 2
fi

echo "[run-matrix] tier=${TIER} selected ${#PICK_IDS[@]} case(s): ${PICK_IDS[*]}"

# Run
SUMMARY="${OUTPUT_DIR}/summary.tsv"
echo -e "case_id\tstate\texit\twall_s" > "$SUMMARY"
TOTAL_FAIL=0
for id in "${PICK_IDS[@]}"; do
  CASE_FILE="${HERE}/cases/${id}-"*".case.sh"
  # shellcheck disable=SC2086
  CASE_FOUND=( $CASE_FILE )
  if [ ! -f "${CASE_FOUND[0]}" ]; then
    echo "[run-matrix] WARN: case ${id} script not found" >&2
    echo -e "${id}\tNOT_FOUND\t-1\t-1" >> "$SUMMARY"
    TOTAL_FAIL=$((TOTAL_FAIL + 1))
    continue
  fi
  CASE_PERSIST_SESSION="$PERSIST_SESSION_GLOBAL"
  if grep -Eq '^[[:space:]]*MULTI_STEP=1([[:space:]]|$)' "${CASE_FOUND[0]}"; then
    CASE_PERSIST_SESSION=1
  fi
  set +e
  PERSIST_SESSION="$CASE_PERSIST_SESSION" bash "${CASE_FOUND[0]}"
  CASE_RC=$?
  set -e
  if [ -f "${OUTPUT_DIR}/${id}/exit-code" ]; then
    EXIT=$(cat "${OUTPUT_DIR}/${id}/exit-code")
  else
    EXIT="-"
  fi
  if [ -f "${OUTPUT_DIR}/${id}/wall-seconds" ]; then
    WALL=$(cat "${OUTPUT_DIR}/${id}/wall-seconds")
  else
    WALL="-"
  fi
  if [ -f "${OUTPUT_DIR}/${id}/stdout.json" ]; then
    STATE=$(. "${HARNESS_LIB}/assert.sh"; classify_case_state "${OUTPUT_DIR}/${id}/stdout.json" "$EXIT")
  else
    STATE="NO_STDOUT"
  fi
  echo -e "${id}\t${STATE}\t${EXIT}\t${WALL}" >> "$SUMMARY"
  if [ "$CASE_RC" -ne 0 ]; then
    TOTAL_FAIL=$((TOTAL_FAIL + 1))
  fi
done

echo
echo "[run-matrix] summary: ${SUMMARY}"
column -t -s $'\t' "$SUMMARY"
echo
if [ "$TOTAL_FAIL" -gt 0 ]; then
  echo "[run-matrix] FAIL — ${TOTAL_FAIL} / ${#PICK_IDS[@]} case(s) failed"
  exit 1
fi
echo "[run-matrix] OK — ${#PICK_IDS[@]} / ${#PICK_IDS[@]} case(s) passed"
exit 0
