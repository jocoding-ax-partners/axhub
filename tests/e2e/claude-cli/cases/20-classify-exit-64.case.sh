#!/usr/bin/env bash
# Case 20 (T2) — classify-exit 64 한국어 4-part. helper-bin direct, $0 model cost.
set -u
CASE_ID="20"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_classify_exit "$CASE_ID" 64 "axhub deploy create --slug paydrop" || true
FAIL=0
t2_assert_korean_systemmessage "$CASE_ID" "원인" || FAIL=$((FAIL + 1))
t2_assert_korean_systemmessage "$CASE_ID" "해결" || FAIL=$((FAIL + 1))
t2_assert_korean_systemmessage "$CASE_ID" "선택" || FAIL=$((FAIL + 1))
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
