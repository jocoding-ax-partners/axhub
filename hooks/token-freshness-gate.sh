#!/usr/bin/env bash
# Phase 3.5 B-08 — Token freshness gate for SKILL deploy Step 3.5.
#
# Spec: .plan/deploy-time-reduction/phase-3-client-cascade-reduced.md §3.4.
#
# Compares the local axhub token file mtime against a Step 3.5 entry timestamp
# (now minus 30 s slack — matches spec §3.4 AUTH_BUNDLE_TS local capture).
# When the token mtime is newer the gate exits 0 immediately. Otherwise it
# polls 5 s × 6 iterations (max 30 s) waiting for the SessionStart-spawned
# `auth-refresh-bg` to land a fresh token. After timeout it does an inline
# `axhub auth status --json` check; UNAUTHORIZED → exit 65 routing.
#
# Env contract:
#   AXHUB_AUTH_BG_REFRESH=0     → silent skip + exit 0
#   AXHUB_TOKEN_PATH            → override token file path (test injection)
#   AXHUB_GATE_FAKE_NOW         → override "now" in seconds (test injection;
#                                  used to bypass `date +%s`)
#   AXHUB_GATE_POLL_INTERVAL    → seconds per poll iteration (default 5)
#   AXHUB_GATE_POLL_ITERATIONS  → number of poll iterations (default 6)
#   AXHUB_GATE_AUTH_PROBE       → command run for inline UNAUTHORIZED check
#                                  (default: `axhub auth status --json`)

set -u

# Phase 25 PR 25.2 — hook safety kill switch. Canonical envs per
# .plan/matrix-absorption/00-overview.md §10.6 (Env Var Taxonomy ADR).
# Legacy DISABLE_AXHUB=1 alias honored through v0.8.0 deprecation window.
if [ "${AXHUB_DISABLE_HOOKS:-0}" = "1" ] || [ "${DISABLE_AXHUB:-0}" = "1" ]; then
  exit 0
fi
case ",${AXHUB_DISABLE_HOOK:-}," in
  *,token-freshness-gate,*) exit 0 ;;
esac

if [ "${AXHUB_AUTH_BG_REFRESH:-1}" = "0" ]; then
  exit 0
fi

NOW=${AXHUB_GATE_FAKE_NOW:-$(date +%s)}
SESSION_TS=$((NOW - 30))

if [ -n "${AXHUB_TOKEN_PATH:-}" ]; then
  TOKEN_PATH="$AXHUB_TOKEN_PATH"
else
  HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"
  TOKEN_PATH=$("$HELPER" path token-file 2>/dev/null || true)
  case "$TOKEN_PATH" in
    ""|\{*)
      TOKEN_PATH="${XDG_CONFIG_HOME:-$HOME/.config}/axhub-plugin/token"
      ;;
  esac
fi

stat_mtime() {
  stat -c %Y "$1" 2>/dev/null \
    || stat -f %m "$1" 2>/dev/null \
    || echo 0
}

POLL_INTERVAL=${AXHUB_GATE_POLL_INTERVAL:-5}
POLL_ITERATIONS=${AXHUB_GATE_POLL_ITERATIONS:-6}
AUTH_PROBE=${AXHUB_GATE_AUTH_PROBE:-"axhub auth status --json"}

inline_auth_check() {
  echo "[token-freshness-gate] inline auth status check" >&2
  if ! eval "$AUTH_PROBE" 2>/dev/null | grep -q '"user_email"'; then
    echo "[token-freshness-gate] auth UNAUTHORIZED, exit 65" >&2
    exit 65
  fi
  exit 0
}

if [ ! -f "$TOKEN_PATH" ]; then
  echo "[token-freshness-gate] token file missing — inline auth status check" >&2
  inline_auth_check
fi

MTIME=$(stat_mtime "$TOKEN_PATH")
if [ "$MTIME" -gt "$SESSION_TS" ]; then
  echo "[token-freshness-gate] token mtime > session_ts, fresh" >&2
  exit 0
fi

POLL=0
while [ "$POLL" -lt "$POLL_ITERATIONS" ]; do
  sleep "$POLL_INTERVAL"
  POLL=$((POLL + 1))
  MTIME=$(stat_mtime "$TOKEN_PATH")
  if [ "$MTIME" -gt "$SESSION_TS" ]; then
    echo "[token-freshness-gate] token refreshed after $((POLL * POLL_INTERVAL))s" >&2
    exit 0
  fi
done

echo "[token-freshness-gate] poll timeout" >&2
inline_auth_check
