#!/usr/bin/env bash
# Case 19 (T1) — 한국어 NL "paydrop 배포해" + token_expired → deploy SKILL.
# 22.4: token-headed.json fixture + AXHUB_FIXTURE_AUTH=deploy_expired → 가짜 axhub shim 의 deploy create
#       가 exit 65 + token_expired stderr → SKILL classify-exit 65 → 한국어 4-part 안내 (감정+원인+해결+선택).
# 22.3 의 auth_missing path 보다 강한 검증 — preflight 가 token 검증 통과 후 deploy 단계에서 expired 발견.
# 26.x: headless Claude subprocess 는 동의 UI/텍스트 메뉴 대신 safe default dry-run 으로
#       deploy command path 까지 진행해야 해요. 실제 --execute 파괴적 fixture 는 Rust/helper E2E 에서 별도 검증해요.
set -u
: "${CLAUDE_E2E_MODEL:=sonnet}"
: "${CLAUDE_E2E_CAP_USD:=1.50}"
export CLAUDE_E2E_MODEL CLAUDE_E2E_CAP_USD
CASE_ID="19"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

ENABLE_SLASH=0 \
AXHUB_PROFILE=paydrop \
FIXTURE_TOKEN="token-headed.json" \
FIXTURE_AXHUB_AUTH="deploy_expired" \
spawn_claude "${CASE_ID}" "paydrop 배포해" 240 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }
if [ "$STATE" = "PASS" ]; then
  RESULT_TEXT=$(jq -r '.result // ""' "$STDOUT" 2>/dev/null || true)
  # 한국어 4-part Korean classify-exit 65 phrase OR 매칭 강화 (22.4).
  # 기존 22.3: 로그인|토큰|axhub. 22.4: 만료|원인 추가 (4-part 카피의 distinctive token).
  if ! printf '%s' "$RESULT_TEXT" | grep -Eq '로그인이 만료|토큰이 만료|다시 로그인'; then
    echo "  FAIL: no Korean classify-exit 65 phrase (로그인이 만료|토큰이 만료|다시 로그인) in final result" >&2
    FAIL=1
  fi
  for forbidden in "품질 게이트" "첫 배포" "app_not_found" "진행할까요" "사전 승인이 필요" "승인 카드"; do
    if printf '%s' "$RESULT_TEXT" | grep -F -q "$forbidden"; then
      echo "  FAIL: deploy token-expired path detoured through '$forbidden'" >&2
      FAIL=1
    fi
  done
  if jq -e '.permission_denials // [] | any(.tool_name == "AskUserQuestion")' "$STDOUT" >/dev/null 2>&1; then
    echo "  FAIL: headless deploy case attempted AskUserQuestion instead of safe default" >&2
    FAIL=1
  fi
  if ! printf '%s' "$RESULT_TEXT" | grep -Eiq "exit[^0-9]{0,40}65"; then
    echo "  FAIL: final result did not surface fixture token-expired exit 65" >&2
    FAIL=1
  fi
  if ! grep -F -q -e "deploy create" -e "deploy create --" "${CASE_DIR}/axhub-argv.log" 2>/dev/null || ! grep -F -q -- "--dry-run" "${CASE_DIR}/axhub-argv.log" 2>/dev/null; then
    echo "  FAIL: fixture shim did not observe deploy create --dry-run argv" >&2
    FAIL=1
  fi
  for forbidden_tool in "auth login" "approval preview" "DEPLOY_DECISION=approve" "AXHUB_E2E_DESTRUCTIVE" "--execute"; do
    if jq -r '.permission_denials[]?.tool_input?.command? // ""' "$STDOUT" | grep -F -q -- "$forbidden_tool"; then
      echo "  FAIL: headless deploy case attempted forbidden tool detour '$forbidden_tool'" >&2
      FAIL=1
    fi
    if grep -F -q -- "$forbidden_tool" "${CASE_DIR}/axhub-argv.log" 2>/dev/null; then
      echo "  FAIL: headless deploy case attempted forbidden tool detour '$forbidden_tool'" >&2
      FAIL=1
    fi
  done
fi
[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0
