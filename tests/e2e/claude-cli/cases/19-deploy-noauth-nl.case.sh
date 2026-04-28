#!/usr/bin/env bash
# Case 19 (T1) — 한국어 NL "배포해" + auth_missing → deploy SKILL.
# token fixture 없음 → deploy SKILL 의 preflight auth_missing path 가 한국어 4-part 안내.
# Plan v5 line 204 spec: "token_expired → deploy → exit 65 | classify-exit emotion+cause+action".
# 22.3 범위는 auth_missing path 로 SKILL routing + 한국어 안내 검증. mock-hub 401 token_expired
# fixture 확장은 22.4 deferred — UX 결과 (한국어 카피 phrase) 동일.
set -u
CASE_ID="19"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 spawn_claude "${CASE_ID}" "배포해" 90 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  if ! grep -F -q -e "로그인" -e "토큰" -e "axhub" "$STDOUT"; then
    echo "  FAIL: no Korean auth/deploy phrase (로그인|토큰|axhub) in output" >&2
    FAIL=1
  fi
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
