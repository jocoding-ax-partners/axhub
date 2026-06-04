#!/usr/bin/env bash
# Case 05 (T3) — 한국어 NL "axhub 배포 어디까지 됐어" → status SKILL.
# fixture CLI 가 배포 목록/상태를 반환하면 repo 진행 상황이 아니라 axhub deploy status 를 답해야 해요.
set -u
: "${CLAUDE_E2E_MODEL:=sonnet}"
: "${CLAUDE_E2E_CAP_USD:=1.50}"
export CLAUDE_E2E_MODEL CLAUDE_E2E_CAP_USD
CASE_ID="05"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "axhub 배포 어디까지 됐어" 90 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  RESULT_TEXT=$(jq -r '.result // ""' "$STDOUT" 2>/dev/null || true)
  STATUS_OK=0
  AUTH_OK=0
  if printf '%s' "$RESULT_TEXT" | grep -Eq '배포|상태' \
    && printf '%s' "$RESULT_TEXT" | grep -Eq 'live|running|완료|성공'; then
    STATUS_OK=1
  fi
  if printf '%s' "$RESULT_TEXT" | grep -Eq '로그인|토큰|auth'; then
    AUTH_OK=1
  fi
  if [ "$STATUS_OK" -ne 1 ] && [ "$AUTH_OK" -ne 1 ]; then
    echo "  FAIL: final result was neither deploy status nor auth guidance" >&2
    FAIL=1
  fi
  if [ "$STATUS_OK" -eq 1 ] && ! grep -F -q "deploy list" "${CASE_DIR}/axhub-argv.log" 2>/dev/null; then
    echo "  FAIL: status success path did not observe deploy list argv" >&2
    FAIL=1
  fi
  if grep -F -q -e "git status" -e "git 상태" -e "변경사항" -e "토큰 예산" -e "예산이 소진" -e "specs/006" -e "PLAN.md" "$STDOUT"; then
    echo "  FAIL: answered repo progress instead of axhub deploy status" >&2
    FAIL=1
  fi
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
