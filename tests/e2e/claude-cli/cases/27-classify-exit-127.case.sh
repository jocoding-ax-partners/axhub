#!/usr/bin/env bash
# Case 27 (T2) — classify-exit 127 한국어 4-part.
set -u
CASE_ID="27"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_classify_exit "$CASE_ID" 127 "axhub apps list" || true
FAIL=0
t2_assert_korean_systemmessage "$CASE_ID" "원인" || FAIL=$((FAIL + 1))
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
