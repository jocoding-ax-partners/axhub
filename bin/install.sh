#!/usr/bin/env bash
# axhub-helpers binary selector — picks the right cross-arch build for current OS/arch
# and creates bin/axhub-helpers symlink (or copy on Windows).
#
# Run automatically by PostInstall hook, or manually after `bun run build:all`.
# Override detection with AXHUB_OS / AXHUB_ARCH env vars (used by tests).
set -euo pipefail

# --- install-time disclosure (idempotent, marker-gated) ---
# Maintainer: keep _AXHUB_DISCLOSURE_VER in sync with RELEASE_VERSION below.
_AXHUB_DISCLOSURE_VER="v0.5.13"
_AXHUB_STATE_DIR="${XDG_STATE_HOME:-${HOME}/.local/state}/axhub-plugin"
_AXHUB_DISCLOSURE_MARKER="${_AXHUB_STATE_DIR}/install-disclosure-shown.txt"
# CI / scripted contexts suppress disclosure (AXHUB_SKIP_AUTODOWNLOAD=1 indicates
# automated test path; AXHUB_NO_DISCLOSURE=1 explicit override for scripts piping
# install.sh stdout/stderr to JSON parser).
_AXHUB_SHOW_DISCLOSURE=1
if [ "${AXHUB_SKIP_AUTODOWNLOAD:-0}" = "1" ] || [ "${AXHUB_NO_DISCLOSURE:-0}" = "1" ]; then
  _AXHUB_SHOW_DISCLOSURE=0
fi
if [ "$_AXHUB_SHOW_DISCLOSURE" = "1" ] && { [ ! -f "$_AXHUB_DISCLOSURE_MARKER" ] || ! grep -qxF "$_AXHUB_DISCLOSURE_VER" "$_AXHUB_DISCLOSURE_MARKER" 2>/dev/null; }; then
  cat >&2 <<'DISCLOSURE'
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
axhub 이 다음을 수행해요:
  (1) 인증 토큰을 keychain (macOS/Windows) / file (Linux) 에 저장해요.
  (2) opt-in telemetry 가 활성화되어 있어요 (AXHUB_TELEMETRY=0 로 disable).
  (3) macOS Gatekeeper 의 helper binary quarantine attribute 를 제거해요.
  (4) auth-refresh 백그라운드 task 가 token 갱신해요.
  (5) helper binary 를 GitHub release 에서 HTTPS 로 다운로드 + 실행해요.
  (6) ~/.claude/settings.json 의 statusLine field 를 추가/관리해요 (other plugins preserved).

거부하려면: AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1 환경변수 설정 후 install.
uninstall 시 orphan stub 이 graceful fallback 을 보장해요.

자세한 내용: https://github.com/jocoding-ax-partners/axhub#trust--uninstall
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
DISCLOSURE
  mkdir -p "$_AXHUB_STATE_DIR"
  printf '%s\n' "$_AXHUB_DISCLOSURE_VER" > "$_AXHUB_DISCLOSURE_MARKER"
fi
# --- end install-time disclosure ---

BIN_DIR="$(cd "$(dirname "$0")" && pwd)"
OS="${AXHUB_OS:-$(uname -s)}"
ARCH="${AXHUB_ARCH:-$(uname -m)}"

# Normalize OS
case "$OS" in
  Darwin) OS_KEY="darwin" ;;
  Linux)  OS_KEY="linux"  ;;
  MINGW*|MSYS_NT*|CYGWIN*|Windows_NT) OS_KEY="windows" ;;
  *)
    echo "지원하지 않는 OS에요 (OS=$OS). 현재 지원: macOS, Linux, Windows." >&2
    exit 1
    ;;
esac

# Normalize ARCH
case "$ARCH" in
  arm64|aarch64) ARCH_KEY="arm64" ;;
  x86_64|amd64)  ARCH_KEY="amd64" ;;
  *)
    echo "지원하지 않는 아키텍처에요 (arch=$ARCH). 현재 지원: arm64, amd64." >&2
    exit 1
    ;;
esac

# Windows only ships amd64 (per package.json build:all)
if [ "$OS_KEY" = "windows" ] && [ "$ARCH_KEY" != "amd64" ]; then
  echo "Windows는 amd64만 지원해요 (요청: $ARCH). arm64는 다음 릴리즈에서 추가될 예정이에요." >&2
  exit 1
fi

# Compose target binary name
EXT=""
[ "$OS_KEY" = "windows" ] && EXT=".exe"
TARGET_NAME="axhub-helpers-${OS_KEY}-${ARCH_KEY}${EXT}"
TARGET_PATH="${BIN_DIR}/${TARGET_NAME}"

# Maintainer: when bumping plugin version (package.json + .claude-plugin/*),
# update this default to match the new release tag. Override via AXHUB_PLUGIN_RELEASE.
RELEASE_VERSION="${AXHUB_PLUGIN_RELEASE:-v0.6.4}"
RELEASE_BASE="https://github.com/jocoding-ax-partners/axhub/releases/download/${RELEASE_VERSION}"

if [ ! -f "$TARGET_PATH" ]; then
  if [ "${AXHUB_SKIP_AUTODOWNLOAD:-0}" = "1" ]; then
    echo "바이너리가 없어요: $TARGET_PATH" >&2
    echo "AXHUB_SKIP_AUTODOWNLOAD=1 — 자동 다운로드 비활성화. 수동: bun run build:all" >&2
    exit 1
  fi
  if ! command -v curl >/dev/null 2>&1; then
    echo "curl 이 없어서 바이너리를 다운로드할 수 없어요. curl 설치 후 다시 시도해주세요." >&2
    exit 1
  fi
  URL="${RELEASE_BASE}/${TARGET_NAME}"
  echo "axhub-helpers 바이너리 다운로드 중: ${RELEASE_VERSION} (${OS_KEY}-${ARCH_KEY})..." >&2
  if ! curl -fsSL "$URL" -o "$TARGET_PATH.tmp"; then
    rm -f "$TARGET_PATH.tmp"
    echo "다운로드 실패: $URL" >&2
    echo "수동 다운로드: gh release download ${RELEASE_VERSION} --pattern '${TARGET_NAME}' -D '${BIN_DIR}'" >&2
    echo "또는 비활성화: export AXHUB_SKIP_AUTODOWNLOAD=1" >&2
    exit 1
  fi
  chmod +x "$TARGET_PATH.tmp"
  mv "$TARGET_PATH.tmp" "$TARGET_PATH"
  echo "다운로드 완료." >&2
fi

# Symlink (or copy on Windows where symlinks need admin)
LINK_PATH="${BIN_DIR}/axhub-helpers"
[ "$OS_KEY" = "windows" ] && LINK_PATH="${BIN_DIR}/axhub-helpers.exe"

# Remove existing link/file before relinking
if [ -e "$LINK_PATH" ] || [ -L "$LINK_PATH" ]; then
  rm -f "$LINK_PATH"
fi

if [ "$OS_KEY" = "windows" ]; then
  cp "$TARGET_PATH" "$LINK_PATH"
else
  ln -s "$TARGET_NAME" "$LINK_PATH"
fi

chmod +x "$LINK_PATH" 2>/dev/null || true

echo "axhub-helpers → $TARGET_NAME (OS=$OS_KEY, arch=$ARCH_KEY)"
