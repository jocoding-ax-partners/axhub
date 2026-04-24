---
description: Run axhub environment diagnostics (CLI presence, auth state, version check)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(which:*)
argument-hint: "(no args)"
model: sonnet
---

Trigger the axhub `doctor` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/doctor/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call.
