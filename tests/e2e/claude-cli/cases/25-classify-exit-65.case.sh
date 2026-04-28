#!/usr/bin/env bash
# Case 25 (T2) — classify-exit 65 (token expired) 한국어 4-part.
set -u
CASE_ID="25"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_classify_exit "$CASE_ID" 65 "axhub apps list" || true
FAIL=0
t2_assert_korean_systemmessage "$CASE_ID" "로그인" || FAIL=$((FAIL + 1))
t2_assert_korean_systemmessage "$CASE_ID" "원인" || FAIL=$((FAIL + 1))
t2_assert_korean_systemmessage "$CASE_ID" "해결" || FAIL=$((FAIL + 1))
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
