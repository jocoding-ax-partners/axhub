---
description: Stream or fetch axhub deployment logs (build or pod source)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(jq:*)
argument-hint: "[deployment-id] [--source build|pod]"
model: sonnet
---

Trigger the axhub `logs` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/logs/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call.
