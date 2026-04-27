#!/usr/bin/env bash
# Phase 17 US-1707: axhub plugin statusline.
#
# Reads ~/.cache/axhub-plugin/last-deploy.json + auth token presence,
# emits ≤80 char Korean status line in 해요체 (Toss tone).
#
# Format:
#   axhub: <user_email> · <profile> · 최근 배포 <SHA8> (<status>)
#   axhub: 로그인 안 됐어요
#
# Wiring (user adds to ~/.claude/settings.json or ~/.claude/settings.local.json):
#   {
#     "statusLine": {
#       "type": "command",
#       "command": "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"
#     }
#   }
#
# Latency budget: <50ms cold (no network, file reads only).
set -u

CACHE="${HOME}/.cache/axhub-plugin/last-deploy.json"
TOKEN_FILE="${HOME}/.config/axhub-plugin/token"

# Auth presence check — env var or token file. No network.
if [ -n "${AXHUB_TOKEN:-}" ] || [ -f "$TOKEN_FILE" ]; then
  AUTH_OK=1
else
  AUTH_OK=0
fi

if [ "$AUTH_OK" = "0" ]; then
  printf 'axhub: 로그인 안 됐어요\n'
  exit 0
fi

# Last deploy from cache (jq optional — fallback to grep if missing).
if [ -f "$CACHE" ] && command -v jq >/dev/null 2>&1; then
  SHA=$(jq -r '.commit_sha // empty' "$CACHE" 2>/dev/null | head -c 8)
  STATUS=$(jq -r '.status // empty' "$CACHE" 2>/dev/null)
  APP=$(jq -r '.app_slug // empty' "$CACHE" 2>/dev/null)
elif [ -f "$CACHE" ]; then
  SHA=$(grep -o '"commit_sha":"[^"]*"' "$CACHE" 2>/dev/null | head -1 | cut -d'"' -f4 | head -c 8)
  STATUS=$(grep -o '"status":"[^"]*"' "$CACHE" 2>/dev/null | head -1 | cut -d'"' -f4)
  APP=$(grep -o '"app_slug":"[^"]*"' "$CACHE" 2>/dev/null | head -1 | cut -d'"' -f4)
fi

PROFILE="${AXHUB_PROFILE:-default}"

if [ -n "${SHA:-}" ] && [ -n "${STATUS:-}" ]; then
  printf 'axhub: %s · %s · 최근 배포 %s (%s)\n' "${APP:-?}" "$PROFILE" "$SHA" "$STATUS"
else
  printf 'axhub: %s · 배포 기록 없어요\n' "$PROFILE"
fi
exit 0
