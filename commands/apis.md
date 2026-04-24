---
description: Browse the axhub API catalog (privacy-aware — cross-team list requires explicit consent)
allowed-tools: Bash(axhub:*), Bash(jq:*)
argument-hint: "[--query <text>]"
model: sonnet
---

Trigger the axhub `apis` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/apis/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call. Note: listing APIs across teams requires explicit user consent per the privacy filter in the skill.
