#!/usr/bin/env bash
# Case 13 (T1) — 한국어 NL "누구로 로그인돼있어" + auth_missing.
# token fixture 없음 → axhub 로그인 안내 한국어 안내.
set -u
CASE_ID="13"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "axhub 에 누구로 로그인돼있어" 90 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
[ "$STATE" = "PASS" ] && assert_phrase_present "$STDOUT" "로그인" || true
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
