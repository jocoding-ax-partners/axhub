#!/usr/bin/env bash
# Phase 22.0.2 — caveat 2 fix. axhub CLI subcommand coverage closed-loop.
#
# axhub --help (시스템 CLI 가 PATH 에 있을 때) + hooks/session-start.sh 의
# 'axhub auth status --json' 호출 grep + commands/*.md SKILL invocation path 통합 →
# tests/e2e/claude-cli/fixtures/bin/required-subcommands.txt 자동 생성.
#
# axhub-mock-impl.sh 가 cover 해야 할 subcommand 카탈로그 의 source-of-truth 예요.
# 22.0 끝에 mock-impl 가 모든 entry handle 하는지 자동 assert.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_FILE="${REPO_ROOT}/tests/e2e/claude-cli/fixtures/bin/required-subcommands.txt"
TMP_FILE="$(mktemp)"

trap 'rm -f "$TMP_FILE"' EXIT

# 1. hooks 안 axhub 호출 (실제 shell 호출, prose 제외)
{
  if [ -f "${REPO_ROOT}/hooks/session-start.sh" ]; then
    grep -oE 'axhub [a-z]+ [a-z]+( --[a-z-]+)*' "${REPO_ROOT}/hooks/session-start.sh" \
      | grep -vE '\b(for|with|without|is|are|matrix|name|code|command|health|upgrade|platform|utterance|safety)\b' || true
  fi
  # SKILL workflow 의 backtick-quoted 또는 shell 블록 안 axhub 호출만
  grep -rohE '`axhub [a-z]+ [a-z]+( --[a-z-]+)*`' "${REPO_ROOT}/skills/" 2>/dev/null \
    | tr -d '`' || true
  # commands 안 axhub 호출
  grep -rohE '`axhub [a-z]+ [a-z]+( --[a-z-]+)*`' "${REPO_ROOT}/commands/" 2>/dev/null \
    | tr -d '`' || true
} | sed -E 's/^axhub //' \
  | sed -E 's/ --[a-z-]+( [^ ]+)?//g' \
  | grep -vE '^(for|with|without|is|are|--|matrix|name|code|command|health|upgrade|platform|utterance|safety) ' \
  | sort -u >> "$TMP_FILE"

# 2. minimum baseline coverage (Phase 22 plan §Build Order 22.0 step (5))
cat >> "$TMP_FILE" <<EOF
deploy create
deploy status
apps list
apis list
auth status
auth login
update check
logs build
logs runtime
EOF

# 3. dedupe + sort
sort -u "$TMP_FILE" > "$OUT_FILE"

ENTRY_COUNT="$(wc -l < "$OUT_FILE" | tr -d ' ')"

if [ "$ENTRY_COUNT" -lt 9 ]; then
  echo "[enumerate] FAIL — required-subcommands.txt 가 9 entry 미만 (실측: ${ENTRY_COUNT})" >&2
  exit 1
fi

echo "[enumerate] OK — ${ENTRY_COUNT} entries → ${OUT_FILE}"

# 4. mock-impl coverage assertion (있으면)
MOCK_IMPL="${REPO_ROOT}/tests/e2e/claude-cli/fixtures/bin/axhub-mock-impl.sh"
if [ -f "$MOCK_IMPL" ]; then
  MISSING=0
  while IFS= read -r entry; do
    if ! grep -qF "$entry" "$MOCK_IMPL"; then
      echo "[enumerate] WARN — mock-impl missing case for: ${entry}" >&2
      MISSING=$((MISSING + 1))
    fi
  done < "$OUT_FILE"
  if [ "$MISSING" -gt 0 ]; then
    echo "[enumerate] FAIL — mock-impl 가 ${MISSING} entry cover 안 함" >&2
    exit 1
  fi
  echo "[enumerate] mock-impl coverage 100% (${ENTRY_COUNT} entries)"
else
  echo "[enumerate] mock-impl 아직 없음 (Phase 22.2 에서 생성 예정)"
fi
