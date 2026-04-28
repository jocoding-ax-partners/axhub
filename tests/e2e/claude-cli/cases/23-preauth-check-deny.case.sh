#!/usr/bin/env bash
# Case 23 (T2) — preauth-check direct deny golden.
# Plan v5 line 208 spec: "T1 consent gate bypass" — direct axhub deploy create Bash → PreToolUse hook BLOCKS.
# 22.3 pivot: T1 → T2. PreToolUse hook = axhub-helpers preauth-check 단일 호출이므로
# stdin 직접 주입이 claude-p orchestration 보다 결정적 + $0 cost. 동일 보안 surface 검증.
#
# Verifies: parseAxhubCommand → is_destructive=true (deploy create)
#           → verifyToken → no_consent_token (token file absent in sandbox)
#           → permissionDecision="deny" + systemMessage 한국어 ("사전 승인")
set -u
CASE_ID="23"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
mkdir -p "$CASE_DIR"

T2_HELPER_BIN="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"

# Empty isolated XDG dirs — consent token 파일 부재 보장.
EMPTY_STATE="${CASE_DIR}/empty-state"
EMPTY_RUNTIME="${CASE_DIR}/empty-runtime"
mkdir -p "$EMPTY_STATE" "$EMPTY_RUNTIME"

PAYLOAD='{"session_id":"t2-preauth-23","tool_call_id":"t2-call-23","tool_name":"Bash","tool_input":{"command":"axhub deploy create --app paydrop --branch main --commit a3f9c1b"}}'

set +e
printf '%s' "$PAYLOAD" \
  | CLAUDE_SESSION_ID="t2-preauth-23" \
    XDG_STATE_HOME="$EMPTY_STATE" \
    XDG_RUNTIME_DIR="$EMPTY_RUNTIME" \
    "$T2_HELPER_BIN" preauth-check \
    > "${CASE_DIR}/stdout.json" 2> "${CASE_DIR}/stderr.log"
RC=$?
set -e
echo "$RC" > "${CASE_DIR}/exit-code"
echo "0" > "${CASE_DIR}/wall-seconds"
echo "[case ${CASE_ID}] exit=${RC}"

FAIL=0
[ "$RC" -eq 0 ] || { echo "  FAIL: preauth-check exit=$RC (expected 0 — hook always returns 0)" >&2; FAIL=1; }

DECISION=$(jq -r '.hookSpecificOutput.permissionDecision // empty' "${CASE_DIR}/stdout.json" 2>/dev/null)
[ "$DECISION" = "deny" ] || { echo "  FAIL: permissionDecision='${DECISION}' (expected 'deny')" >&2; FAIL=1; }

SYSMSG=$(jq -r '.systemMessage // empty' "${CASE_DIR}/stdout.json" 2>/dev/null)
# 22.5: systemMessage 전문 (src/axhub-helpers/index.ts:262-263) lock — distinctive token AND 매칭.
# "이 명령은 사전 승인이 필요해요. 먼저 'paydrop 배포해'라고 말해서 승인 카드를 받으세요."
# 4 token 모두 매칭해야 통과 — production 메시지 drift 시 case 23 fail.
for phrase in "사전 승인" "필요해요" "paydrop 배포해" "승인 카드"; do
  if ! printf '%s' "$SYSMSG" | grep -F -q "$phrase"; then
    echo "  FAIL: systemMessage missing '${phrase}' phrase. got: ${SYSMSG}" >&2
    FAIL=1
  fi
done

[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
