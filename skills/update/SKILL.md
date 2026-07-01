---
name: update
description: 'update: 사용자가 지금 axhub CLI 와 Claude Code 플러그인을 최신으로 올리려는 수동 on-demand 업데이트 요청에 사용해요. "업데이트해줘", "axhub 최신 버전으로", "플러그인 업데이트", "update axhub"처럼 버전 확인/적용 의도가 분명한 경우예요. 첫 셋업·설치=onboarding, 새 앱=init, 배포=deploy, 그 외 axhub 운영 명령=clarity 로 양보해요.'
examples:
  - utterance: "업데이트해줘"
    intent: "update axhub cli and plugin to latest"
  - utterance: "axhub 최신 버전으로 맞춰줘"
    intent: "update axhub cli and plugin to latest"
  - utterance: "update axhub"
    intent: "update axhub cli and plugin to latest"
allows-dependency-execution: false
model: sonnet
---

# axhub update (수동 on-demand 버전 업데이트)

사용자가 직접 불러 **axhub CLI 와 Claude Code 플러그인을 지금 최신으로** 올리는 스킬이에요. 제거된 자동 훅에 의존하지 않고, 사용자가 명시적으로 부른 순간에만 버전 확인과 적용을 진행해요:

- **항상 즉시 확인** — 사용자가 부른 수동 실행이라 바로 버전을 확인해요.
- **최신이어도 결과 보고** — "이미 최신이에요 (CLI vX, plugin vY)" 처럼 결과를 한 줄로 알려요. 사용자가 물었으니 답을 줘요.

전 과정 best-effort·비차단이에요. 실패·구 CLI·네트워크 오류면 raw 에러를 숨기고 한 줄만 안내한 뒤 멈춰요.

**책임 경계.** 이 스킬은 버전 업데이트만 해요. 첫 셋업·CLI 설치는 `onboarding`, 그 외 axhub 운영 명령은 `clarity` 가 맡아요.

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
2. `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 의 `version` 을 읽어 `<PLUGIN_VERSION>` 으로 둬요 (못 읽으면 플러그인 확인은 생략하고 CLI 만 봐요).

**`disabled` 와 `AXHUB_NO_AUTO_UPDATE` — 둘 다 존중해요 (자동 적용 안 함, 안내만).**
- `disabled`(패키지 매니저가 관리하는 설치) → CLI 가 자기를 교체할 수 없어요. 패키지 매니저 업그레이드를 **안내만** 해요.
- `AXHUB_NO_AUTO_UPDATE` → CLAUDE.md 가 문서화한 update kill switch 예요. 새 버전이 있어도 적용하지 않고 **안내만** 해요 (사용자가 직접 불러도요 — 잠긴·CI 환경에서 의도치 않은 binary swap 을 막아요). 받으려면 플래그를 끄거나 안내된 명령을 직접 실행하면 돼요.

---

## 1. 버전 확인 (네트워크 1회)

```bash
axhub update check --plugin-version <PLUGIN_VERSION> --json
```

- `--plugin-version` 은 CLI v0.21.0+ 에서 플러그인 최신 여부도 함께 판정해요. 구 CLI 가 이 플래그를 거부하면 (exit 64) `axhub update check --json` 으로 한 번 더 호출해 CLI-only 로 떨어져요.
- 결과와 무관하게 수동 확인 시각 캐시를 갱신해요:

  ```bash
  mkdir -p "$HOME/.axhub/cache" && : > "$HOME/.axhub/cache/.plugin-update-check"
  ```

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

- `plugin` 블록이 없거나 **`plugin.has_update == false`** → `axhub 플러그인은 이미 최신이에요 (v<plugin.current>).` 한 줄 (plugin 블록이 없으면 = 구 CLI 라 이 줄을 생략해요).
- **`command -v claude` 실패** (Claude Code CLI 없음) → 한 줄 안내만: `axhub 플러그인 새 버전(v<plugin.latest>)이 있어요. Claude Code 에서 /plugin update 로 받아 주세요.`
- **`AXHUB_NO_AUTO_UPDATE` 설정** → 적용하지 않고 한 줄 안내만: `axhub 플러그인 새 버전(v<plugin.latest>)이 있어요. AXHUB_NO_AUTO_UPDATE 설정이라 자동 적용은 안 해요 — claude plugin update axhub@axhub 로 직접 받거나 플래그를 끄면 돼요.`
- **`plugin.has_update == true` 이고 적용 가능** → 적용해요:
  1. 설치 scope 를 먼저 확인해요 — `claude plugin list` 출력에서 `axhub@axhub` 항목의 `Scope:` 값(user/project/local/managed)을 읽어 `<SCOPE>` 로 둬요. 못 찾으면 `user` 로 둬요.
  2. 한 줄: `axhub 플러그인 새 버전(v<plugin.current> → v<plugin.latest>)이 나왔어요. 지금 받을게요…`
  3. 실행: `claude plugin update axhub@axhub --scope <SCOPE>`
  4. **재시작 안내(필수 — 플러그인 업데이트는 재시작해야 적용돼요):** `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.`
  5. 실패하면 raw 에러는 숨기고 한 줄: `플러그인 자동 업데이트가 안 됐어요. claude plugin update axhub@axhub --scope <SCOPE> 를 직접 실행해 주세요.`

---

## 4. 결과 카드

끝나면 두 줄로 요약해요 (한 항목씩):

```text
업데이트 결과
  • CLI: <이미 최신 v X | v X → v Y 업데이트됨 | 패키지 매니저 관리 — 수동 | 업데이트 보류(AXHUB_NO_AUTO_UPDATE) — 수동 | 실패 — 수동 안내>
  • 플러그인: <이미 최신 v X | v X → v Y 받음 (재시작 필요) | 업데이트 보류(AXHUB_NO_AUTO_UPDATE) — 수동 | Claude Code 에서 수동>
```

플러그인을 새로 받았으면 마지막에 **재시작 안내**를 한 번 더 또렷이 남겨요.

---

## 가시성·안전 규칙

- raw JSON·명령 출력·내부 값은 chat 에 echo 하지 않고, 위의 한국어 한 줄들만 보여줘요.
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
