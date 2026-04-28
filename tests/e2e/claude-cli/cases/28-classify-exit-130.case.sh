#!/usr/bin/env bash
# Case 28 (T2) — classify-exit 130 (Ctrl-C) 한국어 4-part.
set -u
CASE_ID="28"
. "${HARNESS_LIB}/t2-helper.sh"
t2_run_classify_exit "$CASE_ID" 130 "axhub deploy create" || true
FAIL=0
t2_assert_korean_systemmessage "$CASE_ID" "원인" || FAIL=$((FAIL + 1))
[ "$FAIL" -gt 0 ] && { echo "[case ${CASE_ID}] FAIL"; exit 1; }
echo "[case ${CASE_ID}] OK"
