---
name: init
description: 이 스킬은 사용자가 새 axhub 앱을 만들거나 템플릿으로 프로젝트를 시작하고 싶어할 때 사용해요. 다음 표현에서 활성화: "새 앱 만들어줘", "결제 앱 만들어줘", "프로젝트 만들어", "Next.js 앱 만들어줘", "FastAPI 앱 만들어줘", "init", "scaffold", "axhub.yaml 만들어줘", "apphub.yaml 만들어줘", 또는 빈 디렉토리에서 새 앱 시작 의도. ax-hub-cli 의 init template 목록을 보여주고 선택한 template 으로 scaffold 해요.
multi-step: true
needs-preflight: false
---

# Init

새 axhub 앱을 current CLI template registry 에서 시작해요. v0.2.0 에서는 helper bootstrap, dependency install, remote template fetch 를 하지 않고 `axhub --json init --list-templates` 를 source of truth 로 써요.

## Workflow

To start an axhub app:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "CLI와 template registry 확인", status: "in_progress", activeForm: "CLI 확인 중" },
     { content: "template 선택", status: "pending", activeForm: "template 고르는 중" },
     { content: "axhub init 실행", status: "pending", activeForm: "프로젝트 만드는 중" },
     { content: "다음 액션 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

1. **CLI 존재를 확인해요.**

   ```bash
   axhub --version
   ```

   실패하면 install/update 안내를 짧게 보여주고 중단해요. auth 는 init template 목록 조회에 필요하지 않아요.

2. **CLI registry 에서 template 목록을 읽어요.**

   ```bash
   axhub --json init --list-templates
   ```

   `schema_version` 이 `init/v1` 인지 확인하고, `templates[].id`, `framework`, `description` 만 사용자에게 보여줘요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — template 선택은 `abort` 예요.

3. **template 을 선택해요.**

   ```json
   {
     "question": "어떤 템플릿으로 시작할까요?",
     "header": "템플릿",
     "options": [
       {"label": "목록에서 선택", "value": "select", "description": "CLI registry 에서 받은 template 중 하나를 골라요"},
       {"label": "직접 입력", "value": "manual", "description": "template id 를 직접 입력해요"},
       {"label": "취소", "value": "abort", "description": "scaffold 를 실행하지 않아요"}
     ]
   }
   ```

   subprocess 에서는 자동 선택하지 않아요.

4. **선택된 template 으로 scaffold 해요.**

   ```bash
   axhub init --from-template "$TEMPLATE_ID" --json
   ```

   visible examples 는 `apphub.yaml` 을 만들 수 있고, legacy builtin flow 는 `axhub.yaml` 을 만들 수 있어요. 둘 다 정상 결과로 다뤄요.

5. **결과와 다음 액션을 안내해요.** 앱 등록, GitHub 연결, env 설정, deploy, open 흐름을 자연어로 이어갈 수 있다고 짧게 말해요.

## NEVER

- NEVER Node, package manager, dependency install 을 자동 실행하지 않아요.
- NEVER helper `fetch-template` 또는 remote `templates.json` 를 v0.2.0 source 로 쓰지 않아요.
- NEVER subprocess 에서 template 을 임의로 고르지 않아요.
- NEVER auth 실패를 init template 조회 실패로 오해하지 않아요.
