#!/usr/bin/env bash
# Case 01 (T1) — /axhub:help slash command smoke.
# Goal: claude -p subprocess 로 /axhub:help 슬래시 명령 실행 → axhub plugin 의 help 슬래시
# 가 발견 + 한국어 메뉴 출력 됨을 확인. 가장 가벼운 PR-blocking smoke.

set -u

CASE_ID="01"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"

# shellcheck source=/dev/null
. "${HARNESS_LIB}/spawn.sh"
# shellcheck source=/dev/null
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=1 spawn_claude "${CASE_ID}" "/axhub:help" 60 || true

EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"

STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL_COUNT=0

if [ "$STATE" != "PASS" ]; then
  echo "  FAIL: expected state PASS, got ${STATE}" >&2
  FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# 한국어 메뉴 phrase 검증 (axhub help SKILL 본문 확인)
if [ "$STATE" = "PASS" ]; then
  if ! assert_phrase_present "$STDOUT" "axhub"; then
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
fi

if [ "$FAIL_COUNT" -gt 0 ]; then
  echo "[case ${CASE_ID}] FAIL (${FAIL_COUNT} assertion failures)" >&2
  exit 1
fi

echo "[case ${CASE_ID}] OK"
exit 0
