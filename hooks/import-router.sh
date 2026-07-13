#!/usr/bin/env bash
set -euo pipefail

if [ -n "${AXHUB_NO_IMPORT_ROUTER:-}" ]; then
  exit 0
fi
[ -f "$HOME/.axhub/config/no-import-router" ] && exit 0

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
  *최신*|*버전*|*업데이트*|*latest*|*'up to date'*|*'version check'*|*update*|*upgrade*)
    exit 0
    ;;
esac

case "$prompt_part" in
  *기존*|*이미\ 만든*|*작업\ 폴더*|*이\ 폴더*|*이\ 앱*|*Express*|*Fastify*|*Nest*|*FastAPI*|*Flask*|*Django*|*Rails*|*Dockerfile*|*Go\ 서버*|*Rust*|*Java*|*PHP*|*.NET*|*import\ existing\ app*)
    cat <<'JSON'
{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"[axhub import router] The current user prompt asks to bring or first-deploy an existing local app to axhub. Before generic Code-mode folder inspection, file reads, shell probes, bootstrap, deploy, update, clarity, Finding tools, or Claude Desktop axhub App/MCP tools, invoke the axhub import skill. The first visible assistant body must start exactly: 기존 앱을 axhub에 가져올 준비를 확인할게요. If the user supplied 작업 폴더/path, treat that absolute path as APP_DIR and run all import-related axhub, git, npm, build, and manifest commands inside it. Visible shell permission cards must start with cd \"<the actual absolute APP_DIR from the user prompt>\" &&, not bare axhub/git/npm and not literal $APP_DIR, unless the tool visibly sets cwd to that exact directory. Do not use the selected Code workspace root as a substitute when it is the parent folder. Do not read package.json, list files, or say bootstrap before invoking import. Never run axhub apps bootstrap for existing/local-folder apps. Use CLI-only import preview first, then ask the import confirmation and proceed only after approval."}}
JSON
    ;;
esac
