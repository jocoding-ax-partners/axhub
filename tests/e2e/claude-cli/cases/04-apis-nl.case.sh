#!/usr/bin/env bash
# Case 04 (T1) — 한국어 NL → apis SKILL routing.
# scope=current_app 기본, 다른 팀 prompt 비등장 (privacy default).
set -u
CASE_ID="04"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"
. "${HARNESS_LIB}/mock-hub.sh"

mock_hub_start || { echo "  FAIL: mock-hub start" >&2; exit 2; }
trap 'mock_hub_stop' EXIT INT TERM

FIXTURE_TOKEN="token-headed.json" ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "axhub 앱이 어떤 API 쓸 수 있는지 보여줘" 60 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
