---
title: Silent Background Auto-Update - Plan
type: feat
date: 2026-09-02
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# Silent Background Auto-Update (조용한 백그라운드 자동 업데이트) - Plan

**Target repo:** 이 저장소(axhub plugin). `ax-hub-cli:` 접두사가 붙은 표면은 읽기 전용 참조이고 이 플랜은 그 저장소를 고치지 않아요. 필요한 CLI 표면(`update check --field-expr`, `update apply --execute --yes --json`)은 0.38.0 에 이미 있어요 (실측).

## Goal Capsule

- **Objective:** 사용자에게 묻지 않고, 세션 시작 훅이 백그라운드에서 axhub CLI 와 플러그인을 최신으로 맞춰요. 에이전트 토큰 0, 권한 팝업 0, 세션 시작 지연 0 이에요.
- **Means:** worker 훅을 Claude Code 의 `"async": true` 로 등록해 harness 가 백그라운드 실행을 맡게 해요 (hooks.md "Run hooks in the background"). 에이전트가 읽던 프롬프트 2개(`auto-update-prompt.md`, `plugin-restart-confirm-prompt.md`)를 삭제하고 훅 스크립트가 판정·적용·사후 알림을 모두 소유해요 (KD2, KD3).
- **Authority hierarchy:** `docs/policy/agent-policy.md` · `docs/policy/dev-policy.md` → 이 플랜의 KTD → `CLAUDE.md`. 정책과 충돌하면 정책이 이겨요. 이 플랜은 CLAUDE.md 의 "네트워크 호출은 hook 이 아니라 prompt(에이전트)가 해요" 원칙을 **이 훅에 한해** 뒤집으므로 새 AP 항목으로 명시해요 (R14).
- **Stop conditions:** U0 spike(훅 종료 뒤 백그라운드 자식 생존)가 macOS 에서 실패하면 설계 전제가 무너지므로 구현을 멈추고 보고해요. Windows 에서만 실패하면 Windows 는 KTD7 fallback 으로 분기하고 나머지는 진행해요. codex drift 게이트(`tests/codex-bundle.test.ts`)가 실패하면 파생 번들을 손으로 고치지 않고 소스·override·치환 테이블을 고쳐 재생성해요 (AP-20). `ax-hub-cli` 를 고쳐야 풀리는 항목은 멈추고 Deferred 로 보고해요.
- **Execution profile:** 훅 bash 3개 + 테스트 + 정책·문서 + codex 재생성이에요. 자동 테스트는 `axhub`·`claude` stub 으로 검증하고, 실제 네트워크 apply 는 U6 수동 검증 1회로 확인해요.
- **Tail ownership:** Scope Boundaries 의 Deferred 항목(`update` 스킬의 로그 기반 실패 보고, What's New 요약, CLI 전용 plan 표면)은 이 플랜의 DoD 밖이에요.

---

## Product Contract

### Summary

현재 auto-update 훅은 24h 마다 에이전트에게 "프롬프트를 읽고 버전을 확인·적용하라"는 컨텍스트를 주고, 에이전트가 `axhub update check` → `axhub update apply` → `claude plugin list` → `claude plugin update` 를 사용자 눈앞에서 실행해요. 이 플랜은 그 실행을 훅이 fork 한 백그라운드 worker 로 옮겨요. 사용자는 묻는 카드도 진행 서사도 보지 않고, 다음 세션 첫 응답 앞에 최대 한 줄("axhub CLI 가 v0.38.0 → v0.39.0 으로 업데이트됐어요" / "플러그인 새 버전을 받았어요, 재시작하면 적용돼요")만 봐요. gstack 의 team-mode auto-upgrade(`bin/gstack-session-update`)와 같은 구조예요.

### Problem Frame

지금 구조의 비용은 네 가지예요.

1. **에이전트 lane 비용.** 프롬프트 읽기 + 명령 3~5회가 매 24h 첫 세션의 토큰을 먹고, Claude Desktop 에서는 `update apply`·`claude plugin list`·`claude plugin update` 마다 권한 카드가 떠요.
2. **유실.** 훅이 캐시를 먼저 touch 하므로 에이전트가 지침을 못 따르면(다른 스킬로 바로 빠짐, Desktop 권한 거절) 그 회차는 조용히 사라지고 24h 뒤에나 재시도해요.
3. **끼어듦.** 사용자의 첫 요청 앞에 "새 버전이 나왔어요. 지금 업데이트할게요…" 서사가 끼어들고, 실패하면 "직접 실행해 주세요" 안내까지 붙어요.
4. **restart-confirm 도 에이전트 몫.** 플러그인 적용 확인을 위해 다음 세션에서 에이전트가 `claude plugin list` 를 또 실행해요. 훅이 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 을 읽으면 명령 없이 더 정확히(이 세션이 실제 로드한 버전) 판정할 수 있어요.

gstack 은 같은 문제를 훅 안 background fork(`git pull --ff-only` + `./setup`)로 풀고, 다음 세션에 `JUST_UPGRADED` marker 로 한 줄만 남겨요. 우리는 pull 대신 CLI 의 `update apply` 와 host 의 `plugin update` 를 같은 자리에서 돌리면 돼요.

### Key Decisions

- KD1. **묻지 않아요.** 선택 카드·snooze·"다시 묻지 않기" 를 두지 않고 새 버전이 있으면 바로 적용해요 (session-settled: user-decided — chosen over gstack 식 4-옵션 카드: 사용자가 은밀한 적용을 명시했어요). Governs R8, R9.
- KD2. **훅이 백그라운드 worker 로 직접 적용해요.** CLI 와 플러그인 둘 다요. 백그라운드는 스크립트가 직접 fork 하지 않고 Claude Code 의 `"async": true` 훅으로 harness 에 맡겨요 (session-settled: user-decided — chosen over 에이전트 lane 유지: 토큰·권한 팝업·유실을 없애는 것이 목적이에요. async 훅은 verified — hooks.md 가 비차단 실행·timeout 미적용·완료 시 `additionalContext` 를 다음 턴에 전달한다고 명시해요). Governs R6, R7, R8, R9, R11, R14.
- KD3. **에이전트 프롬프트 2개를 삭제하고 훅이 전부 소유해요.** restart-confirm 은 훅이 `CLAUDE_PLUGIN_ROOT` 의 plugin.json 버전을 직접 비교해 판정해요 (session-settled: derived from KD2 — chosen over 프롬프트를 남겨 두는 안: 남겨 두면 두 lane 이 같은 marker 를 두고 경합해요). Governs R11, R13.
- KD4. **사후 알림은 세션당 최대 한 줄이고 질문이 없어요. 보안 검증 실패는 반드시 알려요** (session-settled: assumption — "은밀하게" 를 "묻지 않고 서사 없이" 로 읽었어요. 완전 무음을 원하면 R11·R13 의 알림 줄만 빼면 되고 나머지 설계는 그대로예요. R12 의 보안 안내는 빼지 않아요). Governs R11, R12, R13.
- KD5. **판정 입력은 `update check --field-expr` 한 줄이에요.** jq·sed 파싱 없이 공백 구분 필드를 `read` 해요 (session-settled: verified — 실측 출력 `false v0.38.0 false false false 1.27.1`). Governs R7.
- KD6. **codex 판은 같은 async worker 를 치환 테이블로 갈라요.** codex 도 `async: true` command 훅을 공식 지원하고 출력 전달 규칙이 같아요 (verified — developers.openai.com/codex/hooks "Run hooks in the background": 비차단, `additionalContext` 는 다음 안전 지점에 모델 컨텍스트로 전달, 세션 종료 시 미전달 출력 폐기). 다른 점은 셋뿐이에요 — (1) codex 는 async 에도 `timeout`(기본 600s)을 적용하므로 Claude 판의 `timeout: 5` 를 codex 번들에선 제거해요, (2) codex 는 사용자가 훅을 신뢰한 뒤에만 실행해요(기존과 동일), (3) 플러그인 갱신 명령(`codex plugin marketplace upgrade axhub` / `codex plugin add axhub-codex@axhub`)과 marker 접미사(`-codex`)가 달라요. gate·lock·log·알림 채널은 같아요 (session-settled: derived from AP-20). Governs R6, R9, AE6.
- KD7. **throttle 은 24h 를 유지해요** (session-settled: default — POLICY.md 의 "24시간에 1회" 문구와 기존 캐시 파일을 그대로 써요. 단축은 OQ2).

### Requirements

**Gate (훅 전경, 네트워크 0, 수 ms)**

- R1. kill switch(`AXHUB_NO_AUTO_UPDATE` 또는 `~/.axhub/config/no-auto-update`)가 있으면 아무것도 하지 않아요 — 캐시·lock·worker·알림 전부 없음.
- R2. dev 가드(`${CLAUDE_PLUGIN_ROOT}/../../.git` 또는 `${CLAUDE_PLUGIN_ROOT}/.git`)는 그대로예요.
- R3. CLI 는 AP-17 3-경로(`command -v axhub` → `~/.axhub/bin-path` → `~/.axhub/bin/axhub`(.exe))로 찾고, 찾은 절대경로를 worker 인자로 넘겨요. 셋 다 없으면 침묵해요.
- R4. 24h throttle 은 기존 캐시 파일(`~/.axhub/cache/.plugin-update-check`, codex 는 `-codex`)의 mtime 으로 판정하고 훅이 fork 직전에 touch 해요.
- R5. 동시 세션 방어: `~/.axhub/cache/.auto-update.lock/` 을 `mkdir` 원자 lock 으로 잡고 pid 파일을 써요. 30분 TTL 이 지난 lock 은 한 번 회수해요. 못 잡으면 조용히 skip 해요 (캐시는 touch 하지 않아 다음 세션이 재시도해요).
- R6. worker 는 hooks.json 의 별도 SessionStart entry 로 `"async": true` + `"timeout": 5` 를 달아 등록해요. async 를 아는 호스트는 비차단으로 돌리고 timeout 을 적용하지 않아요. async 를 모르는 구 호스트는 동기 훅으로 취급해 5초에 취소하므로 세션 시작 지연이 5초를 넘지 않아요 (apply 는 atomic swap 이라 중간 취소가 안전해요). codex 판은 async 에도 timeout 이 적용되므로 `timeout` 키를 빼서 기본 600s 를 써요. R1~R5 gate 는 worker 자신이 첫 수 ms 에 수행하고 해당 없으면 즉시 exit 0 해요. 적용 진행 중에는 아무 출력도 내지 않아요.

**Worker (백그라운드)**

- R7. `<BIN> update check --plugin-version <PV> --field-expr '[.current,.has_update,.latest,.disabled,.is_downgrade,.plugin.has_update,.plugin.latest]|map(tostring)|join(" ")'` 를 1회 실행해 7필드를 읽어요. 실패·필드 수 불일치는 `CHECK_FAILED` 로 log 하고 종료해요 — 침묵을 "최신" 으로 읽지 않아요. `--field-expr` 를 모르는 구 CLI(exit 64)는 `update check --json` 원문에서 top-level `current`·`has_update`·`latest` 만 추출해 CLI apply 만 진행하고 플러그인은 다음 주기로 미뤄요.
- R8. CLI: `has_update=true` 이고 `disabled=false` 이고 `is_downgrade=false` 이고 halt marker 의 버전이 `latest` 와 다르면 `<BIN> update apply --execute --yes --json` 을 실행해요. exit 0 → log `UPDATED` 와 R11 알림 (`<current>` → `<latest>`). exit 14(digest mismatch)·66(cosign) → `~/.axhub/cache/.auto-update-halt` 에 `<latest>|<exit>` 를 써요. 그 외 실패는 log 만 남기고 다음 주기에 재시도해요.
- R9. 플러그인: `plugin.has_update=true` 면 host CLI 존재를 확인하고(없으면 skip + log) scope 를 읽어(`claude plugin list` 출력의 `axhub@axhub` 블록 `Scope:`, 못 읽으면 `user`) `claude plugin marketplace update axhub`(실패 무시) → `claude plugin update axhub@axhub --scope <scope>` 를 실행해요. 성공 시 기존 marker `~/.axhub/cache/.plugin-update-restart` 에 `<latest>|<scope>` 를 써요. codex 판은 `codex plugin list --json` 의 `axhub-codex@axhub` 항목 `marketplaceSource.sourceType` 으로 갈라 `git` → `codex plugin marketplace upgrade axhub`, `local` → `codex plugin add axhub-codex@axhub` 을 실행하고 marker 는 `.plugin-update-restart-codex` 에 `<latest>` 만 써요.
- R10. `~/.axhub/cache/auto-update.log` 에 run 당 1줄(`<UTC ts> <RESULT> k=v…`)을 append 하고 200줄을 넘으면 앞을 잘라요. 결과 어휘: `UP_TO_DATE`, `UPDATED`, `CHECK_FAILED`, `APPLY_FAILED`, `SECURITY_HALT`, `SKIP_HALTED`, `PLUGIN_UPDATED`, `PLUGIN_FAILED`, `SKIP_DISABLED`, `SKIP_DOWNGRADE`.

**사후 알림 (훅 전경, 네트워크 0, throttle 과 무관하게 매 세션 검사)**

- R11. CLI 적용이 끝나면 worker 자신이 종료 직전에 suppressed JSON 으로 "[axhub] CLI 가 v<old> → v<new> 로 자동 업데이트됐어요. 다음 응답 앞머리에 그 사실만 한 줄로 알리고 다른 명령은 실행하지 마세요" 를 emit 해요 — async 훅 출력은 같은 세션의 다음 턴에 에이전트에게만 전달돼요 (에이전트 명령 0, 사용자에게 raw 노출 없음). 아무 일도 없었으면 출력 없이 exit 0 해요. codex 판도 같은 채널이에요 — codex 는 async 훅 출력을 다음 안전 지점에 모델 컨텍스트로 넣어요.
- R12. 보안 검증 실패(exit 14/66)는 worker 가 종료 직전에 보안 한 줄("보안 검증에 실패했어요. 강제로 진행하지 말고 회사 IT·보안팀에 알려주세요. 지금 버전은 그대로 써도 돼요")을 R11 과 같은 채널로 emit 하고 `.auto-update-halt` 에 `<latest>|<exit>` 를 써요. 같은 버전은 재시도·재통지하지 않고, worker 가 다른 `latest` 를 보면 halt 파일을 지우고 다시 시도해요. codex 판도 같아요.
- R13. restart-confirm 훅은 marker(7일 TTL)가 있으면 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 의 `version` 을 읽어 marker 버전 이상이면 "플러그인 v<x> 가 적용됐어요" 한 줄을 emit 하고 marker 를 삭제해요. 낮으면 "플러그인 새 버전을 받았어요. Claude Code 를 재시작하면 적용돼요" 한 줄을 emit 하고 marker 를 유지해요. `claude plugin list` 는 실행하지 않아요.

**정책·문서**

- R14. `docs/policy/agent-policy.md` 에 AP-26(조용한 백그라운드 자동 업데이트)을 신설해요 — 훅 네트워크 예외, 묻지 않음, 사후 1줄, 보안 필수 안내, kill switch 2계층, log 위치. `CLAUDE.md` "자동 업데이트 hook" 절, `POLICY.md`(네트워크 접근 bullet·로컬 파일 목록·"자동 업데이트와 끄는 법"), `codex-overrides/POLICY.md`·`README.md` 를 같은 방향으로 고쳐요. CHANGELOG narrative 에 훅 wrapper 동작 변경을 명시해요 — codex 는 wrapper 내용을 재신뢰 없이 갱신하므로 POLICY 가 요구하는 계약이에요.
- R15. `skills/update/SKILL.md` 와 references 는 건드리지 않아요 — 수동 on-demand 경로와 byte 예산은 그대로예요.

### Acceptance Examples

- AE1. **CLI 새 버전.** stale 캐시 상태로 세션을 열면 시작 지연이 없고, 30초 안에 log 에 `UPDATED cli=v0.38.0->v0.39.0` 이 남아요. 같은 세션의 다음 사용자 입력에 대한 응답 앞에 "axhub CLI 가 v0.38.0 → v0.39.0 으로 업데이트됐어요" 한 줄이 보이고 그 밖의 명령·서사는 없어요. codex 판도 같은 시점에 같은 줄이에요.
- AE2. **플러그인 새 버전.** worker 가 `claude plugin update axhub@axhub --scope user` 를 실행하고 marker 를 `1.28.0|user` 로 써요. 재시작 전 세션은 "재시작하면 적용돼요" 한 줄, 재시작 뒤 세션은 "플러그인 v1.28.0 이 적용됐어요" 한 줄과 marker 삭제예요. 어느 세션도 `claude plugin list` 를 실행하지 않아요.
- AE3. **cosign 실패.** `update apply` 가 exit 66 이면 halt marker 가 생기고 다음 세션에 보안 한 줄이 1회 나와요. 그 뒤 세션들은 침묵하고 같은 `latest` 에는 apply 를 다시 시도하지 않아요.
- AE4. **kill switch.** `AXHUB_NO_AUTO_UPDATE=1` 이면 캐시·lock·log·marker 어느 것도 생기지 않고 알림도 없어요.
- AE5. **동시 세션 2개.** 같은 초에 세션 둘이 열려도 worker 는 하나만 돌고 log 에 `UPDATED` 는 한 줄이에요.
- AE6. **codex 번들.** 파생 worker 에는 `codex plugin marketplace upgrade axhub` 와 `.plugin-update-restart-codex` 가 있고 `claude plugin` 문자열은 0건이에요.

### Scope Boundaries

**In scope**

- `hooks/session-auto-update.sh` 를 async worker 로 재작성, `hooks/hooks.json` entry 1 에 `async`·`timeout` 추가, `hooks/session-restart-confirm.sh` 재작성.
- `hooks/auto-update-prompt.md`, `hooks/plugin-restart-confirm-prompt.md`, `codex-overrides/hooks/` 동명 2개 삭제와 참조처 정리.
- 테스트·정책·문서·CHANGELOG·codex 재생성.

**Deferred**

- `update` 스킬이 `auto-update.log` 마지막 줄을 읽어 "자동 업데이트가 N일째 실패 중이에요" 를 보고하는 기능.
- What's New 요약(CHANGELOG 구간 bullet) — 별도 플랜.
- CLI 전용 `plugin-support auto-update-plan` 표면 — `--field-expr` 로 충분해 필요 없어요.
- throttle 단축(OQ2).

---

## Planning Contract

### Key Technical Decisions

- KTD1. **파일 2개 구조.** `session-auto-update.sh` 가 async worker 그 자체예요 — R1~R12 (gate·check·apply·알림). `session-restart-confirm.sh` = R13. 파일명·hooks.json command 문자열을 기존과 같게 두는 이유는 codex 훅 trust 가 command 문자열 hash 라서 wrapper 본문 교체는 재신뢰 없이 반영되기 때문이에요 (2026-08-19 실측). 프롬프트 md 2개와 codex override 2개는 삭제해요.
- KTD2. **백그라운드 실행 = harness `async: true`.** hooks.json SessionStart entry 1 을 `{"type":"command","shell":"bash","command":"bash \"${CLAUDE_PLUGIN_ROOT}/hooks/session-auto-update.sh\"","async":true,"timeout":5}` 로 바꿔요 (command 문자열 불변). 스크립트는 fork 하지 않고 평범하게 끝까지 돌며, 마지막에 suppressed JSON 을 stdout 에 내면 harness 가 다음 턴에 `additionalContext` 만 에이전트에게 전달해요 (사용자 화면 노출 없음, 완료 알림도 기본 억제). JSON 이 깨지면 v2.1.202 이전 호스트는 세션이 죽을 수 있으므로 printf 템플릿 하나로만 출력해요. codex 판도 같은 async entry 를 쓰고(`timeout` 만 제거), 스크립트는 어느 호스트에서도 fork 하지 않아요. 관측은 전부 log 파일로 해요.
- KTD3. **lock.** `mkdir "$LOCK" 2>/dev/null` 성공 시 `$$` 를 `$LOCK/pid` 에 써요. 실패 시 `find "$LOCK" -maxdepth 0 -mmin +30` 이 잡히면 `rm -rf` 뒤 `mkdir` 1회 재시도, 그래도 실패면 skip. worker 는 `trap 'rm -rf "$LOCK"' EXIT` 로 정리해요. gstack 의 mv-aside·heartbeat 는 apply 가 수십 초라 과해요.
- KTD4. **파싱.** field-expr 출력 한 줄을 `read CUR HAS LATEST DIS DOWN PHAS PLAT` 로 받고 `[ -n "$PLAT" ]` 로 7필드를 검증해요.
- KTD5. **semver 비교.** macOS `sort` 에 `-V` 가 없으므로 `major.minor.patch` 숫자 3자리 비교 함수를 bash 로 둬요. 앞의 `v` 는 떼요.
- KTD6. **codex 파생.** worker 에 `HOST=claude` 한 줄을 두고 `CODEX_SUBSTITUTIONS` 에 `["HOST=claude", "HOST=codex"]` 를 더해요. marker·캐시 이름은 기존 `.plugin-update-check` → `-codex` 계열 치환이 그대로 먹게 파일명을 그 접두사로 맞춰요. `SOURCE_HASHES.json` 에서 삭제된 prompt 2개를 빼고 `POLICY.md`·`README.md` 해시를 재핀해요.
- KTD7. **Windows.** 두 호스트 모두 harness 가 async 프로세스를 관리하므로 Job Object·자식 생존 문제가 없어요. Claude 는 `"shell": "bash"` 로 Git Bash, codex 는 `commandWindows` 의 `where bash … && bash …` 가드로 Git Bash 를 타요 (bash 부재 시 조용히 skip). 스크립트 안에서 fork 하는 곳이 없으니 Windows 전용 분기도 없어요.

### High-Level Technical Design

```
SessionStart
  ├ session-auto-update.sh  ("async": true; Claude 는 "timeout": 5, codex 는 timeout 생략)
  │   kill switch? ─yes→ exit 0 (아무 흔적 없음)
  │   dev guard?   ─yes→ exit 0
  │   CLI 3-경로 탐색 → 없으면 exit 0
  │   cache mtime < 24h? ─yes→ exit 0
  │   mkdir lock 실패(& TTL 미만)? ─yes→ exit 0 (cache 미touch)
  │   touch cache, trap rm lock
  │   check 1회 (field-expr 7필드)  ─fail→ log CHECK_FAILED, exit 0
  │   CLI: 조건 충족 → apply --execute --yes --json
  │        0 → log UPDATED, NOTICE="CLI v<cur> → v<latest> 업데이트됨"
  │        14/66 → .auto-update-halt=latest|exit, log SECURITY_HALT, NOTICE=보안 1줄
  │        else → log APPLY_FAILED
  │   plugin: plugin.has_update → host cli → scope → marketplace update → plugin update
  │        0 → .plugin-update-restart=latest|scope, log PLUGIN_UPDATED
  │        else → log PLUGIN_FAILED
  │   log 200줄 trim
  │   NOTICE 있으면 suppressed JSON 1회 출력 (두 호스트 모두 다음 안전 지점에 에이전트에게 전달)
  └ session-restart-confirm.sh (동기, ms)
      marker(7d) 없음 → exit 0
      plugin.json version ≥ marker → emit "적용됐어요", rm marker
      else → emit "재시작하면 적용돼요", keep
```

파일 상태(모두 `~/.axhub/cache/`):

| 파일 | 쓰는 쪽 | 읽는 쪽 | 삭제 |
| --- | --- | --- | --- |
| `.plugin-update-check` (기존) | gate touch | gate mtime | 없음 |
| `.auto-update.lock/pid` | worker(gate 단계) | worker(TTL) | worker EXIT |
| `.auto-update-halt` | worker | worker | worker(새 latest) |
| `.plugin-update-restart` (기존) | worker | restart-confirm | restart-confirm(적용 확인) |
| `auto-update.log` | worker | 사람·(Deferred) update 스킬 | 없음(200줄 trim) |

### Assumptions

- A1. (verified, hooks.md "Run hooks in the background") `"async": true` command 훅은 세션을 막지 않고, 실행 중엔 `timeout` 이 적용되지 않으며, 종료 뒤 JSON 의 `additionalContext`·`systemMessage` 가 다음 대화 턴에 에이전트에게만 전달돼요. 세션이 idle 이면 다음 사용자 입력 때 전달돼요. `-p` headless 세션은 teardown 때 async 훅을 kill 해요 — 이 경우 auto-update 는 그 회차를 잃고 lock TTL 뒤 재시도해요 (수용). 문서가 plugin hooks.json 을 따로 구분하지 않으므로 U0 가 plugin 훅에서도 async 가 먹는지 실측해요. codex 도 같은 계약이에요 (verified, developers.openai.com/codex/hooks) — 단 timeout 은 async 에도 적용(기본 600s), 동시 8개 상한, 훅 신뢰 후에만 실행, 세션 종료 시 미전달 출력 폐기.
- A2. `axhub update apply --execute --yes --json` 은 non-TTY 에서 확인 없이 진행해요 (help 실측: `--json`·non-TTY 는 확인을 자동 bypass).
- A3. `--field-expr` 는 CLI 0.38.0 에 있어요. 도입 버전을 모르므로 R7 의 exit 64 fallback 을 둬요.
- A4. 훅 env 의 PATH 에 `claude` 가 있어요. 없으면 플러그인 갱신만 skip 하고 log 에 남겨요 (CLI 는 3-경로로 찾으니 무관).
- A5. 세션 중 `claude plugin update` 는 캐시 디렉터리만 갱신하고 실행 중 세션은 영향받지 않아요 — 현재 에이전트 lane 이 이미 세션 중에 실행하고 있어 새 위험이 아니에요.
- A6. 실행 중 세션의 에이전트가 동시에 `axhub` 를 쓰는 동안 바이너리가 교체돼도 CLI 의 atomic swap 이 방어해요 (help: "atomically swap the running binary"). Windows 의 실행 중 exe 교체는 CLI 책임이에요.

### Sequencing

U0(spike) → U1(worker) → U2(gate·restart-confirm·hooks.json) → U3(테스트) → U4(정책·문서·CHANGELOG) → U5(codex 재생성) → U6(수동 실검증). U3 는 U1 과 병행 가능해요 (stub 기반).

---

## Implementation Units

### U0. Spike: 두 호스트의 plugin 훅에서 `async: true` 동작 확인

- **Goal:** A1 이 Claude Code 와 codex 의 plugin hooks.json 에서 모두 성립하는지, 그리고 codex 가 entry 에 `async` 키를 더해도 재신뢰를 요구하지 않는지 실측해 A1·KTD1·KTD2 에 기록해요.
- **Requirements:** R6, R11
- **Dependencies:** 없음
- **Files:** 임시 (scratchpad) — 레포에 남기지 않아요.
- **Approach:**
  1. `dist/axhub-plugin` 을 `bun run plugin:bundle` 로 만들고 hooks.json SessionStart 에 `{"type":"command","shell":"bash","command":"bash -c 'sleep 20; date > \"$HOME/.axhub/cache/.spike\"; printf %s \"{\\\"hookSpecificOutput\\\":{\\\"hookEventName\\\":\\\"SessionStart\\\",\\\"additionalContext\\\":\\\"[spike] 20초 뒤 도착\\\"}}\"'","async":true,"timeout":5}` entry 를 더해요.
  2. `claude --plugin-dir dist/axhub-plugin` 로 세션을 열어 (a) 시작이 1초 안에 끝나는지, (b) 20초 뒤 `.spike` 가 생기는지, (c) 그 뒤 첫 프롬프트에서 에이전트가 `[spike]` 문맥을 받았는지 봐요. 셋 다 되면 async 가 plugin 훅에서 먹는 거예요.
  3. codex 판: 같은 entry 에서 `timeout` 을 뺀 채 codex 세션(훅 신뢰 후)에서 (a)(b)(c) 를 반복해요. 추가로 (d) 기존 `session-auto-update.sh` entry 에 `async: true` 키만 더한 번들을 재설치했을 때 재신뢰 팝업이 뜨지 않는지 확인해요 — trust hash 가 command 문자열만 보는지의 실측이에요.
- **Test scenarios:** Claude (a)(b)(c) 통과, codex (a)(b)(c)(d) 통과.
- **Verification:** 결과를 이 플랜의 A1·KTD1·KTD2 에 기록해요. Claude (a) 가 실패하면 Stop condition 이에요. codex (d) 가 실패하면 첫 릴리즈 뒤 재신뢰 1회가 필요하다고 README codex 판에 적어요.
- **2026-09-02 실측 (Claude Code 2.1.258, nested `claude -p` + `--plugin-dir dist/axhub-plugin`):** plugin hooks.json 의 `async: true` 가 먹어요. (a) `sleep 6` + `timeout: 3` 훅에서 async 세션은 7초, 동기 세션은 11초(3초 timeout 취소 대기 포함)로 끝나 비차단이 확인됐어요. (b) `sleep 3` 훅은 async 에서 세션 종료 전에 끝나 파일을 남겼고, `sleep 6` 은 `-p` teardown 에 kill 돼 문서(headless 는 teardown 때 cancel)와 일치했어요 — 대화형 세션은 수십 초 이상 살아 있으므로 apply 완주에 문제없어요. (c) `additionalContext` 다음 턴 전달은 `--input-format stream-json` 2턴 headless 로 확인했어요 — 1턴 응답 "1", 훅 완료 뒤 2턴 응답이 훅이 지시한 단어로 시작했어요. 동기 fallback(`timeout: 5`)이 구 호스트에서 5초 안에 취소되는 것도 같은 실험이 보여줘요.
- **2026-09-02 codex 실측 (codex-cli 0.152.0, 격리 `CODEX_HOME` + 로컬 marketplace + `codex exec --dangerously-bypass-hook-trust`):** (a) `sleep 6` 훅에서 async 세션은 4초, 동기 세션은 9초로 끝나 비차단이 확인됐어요. (b) 격리 홈의 토큰 갱신이 거부돼 세션이 4초 만에 끝나면서 async 훅은 문서대로 취소됐고(파일 없음), 동기 실행은 끝까지 돌아 파일을 남겼어요 — 대화형 세션은 길게 살아 있으니 완주에 문제없어요. (d) trust hash 는 command 문자열·hook 객체 JSON 어느 sha256 과도 일치하지 않아 산출식을 확정하지 못했어요 — entry 에 `async` 를 더한 이번 릴리즈는 재신뢰 확인이 1회 뜰 수 있다고 codex README 에 적었어요. Windows 는 이 세션에서 실측할 수 없어 PR test plan 에 남겨요.

### U1. Author the background worker

- **Goal:** R1~R12 를 소유하는 async worker 로 `hooks/session-auto-update.sh` 를 다시 써요.
- **Requirements:** R1~R12
- **Dependencies:** U0
- **Files:**
  - `hooks/session-auto-update.sh` (재작성 — 파일명·hooks.json command 문자열 유지)
- **Approach:**
  1. 인자 없이 시작해 `HOST=claude` 상수 → kill switch → dev 가드 → AP-17 3-경로로 `BIN` → plugin.json 에서 `PV` → 24h throttle → `mkdir` lock 순으로 gate 를 밟고, 해당 없으면 출력 없이 exit 0 해요. 통과하면 `set +e`, `trap 'rm -rf "$LOCK"' EXIT`, `GIT_TERMINAL_PROMPT=0` 을 두고 본 작업으로 들어가요.
  2. host 별 명령 함수(`plugin_scope`, `plugin_update`)를 `HOST` 로 갈라요 — codex 는 KTD6 치환으로 상수만 바뀌어요.
  3. check 1회 → 7필드 `read`. exit 64 면 `--json` 원문에서 `"plugin":{…}` 블록을 지운 뒤 top-level `current`·`has_update`·`latest` 만 `sed` 로 뽑아요.
  4. halt marker(`.auto-update-halt`)의 버전이 `LATEST` 와 같으면 CLI apply 를 건너뛰고 `SKIP_HALTED` 를 log 해요. 다르면 halt 파일을 지워요.
  5. R8 분기대로 apply 하고 exit 코드로 marker·log 를 써요. `--json` 출력은 버려요 — 판정은 exit 코드로만 해요.
  6. R9 플러그인 분기. `claude plugin list` 파싱은 `awk '/axhub@axhub/{f=1} f&&/Scope:/{print $2; exit}'` 한 줄로 해요.
  7. log 1줄 append 후 `tail -n 200` 으로 trim 해요 (임시 파일 → `mv`).
  8. `NOTICE` 가 비어 있지 않으면 마지막에 suppressed JSON(`{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"…"}}`) 을 printf 템플릿 하나로 1회 출력해요. codex 도 같은 출력이에요.
- **Execution note:** Git for Windows 번들 도구(`bash`·`awk`·`sed`·`find`·`tail`·`mv`)만 써요. `jq`·`node` 금지예요.
- **Patterns to follow:** gstack `bin/gstack-session-update` 의 log_entry·lock 정리 순서, 우리 `hooks/session-feedback-contract.sh` 의 AP-17 3-경로 조건식.
- **Test scenarios:**
  - stub `axhub` 가 `v0.38.0 true v0.39.0 false false false 1.27.1` 을 주면 `update apply --execute --yes --json` 이 호출되고 stdout 의 JSON `additionalContext` 에 `v0.38.0 → v0.39.0` 이 있어요 (AE1). 같은 입력에 `HOST=codex` 면 같은 JSON 이 나오고 marker·캐시 이름만 `-codex` 예요.
  - 최신 상태(`has_update=false` 둘 다)면 stdout 이 비어 있고 log 만 `UP_TO_DATE` 예요.
  - gate 단계: kill switch env/marker → 캐시·lock·log 전부 없음 (AE4); fresh 캐시 → 즉시 exit 0, stub 호출 0회; dev 가드 토폴로지 A·B → 침묵; lock 이 살아 있으면(TTL 미만) 캐시 미touch + 호출 0회, TTL 초과면 회수 후 진행 (AE5).
  - `disabled=true` 또는 `is_downgrade=true` 면 apply 가 호출되지 않고 log 가 `SKIP_DISABLED`/`SKIP_DOWNGRADE` 예요.
  - apply stub 이 exit 66 이면 `.auto-update-halt` 가 `v0.39.0|66` 이고 log 가 `SECURITY_HALT` 예요 (AE3). 같은 stub 으로 다시 돌리면 apply 가 호출되지 않아요.
  - plugin `has_update=true` 이고 stub `claude plugin list` 가 `Scope: project` 를 주면 `claude plugin update axhub@axhub --scope project` 가 호출되고 marker 가 `1.28.0|project` 예요 (AE2).
  - check stub 이 exit 1 이거나 필드가 6개면 log 가 `CHECK_FAILED` 이고 apply 는 호출되지 않아요.
  - exit 64 stub 뒤 `--json` 원문 stub 이 오면 CLI apply 만 진행하고 플러그인 명령은 0회예요.
  - log 가 201줄이면 다음 run 뒤 200줄이에요.
- **Verification:** `bun test tests/hook-execution.test.ts` 의 worker describe 가 통과해요.

### U2. Rewrite the gate and restart-confirm hooks

- **Goal:** R1~R6, R11~R13 을 훅 두 개가 소유하게 해요.
- **Requirements:** R1, R2, R3, R4, R5, R6, R11, R12, R13
- **Dependencies:** U1
- **Files:**
  - `hooks/hooks.json` (SessionStart entry 1 에 `"async": true`, `"timeout": 5` 추가 — command 문자열·entry 순서는 유지)
  - `hooks/session-restart-confirm.sh`
  - `hooks/auto-update-prompt.md` (삭제)
  - `hooks/plugin-restart-confirm-prompt.md` (삭제)
- **Approach:**
  1. hooks.json entry 1 을 `{"type":"command","shell":"bash","command":"bash \"${CLAUDE_PLUGIN_ROOT}/hooks/session-auto-update.sh\"","async":true,"timeout":5}` 로 바꿔요. command 문자열은 한 글자도 바꾸지 않아요 (KTD1).
  2. gate(R1~R5)와 lock 은 worker(U1)가 자기 첫 단계로 수행해요 (KTD3). lock 실패 시 cache 를 touch 하지 않아요. AP-17 3-경로 조건식은 `session-feedback-contract.sh` 의 것을 옮기되 절대경로를 `BIN` 변수로 남겨요.
  3. 알림(R11·R12)은 worker 종료 직전 JSON 1회로 끝나요 — 동기 알림 훅은 없어요.
  4. `PV` 는 worker 가 `sed -n 's/.*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json"` 으로 읽고, 못 읽으면 `--plugin-version` 플래그를 생략해요.
  5. `session-restart-confirm.sh` 는 marker 읽기 → plugin.json 버전 읽기 → KTD5 비교 → emit·삭제로 다시 써요. kill switch 2계층은 유지해요.
  6. 프롬프트 md 2개를 삭제하고 `additionalContext` 문안에서 파일 경로 언급을 없애요.
- **Approach 주의:** emit 하는 additionalContext 는 "한 줄 알리고 다른 명령 실행 금지" 를 명시해요 — 에이전트가 `claude plugin list` 나 `axhub --version` 을 덧붙이는 회귀를 막아요.
- **Patterns to follow:** 기존 wrapper 들의 suppressed JSON printf 형식, `${CLAUDE_PLUGIN_ROOT//\\//}` 경로 정규화.
- **Test scenarios:**
  - hooks.json SessionStart entry 1 의 command 가 기존 문자열 그대로이고 `async: true`, `timeout: 5` 가 붙어 있어요. 다른 entry 에는 `async` 가 없어요.
  - `hooks/` 에 `.md` 파일이 0개예요.
  - restart-confirm: plugin.json `1.28.0` ≥ marker `1.28.0|user` → "적용됐어요" emit + marker 삭제; plugin.json `1.27.1` → "재시작" emit + marker 유지; 7일 초과 → 침묵; kill switch → 침묵 (AE2).
- **Verification:** `bun test tests/hook-execution.test.ts` 전체 통과, `bun run lint:tone --strict` 0 err.

### U3. Extend the test contracts and stubs

- **Goal:** U1·U2 의 시나리오를 stub 기반으로 잠그고 프롬프트 삭제 참조를 정리해요.
- **Requirements:** R7~R13
- **Dependencies:** U1, U2
- **Files:**
  - `tests/hook-execution.test.ts`
  - `tests/fixtures/stubs/axhub` (신규, 환경변수로 출력·exit 를 정하는 bash stub)
  - `tests/fixtures/stubs/claude` (신규)
  - `tests/cli-path-resolution.test.ts`, `tests/smooth-behavior.test.ts`, `tests/plugin-bundle.test.ts` — `auto-update-prompt.md`·`plugin-restart-confirm-prompt.md` 참조 제거
- **Approach:**
  1. stub 은 `AXHUB_STUB_CHECK_OUT`·`AXHUB_STUB_CHECK_EXIT`·`AXHUB_STUB_APPLY_EXIT`·`CLAUDE_STUB_LIST_OUT`·`CLAUDE_STUB_UPDATE_EXIT` 를 읽고 호출 인자를 `$STUB_CALLS` 파일에 append 해요.
  2. worker 는 테스트에서 직접 실행(전경)해요 — async 여부는 harness 몫이라 스크립트는 그냥 끝까지 돌고 stdout 을 반환해요. codex 분기는 `HOST=codex` 로 치환한 사본을 같은 방식으로 실행해요.
  3. 기존 auto-update·restart-confirm describe 의 `expectEmit(..., "auto-update-prompt.md")` 류를 새 계약으로 바꿔요.
  4. 프롬프트 파일명을 검사하던 다른 테스트는 참조를 지우고, 대신 "hooks/ 에 `.md` 프롬프트가 0개" 를 단정해요.
- **Patterns to follow:** 같은 파일의 `makeHome`·`makeHomeWithMarker`·`runScript`·`expectSilent` 헬퍼.
- **Test scenarios:** U1·U2 의 목록 전부. 추가로 stub 이 PATH 에 없을 때 gate 가 침묵해요.
- **Verification:** `bun test tests/hook-execution.test.ts tests/cli-path-resolution.test.ts tests/smooth-behavior.test.ts tests/plugin-bundle.test.ts` 통과.

### U4. Update the policy documents and CHANGELOG

- **Goal:** 규칙 소유권을 정책 문서에 두고 사용자 공개 문서를 사실과 맞춰요.
- **Requirements:** R14, R15
- **Dependencies:** U2
- **Files:**
  - `docs/policy/agent-policy.md` (AP-26 신설)
  - `CLAUDE.md` ("자동 업데이트 hook" 절 재작성)
  - `POLICY.md` (네트워크 접근 bullet, 로컬 파일 목록, "자동 업데이트와 끄는 법")
  - `codex-overrides/POLICY.md`, `codex-overrides/README.md`
  - `tests/policy-parity.test.ts` (AP-26 적용 파일 parity)
  - `CHANGELOG.md` (release 시 narrative)
  - `axhub-아키텍처-가이드.md` (untracked 문서 — 프롬프트 언급 1줄만 정정)
- **Approach:**
  1. AP-26 규칙: "auto-update 훅은 사용자에게 묻지 않고 백그라운드 worker 로 CLI 와 플러그인을 적용해요. 훅 계열 중 유일하게 네트워크·바이너리 교체를 하는 예외이고, 전경 훅은 여전히 네트워크 0 이에요. 알림은 세션당 최대 한 줄, 보안 검증 실패(exit 14/66)는 반드시 1회 안내, 같은 버전 재시도 없음. kill switch 2계층. 관측은 `~/.axhub/cache/auto-update.log`." 적용: 훅 3개, CLAUDE.md, POLICY.md.
  2. CLAUDE.md 의 "네트워크 호출은 hook 이 아니라 prompt(에이전트)가 해요" 문장을 AP-26 예외로 바꾸고 파일 목록·marker 표를 갱신해요.
  3. POLICY.md 는 사용자 언어로: "세션 시작 때 24시간에 1회, 뒤에서 조용히 새 버전을 확인하고 설치해요. 묻지 않아요. 설치가 끝나면 다음 세션 첫 답변에 한 줄로 알려요. 끄기: …". 로컬 파일 목록에 lock·log·marker 3개를 더해요.
  4. CHANGELOG narrative(release 단계): "훅 wrapper 가 이제 백그라운드에서 직접 업데이트를 적용해요 — Codex 는 wrapper 내용을 재신뢰 없이 갱신하므로 이 줄이 공지예요".
- **Test scenarios:** policy-parity 가 AP-26 의 적용 파일 각각에서 핵심 문구(`묻지 않`, `auto-update.log`, `no-auto-update`)를 찾아요. lint:tone 0 err.
- **Verification:** `bun test tests/policy-parity.test.ts`, `bun run lint:tone --strict`.

### U5. Regenerate the codex derived bundle

- **Goal:** AP-20 대로 소스·치환 테이블·override 만 고쳐 codex 번들을 재생성해요.
- **Requirements:** R9(codex 분기), AE6
- **Dependencies:** U1, U2, U4
- **Files:**
  - `scripts/build-plugin-bundle.ts` (`CODEX_SUBSTITUTIONS` 에 `HOST=` 치환 추가)
  - `codex-overrides/hooks/auto-update-prompt.md`, `codex-overrides/hooks/plugin-restart-confirm-prompt.md` (삭제)
  - `codex-overrides/SOURCE_HASHES.json` (삭제 2건 제거, POLICY/README 재핀)
  - `tests/codex-bundle.test.ts` (auto-update-prompt 단정 → worker 단정)
  - `plugins/axhub`, `plugins/axhub-codex` (재생성 산출물)
- **Approach:**
  1. 치환 1건(`HOST=claude` → `HOST=codex`)을 더하고 worker 의 캐시·marker 파일명이 기존 `-codex` 치환에 걸리는지 확인해요. `transformCodexHooks` 의 hooks.json 재작성에서 entry 1 의 `timeout` 키만 제거하고 `async: true` 는 유지해요 — codex 는 async 훅에도 timeout 을 적용하므로 5초면 apply 가 잘려요 (기본 600s 사용). DP-8 compat matrix 에 (7) "async 훅 지원·timeout 적용·출력 다음 안전 지점 전달" 을 재검증 항목으로 더해요.
  2. codex 판 플러그인 갱신 명령은 worker 의 `HOST=codex` 분기가 소유하므로 별도 override 문서는 필요 없어요.
  3. `bun run plugin:bundle:all` 로 재생성하고 drift·FORBIDDEN·hash-pin 게이트를 돌려요.
- **Test scenarios:** 파생 worker 에 `claude plugin` 0건, `codex plugin marketplace upgrade axhub` 1건 이상, `.plugin-update-check-codex`·`.plugin-update-restart-codex` 존재 (AE6). 파생 hooks/ 에 `.md` 0개.
- **Verification:** `bun test tests/codex-bundle.test.ts tests/plugin-bundle.test.ts`.

### U6. Manual end-to-end verification

- **Goal:** stub 이 아닌 실제 CLI·host 로 AE1·AE2 를 한 번 확인해요.
- **Requirements:** R6, R8, R9, R11, R13
- **Dependencies:** U5
- **Files:** 없음 (결과를 PR 본문 test plan 에 기록)
- **Approach:**
  1. macOS: `dist/axhub-plugin` 을 `--plugin-dir` 로 로드, 캐시를 2일 전으로 `touch -t`, 세션 시작 → log 확인 → 다음 세션 알림 줄 확인. 최신 상태면 `UP_TO_DATE` 한 줄로 충분해요.
  2. Windows Git Bash: 같은 순서 + worker 생존(log 줄) 확인.
  3. 실패 경로 1건: `AXHUB_NO_AUTO_UPDATE=1` 로 아무 파일도 생기지 않음 확인.
- **Verification:** 세 결과를 PR test plan 체크박스로 남겨요.

---

## Verification Contract

| 게이트 | 명령 | 기대 |
| --- | --- | --- |
| 훅 단위 | `bun test tests/hook-execution.test.ts` | worker(gate 포함)·restart-confirm describe 전부 PASS |
| 참조 정리 | `bun test tests/cli-path-resolution.test.ts tests/smooth-behavior.test.ts tests/plugin-bundle.test.ts` | PASS, `hooks/*.md` 0개 |
| 정책 parity | `bun test tests/policy-parity.test.ts` | AP-26 적용 파일 parity PASS |
| codex drift | `bun run plugin:bundle:all && bun test tests/codex-bundle.test.ts` | PASS, hash-pin 갱신 |
| tone | `bun run lint:tone --strict` | 0 err |
| 예산 | `bun run plugin:budget` | 무변화(skill 미수정) |
| 수동 | U6 | 시작 지연 없음, log 줄, 알림 1줄 |

## Definition of Done

- [x] U0 결과가 A1·KTD2 에 기록됐어요 (2026-09-02 Claude 실측).
- [x] `hooks/` 에 프롬프트 `.md` 가 없고 async worker(`session-auto-update.sh`)·restart-confirm 2개가 R1~R13 을 소유해요.
- [x] 에이전트가 auto-update 경로에서 실행하는 명령이 0개예요 (알림 한 줄만).
- [x] AE1~AE6 이 자동 테스트(tests/hook-execution.test.ts·tests/codex-bundle.test.ts)로 확인됐어요. 대화형 세션의 다음-턴 알림 전달(AE1 후반)·codex 실세션·Windows 는 U6 수동 확인이 남아 있어요.
- [x] AP-26 이 신설되고 CLAUDE.md·POLICY.md·codex 판이 같은 방향이에요.
- [x] codex 번들이 재생성되고 drift·hash-pin 게이트가 통과해요.
- [x] `skills/` 는 변경 0 이에요.
- [ ] `gitnexus_detect_changes()` 로 변경 범위가 hooks·tests·docs·scripts·plugins(재생성)로 한정됐어요.

## Risks & Dependencies

- **plugin 훅에서의 `async` 지원(A1).** hooks.md 는 settings.json 예시만 보여줘요. U0 (a) 가 실패하면 Stop condition 이에요.
- **구 Claude Code 호스트.** `async` 를 모르는 버전은 worker 를 동기로 돌려요. `timeout: 5` 가 지연을 5초로 막지만 그 호스트에선 자동 업데이트가 사실상 안 돌아요 (수동 `update` 스킬로 fallback, atomic swap 이라 중간 취소는 안전).
- **headless `-p` 세션.** teardown 때 async 훅이 kill 돼요. 그 회차는 잃고 lock TTL(30분) 뒤 다음 세션이 재시도해요. 항상 `-p` 로만 쓰는 환경은 자동 업데이트가 안 돼요 — 수용.
- **codex 훅 신뢰.** codex 는 사용자가 훅을 신뢰해야 async worker 가 돌아요 — 신뢰 전엔 자동 업데이트가 없고 `update` 스킬이 1차예요 (기존과 동일, KTD8 of codex compat plan). entry 에 `async` 키를 더해도 재신뢰가 필요 없는지는 U0 (d) 가 확정해요.
- **worker 와 `update` 스킬 동시 실행.** 사용자가 같은 순간 수동 update 를 부르면 apply 가 겹칠 수 있어요. CLI 의 atomic swap 이 방어하고 lock 은 worker 간만 잡아요 — 수용.
- **codex 신뢰 계약.** wrapper 내용 변경은 재신뢰 없이 반영돼요. CHANGELOG 명시가 POLICY 계약이라 U4 에 포함했어요.
- **관측 상실.** 전경 서사가 사라지므로 실패가 사용자에게 보이지 않아요. log 파일 + 보안 실패 필수 안내 + Deferred 의 update 스킬 보고로 보완해요.
- **구 CLI.** `--field-expr` 부재 시 R7 fallback 으로 CLI 만 올리고, 올라간 뒤 다음 주기에 정상 경로로 합류해요.

## Open Questions

- OQ1. 사후 알림 한 줄(R11·R13)도 없애 완전 무음으로 갈지. 기본은 유지 — 플러그인은 재시작 안내가 기능상 필요하고, CLI 는 동작이 바뀐 이유를 한 줄로 알 수 있어야 해요.
- OQ2. throttle 을 24h 에서 6h 로 줄일지. 배경 실행이라 사용자 비용은 0 이지만 백엔드 check 호출이 4배예요. 기본은 24h 유지.

## Sources & Research

- Claude Code hooks reference (https://code.claude.com/docs/en/hooks.md) "Run hooks in the background" — `async: true` 는 command 훅 전용, 비차단, 실행 중 timeout 미적용, 완료 후 `additionalContext`·`systemMessage` 를 다음 턴에 에이전트에게만 전달, `-p` 세션은 teardown 때 kill, 오래 살아야 하면 fully detached process 권장, v2.1.202 이전엔 깨진 JSON 이 세션을 죽였음, 완료 알림은 기본 억제.
- Codex hooks reference (https://developers.openai.com/codex/hooks) "Run hooks in the background" — `async: true` 지원, 같은 trust review·timeout(기본 600s, async 에도 적용)·large-output 규칙, 출력은 턴 진행 중이면 그 턴의 다음 모델 요청에·아니면 다음 사용자 턴에 전달, 동시 8개 상한, 세션 종료 시 미전달 출력 폐기, `SessionEnd` 는 항상 동기. 로컬 실측 버전 codex-cli 0.152.0.
- gstack `bin/gstack-session-update` — lock·log·`just-upgraded-from` marker 패턴 (fork 패턴은 두 호스트 모두 async 훅으로 대체해 채택하지 않았어요).
- gstack `bin/gstack-update-check` — "침묵 ≠ 최신"(CHECK_FAILED) 교훈.
- `axhub update apply --help` (0.38.0) — `--execute`·`--yes`·non-TTY bypass·atomic swap·downgrade gate·cosign.
- `axhub update check --plugin-version 1.27.1 --field-expr '…|join(" ")'` 실측 출력 `false v0.38.0 false false false 1.27.1`.
- 현재 `hooks/session-auto-update.sh`, `hooks/session-restart-confirm.sh`, `hooks/auto-update-prompt.md`, `hooks/plugin-restart-confirm-prompt.md`, `scripts/build-plugin-bundle.ts` 의 codex 훅 변환, `tests/hook-execution.test.ts` 계약.
