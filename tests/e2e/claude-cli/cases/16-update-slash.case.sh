#!/usr/bin/env bash
# Case 16 (T1) — /axhub:update slash. 버전 확인 표시 (no binary swap).
set -u
CASE_ID="16"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=1 spawn_claude "${CASE_ID}" "/axhub:update" 60 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
