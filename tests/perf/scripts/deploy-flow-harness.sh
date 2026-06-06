#!/usr/bin/env bash
# Phase 0.5 deploy-flow harness.
#
# Simulates the deploy SKILL Step 0..5 cascade against the mock backend
# fixture so the perf walltime suite can measure full-cascade walltime
# without depending on a real axhub CLI or backend.
#
# Each step boundary calls `axhub-helpers mark <phase_name>` to record a
# marker into AXHUB_PHASE_MARKER_FILE. After Step 5 finishes the harness
# calls `axhub-helpers emit-deploy-complete` so the telemetry envelope
# captures all phase_durations_ms.
#
# Required env:
#   AXHUB_BACKEND_URL          mock backend base URL (e.g. http://127.0.0.1:1234)
#   AXHUB_PHASE_MARKER_FILE    per-run marker file path (must be unique per run)
#
# Optional env:
#   AXHUB_HELPER_BIN           path to axhub-helpers binary (default: bin/axhub-helpers[.exe])
#   AXHUB_TELEMETRY            "1" enables emit_deploy_complete writing to usage.jsonl
#   AXHUB_PERF_AUTO_APPROVE    "1" auto-confirms (no AskUserQuestion delays)
#   HARNESS_DEPLOY_ID          deploy id used in SSE status URL (default: deploy-xyz789)

set -euo pipefail

if [[ -z "${AXHUB_BACKEND_URL:-}" ]]; then
  echo "harness: AXHUB_BACKEND_URL is required" >&2
  exit 64
fi
if [[ -z "${AXHUB_PHASE_MARKER_FILE:-}" ]]; then
  echo "harness: AXHUB_PHASE_MARKER_FILE is required" >&2
  exit 64
fi

helper_bin="${AXHUB_HELPER_BIN:-}"
if [[ -z "$helper_bin" ]]; then
  if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == CYGWIN* || "$(uname -s)" == MSYS* ]]; then
    helper_bin="bin/axhub-helpers.exe"
  else
    helper_bin="bin/axhub-helpers"
  fi
fi
if [[ ! -x "$helper_bin" ]]; then
  echo "harness: helper not executable at $helper_bin" >&2
  exit 64
fi

TMP_CURL_ERR="$(mktemp -t axhub-curl-err.XXXXXX 2>/dev/null || mktemp)"
trap 'rm -f "$TMP_CURL_ERR"' EXIT

mark() {
  "$helper_bin" mark "$1" >/dev/null 2>&1 || true
}

post_json() {
  local url="$1"
  if ! curl -fsS --max-time 30 -X POST -H "content-type: application/json" \
    --data "{}" "$url" >/dev/null 2>"$TMP_CURL_ERR"; then
    echo "harness: POST failed for $url" >&2
    cat "$TMP_CURL_ERR" >&2
    exit 75
  fi
}

read_sse() {
  local url="$1"
  # Mock SSE stream starts after MOCK_BACKEND_LATENCY_MS and then finishes in
  # ~1s (4 events × 250ms). Keep the cap above the configured latency so CI
  # latency injection measures the full successful cascade instead of timing
  # out exactly as the stream begins.
  local latency_ms="${MOCK_BACKEND_LATENCY_MS:-5000}"
  case "$latency_ms" in
    ''|*[!0-9]*) latency_ms=5000 ;;
  esac
  local latency_sec=$(( (latency_ms + 999) / 1000 ))
  local max_time=$(( latency_sec + 5 ))
  if ! curl -fsS --max-time "$max_time" -H "accept: text/event-stream" \
    "$url" >/dev/null 2>"$TMP_CURL_ERR"; then
    echo "harness: SSE read failed for $url" >&2
    cat "$TMP_CURL_ERR" >&2
    exit 75
  fi
}

deploy_id="${HARNESS_DEPLOY_ID:-deploy-xyz789}"

if [[ "${AXHUB_PERF_FORCE_UNAUTH:-}" == "1" ]]; then
  mark "step_0_auth_refresh"
  post_json "$AXHUB_BACKEND_URL/api/v1/auth/refresh"
fi

mark "step_0_preflight"

mark "step_1_resolve"
post_json "$AXHUB_BACKEND_URL/api/v1/resolve"

mark "step_1_1_bootstrap_plan"
post_json "$AXHUB_BACKEND_URL/api/v1/apps"

mark "step_2_preview_card"
mark "step_3_consent"

mark "step_4_deploy_create"
post_json "$AXHUB_BACKEND_URL/api/v1/deploys"

mark "step_5_watch"
read_sse "$AXHUB_BACKEND_URL/api/v1/deploys/$deploy_id/status"

mark "step_5_watch_complete"

"$helper_bin" emit-deploy-complete 0 "axhub deploy create" >/dev/null 2>&1 || true

exit 0
