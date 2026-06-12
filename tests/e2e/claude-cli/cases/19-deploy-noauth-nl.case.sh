#!/usr/bin/env bash
# Case 19 (T1) — 한국어 NL "paydrop 배포해" + token_expired → post-diet deploy SKILL.
# post-diet flow: Step 1 guard 가 `axhub plugin-support preflight --json` 으로 게이트하고 (auth_ok=true —
#       토큰은 preflight 시점엔 fine), Step 1.1 이 `axhub plugin-support deploy-prep` 으로 resolve+preflight 를
#       한 번에 받아요. 이후 token-gate / deploy-approved-run / deploy create 가 실행되는 순간 저장된 토큰이
#       만료된 게 드러나 exit 65 → SKILL 이 classify-exit 65 → 한국어 4-part 안내 (감정+원인+해결+선택).
# 검증 강도: preflight 통과(auth_ok=true) 후 실행 단계에서 expired 발견 — auth_missing 보다 강한 path.
# headless: Claude subprocess 는 동의 UI/텍스트 메뉴 대신 safe default 로 preview-confirm 없이 진행해요.
# Visibility: 새 SKILL Visibility 룰상 raw exit code / 내부 helper 명(plugin-support) 은 사용자 result 에
#       절대 노출되면 안 돼요 — 사용자에겐 한국어 한 줄 안내만.
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
  # Visibility 룰: raw exit-code jargon 이 사용자 result 에 노출되면 FAIL (감정+원인+해결 한국어만).
  if printf '%s' "$RESULT_TEXT" | grep -Eiq "exit[^0-9]{0,40}65"; then
    echo "  FAIL: final result leaked raw exit-code jargon (exit 65) to user-facing text" >&2
    FAIL=1
  fi
  # 내부 helper surface 명(plugin-support) 도 사용자 result 에 새면 안 돼요.
  if printf '%s' "$RESULT_TEXT" | grep -F -q "plugin-support"; then
    echo "  FAIL: internal helper surface name 'plugin-support' leaked to user-facing result" >&2
    FAIL=1
  fi
  # post-diet flow: guard preflight + deploy-prep 가 실제로 실행됐는지 argv 로 검증.
  if ! grep -F -q -- "plugin-support preflight" "${CASE_DIR}/axhub-argv.log" 2>/dev/null; then
    echo "  FAIL: fixture shim did not observe 'plugin-support preflight' argv (Step 1 guard skipped)" >&2
    FAIL=1
  fi
  if ! grep -F -q -- "plugin-support deploy-prep" "${CASE_DIR}/axhub-argv.log" 2>/dev/null; then
    echo "  FAIL: fixture shim did not observe 'plugin-support deploy-prep' argv (Step 1.1 prep skipped)" >&2
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
