#!/usr/bin/env bash
# axhub-helpers binary selector — picks the right cross-arch build for current OS/arch
# and creates bin/axhub-helpers symlink (or copy on Windows).
#
# Run automatically by PostInstall hook, or manually after `bun run build:all`.
# Override detection with AXHUB_OS / AXHUB_ARCH env vars (used by tests).
set -euo pipefail

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
  echo "Windows는 amd64만 지원해요 (요청: $ARCH). arm64는 다음 릴리즈에서 추가 예정입니다." >&2
  exit 1
fi

# Compose target binary name
EXT=""
[ "$OS_KEY" = "windows" ] && EXT=".exe"
TARGET_NAME="axhub-helpers-${OS_KEY}-${ARCH_KEY}${EXT}"
TARGET_PATH="${BIN_DIR}/${TARGET_NAME}"

if [ ! -f "$TARGET_PATH" ]; then
  echo "바이너리가 없어요: $TARGET_PATH" >&2
  echo "먼저 'bun run build:all' 실행해주세요." >&2
  exit 1
fi

# Symlink (or copy on Windows where symlinks need admin)
LINK_PATH="${BIN_DIR}/axhub-helpers"
[ "$OS_KEY" = "windows" ] && LINK_PATH="${BIN_DIR}/axhub-helpers.exe"

# Remove existing link/file before relinking
[ -e "$LINK_PATH" ] || [ -L "$LINK_PATH" ] && rm -f "$LINK_PATH"

if [ "$OS_KEY" = "windows" ]; then
  cp "$TARGET_PATH" "$LINK_PATH"
else
  ln -s "$TARGET_NAME" "$LINK_PATH"
fi

chmod +x "$LINK_PATH" 2>/dev/null || true

echo "axhub-helpers → $TARGET_NAME (OS=$OS_KEY, arch=$ARCH_KEY)"
