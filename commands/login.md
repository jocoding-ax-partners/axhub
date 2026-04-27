---
description: axhub 인증 (브라우저 OAuth 로그인, 헤드리스 환경에선 토큰 붙여넣기 자동 전환)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), AskUserQuestion
argument-hint: "(no args)"
model: sonnet
---

Trigger the axhub `auth` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/auth/SKILL.md`. Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call. Note: this command opens a browser window to complete the OAuth flow.
