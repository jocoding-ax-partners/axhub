#!/usr/bin/env bash
# Case 03 (T1) — 한국어 NL "내 axhub 앱 목록 보여줘" → apps SKILL.
# auth_ok (token-headed.json fixture) + mock-hub /v1/apps 응답 검증.
set -u
CASE_ID="03"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"
. "${HARNESS_LIB}/mock-hub.sh"

mock_hub_start || { echo "  FAIL: mock-hub start" >&2; exit 2; }
trap 'mock_hub_stop' EXIT INT TERM

FIXTURE_TOKEN="token-headed.json" ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "내 axhub 앱 목록 보여줘" 60 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
