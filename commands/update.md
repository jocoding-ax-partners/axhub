---
description: axhub CLI 업데이트 (cosign 서명 검증 — 회사 보안 정책 호환, destructive 작업)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), AskUserQuestion
argument-hint: "[--check-only] [--force]"
model: sonnet
---

Trigger the axhub `update` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/update/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call. Note: AXHUB_REQUIRE_COSIGN=1 is the default; cosign signature verification runs before binary replacement.
