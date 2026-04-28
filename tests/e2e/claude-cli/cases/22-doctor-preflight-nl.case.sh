#!/usr/bin/env bash
# Case 22 (T1) — 한국어 NL "환경 점검" → doctor SKILL.
# Plan v5 line 207 spec: "doctor → halt | non-zero | version-skew copy" (cli_too_old=true).
# 22.3 범위는 doctor SKILL routing + preflight 출력 한국어 안내 phrase 검증.
# cli_too_old stub 강제 (preflight injection 변경 필요) 는 22.4 deferred.
set -u
CASE_ID="22"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "환경 점검" 90 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  if ! grep -F -q -e "확인" -e "버전" -e "axhub" "$STDOUT"; then
    echo "  FAIL: no Korean doctor phrase (확인|버전|axhub) in output" >&2
    FAIL=1
  fi
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
