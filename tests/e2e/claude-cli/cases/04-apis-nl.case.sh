#!/usr/bin/env bash
# Case 04 (T1) — 한국어 NL "axhub 앱이 어떤 API 쓸 수 있는지 보여줘" → apis SKILL.
# auth_ok fixture + axhub catalog resources --json --limit 50 응답의 current-app API catalog 요약 검증.
set -u
CASE_ID="04"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

AXHUB_PROFILE="paydrop" FIXTURE_TOKEN="token-headed.json" ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "axhub 앱이 어떤 API 쓸 수 있는지 보여줘" 90 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  grep -F -q "paydrop-public" "$STDOUT" || { echo "  FAIL: paydrop-public missing" >&2; FAIL=1; }
  grep -F -q "balance" "$STDOUT" || { echo "  FAIL: balance missing" >&2; FAIL=1; }
  if grep -F -q "다른 팀" "$STDOUT"; then
    echo "  FAIL: forbidden cross-team phrase present" >&2
    FAIL=1
  fi
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
