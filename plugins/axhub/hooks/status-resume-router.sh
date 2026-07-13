#!/usr/bin/env bash
set -euo pipefail

[ -n "${AXHUB_NO_STATUS_ROUTER:-}" ] && exit 0
[ -f "$HOME/.axhub/config/no-status-router" ] && exit 0

input="$(cat)"

# "prompt": 키 이후 구간만 매칭 — cwd·transcript_path 경로 오탐(F1) 차단.
# 값 내 따옴표는 \" 이스케이프라 원문 "prompt": 는 키뿐이에요. 키 부재 =
# fail-closed (스킬 frontmatter 라우팅은 훅 없이도 동작). 스키마 가정·극단
# 케이스 상세는 update-router.sh 헤더 주석 참고.
case "$input" in
  *\"prompt\":*) ;;
  *) exit 0 ;;
esac
prompt_part=${input#*\"prompt\":}

case "$prompt_part" in
  *axhub*|*Axhub*|*AxHub*|*AXHUB*) ;;
  *) exit 0 ;;
esac

case "$prompt_part" in
  *최신*|*버전*|*업데이트*|*latest*|*"up to date"*|*"version check"*|*update*|*upgrade*)
    exit 0
    ;;
esac

case "$prompt_part" in
  *상태*|*진행*|*이어*|*이어서*|*계속*|*확인*|*배포*|*만들기*|*bootstrap*|*deployment*|*status*)
    printf '%s\n' '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"[axhub status/resume router] The current user prompt asks to continue or check an axhub app creation/deployment/status flow. Before generic Code-mode handling, shell probes, Finding tools, App/MCP tools, or unrelated plugins, route through axhub skills. If wording mentions app creation, bootstrap, 앱 만들기, or a previous bootstrap folder, invoke the axhub bootstrap skill and use its resume/status path. Otherwise use deploy for an already-connected app deployment status. Do not run generic probes such as command -v axhub, pwd, ls, find, cat, echo checks, bash -lc, sh -c, pipes, redirects, grep, head, tail, sed, awk, jq, or sleep. Use one direct axhub CLI command per visible tool call with Korean titles: axhub plugin-support init-resume route --json, axhub apps get <app> --json, axhub deploy list --app <app> --json, axhub apps bootstrap-status <bootstrap-id> --tenant <tenant> --json, axhub deploy status <deployment-id> --tenant <tenant> --json, and axhub deploy verify <deployment-id> --app <app> --json. Never call Claude Desktop axhub App/MCP tools such as Deployment list (axhub), App get (axhub), or Tenant recent deployments (axhub), and never run nonexistent axhub deployment list."}}'
    ;;
esac
