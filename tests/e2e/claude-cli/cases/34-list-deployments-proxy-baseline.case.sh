#!/usr/bin/env bash
# Case 34 (T2) — list-deployments AXHUB_ALLOW_PROXY=1 baseline (TLS pin 우회 path).
# Plan v5 line 219 spec: golden-file diff: TLS pin 우회 baseline path
#   (list-deployments.ts:109 proxyOverrideEnabled → :140-144 verifyHubApiTlsPin early return) 활성화 확인.
#
# 22.3 minimal verification: token 부재 sandbox + AXHUB_ALLOW_PROXY=1 + mock-hub URL.
# 1) list-deployments 가 EXIT_LIST_AUTH=65 반환 (resolveToken null path)
# 2) stderr 에 'tls'|'cert'|'spki' phrase 부재 — TLS pin 코드가 proxyOverrideEnabled() 에서 early-return.
#    pin 활성 시 hub-api.jocodingax.ai 호스트 검사 + SPKI hash 계산 단계로 진입했을 텐데, mock-hub 는
#    127.0.0.1 라 hostname mismatch — 우회 미활성 시 TLS connect timeout 또는 endpoint_invalid 에러 leak 가능.
set -u
CASE_ID="34"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
mkdir -p "$CASE_DIR/.config/axhub-plugin"

. "${HARNESS_LIB}/mock-hub.sh"
mock_hub_start || { echo "  FAIL: mock-hub start" >&2; exit 2; }
trap 'mock_hub_stop' EXIT INT TERM

T2_HELPER_BIN="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"

set +e
XDG_CONFIG_HOME="${CASE_DIR}/.config" \
AXHUB_ALLOW_PROXY=1 \
AXHUB_ENDPOINT="${MOCK_HUB_URL:-http://127.0.0.1:18080}" \
"$T2_HELPER_BIN" list-deployments --app paydrop --limit 1 \
  > "${CASE_DIR}/stdout.json" 2> "${CASE_DIR}/stderr.log"
RC=$?
set -e
echo "$RC" > "${CASE_DIR}/exit-code"
echo "0" > "${CASE_DIR}/wall-seconds"
echo "[case ${CASE_ID}] exit=${RC}"

FAIL=0
# token 없으니 EXIT_LIST_AUTH=65 expected.
[ "$RC" -eq 65 ] || { echo "  FAIL: list-deployments exit=$RC (expected 65 EXIT_LIST_AUTH)" >&2; FAIL=1; }
# TLS pin 코드는 proxy override 로 skip 되어야 함. cert / tls / spki 단어 stderr leak 차단.
if grep -i -q -e 'tls' -e 'cert' -e 'spki' "${CASE_DIR}/stderr.log"; then
  echo "  FAIL: TLS pin error phrase leaked despite AXHUB_ALLOW_PROXY=1" >&2
  cat "${CASE_DIR}/stderr.log" >&2
  FAIL=1
fi

[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
