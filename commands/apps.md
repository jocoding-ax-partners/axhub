---
description: 현재 프로필의 axhub 앱 목록/관리 (삭제 등 파괴적 작업은 승인 필요)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(jq:*)
argument-hint: "[--all] [--slug-prefix <name>]"
model: haiku
---

Trigger the axhub `apps` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/apps/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call.
