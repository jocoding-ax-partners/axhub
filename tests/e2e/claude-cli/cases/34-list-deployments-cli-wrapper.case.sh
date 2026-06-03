#!/usr/bin/env bash
# Case 34 (T2) — list-deployments delegates to the canonical axhub CLI wrapper.
# Current helper must not own backend HTTP/TLS/auth policy. The positive evidence
# is the fixture axhub shim receiving `--json deploy list` and returning the
# canonical auth error shape through stdout.
set -u
CASE_ID="34"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
mkdir -p "$CASE_DIR/.config/axhub-plugin" "$CASE_DIR/shim"

T2_HELPER_BIN="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"
AXHUB_SHIM="${CLAUDE_PLUGIN_ROOT}/tests/e2e/claude-cli/fixtures/bin/axhub"

set +e
SHIM_CASE_DIR="${CASE_DIR}/shim" \
AXHUB_BIN="$AXHUB_SHIM" \
AXHUB_FIXTURE_DEPLOY_LIST_AUTH="invalid" \
"$T2_HELPER_BIN" list-deployments --app paydrop --limit 1 \
  > "${CASE_DIR}/stdout.json" 2> "${CASE_DIR}/stderr.log"
RC=$?
set -e
echo "$RC" > "${CASE_DIR}/exit-code"
echo "$RC" > "${CASE_DIR}/helper-exit-code"
echo "0" > "${CASE_DIR}/wall-seconds"
echo "[case ${CASE_ID}] exit=${RC}"

FAIL=0
[ "$RC" -eq 65 ] || { echo "  FAIL: list-deployments exit=$RC (expected 65 from CLI auth error)" >&2; FAIL=1; }

ERROR_CODE=$(jq -r '.error_code // empty' "${CASE_DIR}/stdout.json" 2>/dev/null)
[ "$ERROR_CODE" = "token_invalid" ] || { echo "  FAIL: stdout .error_code='${ERROR_CODE}' (expected token_invalid from CLI wrapper)" >&2; FAIL=1; }

[ -f "${CASE_DIR}/shim/shim-called" ] || { echo "  FAIL: axhub fixture shim was not invoked" >&2; FAIL=1; }

if grep -i -q -e 'tls' -e 'cert' -e 'spki' "${CASE_DIR}/stderr.log"; then
  echo "  FAIL: helper leaked legacy TLS-pin diagnostics instead of CLI wrapper output" >&2
  cat "${CASE_DIR}/stderr.log" >&2
  FAIL=1
fi

[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "0" > "${CASE_DIR}/exit-code"
echo "[case ${CASE_ID}] OK"
