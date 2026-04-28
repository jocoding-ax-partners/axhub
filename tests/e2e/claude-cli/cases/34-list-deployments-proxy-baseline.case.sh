#!/usr/bin/env bash
# Case 34 (T2) — list-deployments AXHUB_ALLOW_PROXY=1 + mock-hub 401 positive evidence.
# 22.4: token-headed.json fixture + numeric --app 42 + MOCK_HUB_AUTH_FAIL=1 → list-deployments 가
#       mock-hub 까지 fetch 도달 → 401 응답 → exit_code 65 + stdout JSON .error_code='auth.token_invalid'.
# proxyOverrideEnabled() 의 verifyHubApiTlsPin early-return 이 actually engaged 됐다는 결정적 evidence:
#   - TLS pin 활성 시 hostname mismatch (127.0.0.1 vs hub-api.jocodingax.ai) 로 fetch 도달 불가
#   - mock-hub 가 401 응답 받았다 = TLS pin 코드 skip 됐다
# 22.3 의 token absent path (exit 65 + tls phrase 부재 baseline) 보다 강한 verification.
set -u
CASE_ID="34"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
mkdir -p "$CASE_DIR/.config/axhub-plugin"

. "${HARNESS_LIB}/mock-hub.sh"
MOCK_HUB_AUTH_FAIL=1 mock_hub_start || { echo "  FAIL: mock-hub start" >&2; exit 2; }
trap 'mock_hub_stop' EXIT INT TERM

# token 배치 — list-deployments resolveToken 은 file content 를 raw bearer 로 trim/사용.
# token-headed.json (구조화 JSON) 는 SKILL flows 용; helper bin 직접 호출엔 raw token string 필요.
printf 'axhub_pat_test_HEADED_TOKEN_DO_NOT_USE_IN_PROD\n' > "${CASE_DIR}/.config/axhub-plugin/token"
chmod 600 "${CASE_DIR}/.config/axhub-plugin/token"

T2_HELPER_BIN="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"

set +e
XDG_CONFIG_HOME="${CASE_DIR}/.config" \
AXHUB_ALLOW_PROXY=1 \
AXHUB_ENDPOINT="${MOCK_HUB_URL:-http://127.0.0.1:18080}" \
"$T2_HELPER_BIN" list-deployments --app 42 --limit 1 \
  > "${CASE_DIR}/stdout.json" 2> "${CASE_DIR}/stderr.log"
RC=$?
set -e
echo "$RC" > "${CASE_DIR}/exit-code"
echo "0" > "${CASE_DIR}/wall-seconds"
echo "[case ${CASE_ID}] exit=${RC}"

FAIL=0
# mock-hub 401 응답 → list-deployments.ts:281-288 path → EXIT_LIST_AUTH=65.
[ "$RC" -eq 65 ] || { echo "  FAIL: list-deployments exit=$RC (expected 65 from mock-hub 401)" >&2; FAIL=1; }

# Positive evidence (22.4 신규): stdout JSON .error_code 가 'auth.token_invalid' (401 path).
# 22.3 의 'auth.token_missing' (token 부재 path) 와 구별 — mock-hub 가 401 응답한 결정적 증거.
ERROR_CODE=$(jq -r '.error_code // empty' "${CASE_DIR}/stdout.json" 2>/dev/null)
[ "$ERROR_CODE" = "auth.token_invalid" ] || { echo "  FAIL: stdout .error_code='${ERROR_CODE}' (expected 'auth.token_invalid' — mock-hub 401 도달 + proxy override engaged 결정적 증거)" >&2; FAIL=1; }

# 22.3 보존: TLS pin 코드 진입 없었다 (proxyOverrideEnabled early-return).
if grep -i -q -e 'tls' -e 'cert' -e 'spki' "${CASE_DIR}/stderr.log"; then
  echo "  FAIL: TLS pin error phrase leaked despite AXHUB_ALLOW_PROXY=1" >&2
  cat "${CASE_DIR}/stderr.log" >&2
  FAIL=1
fi

[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
