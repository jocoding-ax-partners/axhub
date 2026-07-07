#!/usr/bin/env bash
set -euo pipefail

if [ -n "${AXHUB_NO_IMPORT_ROUTER:-}" ]; then
  exit 0
fi

input="$(cat)"

case "$input" in
  *axhub*) ;;
  *) exit 0 ;;
esac

case "$input" in
  *최신*|*버전*|*업데이트*|*latest*|*'up to date'*|*'version check'*|*update*|*upgrade*)
    exit 0
    ;;
esac

case "$input" in
  *기존*|*이미\ 만든*|*작업\ 폴더*|*이\ 폴더*|*이\ 앱*|*Express*|*Fastify*|*Nest*|*FastAPI*|*Flask*|*Django*|*Rails*|*Dockerfile*|*Go\ 서버*|*Rust*|*Java*|*PHP*|*.NET*|*import\ existing\ app*)
    cat <<'JSON'
{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"[axhub import router] The current user prompt asks to bring or first-deploy an existing local app to axhub. Before generic Code-mode folder inspection, file reads, shell probes, bootstrap, deploy, update, clarity, Finding tools, or Claude Desktop axhub App/MCP tools, invoke the axhub import skill. The first visible assistant body must start exactly: 기존 앱을 axhub에 가져올 준비를 확인할게요. If the user supplied 작업 폴더/path, treat that absolute path as APP_DIR and run all import-related axhub, git, npm, build, and manifest commands inside it. Visible shell permission cards must start with cd \"<the actual absolute APP_DIR from the user prompt>\" &&, not bare axhub/git/npm and not literal $APP_DIR, unless the tool visibly sets cwd to that exact directory. Do not use the selected Code workspace root as a substitute when it is the parent folder. Do not read package.json, list files, or say bootstrap before invoking import. Never run axhub apps bootstrap for existing/local-folder apps. Use CLI-only import preview first, then ask the import confirmation and proceed only after approval."},"systemMessage":"[axhub import router] Existing/local-folder axhub app imports must route through the axhub import skill before generic Code-mode probes. First visible assistant body starts with: 기존 앱을 axhub에 가져올 준비를 확인할게요. If 작업 폴더/path is supplied, pin that actual absolute path as APP_DIR. Every visible import-related command must either run with cwd set to that exact directory or begin cd \"<actual absolute APP_DIR>\" &&; never use bare axhub/git/npm from the parent workspace and never run axhub apps bootstrap before import."}
JSON
    ;;
esac
