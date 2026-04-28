#!/usr/bin/env bash
# Case 21 (T2) — classify-exit 67 한국어 4-part.
set -u
CASE_ID="21"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_classify_exit "$CASE_ID" 67 "axhub deploy create --slug paydropx" || true
FAIL=0
t2_assert_korean_systemmessage "$CASE_ID" "원인" || FAIL=$((FAIL + 1))
t2_assert_korean_systemmessage "$CASE_ID" "선택" || FAIL=$((FAIL + 1))
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
