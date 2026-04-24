#!/usr/bin/env bash
# Phase 5 US-502: tests for SessionStart shim + install.sh auto-download.
# Covers: shim execs helper if present, AXHUB_SKIP_AUTODOWNLOAD opt-out,
# install.sh download-on-missing fallback, AXHUB_PLUGIN_RELEASE override.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_SH="${REPO_ROOT}/bin/install.sh"
SHIM="${REPO_ROOT}/hooks/session-start.sh"

if [ ! -x "$INSTALL_SH" ]; then
  echo "FAIL: bin/install.sh missing or not executable" >&2
  exit 1
fi
if [ ! -x "$SHIM" ]; then
  echo "FAIL: hooks/session-start.sh missing or not executable" >&2
  exit 1
fi

PASS=0
FAIL=0

scratch() {
  local d
  d="$(mktemp -d)"
  echo "$d"
}

assert() {
  local label="$1" actual="$2" expected="$3"
  if [ "$actual" = "$expected" ]; then
    echo "PASS: $label"
    PASS=$((PASS+1))
  else
    echo "FAIL: $label → expected '$expected', got '$actual'"
    FAIL=$((FAIL+1))
  fi
}

# ----------------------------------------------------------------------------
# 1. install.sh — AXHUB_SKIP_AUTODOWNLOAD=1 + missing binary → exit 1
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
trap "rm -rf '$SCRATCH'" EXIT
cp "$INSTALL_SH" "$SCRATCH/install.sh"
# No binaries seeded — install should fail with skip flag set
if AXHUB_OS=Darwin AXHUB_ARCH=arm64 AXHUB_SKIP_AUTODOWNLOAD=1 bash "$SCRATCH/install.sh" >/dev/null 2>&1; then
  assert "AXHUB_SKIP_AUTODOWNLOAD=1 + missing binary" "ok" "fail-expected"
else
  assert "AXHUB_SKIP_AUTODOWNLOAD=1 + missing binary" "fail-expected" "fail-expected"
fi
rm -rf "$SCRATCH"

# ----------------------------------------------------------------------------
# 2. install.sh — present binary + skip flag → still success (no download)
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
cp "$INSTALL_SH" "$SCRATCH/install.sh"
printf '#!/bin/sh\necho stub' > "$SCRATCH/axhub-helpers-darwin-arm64"
chmod +x "$SCRATCH/axhub-helpers-darwin-arm64"
if AXHUB_OS=Darwin AXHUB_ARCH=arm64 AXHUB_SKIP_AUTODOWNLOAD=1 bash "$SCRATCH/install.sh" >/dev/null 2>&1; then
  if [ -L "$SCRATCH/axhub-helpers" ]; then
    actual="$(readlink "$SCRATCH/axhub-helpers")"
    assert "binary present + skip flag → symlinked, no download" "$actual" "axhub-helpers-darwin-arm64"
  else
    assert "binary present + skip flag → symlinked" "no-symlink" "axhub-helpers-darwin-arm64"
  fi
else
  assert "binary present + skip flag → success" "fail" "ok"
fi
rm -rf "$SCRATCH"

# ----------------------------------------------------------------------------
# 3. install.sh — AXHUB_PLUGIN_RELEASE override is respected (no actual fetch)
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
cp "$INSTALL_SH" "$SCRATCH/install.sh"
# Force download by leaving binary missing + override release version + use a
# bogus URL via AXHUB_PLUGIN_RELEASE. Should fail-fast on curl, not on URL
# guessing logic.
output="$(AXHUB_OS=Darwin AXHUB_ARCH=arm64 AXHUB_PLUGIN_RELEASE=v999.999.999 bash "$SCRATCH/install.sh" 2>&1 || true)"
case "$output" in
  *v999.999.999*) assert "AXHUB_PLUGIN_RELEASE override echoed" "ok" "ok" ;;
  *)              assert "AXHUB_PLUGIN_RELEASE override echoed" "missing" "ok" ;;
esac
rm -rf "$SCRATCH"

# ----------------------------------------------------------------------------
# 4. SessionStart shim — helper present → execs helper (does not call install.sh)
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
mkdir -p "$SCRATCH/bin"
mkdir -p "$SCRATCH/hooks"
cp "$SHIM" "$SCRATCH/hooks/session-start.sh"
# Stub helper that prints a unique sentinel (not actual session-start)
cat > "$SCRATCH/bin/axhub-helpers" <<'EOF'
#!/bin/sh
echo "STUB_HELPER_RAN_SESSION_START"
EOF
chmod +x "$SCRATCH/bin/axhub-helpers"
# install.sh deliberately ABSENT — if shim tries to call it on the present-helper
# path, the test reveals the bug.
output="$(CLAUDE_PLUGIN_ROOT="$SCRATCH" bash "$SCRATCH/hooks/session-start.sh" </dev/null 2>&1 || true)"
if echo "$output" | grep -q "STUB_HELPER_RAN_SESSION_START"; then
  assert "shim execs helper when present (no install.sh needed)" "ok" "ok"
else
  assert "shim execs helper" "missing-sentinel" "ok"
fi
rm -rf "$SCRATCH"

# ----------------------------------------------------------------------------
# 5. SessionStart shim — helper + install.sh both missing → graceful systemMessage
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
mkdir -p "$SCRATCH/bin"
mkdir -p "$SCRATCH/hooks"
cp "$SHIM" "$SCRATCH/hooks/session-start.sh"
output="$(CLAUDE_PLUGIN_ROOT="$SCRATCH" bash "$SCRATCH/hooks/session-start.sh" </dev/null 2>&1 || true)"
if echo "$output" | grep -q "install.sh 없음"; then
  assert "shim emits Korean error JSON when install.sh missing" "ok" "ok"
else
  assert "shim graceful on no install.sh" "missing-error" "ok"
fi
rm -rf "$SCRATCH"

# ----------------------------------------------------------------------------
# 6. SessionStart shim — Phase 7 US-701: token-init auto-trigger when token
#    file missing + axhub auth status returns user_email
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
mkdir -p "$SCRATCH/bin" "$SCRATCH/hooks"
cp "$SHIM" "$SCRATCH/hooks/session-start.sh"
# Helper stub that records which subcommand it was called with
cat > "$SCRATCH/bin/axhub-helpers" <<'EOF'
#!/bin/sh
echo "STUB_HELPER_CALLED:$1" >> /tmp/axhub-shim-test-trace
echo "{}"
EOF
chmod +x "$SCRATCH/bin/axhub-helpers"
# Stub axhub on PATH that returns a valid auth status
mkdir -p "$SCRATCH/stub-bin"
cat > "$SCRATCH/stub-bin/axhub" <<'EOF'
#!/bin/sh
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  echo '{"user_email":"test@example.com","scopes":["read","write"]}'
fi
EOF
chmod +x "$SCRATCH/stub-bin/axhub"
rm -f /tmp/axhub-shim-test-trace
TOKEN_HOME="$(mktemp -d)"
PATH="$SCRATCH/stub-bin:$PATH" XDG_CONFIG_HOME="$TOKEN_HOME" CLAUDE_PLUGIN_ROOT="$SCRATCH" bash "$SCRATCH/hooks/session-start.sh" </dev/null >/dev/null 2>&1 || true
trace="$(cat /tmp/axhub-shim-test-trace 2>/dev/null || echo '')"
case "$trace" in
  *"STUB_HELPER_CALLED:token-init"*) assert "shim auto-triggers token-init when token missing + axhub auth valid" "ok" "ok" ;;
  *)                                 assert "shim token-init auto-trigger" "missing" "ok" ;;
esac
rm -f /tmp/axhub-shim-test-trace
rm -rf "$TOKEN_HOME"
rm -rf "$SCRATCH"

# ----------------------------------------------------------------------------
# 7. SessionStart shim — Phase 7 US-701: token-init SKIPPED when token already exists
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
mkdir -p "$SCRATCH/bin" "$SCRATCH/hooks"
cp "$SHIM" "$SCRATCH/hooks/session-start.sh"
cat > "$SCRATCH/bin/axhub-helpers" <<'EOF'
#!/bin/sh
echo "STUB_HELPER_CALLED:$1" >> /tmp/axhub-shim-test-trace
echo "{}"
EOF
chmod +x "$SCRATCH/bin/axhub-helpers"
mkdir -p "$SCRATCH/stub-bin"
cat > "$SCRATCH/stub-bin/axhub" <<'EOF'
#!/bin/sh
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  echo '{"user_email":"test@example.com","scopes":["read","write"]}'
fi
EOF
chmod +x "$SCRATCH/stub-bin/axhub"
TOKEN_HOME="$(mktemp -d)"
mkdir -p "$TOKEN_HOME/axhub-plugin"
echo "axhub_pat_existing_token_value" > "$TOKEN_HOME/axhub-plugin/token"
chmod 0600 "$TOKEN_HOME/axhub-plugin/token"
rm -f /tmp/axhub-shim-test-trace
PATH="$SCRATCH/stub-bin:$PATH" XDG_CONFIG_HOME="$TOKEN_HOME" CLAUDE_PLUGIN_ROOT="$SCRATCH" bash "$SCRATCH/hooks/session-start.sh" </dev/null >/dev/null 2>&1 || true
trace="$(cat /tmp/axhub-shim-test-trace 2>/dev/null || echo '')"
case "$trace" in
  *"STUB_HELPER_CALLED:token-init"*) assert "shim does NOT call token-init when token file present" "called" "skipped" ;;
  *)                                 assert "shim skips token-init when token already exists" "ok" "ok" ;;
esac
rm -f /tmp/axhub-shim-test-trace
rm -rf "$TOKEN_HOME"
rm -rf "$SCRATCH"

# ----------------------------------------------------------------------------
# 8. SessionStart shim — Phase 7 US-701: AXHUB_SKIP_AUTODOWNLOAD=1 skips token-init
# ----------------------------------------------------------------------------
SCRATCH="$(scratch)"
mkdir -p "$SCRATCH/bin" "$SCRATCH/hooks"
cp "$SHIM" "$SCRATCH/hooks/session-start.sh"
cat > "$SCRATCH/bin/axhub-helpers" <<'EOF'
#!/bin/sh
echo "STUB_HELPER_CALLED:$1" >> /tmp/axhub-shim-test-trace
echo "{}"
EOF
chmod +x "$SCRATCH/bin/axhub-helpers"
mkdir -p "$SCRATCH/stub-bin"
cat > "$SCRATCH/stub-bin/axhub" <<'EOF'
#!/bin/sh
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  echo '{"user_email":"test@example.com","scopes":["read","write"]}'
fi
EOF
chmod +x "$SCRATCH/stub-bin/axhub"
TOKEN_HOME="$(mktemp -d)"
rm -f /tmp/axhub-shim-test-trace
PATH="$SCRATCH/stub-bin:$PATH" XDG_CONFIG_HOME="$TOKEN_HOME" AXHUB_SKIP_AUTODOWNLOAD=1 CLAUDE_PLUGIN_ROOT="$SCRATCH" bash "$SCRATCH/hooks/session-start.sh" </dev/null >/dev/null 2>&1 || true
trace="$(cat /tmp/axhub-shim-test-trace 2>/dev/null || echo '')"
case "$trace" in
  *"STUB_HELPER_CALLED:token-init"*) assert "AXHUB_SKIP_AUTODOWNLOAD=1 skips token-init" "called" "skipped" ;;
  *)                                 assert "AXHUB_SKIP_AUTODOWNLOAD=1 skips token-init" "ok" "ok" ;;
esac
rm -f /tmp/axhub-shim-test-trace
rm -rf "$TOKEN_HOME"
rm -rf "$SCRATCH"

echo "---"
echo "Total: $((PASS+FAIL)) | PASS: $PASS | FAIL: $FAIL"
exit "$FAIL"
