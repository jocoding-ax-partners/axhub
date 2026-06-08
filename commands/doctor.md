---
description: axhub 환경 진단 (CLI 설치 / 인증 상태 / 버전 호환성 점검)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(which:*)
argument-hint: "(no args)"
model: sonnet
---

Trigger the axhub `doctor` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/doctor/SKILL.md`. Slash invocation starts the doctor skill and does not bypass preview/confirmation rules for any follow-up destructive workflow.
