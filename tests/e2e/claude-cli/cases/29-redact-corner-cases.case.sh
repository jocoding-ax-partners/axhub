#!/usr/bin/env bash
# Case 29 (T2) — redact() corner cases golden. Bearer token + axhub_pat_ + base64 + ANSI strip.
set -u
CASE_ID="29"
. "${HARNESS_LIB}/t2-helper.sh"

INPUT='Bearer abc123xyz789longertoken
axhub_pat_test_secret_token_456789
sk-1234567890abcdefABCDEF
ANSI \033[31m red \033[0m text'

t2_run_redact "$CASE_ID" "$INPUT" || true

EXIT=$(cat "${OUTPUT_DIR}/${CASE_ID}/exit-code")
[ "$EXIT" -ne 0 ] && { echo "[case ${CASE_ID}] FAIL: redact exit=${EXIT}"; exit 1; }

STDOUT="${OUTPUT_DIR}/${CASE_ID}/stdout.json"
FAIL=0
# axhub_pat_ 가 그대로 남으면 fail (redact 가 처리해야 함)
if grep -F -q "axhub_pat_test_secret_token_456789" "$STDOUT"; then
  echo "  FAIL [${CASE_ID}]: axhub_pat_ raw token leaked" >&2
  FAIL=$((FAIL + 1))
fi
# Bearer abc... 도 redacted 되어야 함
if grep -F -q "abc123xyz789longertoken" "$STDOUT"; then
  echo "  FAIL [${CASE_ID}]: Bearer token leaked" >&2
  FAIL=$((FAIL + 1))
fi
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
