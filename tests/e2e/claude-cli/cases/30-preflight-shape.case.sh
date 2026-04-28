#!/usr/bin/env bash
# Case 30 (T2) — preflight --json 출력 shape lock golden.
set -u
CASE_ID="30"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_preflight "$CASE_ID" || true

STDOUT="${OUTPUT_DIR}/${CASE_ID}/stdout.json"
FAIL=0
# Required fields per Phase 17 PreflightOutput
for field in cli_version auth_ok current_app current_env last_deploy_id last_deploy_status plugin_version; do
  if ! jq -e "has(\"${field}\")" "$STDOUT" >/dev/null 2>&1; then
    echo "  FAIL [${CASE_ID}]: preflight missing field '${field}'" >&2
    FAIL=$((FAIL + 1))
  fi
done
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
