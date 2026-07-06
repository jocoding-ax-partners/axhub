---
name: update
description: '현재 버전을 확인할게요. 스킬 실행 전 사용자 문장 0개: 먼저 아무 말 없이 이 스킬을 실행해요. For explicit axhub CLI and Claude Code plugin version check/update requests: "업데이트해줘", "axhub 최신 버전으로", "플러그인 업데이트", "update axhub", "Check whether axhub is up to date and update it if needed." Invoke this skill before writing any explanatory assistant sentence. First visible assistant text must start exactly with "현재 버전을 확인할게요." Start directly; do not explain why this path was chosen or name the chosen skill. Mixed app-status/log/deploy requests after update still stop after version check/update: write exactly "업데이트 확인은 끝났어요. 앱 상태 확인은 별도 상태 확인 흐름에서 처리해요." and end the assistant turn. Standalone first setup=onboarding, new app=bootstrap, deploy=deploy. axhub CLI·플러그인 버전 전용이라, 발화의 axhub 언급이나 대화의 axhub 맥락(직전 axhub 작업)이 있을 때만 이 스킬을 써요 — 맥락 없는 일반 "업데이트해줘" 나 npm·다른 도구 업데이트 발화에는 쓰지 않아요.'
examples:
  - utterance: "업데이트해줘"
    intent: "update axhub cli and plugin to latest"
  - utterance: "axhub 최신 버전으로 맞춰줘"
    intent: "update axhub cli and plugin to latest"
  - utterance: "axhub가 최신인지 확인하고, 아니면 최신으로 업데이트해줘"
    intent: "check and update axhub cli and plugin even in an empty workspace"
  - utterance: "check whether axhub is up to date and then tell me my app status"
    intent: "check/update axhub only, then stop this assistant turn without app status tools"
  - utterance: "update axhub"
    intent: "update axhub cli and plugin to latest"
allows-dependency-execution: false
model: sonnet
---

# 버전 업데이트 (수동 on-demand)

**CRITICAL desktop first line.** Claude Desktop 에서 이 경로가 선택되면 사용자에게 보이는 첫 문장은 반드시 정확히 `현재 버전을 확인할게요.` 여야 해요. 스킬 호출 전 사전 안내 문장도 쓰지 않아요. 그 앞에 어떤 설명도 붙이지 않아요. 선택한 스킬 이름이나 선택 이유도 말하지 않아요. native UI 가 자동으로 붙이는 배지는 제어할 수 없지만, assistant 본문에는 같은 말을 반복하지 않아요.

**CRITICAL desktop-visible probe narration.** `claude plugin list` 는 내부 판정을 위한 도구예요. 이 도구 뒤에 사용자에게 보이는 중간 문장은 반드시 `현재 플러그인 버전을 확인했어요.` 또는 생략 둘 중 하나예요. 플러그인 버전값과 설치 위치값을 같은 문장에 섞지 않아요. `scope`, `user`, `project`, `local`, `managed`, `Scope:` 같은 설치 위치 원문은 chat 에 쓰지 않아요. 버전 숫자는 최종 결과 카드나 업데이트 안내처럼 사용자에게 필요한 자리에서만 보여줘요.

**CRITICAL mixed-request hard stop.** 사용자가 "업데이트 확인하고 앱 상태도 봐줘"처럼 다른 axhub 운영 요청을 함께 말해도, 이 스킬 실행 중에는 앱 목록·앱 상태·최근 배포·로그·환경변수 조회를 절대 이어서 실행하지 않아요. 결과 카드 뒤에는 `업데이트 확인은 끝났어요. 앱 상태 확인은 별도 상태 확인 흐름에서 처리해요.` 라고만 쓰고 **현재 assistant turn 을 즉시 끝내요.** 원문이 영어로 `then`, `and then`, `after that`, `help me understand` 를 써도 두 번째 요청을 같은 응답에서 만족시키지 않아요. `앱 상태는 이어서 확인할게요`, `이제 이어서 앱 상태를 확인할게요`, `현재 앱 상태를 정리해드릴게요`, `결과 나오는 대로 알려드릴게요`처럼 후속 작업을 약속하거나 실제 상태를 요약하는 문장은 쓰지 않아요.

**CRITICAL no background handoff.** 이 스킬에서는 Task/Subagent/Agent/백그라운드 작업을 만들지 않아요. `axhub 앱 상태 조회`, `axhubed 앱 상태 조회`, `앱 상태 조회`, `상태 조회`, `앱 상태 백그라운드 조회` 같은 작업·카드·제목이 떠오르면 실행하지 말고 결과 카드에서 멈춰요.

사용자가 직접 **axhub CLI 와 Claude Code 플러그인을 지금 최신으로** 맞추려는 요청이에요. 제거된 자동 훅에 의존하지 않고, 사용자가 명시적으로 요청한 순간에만 버전 확인과 적용을 진행해요:

- **항상 즉시 확인** — 사용자가 부른 수동 실행이라 바로 버전을 확인해요.
- **최신이어도 결과 보고** — "이미 최신이에요 (CLI vX, plugin vY)" 처럼 결과를 한 줄로 알려요. 사용자가 물었으니 답을 줘요.

전 과정 best-effort·비차단이에요. 실패·구 CLI·네트워크 오류면 raw 에러를 숨기고 한 줄만 안내한 뒤 멈춰요.

**책임 경계.** 이 경로는 버전 업데이트만 해요. 첫 셋업·CLI 설치는 `onboarding` 소관이고, 그 외 axhub 운영 명령은 새 흐름에 양보한 뒤 이 응답에서 처리하지 않고 멈춰요.

**첫 응답 계약.** 선택 이유를 설명하지 않아요. 빈 폴더여도 "axhub 프로젝트가 아니다" 라고 추론하지 말고 바로 버전 확인을 진행해요.

**섞인 요청 처리.** 사용자가 "최신인지 확인하고 내 앱 상태도 봐줘"처럼 버전 확인과 다른 axhub 운영 요청을 함께 말해도, 이 스킬은 **버전 확인/업데이트 결과까지만** 처리해요. 앱 목록·앱 상태·배포 상태·로그·환경변수·데이터 조회는 여기서 직접 실행하지 않아요. 특히 Claude Desktop 에 노출되는 `App list (axhub)`, `Tenant recent deployments (axhub)`, `App get (axhub)` 같은 axhub App/MCP 도구는 read 라도 이 스킬에서 호출하지 않아요. `앱 상태 조회`, `배포 상태 조회`, `최근 배포 조회` 같은 tool 제목이 떠올랐다면 이 스킬에서는 실행하지 말고 멈춰요. Task/Subagent/Agent 로 우회하지 않아요.

**보이는 tool 제목 계약.** Bash/명령 도구를 부를 때 description/title/summary 는 아래 고정 한국어 라벨 중 하나만 써요. 라벨 안에 `axhub` 를 넣지 않아요. `axhubing CLI 설치 여부 확인` 처럼 제품명을 영어 동사처럼 만든 제목은 절대 쓰지 않아요.

**Desktop-visible command allowlist.** Bash/명령 도구로 사용자에게 보일 수 있는 command 는 아래 계열만 써요: `command -v axhub`, `axhub update check ...`, `axhub update apply --execute --yes`, `axhub --version`, `command -v claude`, `claude plugin list`, `claude plugin update axhub@axhub --scope <SCOPE>`. Claude Desktop 에서는 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 같은 플러그인 캐시 파일을 읽지 않아요. 캐시 경로가 작업 디렉토리 밖이라 초보자에게 불필요한 읽기 권한 팝업이 떠요. 플러그인 현재 버전은 가능하면 **정확히 `claude plugin list` 만 실행한 출력**의 `axhub@axhub` 항목에서 내부 변수로만 읽고, `claude` CLI 가 없거나 파싱할 수 없으면 플러그인 버전 비교는 생략해요. `claude plugin list 2>&1`, `claude plugin list 2>&1 | grep ...`, `sed`, `awk`, `head`, `tail`, command substitution, shell wrapper, file test, pipe, redirect, text filter, 파일 읽기 도구를 쓰지 않아요. 실행하려는 command 가 `claude plugin list 2>&1` 로 떠오르면 **반드시 `claude plugin list` 로 바꿔요.** 출력이 길어도 전체 `claude plugin list` 결과를 도구 응답에서 내부적으로 읽고 사용자에게 echo 하지 않아요. 플러그인 버전, 설치 scope, 다음 CLI 확인을 영어 내부 로그처럼 chat 에 쓰지 말고, 필요한 경우 `현재 플러그인 버전을 확인했어요.` 라고만 말해요. 버전과 설치 위치를 같은 문장에 섞지 않아요.

| 단계 | tool description/title/summary |
| --- | --- |
| CLI 존재 확인 (`command -v axhub`) | `CLI 설치 확인` |
| 버전 확인 (`axhub update check ...`) | `버전 확인` |
| CLI 업데이트 적용 | `CLI 업데이트 적용` |
| 업데이트 후 버전 재확인 | `업데이트 후 버전 확인` |
| 플러그인 설치 위치 확인 | `플러그인 설치 위치 확인` |
| 플러그인 업데이트 적용 | `플러그인 업데이트 받기` |

---

## 진행 체크리스트 (TodoWrite — 있을 때만)

TodoWrite 도구가 host 에 노출됐을 때만 호출해요 (Claude Desktop 처럼 없으면 조용히 진행하고, 도구 가용성·생략은 언급하지 않아요). 고정 목록을 붙여넣지 말고 **이번 실행의 실제 일에서 도출**해요 — 버전 확인 뒤 CLI·플러그인 중 이미 최신인 쪽은 바로 `completed` 로 시작하고, `disabled`·`AXHUB_NO_AUTO_UPDATE` 로 안내-only 인 항목은 적용 대신 "안내" 로 닫아요. 참고 shape:

```typescript
TodoWrite({ todos: [
  { content: "버전 확인 (axhub update check)", status: "in_progress", activeForm: "버전 확인하는 중" },
  { content: "CLI 업데이트",                  status: "pending",     activeForm: "CLI 업데이트하는 중" },
  { content: "플러그인 업데이트",              status: "pending",     activeForm: "플러그인 업데이트하는 중" },
  { content: "결과 보고",                     status: "pending",     activeForm: "결과 정리하는 중" }
]})
```

**태스크 하나가 끝날 때마다** 전체 todos 배열로 다시 호출해 끝난 항목은 `completed`, 다음 항목은 `in_progress` 로 갱신해요 — 끝에 한꺼번에 말고 매 태스크 직후에요. 이전 스킬 todo 가 남아 있으면 patch 하지 말고 위 배열 전체로 교체해요. 종료 시 미완료 todo 0 개.

---

## 0. 사전 점검 (네트워크 0)

1. `command -v axhub` 가 실패하면 멈춰요 — CLI 가 아직 없는 건 설치 소관이에요. 한 줄: `axhub CLI 가 아직 없어요. "온보딩" 이라고 말하면 설치부터 도와드려요.` (재설치를 여기서 시도하지 않아요.)
2. Claude Desktop 에서는 플러그인 캐시의 `plugin.json` 을 읽지 않아요. 먼저 `command -v claude` 를 확인하고, 가능하면 `claude plugin list` 에서 `axhub@axhub` 의 현재 버전을 내부 변수 `<PLUGIN_VERSION>` 으로만 둬요. `claude` CLI 가 없거나 목록에서 못 찾으면 `<PLUGIN_VERSION>` 없이 CLI 업데이트 확인만 진행해요. 이 단계에서 설치 경로, Scope, manifest 경로, raw 목록, `user scope`/`local scope` 같은 scope 원문, 영어 진행 로그는 사용자에게 말하지 않아요. 필요한 경우 `현재 플러그인 버전을 확인했어요.` 만 말해요. 설치 위치값은 업데이트 명령의 `--scope` 인자에만 쓰고 chat 에는 쓰지 않아요.
3. `claude plugin list` 에 `axhub@axhub` 가 여러 번 나오면, enabled 항목 중 **가장 높은 semver** 를 `<PLUGIN_VERSION>` 으로 삼아요. 같은 버전이 여러 scope 에 있으면 업데이트 대상 `<SCOPE>` 는 현재 작업공간에 가장 가까운 항목(`local` → `project` → `user`)을 고르고, 이 선택 근거는 chat 에 쓰지 않아요. 낮은 버전이 함께 남아 있어도 사용자에게 중복 설치·scope 원문을 설명하지 않고, 최종 카드에는 선택된 최고 버전만 써요.

**중복 설치 판정 알고리즘.** `claude plugin list` 결과를 읽을 때는 먼저 모든 `axhub@axhub` block 을 끝까지 훑고, `Status: ✔ enabled` 인 block 만 모아 `version`, `scope` 를 내부 표로 만들어요. 그 다음 정렬해서 최고 semver 를 `<PLUGIN_VERSION>` 으로 확정한 뒤에만 `axhub update check --plugin-version <PLUGIN_VERSION> --json` 을 실행해요. 낮은 버전 block 이 남아 있어도 그것은 cleanup 대상이 아니며, 최신성 판정과 업데이트 대상 선택에서 무시해요. `<PLUGIN_VERSION>` 이 `plugin.latest` 이상이면 `plugin.has_update` 가 true 처럼 보여도 플러그인 업데이트를 실행하지 말고 `axhub 플러그인은 이미 최신이에요 (v<PLUGIN_VERSION>).` 로 닫아요. 즉, `local 1.8.2` 와 `user 1.8.0` 이 함께 있으면 현재 버전은 `1.8.2` 이고, `user 1.8.0 → 1.8.2` 같은 정리성 업데이트나 결과 카드를 만들지 않아요.

**`disabled` 와 `AXHUB_NO_AUTO_UPDATE` — 둘 다 존중해요 (자동 적용 안 함, 안내만).**
- `disabled`(패키지 매니저가 관리하는 설치) → CLI 가 자기를 교체할 수 없어요. 패키지 매니저 업그레이드를 **안내만** 해요.
- `AXHUB_NO_AUTO_UPDATE` → CLAUDE.md 가 문서화한 update kill switch 예요. 새 버전이 있어도 적용하지 않고 **안내만** 해요 (사용자가 직접 불러도요 — 잠긴·CI 환경에서 의도치 않은 binary swap 을 막아요). 받으려면 플래그를 끄거나 안내된 명령을 직접 실행하면 돼요.

---

## 1. 버전 확인 (네트워크 1회)

```bash
axhub update check --plugin-version <PLUGIN_VERSION> --json
```

- `--plugin-version` 은 CLI v0.21.0+ 에서 플러그인 최신 여부도 함께 판정해요. 구 CLI 가 이 플래그를 거부하면 (exit 64) `axhub update check --json` 으로 한 번 더 호출해 CLI-only 로 떨어져요.
- 수동 확인 기록은 Claude Desktop 경로에서 갱신하지 않아요. `axhub update check ...` 뒤에 별도 `mkdir`/touch/marker command 를 실행하지 말아요. 별도 로컬 기록 작업명은 chat 에 쓰지 않아요.

- 출력 JSON 을 읽어요:
  - CLI: `{ current, latest, has_update, disabled }`
  - (있으면) 플러그인: `plugin: { current, latest, has_update }`
- 호출이 실패하거나 JSON 이 비면 (구 CLI·네트워크 실패) 한 줄 안내 후 멈춰요: `버전 확인을 못 했어요. 잠시 뒤 다시 시도해 주세요.`

---

## 2. CLI 업데이트

먼저 **안내-only 조건**을 봐요: `disabled == true` (패키지 매니저 관리 설치) 또는 `AXHUB_NO_AUTO_UPDATE` 설정. 둘 중 하나면 적용하지 않고 안내만 해요.

- **안내-only + `has_update == true`** → 한 줄 안내:
  - `disabled` → `axhub 는 패키지 매니저가 관리하는 설치예요. 패키지 매니저로 업그레이드해 주세요 (예: brew upgrade axhub).`
  - `AXHUB_NO_AUTO_UPDATE` → `axhub 새 버전(v<latest>)이 있어요. AXHUB_NO_AUTO_UPDATE 설정이라 자동 적용은 안 해요 — axhub update apply 로 직접 받거나 플래그를 끄면 돼요.`
- **`has_update == false`** → `axhub 는 이미 최신이에요 (v<current>).` 한 줄.
- **`has_update == true` 이고 안내-only 가 아님** → 알리고 바로 적용해요:
  1. 한 줄: `axhub 새 버전(v<current> → v<latest>)이 나왔어요. 지금 업데이트할게요…`
  2. 실행: `axhub update apply --execute --yes`
  3. exit code 로 갈라요 (판정은 CLI 가 함):
     - **exit 0** → `axhub --version` 으로 재확인하고 한 줄: `axhub v<새 버전> 으로 업데이트됐어요.`
     - **exit 14 (digest mismatch — 변조 신호) / exit 66 (cosign_enforce_failed)** → **하드 스톱**. `보안 검증에 실패했어요. 강제로 진행하지 말고 회사 IT·보안팀에 알려주세요. 지금 버전은 그대로 써도 돼요.` 로 안내하고 멈춰요.
     - **exit 15 (swap failed)** → 자동 재시도하지 말고 `업데이트 적용 중 교체가 막혔어요. "설치 상태 진단해줘" 라고 말해 주세요.` 로 안내해요.
     - **exit 4 (미인증)** → `로그인이 필요해요. "다시 로그인해줘" 라고 말해 주세요.` 로 낮춰요.
     - **그 외 비-0** → raw 에러는 숨기고 한 줄: `자동 업데이트가 안 됐어요. axhub update apply 를 직접 한 번 실행해 주세요.`

---

## 3. 플러그인 업데이트 (`claude plugin update` — 재시작 후 반영)

- `plugin` 블록이 없거나 **`plugin.has_update == false`** 또는 `<PLUGIN_VERSION> >= plugin.latest` → `axhub 플러그인은 이미 최신이에요 (v<PLUGIN_VERSION 또는 plugin.current>).` 한 줄 (plugin 블록이 없으면 = 구 CLI 라 이 줄을 생략해요). 이때 낮은 중복 scope 가 있어도 `claude plugin update` 를 실행하지 않아요.
- **`command -v claude` 실패** (Claude Code CLI 없음) → 한 줄 안내만: `axhub 플러그인 새 버전(v<plugin.latest>)이 있어요. Claude Code 에서 /plugin update 로 받아 주세요.`
- **`AXHUB_NO_AUTO_UPDATE` 설정** → 적용하지 않고 한 줄 안내만: `axhub 플러그인 새 버전(v<plugin.latest>)이 있어요. AXHUB_NO_AUTO_UPDATE 설정이라 자동 적용은 안 해요 — claude plugin update axhub@axhub 로 직접 받거나 플래그를 끄면 돼요.`
- **`plugin.has_update == true` 이고 적용 가능하며 `<PLUGIN_VERSION> < plugin.latest`** → 적용해요:
  1. 설치 위치를 먼저 확인해요 — `claude plugin list` 출력에서 `axhub@axhub` 항목의 `Scope:` 값(user/project/local/managed)을 읽어 내부 변수 `<SCOPE>` 로만 둬요. 같은 이름이 여러 번 나오면 enabled 항목 중 가장 높은 semver 를 현재 버전으로 보고, **그 최고 버전을 가진 block 들 안에서만** `local` → `project` → `user` 순서로 `<SCOPE>` 를 골라요. 낮은 버전 block 의 scope 는 업데이트 대상이 아니며, 사용자에게는 `플러그인 설치 위치를 확인할게요.` 라고 말하고 `Scope:` 원문은 보여주지 않아요. 못 찾으면 `user` 로 둬요.
  2. 한 줄: `axhub 플러그인 새 버전(v<plugin.current> → v<plugin.latest>)이 나왔어요. 지금 받을게요…`
  3. 실행: `claude plugin update axhub@axhub --scope <SCOPE>`
  4. 성공하면 `claude plugin list` 를 한 번 더 실행해 `axhub@axhub` enabled 항목 중 가장 높은 semver 를 내부 변수 `<PLUGIN_UPDATED_VERSION>` 으로 둬요. 이 값이 `plugin.latest` 보다 높으면 `<PLUGIN_UPDATED_VERSION>` 을 최종 카드에 써요. 확인이 안 되면 `plugin.latest` 를 써요.
  5. **재시작 안내(필수 — 플러그인 업데이트는 재시작해야 적용돼요):** `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.`
  6. 실패하면 raw 에러는 숨기고 한 줄: `플러그인 자동 업데이트가 안 됐어요. claude plugin update axhub@axhub --scope <SCOPE> 를 직접 실행해 주세요.`

---

## 4. 결과 카드

끝나면 두 줄로 요약해요 (한 항목씩):

```text
업데이트 결과
  • CLI: <이미 최신 v X | v X → v Y 업데이트됨 | 패키지 매니저 관리 — 수동 | 업데이트 보류(AXHUB_NO_AUTO_UPDATE) — 수동 | 실패 — 수동 안내>
  • 플러그인: <이미 최신 v X | v X → v Y 받음 (재시작 필요) | 업데이트 보류(AXHUB_NO_AUTO_UPDATE) — 수동 | Claude Code 에서 수동>
```

플러그인을 새로 받았으면 마지막에 **재시작 안내**를 한 번 더 또렷이 남겨요.

원래 요청에 앱 상태 조회 같은 다른 axhub 작업이 함께 있었으면, 결과 카드 뒤에 한 줄만 덧붙이고 멈춰요: `업데이트 확인은 끝났어요. 앱 상태 확인은 별도 상태 확인 흐름에서 처리해요.` 이 문장을 쓴 즉시 현재 assistant turn 을 끝내요. update 실행 뒤 같은 assistant 응답 안에서 앱 목록·배포 상태 도구, 추가 Bash/MCP/App 도구, Task/Subagent/Agent/백그라운드 작업을 절대 만들지 않아요.

---

## 가시성·안전 규칙

- raw JSON·명령 출력·내부 값은 chat 에 echo 하지 않고, 위의 한국어 한 줄들만 보여줘요.
- 사용자에게 보이는 Bash/tool call 제목은 한국어 명사구로만 써요. `axhubing`, `axhubed`, `updating` 처럼 제품명을 영어 동사처럼 보이게 만드는 제목을 쓰지 않아요. 예: `버전 확인`, `CLI 업데이트 적용`, `업데이트 후 버전 확인`.
- 진행 문구도 한국어 사용자 문장만 써요. 영어 라벨, 내부 필드명, 설치 위치 원문, raw 상태값, 반말형 짧은 메모가 섞인 문장은 쓰지 않아요.
- 플러그인 확인 직후에는 `현재 플러그인 버전을 확인했어요.` 만 보여줘요. 현재 버전 숫자와 설치 위치값을 묶어 설명하는 문장을 만들지 않아요.
- 대신 `현재 플러그인 버전을 확인했어요.`, `CLI는 이미 최신이에요. 플러그인 새 버전을 받을게요.`, `플러그인 설치 위치를 확인했어요.`, `플러그인 새 버전을 받았어요.` 라고 말해요.
- 최종 카드 밖에서 내부 필드명이나 영어 라벨을 보여주지 않아요. 버전 숫자는 결과 카드나 업데이트 안내처럼 사용자에게 필요한 문장에서만 보여줘요.
- `claude plugin list` 에 같은 플러그인의 낡은 항목과 새 항목이 같이 남아 있어도, chat 에는 낡은 중복 항목을 나열하지 않아요. 사용자가 알아야 할 건 "받은 최신 버전" 과 "재시작 필요" 예요.
- 최고 enabled 버전이 이미 최신이면, 낮은 중복 항목을 최신화하기 위한 `claude plugin update` 를 실행하지 않아요. 이 경우 사용자가 알아야 할 건 "이미 최신" 이에요.
- mixed request 의 남은 작업을 말할 때는 `업데이트 확인은 끝났어요. 앱 상태 확인은 별도 상태 확인 흐름에서 처리해요.` 만 써요. `앱 상태는 이어서 확인할게요`, `이제 이어서 앱 상태를 확인할게요`, `현재 앱 상태를 정리해드릴게요`, `백그라운드에서 조회하고 있어요`, `결과 나오는 대로 알려드릴게요` 같은 후속 작업 문장이나 상태 요약 문장은 쓰지 않아요.
- 사용자가 직접 부른 거라 적용 전 "적용할까요?" 를 다시 묻지 않아요 (간단한 1-shot 업데이트). 단 exit 14/66 보안 실패는 무조건 하드 스톱이에요.
- 전 과정 비차단 — 한 단계가 막혀도 raw 에러를 숨기고 다음으로 넘어가거나 한 줄 안내 후 멈춰요.

## NEVER

- NEVER `command -v axhub` 실패 상태에서 재설치를 시도하지 말아요 — 설치는 onboarding 소관이라 안내만 하고 멈춰요.
- NEVER `disabled == true` 인데 `axhub update apply` 를 실행하지 말아요 — 패키지 매니저 관리 설치는 자기 교체가 안 돼요.
- NEVER `AXHUB_NO_AUTO_UPDATE` 가 설정됐는데 자동 적용하지 말아요 — 문서화된 update kill switch 라, 사용자가 직접 불러도 안내만 해요.
- NEVER exit 14/66 (보안 검증 실패) 을 무시하고 강제 진행하지 말아요. 하드 스톱이에요.
- NEVER raw JSON·stderr·내부 device/installation id 를 chat 에 출력하지 말아요.
- NEVER 플러그인 업데이트를 받고도 재시작 안내를 빼먹지 말아요 — 재시작 전엔 새 버전이 안 떠요.
- NEVER 확인하지 않은 버전을 "업데이트됨" 으로 보고하지 말아요 — `axhub --version` 재확인 뒤에만 새 버전을 말해요.
- NEVER `claude plugin list` 에서 처음 발견한 낡은 `axhub@axhub` 항목만 보고 업데이트 여부를 판단하지 말아요 — enabled 항목 전체를 읽고 최고 semver 로 판단해요.
- NEVER 최고 enabled semver 가 이미 최신인데도 낮은 중복 scope 를 기준으로 `v낮은버전 → v최신버전 받음` 결과를 만들지 말아요.
- NEVER 이 스킬에서 앱 목록·앱 상태·최근 배포 상태를 직접 조회하지 말아요. `App list (axhub)`, `Tenant recent deployments (axhub)`, `App get (axhub)` 같은 Claude Desktop axhub App/MCP 도구도 호출하지 말아요 — read 작업이어도 이 스킬에서는 금지예요.
- NEVER Task/Subagent/Agent/백그라운드 작업으로 mixed request 의 남은 앱 상태 확인을 우회하지 말아요.
