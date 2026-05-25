---
name: setup
description: '이 스킬은 axhub 를 처음 쓰는 사람이 셋업/온보딩 전체 과정을 한 번에 진행하고 싶어할 때 사용해요. 다음 표현에서 활성화: "셋업해줘", "셋업 해줘", "처음인데", "처음 사용", "처음 써", "온보딩", "온보딩해줘", "시작하기", "axhub 시작", "axhub 처음", "초기 셋업", "setup", "set up", "onboard", "onboarding", "getting started", "get started", "first time", 또는 첫 사용자 셋업 의도. axhub CLI 설치(install-cli)·로그인(auth)·node 환경 감지를 순서대로 안내하고, node 가 없으면 consent 후 설치해요. 끝나면 첫 앱 만들기(init)로 연결해요. 환경 진단(doctor)이나 새 앱 초기화(init)와 달리 처음 사용자의 순차 온보딩을 담당해요.'
examples:
  - utterance: "셋업해줘"
    intent: "onboard axhub first-time setup"
  - utterance: "처음인데 어떻게 시작해"
    intent: "onboard axhub first-time setup"
  - utterance: "온보딩"
    intent: "onboard axhub first-time setup"
  - utterance: "getting started"
    intent: "onboard axhub first-time setup"
  - utterance: "set up axhub"
    intent: "onboard axhub first-time setup"
  - utterance: "first time using axhub"
    intent: "onboard axhub first-time setup"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: sonnet
---

# Setup (first-run onboarding orchestrator)

처음 axhub 를 쓰는 사람을 위한 순차 온보딩 진입점이에요. CLI 설치 → 로그인 → node 환경 →
(없으면) 설치 → 준비 완료 → 첫 앱까지 손잡고 안내해요. 설치 로직은 기존 skill
(`install-cli`/`auth`/`init`)에 위임하고, setup 은 순서와 node 환경만 직접 책임져요.

## Workflow

To onboard a first-time user:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "CLI 설치 확인",   status: "in_progress", activeForm: "CLI 보는 중" },
     { content: "로그인 확인",     status: "pending",     activeForm: "로그인 보는 중" },
     { content: "node 환경 확인",  status: "pending",     activeForm: "node 보는 중" },
     { content: "준비 상태 안내",  status: "pending",     activeForm: "정리하는 중" },
     { content: "첫 앱 안내",      status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

**핸드오프 모델:** setup 은 sibling skill 을 `Skill()` 로 위임해요. Claude Code 에서 `Skill()` 은
콘텐츠를 같은 대화 흐름에 로드하므로, 위임한 skill 이 끝나면 제어가 setup 으로 돌아와요. 그래서
setup 은 **먼저 전체 상태를 감지(detect-first)** 한 뒤 첫 번째 빈 곳(gap)으로만 위임하고, 복귀하면
다음 gap 으로 이어가요. 위임한 skill 이 자기 AskUserQuestion 에서 끝나서 제어가 안 돌아오면, ready
카드가 이미 다음에 말할 자연어(예: "로그인해줘")를 안내해 둬요. 어느 쪽이든 사용자는 막히지 않아요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — node 설치 → `나중에` (subprocess 에선 런타임 자동설치 안 해요), 첫 앱 → `아니요` (새 앱 자동 생성 안 해요).

1. **전체 상태 감지 (read-only, 위임 전).** preflight 를 먼저 부르지 말아요 — CLI 가 아직 없을 수 있어요.

   ```bash
   # CLI — 부재 가능, --version 으로만 확인
   axhub --version 2>/dev/null || echo "cli-missing"
   # node — 머신 레벨 존재 확인 (없으면 Step 4 에서 설치)
   node --version 2>/dev/null || echo "node-missing"
   # node 가 있을 때만 부가 advisory: pm 선호(lockfile) + 권장 버전(.nvmrc/engines)
   ls bun.lockb pnpm-lock.yaml package-lock.json yarn.lock 2>/dev/null
   cat .nvmrc 2>/dev/null; node -p "require('./package.json').engines?.node" 2>/dev/null
   ```

   auth 는 CLI 가 확인됐을 때만 helper preflight 로 확인해요. CLI 가 없으면 auth 는 "CLI 설치 후 확인" 으로 표시해요.

   ```bash
   "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers" preflight --json   # auth_ok, user_email
   ```

2. **온보딩 체크리스트 카드.** 현 상태를 ✓/✗ 로 보여주고 첫 번째 gap 을 안내해요 (해요체).

   ```
   axhub 온보딩 상태:
     ✓ CLI:    v<CLI_VERSION>     (또는 ✗ 아직 설치 안 됨)
     ✓ 로그인:  <USER_EMAIL>       (또는 ✗ 로그인 필요 / CLI 설치 후 확인)
     ✓ node:    v<NODE_VERSION>    (또는 ✗ 없음 — 설치 안내할게요)
   ```

3. **첫 gap 으로만 위임 (consent).** 위에서 본 첫 번째 ✗ 를 채워요.

   - CLI ✗ → `Skill("axhub:install-cli")`
   - CLI ✓ · 로그인 ✗ → `Skill("axhub:auth")`
   - node ✗ → Step 4
   - 위임이 끝나 제어가 돌아오면 → Step 1 로 다시 감지해서 다음 gap. 안 돌아오면 → Step 5 카드가 다음 phrase 안내.

4. **node 런타임 consent-gate 설치.** silent 설치는 안 해요 — consent 한 번 받고 진행해요. node 는 axhub 와 달리 공식 `curl|bash` installer 가 없어서, 최선 방법 fallback chain 으로 설치해요.

   ```json
   {
     "questions": [{
       "question": "node 가 없어요. 지금 설치할까요?",
       "header": "node 설치",
       "multiSelect": false,
       "options": [
         {"label": "지금 설치", "description": "있는 패키지 매니저로 설치, 없으면 nvm/fnm 로 설치"},
         {"label": "나중에", "description": "지금은 그대로 두고 안내만 보기"}
       ]
     }]
   }
   ```

   "지금 설치" 선택 시 fallback chain (consent 받았으니 Bash 로 실행):

   - **1순위 — 이미 있는 패키지 매니저** (가장 깨끗): node 는 axhub CLI 와 달리 OAuth client_id 발급이 없어서, install-cli 가 pm 채널을 피하는 이유(pm 갱신 지연 → 구버전 client_id → 로그인 실패)가 node 에는 해당 안 돼요. 그래서 node 는 pm 설치가 안전해요.
     - macOS: `brew install node`
     - Windows: `winget install OpenJS.NodeJS.LTS` 또는 `scoop install nodejs-lts`
     - Linux: `apt-get install -y nodejs` / `dnf install -y nodejs` / `pacman -S nodejs` (배포판별)
   - **2순위 — 패키지 매니저 없음 → nvm 설치 스크립트** (⚠️ supply-chain deviation, 아래 NEVER 예외 참고):
     ```bash
     curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
     # shell rc 재로드 후
     nvm install --lts
     ```
     nvm 버전 태그(`v0.40.1`)는 핀 고정해요 — 임의 latest 금지. 도메인은 `raw.githubusercontent.com/nvm-sh` 한정.
   - **3순위 — 다 안 되면** → https://nodejs.org LTS 다운로드를 안내만 해요.

   설치 후 `node --version` 으로 재확인해요.

5. **준비/handoff 카드.** doctor 스타일 ✓/✗ 로 마무리해요.

   ```
   axhub 셋업 결과:
     ✓ CLI:    v<CLI_VERSION>
     ✓ 로그인:  <USER_EMAIL>
     ✓ node:    v<NODE_VERSION> (pm: <bun|npm|pnpm>)
   이제 배포할 준비 끝났어요!
   ```

   - **PATH 갱신 갭:** 방금 설치했는데 `--version` 재확인이 실패하면, installer 는 성공했지만 현재 셸 PATH 가 아직 갱신 안 된 거예요. ✗ 로 끝내지 말고 "새 터미널을 열거나 셸 설정을 다시 불러온 뒤 '셋업해줘' 라고 다시 말해주세요" 라고 안내해요.
   - **로그인 위임 실패** (브라우저 닫음/토큰 취소): ✗ 표시 + "'다시 로그인해줘' 라고 말해주세요" 안내하고 멈추지 않아요.
   - **node 버전 mismatch** (.nvmrc/engines 와 다름): ⚠ 경고만 하고 진행해요 — 런타임이 있으면 배포는 돼요. 권장 버전은 알려줘요.

6. **첫 앱 연결.** 모두 ✓ 이고 현재 디렉토리가 비었거나 `axhub.yaml`/`apphub.yaml` 이 없으면, 첫 앱 만들기를 제안해요.

   ```json
   {
     "questions": [{
       "question": "첫 앱 만들래요?",
       "header": "첫 앱",
       "multiSelect": false,
       "options": [
         {"label": "네", "description": "init 으로 새 앱 scaffold 시작"},
         {"label": "아니요", "description": "지금은 안 만들고 마무리"}
       ]
     }]
   }
   ```

   "네" 선택 시 → `Skill("axhub:init")`. 이미 앱이 있으면 "'배포해' 라고 하시면 올려드려요" 로 마무리해요.

## NEVER

- NEVER preflight 를 Step 1 의 CLI 확인 이전에 호출 — CLI 부재 상태로 fire 되면 무한 루프 위험 (`install-cli` NEVER 미러).
- NEVER node 런타임을 확인 없이 silent 설치 — consent 한 번은 필수예요.
- **node 설치 예외 (사용자 명시 선택):** consent 후 패키지 매니저 실행 + (없으면) nvm/fnm 설치 스크립트 pipe 를 허용해요. 단 (a) consent 필수, (b) nvm/fnm 버전 태그 핀 고정, (c) nodejs.org/nvm-sh/fnm 외 도메인 금지. axhub CLI 외 third-party 자동설치의 유일 예외라, 다른 skill 로 무단 확산하지 말아요.
- NEVER 프로젝트 의존성 자동설치(npm/bun 의 install)를 여기 추가 — `allows-dependency-execution: false` 라 CI 가 막아요. 별 PR 로 다뤄요.
- NEVER `install-cli`/`auth`/`init` 의 설치·로그인 로직을 재구현 — `Skill()` 위임만 해요.
- NEVER subprocess(`claude -p`/CI/headless)에서 자동 설치하거나 위임 — D1 guard 로 안전 기본값만.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `../install-cli/SKILL.md` — OS 감지 / race-check / post-verify 패턴 재사용 source.
