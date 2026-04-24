---
description: Show axhub deployment status (current or by ID)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(jq:*)
argument-hint: "[deployment-id]"
model: sonnet
---

Trigger the axhub `status` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/status/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call.
