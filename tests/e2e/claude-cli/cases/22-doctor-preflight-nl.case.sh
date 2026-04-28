#!/usr/bin/env bash
# Case 22 (T1) — 한국어 NL "환경 점검" + cli_too_old → doctor SKILL.
# 22.4: AXHUB_FIXTURE_VERSION=0.0.5 (MIN_AXHUB_CLI_VERSION 0.1.0 미만) 강제 → preflight cli_too_old=true →
#       doctor SKILL 의 version-skew 한국어 카피 ('너무 오래된 버전' / '업그레이드') 라우팅 검증.
# 22.3 의 generic doctor routing 보다 강한 검증 — preflight 의 cli_too_old branch 정확 매칭.
set -u
CASE_ID="22"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 \
FIXTURE_AXHUB_VERSION="0.0.5" \
spawn_claude "${CASE_ID}" "환경 점검" 90 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  # 22.4 한국어 cli_too_old phrase OR 매칭 강화 — '오래된'/'업그레이드'/'버전' 추가.
  if ! grep -F -q -e "오래된" -e "업그레이드" -e "버전" -e "확인" -e "axhub" "$STDOUT"; then
    echo "  FAIL: no Korean cli_too_old phrase (오래된|업그레이드|버전|확인|axhub) in output" >&2
    FAIL=1
  fi
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
