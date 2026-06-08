#!/usr/bin/env bash
# Portable axhub-helpers resolver for plugin hooks.
#
# Release installs normally provide ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers.
# Local directory installs can be source checkouts where bin/ is absent, so
# hooks fall back to the helper built under target/ without crossing into older
# cached plugin versions.
set -u

ROOT="${CLAUDE_PLUGIN_ROOT:-}"
if [ -z "$ROOT" ]; then
  ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fi

resolve_helper() {
  local os arch suffix
  os="$(uname -s 2>/dev/null || true)"
  arch="$(uname -m 2>/dev/null || true)"
  suffix=""
  case "${os}:${arch}" in
    Darwin:arm64|Darwin:aarch64) suffix="darwin-arm64" ;;
    Darwin:x86_64|Darwin:amd64) suffix="darwin-amd64" ;;
    Linux:arm64|Linux:aarch64) suffix="linux-arm64" ;;
    Linux:x86_64|Linux:amd64) suffix="linux-amd64" ;;
  esac

  local candidates=()
  candidates+=("${ROOT}/bin/axhub-helpers")
  if [ -n "$suffix" ]; then
    candidates+=("${ROOT}/bin/axhub-helpers-${suffix}")
  fi
  candidates+=("${ROOT}/target/release/axhub-helpers")
  candidates+=("${ROOT}/target/debug/axhub-helpers")

  local candidate
  for candidate in "${candidates[@]}"; do
    if [ -x "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

if [ "${1:-}" = "--resolve-helper" ]; then
  resolve_helper
  exit $?
fi

if HELPER="$(resolve_helper)"; then
  exec "$HELPER" "$@"
fi

# Fail-open for context/telemetry hooks and local quality helper gates.
case "${1:-}" in
  prompt-route|classify-exit|test-classifier|state-update|commit-gate|tdd-inject|verify-deploy-artifact)
    cat >/dev/null || true
    exit 0
    ;;
  *)
    printf '%s\n' "[axhub] axhub-helpers binary not found under ${ROOT}" >&2
    exit 127
    ;;
esac
