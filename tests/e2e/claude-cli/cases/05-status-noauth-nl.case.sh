#!/usr/bin/env bash
# Case 05 (T3) — 한국어 NL "어디까지 됐어" + auth_missing → status SKILL.
# token fixture 없음 → status SKILL 의 한국어 안내 (로그인 / 토큰 / 확인 / axhub OR 매칭).
# Plan v5 line 190 spec: "exit-65 path | empathy catalog 토큰 만료 4-part Korean rendered"
# — 22.3 범위는 SKILL routing + 한국어 안내 검증. token_expired 4-part 정확 매칭은 22.4 deferred.
set -u
CASE_ID="05"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "어디까지 됐어" 60 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  if ! grep -F -q -e "로그인" -e "토큰" -e "확인" -e "axhub" "$STDOUT"; then
    echo "  FAIL: no Korean status phrase (로그인|토큰|확인|axhub) in output" >&2
    FAIL=1
  fi
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
