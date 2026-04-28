#!/usr/bin/env bash
# Case 02 (T1) — /axhub:doctor slash. 환경 진단 + 한국어 출력 smoke.
set -u
CASE_ID="02"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=1 spawn_claude "${CASE_ID}" "/axhub:doctor" || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
[ "$STATE" = "PASS" ] && assert_phrase_present "$STDOUT" "axhub" || true
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
