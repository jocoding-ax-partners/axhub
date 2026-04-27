#!/usr/bin/env bash
# LIMITATION: libsecret-tools CLI path only — Coverage: 40% READ-path
# (axhub-helpers contract). End-to-end ~15% (ax-hub-cli WRITE-path D-Bus
# is upstream). Does NOT validate gnome-keyring-daemon, kwalletd5, or
# headless systemd-keyring user-bus.
#
# Phase 11 US-1105 — Linux secret-tool keychain bridge runtime smoke.
# Builds + runs ubuntu:24.04 container with libsecret-tools, seeds a test
# token via secret-tool, runs axhub-helpers-linux-amd64 token-init inside
# container. Captures result to .omc/evidence/phase-11-linux-docker-smoke.txt
# with mandatory LIMITATION banner first line.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EVIDENCE_DIR="$REPO_ROOT/.omc/evidence"
EVIDENCE_FILE="$EVIDENCE_DIR/phase-11-linux-docker-smoke.txt"
DOCKERFILE="$REPO_ROOT/tests/smoke-linux-docker.Dockerfile"
HELPER_BIN="$REPO_ROOT/bin/axhub-helpers-linux-amd64"
IMAGE_TAG="axhub-linux-smoke:phase-11"

mkdir -p "$EVIDENCE_DIR"

# Mandatory LIMITATION banner first line.
write_banner() {
  cat > "$EVIDENCE_FILE" <<'BANNER'
LIMITATION: libsecret-tools CLI path only — Coverage: 40% READ-path (axhub-helpers contract). End-to-end ~15% (ax-hub-cli WRITE-path D-Bus is upstream). Does NOT validate gnome-keyring-daemon, kwalletd5, or headless systemd-keyring user-bus.

BANNER
}

# Graceful fallback if docker not available.
if ! command -v docker >/dev/null 2>&1; then
  write_banner
  echo "STATUS: SKIPPED (docker not available)" >> "$EVIDENCE_FILE"
  echo "ACTION: run on machine with docker for v0.1.8 evidence" >> "$EVIDENCE_FILE"
  echo "TIMESTAMP: $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$EVIDENCE_FILE"
  echo "[skip] docker not available — wrote SKIPPED evidence" >&2
  exit 0
fi

# Bail early if linux-amd64 binary not present.
if [ ! -f "$HELPER_BIN" ]; then
  write_banner
  echo "STATUS: SKIPPED (axhub-helpers-linux-amd64 not built)" >> "$EVIDENCE_FILE"
  echo "ACTION: run 'bun run build:linux-amd64' first" >> "$EVIDENCE_FILE"
  echo "TIMESTAMP: $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$EVIDENCE_FILE"
  echo "[skip] linux-amd64 binary not built — wrote SKIPPED evidence" >&2
  exit 0
fi

write_banner
{
  echo "STATUS: RUNNING"
  echo "TIMESTAMP_START: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "$EVIDENCE_FILE"

# Build container. Force linux/amd64 since axhub-helpers-linux-amd64 is x86_64
# and Apple Silicon hosts default to arm64 layer when not pinned.
echo "[build] $IMAGE_TAG (linux/amd64)..." >&2
if ! docker build --platform linux/amd64 -t "$IMAGE_TAG" -f "$DOCKERFILE" "$REPO_ROOT" >> "$EVIDENCE_FILE" 2>&1; then
  echo "STATUS: FAILED (docker build)" >> "$EVIDENCE_FILE"
  echo "[fail] docker build failed — see evidence file" >&2
  exit 0
fi

# Run smoke: spawn dbus session inside container, seed test token via
# secret-tool, then call axhub-helpers token-init.
SMOKE_SCRIPT='set -e
echo "[container] starting dbus session"
eval "$(dbus-launch --sh-syntax)"
export DBUS_SESSION_BUS_ADDRESS DBUS_SESSION_BUS_PID
echo "[container] libsecret-tools version=$(dpkg-query -W libsecret-tools | cut -f2)"

# Spawn gnome-keyring-daemon to back secret-service
echo -n "test_password" | gnome-keyring-daemon --unlock >/dev/null 2>&1 || true
gnome-keyring-daemon --start --components=secrets >/dev/null 2>&1 || true

# Seed go-keyring-base64 envelope
JSON='\''{"schema_version":2,"access_token":"axhub_pat_phase11_smoke_test_token_value_long_enough_to_pass","token_type":"bearer","expires_at":"2030-01-01T00:00:00Z","scopes":["read","write"]}'\''
B64=$(printf "%s" "$JSON" | base64 -w0)
ENVELOPE="go-keyring-base64:$B64"
echo "[container] seeding secret-tool with envelope"
echo -n "$ENVELOPE" | secret-tool store --label=axhub service axhub
echo "[container] secret-tool store exit=$?"

# Verify roundtrip
RECOVERED=$(secret-tool lookup service axhub)
echo "[container] secret-tool lookup recovered length=${#RECOVERED}"

# Run axhub-helpers token-init (read path)
echo "[container] running axhub-helpers token-init"
/work/bin/axhub-helpers-linux-amd64 token-init
TOKEN_INIT_EXIT=$?
echo "[container] token-init exit=$TOKEN_INIT_EXIT"

# Read written token file (helper writes to ~/.config/axhub-plugin/token by default)
TOKEN_FILE="$HOME/.config/axhub-plugin/token"
if [ -f "$TOKEN_FILE" ]; then
  echo "[container] token file mode=$(stat -c %a "$TOKEN_FILE") size=$(wc -c < "$TOKEN_FILE")"
  echo "[container] token first 16 chars=$(head -c 16 "$TOKEN_FILE")"
fi

exit $TOKEN_INIT_EXIT
'

echo "[run] container smoke (linux/amd64)..." >&2
if docker run --rm --platform linux/amd64 \
  -v "$REPO_ROOT:/work" \
  -e HOME=/tmp \
  -w /work \
  "$IMAGE_TAG" -c "$SMOKE_SCRIPT" >> "$EVIDENCE_FILE" 2>&1; then
  {
    echo ""
    echo "STATUS: PASSED"
    echo "TIMESTAMP_END: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  } >> "$EVIDENCE_FILE"
  echo "[pass] Linux Docker smoke green — see $EVIDENCE_FILE" >&2
else
  {
    echo ""
    echo "STATUS: FAILED (container exec)"
    echo "TIMESTAMP_END: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  } >> "$EVIDENCE_FILE"
  echo "[fail] Linux Docker smoke failed — see $EVIDENCE_FILE" >&2
  exit 0
fi
