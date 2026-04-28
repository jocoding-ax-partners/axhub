#!/usr/bin/env bash
# Phase 22.2 — T2 helper-bin direct invocation helpers (no claude -p, $0 model cost).
# golden file diff style — 결정적 회귀 검증.

: "${CLAUDE_PLUGIN_ROOT:?t2-helper.sh requires CLAUDE_PLUGIN_ROOT}"
: "${OUTPUT_DIR:?t2-helper.sh requires OUTPUT_DIR}"

T2_HELPER_BIN="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"
T2_GOLDEN_DIR="${CLAUDE_PLUGIN_ROOT}/tests/e2e/claude-cli/fixtures/golden"

# t2_run_classify_exit <case_id> <exit_code> <command_str>
# stdout to ${OUTPUT_DIR}/${case_id}/stdout.json
t2_run_classify_exit() {
  local case_id="$1"
  local exit_code="$2"
  local command_str="$3"
  local case_out="${OUTPUT_DIR}/${case_id}"
  rm -rf "$case_out"
  mkdir -p "$case_out"
  local started_at
  started_at=$(date +%s)
  printf '{"tool_input":{"command":"%s"},"tool_response":{"exit_code":%s}}' "$command_str" "$exit_code" \
    | "$T2_HELPER_BIN" classify-exit > "${case_out}/stdout.json" 2> "${case_out}/stderr.log"
  local rc=$?
  echo "$rc" > "${case_out}/exit-code"
  echo "$(($(date +%s) - started_at))" > "${case_out}/wall-seconds"
  return "$rc"
}

# t2_run_redact <case_id> <input_str>
t2_run_redact() {
  local case_id="$1"
  local input_str="$2"
  local case_out="${OUTPUT_DIR}/${case_id}"
  rm -rf "$case_out"
  mkdir -p "$case_out"
  printf '%s' "$input_str" | "$T2_HELPER_BIN" redact > "${case_out}/stdout.json" 2> "${case_out}/stderr.log"
  local rc=$?
  echo "$rc" > "${case_out}/exit-code"
  echo "0" > "${case_out}/wall-seconds"
  return "$rc"
}

# t2_run_preflight <case_id>
t2_run_preflight() {
  local case_id="$1"
  local case_out="${OUTPUT_DIR}/${case_id}"
  rm -rf "$case_out"
  mkdir -p "$case_out"
  "$T2_HELPER_BIN" preflight --json > "${case_out}/stdout.json" 2> "${case_out}/stderr.log"
  local rc=$?
  echo "$rc" > "${case_out}/exit-code"
  echo "0" > "${case_out}/wall-seconds"
  return "$rc"
}

# t2_run_consent_mint <case_id>
# binding JSON 은 stdin 으로. CLAUDE_SESSION_ID env + sandbox config home 필수.
t2_run_consent_mint() {
  local case_id="$1"
  local case_out="${OUTPUT_DIR}/${case_id}"
  rm -rf "$case_out"
  mkdir -p "$case_out/.config/axhub-plugin/consent"
  local commit_sha
  commit_sha="$(printf '%040d' 0)"
  printf '{"tool_call_id":"t2-tool-%s","action":"deploy_create","app_id":"mock-app","profile":"prod","branch":"main","commit_sha":"%s"}' \
    "$case_id" "$commit_sha" \
    | CLAUDE_SESSION_ID="t2-consent-${case_id}-$$" \
      XDG_CONFIG_HOME="${case_out}/.config" \
      "$T2_HELPER_BIN" consent-mint \
        > "${case_out}/stdout.json" 2> "${case_out}/stderr.log"
  local rc=$?
  echo "$rc" > "${case_out}/exit-code"
  echo "0" > "${case_out}/wall-seconds"
  return "$rc"
}

# t2_assert_korean_systemmessage <case_id> <expected_phrase>
t2_assert_korean_systemmessage() {
  local case_id="$1"
  local expected="$2"
  local stdout="${OUTPUT_DIR}/${case_id}/stdout.json"
  local actual
  actual=$(jq -r '.systemMessage // empty' "$stdout" 2>/dev/null)
  if [ -z "$actual" ]; then
    echo "  FAIL [${case_id}]: .systemMessage missing in stdout" >&2
    return 1
  fi
  if grep -F -q -- "$expected" <<<"$actual"; then
    return 0
  fi
  echo "  FAIL [${case_id}]: phrase '$expected' missing from systemMessage" >&2
  return 1
}
