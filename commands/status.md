---
description: axhub 배포 상태 확인 (한국어 진행 상황 안내 + 자동 watch 옵션)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(jq:*)
argument-hint: "[deployment-id]"
model: haiku
---

Trigger and fully execute the axhub `status` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/status/SKILL.md` now. Do not merely acknowledge that the skill was invoked, and do not stop with "결과 대기 중". If `$ARGUMENTS` is an app slug rather than a deployment id, resolve the app, run `axhub deploy list --app "$APP" --json`, choose the most recent deployment in headless mode, then run `axhub deploy status "$DEPLOYMENT_ID" --app "$APP" --json` and summarize the actual result in Korean.

Slash invocation does NOT bypass the AskUserQuestion preview card or HMAC consent token requirement for destructive operations — the PreToolUse hook will still verify consent before any destructive bash call.
