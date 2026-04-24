---
description: axhub API 카탈로그 보기 (privacy-aware — 다른 팀 목록은 명시 동의 필요)
allowed-tools: Bash(axhub:*), Bash(jq:*)
argument-hint: "[--query <text>]"
model: sonnet
---

Trigger the axhub `apis` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/apis/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call. Note: listing APIs across teams requires explicit user consent per the privacy filter in the skill.
