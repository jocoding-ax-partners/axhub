---
description: axhub 인증 (브라우저 OAuth 로그인, 헤드리스 환경에선 토큰 붙여넣기 자동 전환)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), AskUserQuestion
argument-hint: "(no args)"
model: sonnet
---

Trigger the axhub `auth` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/auth/SKILL.md`. Slash invocation starts the auth skill, but login/logout/PAT mutations still follow the skill preview and confirmation rules. Note: this command may open a browser window to complete the OAuth flow.
