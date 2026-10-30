#!/usr/bin/env bash
# The host allows ten seconds for shell/CLI startup and a five-second policy
# check. No login, repair, policy parsing, or cache writes belong here.
set -o pipefail

[ "${AXHUB_NO_POLICY_SYNC:-}" = "1" ] && exit 0
{ [ -e "$HOME/.axhub/config/no-policy-sync" ] || [ -L "$HOME/.axhub/config/no-policy-sync" ]; } && exit 0

fail() {
  # Never echo helper stdout/stderr: older CLIs and failures can contain secrets.
  printf '%s\n' "AXHub policy sync: $1. Run axhub tenants developer-policy status --json in a terminal; repair the connection or credentials, then resend your prompt. See POLICY.md#organization-policy-recovery." >&2
  exit 2
}
case "${1:-}" in
  SessionStart|UserPromptSubmit) EVENT=$1 ;;
  *) fail PROTOCOL ;;
esac

# This existence check is only the missing-CLI fallback for hosts launched in
# the project directory. With a CLI available, host JSON cwd is authoritative:
# never skip based on process cwd before the helper validates that input.
# A .git file is a worktree boundary just like a .git directory.
DIR=$(pwd -P 2>/dev/null) || fail BINDING_CONFLICT
BOUND=0
while :; do
  [ -r "$DIR" ] && [ -x "$DIR" ] || { BOUND=1; break; }
  for part in "$DIR/.axhub" "$DIR/.axhub/state"; do
    if [ -L "$part" ] || { [ -e "$part" ] && { [ ! -d "$part" ] || [ ! -r "$part" ] || [ ! -x "$part" ]; }; }; then
      BOUND=1
      break
    fi
  done
  [ "$BOUND" = 1 ] && break
  if [ -e "$DIR/.axhub/state/developer-policy.json" ] || [ -L "$DIR/.axhub/state/developer-policy.json" ]; then
    BOUND=1
    break
  fi
  { [ -e "$DIR/.git" ] || [ -L "$DIR/.git" ]; } && break
  [ "$DIR" = / ] && break
  DIR=${DIR%/*}
  [ -n "$DIR" ] || DIR=/
done

BIN=""
if command -v axhub >/dev/null 2>&1; then
  BIN=$(command -v axhub)
elif [ -f "$HOME/.axhub/bin-path" ]; then
  IFS= read -r BIN < "$HOME/.axhub/bin-path" || :
  BIN=${BIN%$'\r'}
  { [ -n "$BIN" ] && [ -x "$BIN" ]; } || BIN=""
fi
if [ -z "$BIN" ]; then
  for candidate in "$HOME/.axhub/bin/axhub" "$HOME/.axhub/bin/axhub.exe"; do
    [ -x "$candidate" ] && { BIN="$candidate"; break; }
  done
fi
if [ -z "$BIN" ]; then
  [ "$BOUND" = 0 ] && exit 0
  fail UNSUPPORTED
fi

# One CLI process fetches the authenticated current policy and strictly adapts
# it to the host protocol. Never fall back to stale policy after a failure.
OUTPUT=$("$BIN" --timeout 5s plugin-support policy-context --hook "$EVENT" 2>/dev/null)
STATUS=$?
# Only an unsupported CLI gets the missing-CLI unbound exception. New CLI
# protocol/auth/network failures must never be hidden by process-cwd evidence.
if [ "$STATUS" = 64 ]; then
  [ "$BOUND" = 0 ] && exit 0
  fail UNSUPPORTED
fi
[ "$STATUS" = 0 ] || fail PROTOCOL
[ -z "$OUTPUT" ] || printf '%s\n' "$OUTPUT"
