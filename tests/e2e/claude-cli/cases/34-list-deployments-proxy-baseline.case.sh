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

# token 배치 — list-deployments.ts:89-100 의 tokenFromFile() 은 readFileSync(path, "utf8").trim() 후
# 결과를 그대로 Bearer 헤더에 사용. token-headed.json (구조화 JSON) 을 그대로 두면 JSON blob 이
# Bearer 값으로 들어가 HTTP header invalid 에러 (실측: "Header has invalid value: 'Bearer {\n...}'").
# SKILL flow 용 JSON fixture 와 helper-bin 용 raw bearer 는 형식이 다름 — case 34 는 후자.
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

# 22.5: mock-hub log assertion — fetch 가 실재 mock-hub 까지 도달했음을 결정적으로 검증.
# 22.4 의 stdout error_code='auth.token_invalid' 는 list-deployments.ts:280-288 path 진입 증거지만,
# mock-hub log 에 GET /api/v1/apps/42/deployments line 이 있어야 fetch URL 이 정확히 mock-hub 로
# 라우팅됐다는 결정적 evidence — DNS / 환경변수 / endpoint resolve 단계 sanity check.
MOCK_LOG="${OUTPUT_DIR}/mock-hub.log"
if [ ! -f "$MOCK_LOG" ]; then
  echo "  FAIL: mock-hub log file not found at ${MOCK_LOG}" >&2
  FAIL=1
elif ! grep -F -q 'GET /api/v1/apps/42/deployments' "$MOCK_LOG"; then
  echo "  FAIL: mock-hub log missing 'GET /api/v1/apps/42/deployments' line" >&2
  echo "  log content:" >&2
  cat "$MOCK_LOG" >&2
  FAIL=1
fi

[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
