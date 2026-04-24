---
description: 현재 프로필이 볼 수 있는 axhub 앱 목록 (읽기 전용)
allowed-tools: Bash(axhub:*), Bash(jq:*)
argument-hint: "[--all] [--slug-prefix <name>]"
model: sonnet
---

Trigger the axhub `apps` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/apps/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call.
