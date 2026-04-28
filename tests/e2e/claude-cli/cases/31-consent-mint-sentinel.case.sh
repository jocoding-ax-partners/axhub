#!/usr/bin/env bash
# Case 31 (T2) — consent-mint HMAC sentinel golden.
set -u
CASE_ID="31"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_consent_mint "$CASE_ID" || true

EXIT=$(cat "${OUTPUT_DIR}/${CASE_ID}/exit-code")
[ "$EXIT" -ne 0 ] && { echo "[case ${CASE_ID}] FAIL: consent-mint exit=${EXIT}"; cat "${OUTPUT_DIR}/${CASE_ID}/stderr.log" >&2; exit 1; }

STDOUT="${OUTPUT_DIR}/${CASE_ID}/stdout.json"
FAIL=0
# Mint result shape: { token_id, expires_at, file_path }
TOKEN_ID=$(jq -r '.token_id // empty' "$STDOUT" 2>/dev/null)
EXPIRES=$(jq -r '.expires_at // empty' "$STDOUT" 2>/dev/null)
FILE_PATH=$(jq -r '.file_path // empty' "$STDOUT" 2>/dev/null)
if [ -z "$TOKEN_ID" ]; then
  echo "  FAIL [${CASE_ID}]: .token_id missing" >&2
  FAIL=$((FAIL + 1))
fi
if [ -z "$EXPIRES" ]; then
  echo "  FAIL [${CASE_ID}]: .expires_at missing" >&2
  FAIL=$((FAIL + 1))
fi
if [ -z "$FILE_PATH" ]; then
  echo "  FAIL [${CASE_ID}]: .file_path missing" >&2
  FAIL=$((FAIL + 1))
fi
# token file 실제 생성 확인 (sandbox consent dir)
if [ -n "$FILE_PATH" ] && [ ! -f "$FILE_PATH" ]; then
  echo "  FAIL [${CASE_ID}]: consent token file not written: $FILE_PATH" >&2
  FAIL=$((FAIL + 1))
fi
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
