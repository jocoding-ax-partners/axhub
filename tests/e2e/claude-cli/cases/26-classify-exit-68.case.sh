#!/usr/bin/env bash
# Case 26 (T2) — classify-exit 68 한국어 4-part.
set -u
CASE_ID="26"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_classify_exit "$CASE_ID" 68 "axhub deploy status" || true
FAIL=0
t2_assert_korean_systemmessage "$CASE_ID" "원인" || FAIL=$((FAIL + 1))
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
