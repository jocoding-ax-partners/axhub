#!/usr/bin/env bash
# Case 19 (T1) — 한국어 NL "배포해" + token_expired → deploy SKILL.
# 22.4: token-headed.json fixture + AXHUB_FIXTURE_AUTH=expired → 가짜 axhub shim 의 deploy create
#       가 exit 65 + token_expired stderr → SKILL classify-exit 65 → 한국어 4-part 안내 (감정+원인+해결+선택).
# 22.3 의 auth_missing path 보다 강한 검증 — preflight 가 token 검증 통과 후 deploy 단계에서 expired 발견.
set -u
CASE_ID="19"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 \
FIXTURE_TOKEN="token-headed.json" \
FIXTURE_AXHUB_AUTH="expired" \
spawn_claude "${CASE_ID}" "배포해" 90 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  # 한국어 4-part Korean classify-exit 65 phrase OR 매칭 강화 (22.4).
  # 기존 22.3: 로그인|토큰|axhub. 22.4: 만료|원인 추가 (4-part 카피의 distinctive token).
  if ! grep -F -q -e "로그인" -e "토큰" -e "만료" -e "원인" -e "axhub" "$STDOUT"; then
    echo "  FAIL: no Korean classify-exit 65 phrase (로그인|토큰|만료|원인|axhub) in output" >&2
    FAIL=1
  fi
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
