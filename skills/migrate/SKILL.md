---
name: migrate
description: 이 스킬은 기존 앱 올려줘, 만든 앱 가져오기, migrate, import existing app, 기존 프로젝트 배포 의도에서 활성화해요.
examples:
  - utterance: "기존 앱 올려줘"
    intent: "migrate an existing app into axhub"
  - utterance: "이 Next.js 프로젝트 axhub 로 가져와"
    intent: "migrate an existing app into axhub"
  - utterance: "migrate this repo"
    intent: "migrate an existing app into axhub"
  - utterance: "import existing app"
    intent: "migrate an existing app into axhub"
  - utterance: "이미 만든 앱 배포해줘"
    intent: "migrate an existing app into axhub"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# 기존 앱 가져오기

이미 만든 웹 앱을 axhub 앱으로 등록하고, 감지 결과를 확인한 뒤 기존 배포 경로로 올려요. 로컬 감지는 helper 가 하고, 권위 있는 감지와 배포는 backend/CLI 경로를 재사용해요.

## Workflow

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "로컬 프로젝트 구조와 후보 앱을 감지해요", status: "in_progress", activeForm: "후보 앱을 감지하는 중" },
     { content: "가져올 앱과 감지 신뢰도를 확인해요", status: "pending", activeForm: "감지 결과를 확인하는 중" },
     { content: "axhub.yaml 초안과 필수 env 안내를 준비해요", status: "pending", activeForm: "manifest 초안을 준비하는 중" },
     { content: "기존 consent/CLI 경로로 앱 등록·git 연결·배포를 실행해요", status: "pending", activeForm: "배포 경로를 실행하는 중" },
     { content: "라이브 URL 과 다음 수정 포인트를 안내해요", status: "pending", activeForm: "결과를 정리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **로컬 light pre-scan.** 현재 디렉터리 또는 사용자가 지정한 경로를 helper 로 감지해요. helper 결과는 후보·힌트·env 이름만 다루고, secret 값은 절대 출력하지 않아요.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
   "$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --json
   ```

2. **후보 선택과 confidence 확인.** 후보가 2개 이상이면 앱 하나를 고르게 해요. confidence 가 `0.80` 이상이면 확인 후 진행하고, `0.60..0.79` 는 수정 가능한 계획으로 보여줘요. `0.60` 미만, start command 없음, 후보 모호함은 진행을 막고 `axhub.yaml` 또는 Dockerfile/compose 를 요청해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

   ```json
   {
     "questions": [{
       "question": "어느 앱을 가져올까요?",
       "header": "앱 선택",
       "multiSelect": false,
       "options": [
         {"label": "첫 후보", "description": "가장 높은 confidence 후보 하나만 가져와요"},
         {"label": "직접 선택", "description": "후보 목록에서 다른 앱 경로를 고르게 해요"},
         {"label": "중단", "description": "후보가 모호하면 변경 없이 멈춰요"}
       ]
     }]
   }
   ```

   ```json
   {
     "questions": [{
       "question": "감지 계획으로 배포할까요?",
       "header": "계획 확인",
       "multiSelect": false,
       "options": [
         {"label": "계속", "description": "현재 감지 계획으로 consent 발급과 배포를 진행해요"},
         {"label": "manifest만", "description": "axhub.yaml 초안만 만들고 배포는 멈춰요"},
         {"label": "중단", "description": "파일과 원격 상태를 바꾸지 않아요"}
       ]
     }]
   }
   ```

3. **manifest 초안 준비.** helper 의 `suggested_manifest` 를 `axhub.yaml` 초안으로 보여줘요. required env 는 이름과 scope 만 포함하고, 값 설정은 `axhub env set` 경로로 안내해요. 기존 `apphub.yaml` 이 있으면 읽기는 계속 되지만 새 파일은 `axhub.yaml` 로 만들어요.

4. **기존 mutation 경로 재사용.** 앱 등록, git 연결, env 값 저장, 배포는 기존 CLI/consent 경로만 써요. helper 로 consent 를 우회하지 않아요. backend preview 는 `POST /api/v1/apps/detect` 를 사용하고, local dir 은 archive 또는 GitHub repo-ref 형태로만 넘겨요.

5. **결과 검증.** 배포가 끝나면 live URL, deployment id, 감지된 build/runtime env 구분, Dockerfile/compose/auto 중 선택된 ladder 를 보여줘요. 실패하면 deploy error empathy catalog 형식으로 원인·확인 방법·재시도 명령을 짧게 안내해요.

## NEVER

- NEVER secret 값을 `axhub.yaml`, 로그, 질문 옵션, TodoWrite 에 넣지 않아요.
- NEVER low confidence 나 다중 후보를 조용히 배포하지 않아요.
- NEVER 앱 등록·git 연결·배포 consent 를 helper 로 우회하지 않아요.
- NEVER 새 배포 경로를 만들지 않고 기존 CLI/backend 경로를 재사용해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
