#!/usr/bin/env bash
# Phase 22.1 — 5-state classifier + assertion helpers.
# State: PASS | FAIL | SKIP | TIMEOUT | BUDGET_EXCEEDED
#
# Plan v5 SB-3 (baseline-LOCKED): cap-hit 결정적 marker = subtype "error_max_budget_usd".
# stop_reason 으로는 cap-hit / graceful abort 구분 불가 (둘 다 "end_turn") — useless.

set -u

# classify_case_state <stdout_path> <exit_code>
# echoes: PASS | FAIL | SKIP | TIMEOUT | BUDGET_EXCEEDED
classify_case_state() {
  local stdout_path="$1"
  local exit_code="$2"

  # TIMEOUT — kernel timeout signal
  if [ "$exit_code" -eq 124 ]; then
    echo "TIMEOUT"
    return 0
  fi

  # BUDGET_EXCEEDED — explicit subtype marker
  local subtype=""
  if [ -s "$stdout_path" ]; then
    subtype=$(jq -r '.subtype // empty' "$stdout_path" 2>/dev/null || echo "")
  fi
  if [ "$subtype" = "error_max_budget_usd" ]; then
    echo "BUDGET_EXCEEDED"
    return 0
  fi

  # PASS branch 1: claude --output-format json (T1) — exit 0 + is_error false
  # 주의: jq 의 `//` 가 boolean false 를 falsy 로 처리하므로 `// empty` 사용 시
  # is_error=false 가 빈 문자열로 변환됨. tostring 으로 명시 변환.
  if [ "$exit_code" -eq 0 ] && [ -s "$stdout_path" ]; then
    local is_error
    is_error=$(jq -r 'if has("is_error") then .is_error | tostring else "missing" end' "$stdout_path" 2>/dev/null || echo "missing")
    if [ "$is_error" = "false" ]; then
      echo "PASS"
      return 0
    fi
    # PASS branch 2: T2 helper-bin output — claude shape 안 가지지만 valid JSON + exit 0
    # T2 의 classify-exit / preflight / consent-mint 등은 is_error 필드 자체가 없음.
    # has("is_error") 가 false 면 helper-bin output 으로 간주, exit 0 이면 PASS.
    if [ "$is_error" = "missing" ]; then
      if jq empty "$stdout_path" >/dev/null 2>&1; then
        echo "PASS"
        return 0
      fi
    fi
  fi

  # SKIP — explicit skip flag (rare, case-spec)
  if [ -s "$stdout_path" ] && jq -e '.skip == true' "$stdout_path" >/dev/null 2>&1; then
    echo "SKIP"
    return 0
  fi

  # FAIL — everything else
  echo "FAIL"
  return 0
}

# assert_phrase_present <stdout_path> <phrase>
assert_phrase_present() {
  local stdout_path="$1"
  local phrase="$2"
  if grep -F -q -- "$phrase" "$stdout_path"; then
    return 0
  fi
  echo "  FAIL: phrase missing: '$phrase'" >&2
  return 1
}

# assert_phrase_absent <stdout_path> <phrase>
assert_phrase_absent() {
  local stdout_path="$1"
  local phrase="$2"
  if ! grep -F -q -- "$phrase" "$stdout_path"; then
    return 0
  fi
  echo "  FAIL: forbidden phrase present: '$phrase'" >&2
  return 1
}

# assert_exit_eq <exit_code> <expected>
assert_exit_eq() {
  local actual="$1"
  local expected="$2"
  if [ "$actual" -eq "$expected" ]; then
    return 0
  fi
  echo "  FAIL: exit ${actual} ≠ expected ${expected}" >&2
  return 1
}

# assert_no_system_axhub <case_dir>
# fixture shim 의 sentinel 검증 — shim 호출 시 ${case_dir}/shim-called touch.
# 시스템 /usr/local/bin/axhub 호출은 sentinel 없음 → fail.
assert_shim_used_or_skipped() {
  local case_dir="$1"
  local require_shim="${2:-0}"  # 1 = require shim was called, 0 = OK either way
  if [ "$require_shim" = "1" ]; then
    if [ -f "${case_dir}/shim-called" ]; then
      return 0
    fi
    echo "  FAIL: fixture axhub shim was not invoked (system axhub leak?)" >&2
    return 1
  fi
  return 0
}

# assert_routed_skill <stdout_path> <expected_skill>
# Routed SKILL strict 단일 매칭. messages[].content[].name == "Skill" 의 input.skill 검사.
assert_routed_skill() {
  local stdout_path="$1"
  local expected="$2"
  local actual
  actual=$(jq -r '.messages[]? | .content[]? | select(.type=="tool_use" and .name=="Skill") | .input.skill // empty' "$stdout_path" 2>/dev/null | head -1)
  if [ "$actual" = "$expected" ]; then
    return 0
  fi
  echo "  FAIL: routed skill '${actual:-<none>}' ≠ expected '${expected}'" >&2
  return 1
}
