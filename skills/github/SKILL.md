---
name: github
description: '이 스킬은 사용자가 axhub 앱과 GitHub repo 를 연결하거나 끊고 싶어할 때 사용해요. 다음 표현에서 활성화: "GitHub 연결", "repo 연결", "GitHub repo 연결해", "내 repo 붙여", "git 연결", "repo 끊어", "github disconnect", 또는 GitHub 연동 의도. GitHub App 설치가 없으면 install URL 을 안내해요.'
multi-step: true
needs-preflight: true
---

# GitHub

axhub 앱과 GitHub repo 연결 상태를 안전하게 확인하고 connect/disconnect 를 consent 로 보호해요.

## Workflow

To connect GitHub:

!`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "앱과 auth 컨텍스트 확인", status: "in_progress", activeForm: "컨텍스트 확인 중" },
     { content: "GitHub 작업 분기", status: "pending", activeForm: "작업 고르는 중" },
     { content: "repo 연결 상태 처리", status: "pending", activeForm: "GitHub 처리 중" },
     { content: "다음 deploy 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   같은 순서로 사용자에게 짧은 단계표도 보여줘요:

   ```
   작업 단계
   └ □ 앱과 auth 컨텍스트 확인
     □ GitHub 작업 분기
     □ repo 연결 상태 처리
     □ 다음 deploy 안내
   ```

1. **preflight 와 current app 을 확인해요.** 앱이 없으면 `apps` skill 흐름으로 먼저 고르게 해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `목록만` 예요.

2. **작업을 고르게 해요.**

   ```json
   {
     "question": "GitHub 연동 작업을 고를까요?",
     "header": "GitHub",
     "options": [
       {"label": "목록만", "value": "repos", "description": "연결 가능한 repo 목록을 봐요"},
       {"label": "연결", "value": "connect", "description": "앱에 repo 와 branch 를 연결해요"},
       {"label": "연결 해제", "value": "disconnect", "description": "exact confirm 과 consent 가 필요해요"}
     ]
   }
   ```

3. **repo 목록은 read-only 로 실행해요.**

   ```bash
   axhub github repos list --account "$ACCOUNT" --json
   ```

4. **connect 는 consent 후 실행해요.**

   ```bash
   axhub github connect "$APP" --repo "$OWNER_REPO" --branch "$BRANCH" --account "$ACCOUNT" --json
   ```

   `consent-mint` 에 `action=github_connect`, top-level `app_id`, `context={repo,branch,account}` 를 넣어요. GitHub App 설치가 없다는 응답이면 install URL 만 안내해요.

5. **disconnect 는 exact confirm 후 실행해요.**

   ```bash
   axhub github disconnect "$APP" --force --confirm "$APP" --json
   ```

   `consent-mint` 에 `action=github_disconnect`, top-level `app_id`, `context={slug}` 를 넣어요.

## NEVER

- NEVER GitHub App install URL 을 자동으로 열거나 권한을 부여하지 않아요.
- NEVER owner/repo 를 추측해서 connect 하지 않아요.
- NEVER disconnect 를 subprocess 에서 자동 실행하지 않아요.
- NEVER `--json` 을 빼지 않아요.
