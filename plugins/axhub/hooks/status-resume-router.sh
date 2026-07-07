#!/usr/bin/env bash
set -euo pipefail

[ -n "${AXHUB_NO_STATUS_ROUTER:-}" ] && exit 0

input="$(cat)"

case "$input" in
  *axhub*) ;;
  *) exit 0 ;;
esac

case "$input" in
  *최신*|*버전*|*업데이트*|*latest*|*"up to date"*|*"version check"*|*update*|*upgrade*)
    exit 0
    ;;
esac

case "$input" in
  *상태*|*진행*|*이어*|*이어서*|*계속*|*확인*|*배포*|*만들기*|*bootstrap*|*deployment*|*status*)
    printf '%s\n' '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"[axhub status/resume router] The current user prompt asks to continue or check an axhub app creation/deployment/status flow. Before generic Code-mode handling, shell probes, Finding tools, App/MCP tools, or unrelated plugins, route through axhub skills. If wording mentions app creation, bootstrap, 앱 만들기, or a previous bootstrap folder, invoke the axhub bootstrap skill and use its resume/status path. Otherwise use deploy for an already-connected app deployment status. Do not run generic probes such as command -v axhub, pwd, ls, find, cat, echo checks, bash -lc, sh -c, pipes, redirects, grep, head, tail, sed, awk, jq, or sleep. Use one direct axhub CLI command per visible tool call with Korean titles: axhub plugin-support init-resume route --json, axhub apps bootstrap-status <bootstrap-id> --tenant <tenant> --json, axhub deploy status <deployment-id> --tenant <tenant> --json, and axhub deploy verify <deployment-id> --app <app> --json."},"systemMessage":"[axhub status/resume router] Continue/check axhub app creation or deployment status via axhub bootstrap/deploy skills before generic Code-mode shell probes. Never use command -v axhub, pwd, ls, find, cat, echo checks, pipes, grep, head, sleep, or App/MCP tools for this routing step; use direct axhub CLI resume/status/verify commands only."}'
    ;;
esac
