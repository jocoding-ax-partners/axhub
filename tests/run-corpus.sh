#!/usr/bin/env bash
# POSIX compatibility wrapper. Canonical implementation is tests/run-corpus.ts
# so package scripts and CI can run natively on Windows without Git Bash.
set -euo pipefail

PLUGIN_ROOT="${PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
exec bun "$PLUGIN_ROOT/tests/run-corpus.ts" "$@"
