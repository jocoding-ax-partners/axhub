---
description: Authenticate with axhub (opens browser for OAuth flow)
allowed-tools: Bash(axhub:*), AskUserQuestion
argument-hint: "(no args, optional --token-file <path>)"
model: sonnet
---

Trigger the axhub `auth` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/auth/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call. Note: this command opens a browser window to complete the OAuth flow.
