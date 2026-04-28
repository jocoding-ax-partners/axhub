#!/usr/bin/env bash
# Phase 22.2 — Bun-based localhost HTTP mock-hub.
# AXHUB_ALLOW_PROXY=1 가 list-deployments.ts:140-141 에서 TLS pin 우회 시켜
# http://127.0.0.1:18080 로 라우팅 가능. helper bin 변경 0건 (P0-2).
#
# Usage: source mock-hub.sh; mock_hub_start; trap 'mock_hub_stop' EXIT

: "${MOCK_HUB_PORT:=18080}"
: "${MOCK_HUB_LOG:=${OUTPUT_DIR:-/tmp}/mock-hub.log}"
: "${MOCK_HUB_PID_FILE:=${OUTPUT_DIR:-/tmp}/mock-hub.pid}"

mock_hub_start() {
  if [ -f "$MOCK_HUB_PID_FILE" ] && kill -0 "$(cat "$MOCK_HUB_PID_FILE")" 2>/dev/null; then
    echo "[mock-hub] already running (pid $(cat "$MOCK_HUB_PID_FILE"))"
    return 0
  fi
  rm -f "$MOCK_HUB_LOG"
  bun "${CLAUDE_PLUGIN_ROOT}/tests/e2e/claude-cli/lib/mock-hub.ts" \
    --port "$MOCK_HUB_PORT" \
    --log "$MOCK_HUB_LOG" \
    > /dev/null 2>&1 &
  echo $! > "$MOCK_HUB_PID_FILE"
  # readiness probe (≤2s)
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    if curl -fsS "http://127.0.0.1:${MOCK_HUB_PORT}/_ping" >/dev/null 2>&1; then
      echo "[mock-hub] ready at http://127.0.0.1:${MOCK_HUB_PORT} (pid $(cat "$MOCK_HUB_PID_FILE"))"
      return 0
    fi
    sleep 0.2
  done
  echo "[mock-hub] failed to start" >&2
  mock_hub_stop
  return 1
}

mock_hub_stop() {
  if [ -f "$MOCK_HUB_PID_FILE" ]; then
    local pid
    pid=$(cat "$MOCK_HUB_PID_FILE")
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      sleep 0.2
      kill -9 "$pid" 2>/dev/null || true
    fi
    rm -f "$MOCK_HUB_PID_FILE"
  fi
}

mock_hub_log() {
  cat "$MOCK_HUB_LOG" 2>/dev/null || true
}
