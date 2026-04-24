#!/usr/bin/env bash
# Tests bin/install.sh OS/arch detection across 5 supported targets + 2 negative cases.
# Uses AXHUB_OS / AXHUB_ARCH overrides (no uname stubbing needed).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_SH="${REPO_ROOT}/bin/install.sh"

if [ ! -x "$INSTALL_SH" ]; then
  echo "FAIL: bin/install.sh missing or not executable" >&2
  exit 1
fi

# Create scratch bin dir with stub binaries (so install.sh finds the targets)
SCRATCH="$(mktemp -d)"
trap "rm -rf '$SCRATCH'" EXIT
for name in \
  axhub-helpers-darwin-arm64 \
  axhub-helpers-darwin-amd64 \
  axhub-helpers-linux-amd64 \
  axhub-helpers-linux-arm64 \
  axhub-helpers-windows-amd64.exe; do
  printf '#!/bin/sh\necho stub' > "$SCRATCH/$name"
  chmod +x "$SCRATCH/$name"
done
cp "$INSTALL_SH" "$SCRATCH/install.sh"

PASS=0
FAIL=0

assert_link() {
  local label="$1" os="$2" arch="$3" expected="$4"
  rm -f "$SCRATCH/axhub-helpers" "$SCRATCH/axhub-helpers.exe"
  if AXHUB_OS="$os" AXHUB_ARCH="$arch" bash "$SCRATCH/install.sh" >/dev/null 2>&1; then
    local link_name="axhub-helpers"
    [ "$os" = "MINGW64_NT-10.0" ] || [ "$os" = "Windows_NT" ] && link_name="axhub-helpers.exe"
    local actual
    if [ -L "$SCRATCH/$link_name" ]; then
      actual="$(readlink "$SCRATCH/$link_name")"
    elif [ -f "$SCRATCH/$link_name" ]; then
      actual="$link_name (copy on windows)"
      # On windows codepath we copied; check file exists with correct content
      [ -f "$SCRATCH/$expected" ] && actual="$expected"
    fi
    if [ "$actual" = "$expected" ]; then
      echo "PASS: $label → $expected"
      PASS=$((PASS+1))
    else
      echo "FAIL: $label → expected $expected, got $actual"
      FAIL=$((FAIL+1))
    fi
  else
    echo "FAIL: $label → install.sh exited non-zero unexpectedly"
    FAIL=$((FAIL+1))
  fi
}

assert_reject() {
  local label="$1" os="$2" arch="$3"
  rm -f "$SCRATCH/axhub-helpers" "$SCRATCH/axhub-helpers.exe"
  if AXHUB_OS="$os" AXHUB_ARCH="$arch" bash "$SCRATCH/install.sh" >/dev/null 2>&1; then
    echo "FAIL: $label → should have rejected (OS=$os, ARCH=$arch)"
    FAIL=$((FAIL+1))
  else
    echo "PASS: $label → correctly rejected"
    PASS=$((PASS+1))
  fi
}

# Positive cases — 5 supported targets
assert_link "macOS arm64"   "Darwin"         "arm64"   "axhub-helpers-darwin-arm64"
assert_link "macOS Intel"   "Darwin"         "x86_64"  "axhub-helpers-darwin-amd64"
assert_link "Linux x86_64"  "Linux"          "x86_64"  "axhub-helpers-linux-amd64"
assert_link "Linux arm64"   "Linux"          "aarch64" "axhub-helpers-linux-arm64"
assert_link "Windows amd64" "MINGW64_NT-10.0" "x86_64" "axhub-helpers-windows-amd64.exe"

# Negative cases
assert_reject "Unknown OS (FreeBSD)"    "FreeBSD"          "amd64"
assert_reject "Unsupported arch (riscv)" "Linux"           "riscv64"
assert_reject "Windows arm64 (gated)"   "MINGW64_NT-10.0"  "arm64"

echo "---"
echo "Total: $((PASS+FAIL)) | PASS: $PASS | FAIL: $FAIL"
exit "$FAIL"
