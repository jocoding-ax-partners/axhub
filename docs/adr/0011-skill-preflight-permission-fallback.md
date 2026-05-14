# ADR-0011: SKILL preprocessing preflight 권한 fallback

## Status

Accepted (2026-05-14, PR #99) — Option B 단독 path 채택, Option A 매니페스트 wildcard
spec probe (§검증된 가정 #4) 는 Phase 27.y RFC 로 follow-up (`feat(plugin): permissions
manifest wildcard support`). 본 ADR Decision 의 lite/deploy variant codegen + strict-anchor
denialRegex `(?:Shell|Bash)` + unrecognized stderr passthrough 3 mechanism 으로 PR #99
ship.

본 ADR 의 범위는 SKILL preprocessing `!command` injection layer 의 fail-open contract 예요.
`!command` injection 이 아닌 in-workflow manual `axhub-helpers` 호출 (예: `skills/apis/SKILL.md:40`,
`skills/apps/SKILL.md:40`, `skills/deploy/SKILL.md:386`, `skills/doctor/SKILL.md:102`,
`skills/status/SKILL.md:51`) 은 본 fix 범위 외 — Claude Code Bash tool 권한 게이트
(PreToolUse hook fail-open) 적용 layer 라 별도 예요.

**deploy:101 도 현재 `stdio:'inherit'` 로 동일 결함 보유 — 본 fix 가 codegen 본문 교체로 동시 해결**
(iteration 4 Minor (d)). 9 SKILL + 1 template 모두 단일 fix path 로 정합.

## Context

9 SKILL + 1 template (routing-stats / trace / env / github / recover / apps / apis / deploy / verify
+ `_template/SKILL.md.tmpl`) 의 `!${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json` 줄이
Claude Code 의 first-run permission prompt 와 충돌해서 vibe coder 에게 raw 영문
"Shell command permission check failed … requires approval" 텍스트가 노출돼요.
docs/HOOKS.md §3 의 fail-open 계약은 Rust helper hook 진입점만 다루고 SKILL preprocessing 레이어에는 적용 안 돼요.

실측: deploy:101 만 이미 Node runner 안에 들어 있고 나머지 8 SKILL + 1 template 은 raw shell substitution.
codegen 이 lite variant (8 SKILL + 1 template) + deploy variant (deploy:101 만, PowerShell setup 블록 포함)
두 variant 를 single source 에서 emit. deploy:101 본문 교체 + 나머지 8 SKILL + 1 template 신규 wrap.

## 검증된 가정 (Step 0.5/0.7 — iteration 2 신규, iteration 4 강화)

1. **권한 layer probe (Step 0.5)**: outer Claude Code Bash 권한 게이트가 `!node -e "..."` 자체에는
   권한을 묻지 않고, inner `spawnSync(helper, ['preflight', '--json'])` 의 stderr 가 surface 에 노출돼요.
   따라서 B fallback 의 strict-anchor regex 가 inner stderr 를 잡을 수 있어요.
   (probe 결과: `tests/e2e/claude-cli/permission-prompt-surface.test.ts` fixture)

2. **Node runner stderr capture mechanism (iteration 3 Critical #1)**:
   `cp.spawnSync(helper, ['preflight', '--json'], {stdio:['inherit','inherit','pipe'], env})`
   이 inner stderr 를 `result.stderr` 로 capture 하면서 stdout 은 parent 로 forward 해요.
   `stdio:'inherit'` 이면 `result.stderr` 가 null/빈 Buffer 라 strict-anchor regex 매칭 0 회 → silent no-op.
   stdio[2]='pipe' 필수예요.

3. **denialRegex 미매칭 unrecognized stderr passthrough (iteration 4 Major (b))**:
   strict-anchor regex 매칭 시에만 Korean systemMessage swallow, 미매칭 stderr (예: Rust panic backtrace,
   deprecation warning) 는 `process.stderr.write(stderrText)` 로 parent 에 passthrough 해요.
   silent black hole 방지, ADR-0010 "raw stderr 가 chat 으로 흘러요" 정합이에요.

4. **Option A 매니페스트 spec probe (Step 0.7)**: `.claude-plugin/plugin.json` 의
   `permissions: ["Bash(*/axhub-helpers preflight*)"]` wildcard 패턴 인식 여부 binary 예요.
   - **인식 ✓** → A+B 혼합 (defense-in-depth) 채택. A happy path TTFD=0, B fallback ADR-0010 graceful degradation 정합.
   - **인식 ✗** → B 단독 채택, follow-up Issue 로 Phase 27.y RFC 일정 명시.
   (probe 결과: `tests/fixtures/permission-manifest-probe/plugin.json`)

5. **Shell layer 가정 (PR #99 review M3 보강)**: Claude Code 가 SKILL `!` ` ` ` 블록을
   POSIX sh 호환 layer 로 invoke 해요 (Windows 에서도 Git Bash / WSL 의 sh 호환). 이 가정 위에서
   codegen 출력의 `node -e "..."` 가 `${CLAUDE_PLUGIN_ROOT}` 를 shell 단에서 확장 + `\"` 페어를
   `"` 로 unescape 한 뒤 `node -e` 에 전달돼요. native `cmd.exe` (escape 룰 `""`) 또는
   PowerShell raw invocation 환경은 본 fix scope 외 — Claude Code 가 그 환경에서 SKILL `!command` 를
   호출하기 시작하면 codegen 출력 escape 룰 재검토가 필요해요. cross-platform 테스트는
   `tests/skill-preflight-permission-fallback.test.ts` 의 buildScript() 가 shell unescape (`\\"` →
   `"`) 를 명시적 시뮬레이션하는 형태로 mock 해요. 실제 production trace 는 Phase 28.x follow-up
   으로 명시.

6. **denialRegex wording fuzz 의 (Shell|Bash) 매칭 (PR #99 review M1 보강)**: Claude Code 의 permission
   denial 첫 토큰이 `Shell command` 또는 `Bash command` 중 어느 쪽이든 매칭하도록 strict-anchor regex
   를 `/^(?:Shell|Bash) command permission check failed.*requires approval/im` 로 확장. tool 분기
   wording 변경 (e.g., bash 도구 vs shell 도구) 에 robust 해져요. 그래도 `.*requires approval`
   suffix 와 `^...command permission check failed` prefix anchor 는 유지 — ADR-0010 §42 strict-anchor
   정책 정합. 추가 wording 변형 (`Shell tool` / `permission denied`) 은 미매칭 → passthrough 분기로
   raw stderr 가 chat 에 표시. catastrophic 아니지만 본 PR UX 목표 약화는 trade-off 로 명시.

## Decision

SKILL preprocessing `!command` injection 라인을 cross-shell Node runner 로 wrap 하고,
permission denial 가 strict-anchor regex (`/^(?:Shell|Bash) command permission check failed.*requires approval/im`)
패턴 매칭 시 한국어 systemMessage 한 줄 출력 후 exit 0 으로 흐름을 SKILL Step 0 에 넘겨요.
미매칭 unrecognized stderr 는 parent 로 passthrough 해요 (ADR-0010 정합).
fail-open 원칙을 SKILL preprocessing 까지 확장하는 ADR 이에요.

codegen-preflight-injection.ts 의 emit 분기 (iteration 4 Major (a)):
- **lite variant** (8 SKILL + 1 template 적용): Node runner + stderr-pipe + denialRegex fallback +
  미매칭 passthrough only.
- **deploy variant** (deploy:101 적용): PowerShell `$env:PATH` setup 블록 (deploy:85-95) + lite variant body.

Node runner 의 `stdio` 옵션은 `['inherit', 'inherit', 'pipe']` 예요 — stdin/stdout inherit (사용자 입력 +
deploy:101 의 systemMessage JSON 출력 parent forward), stderr pipe (Node runner capture).
`stdio:'inherit'` 으로 두면 `result.stderr` 가 null/빈 Buffer 라 regex 매칭 0 회, silent no-op 발생해요.

regex 는 ADR-0010 의 "Pattern relaxation 비채택" 정책 정합으로 strict-anchor 적용
(iteration 1 의 `/requires approval|permission/` 에서 좁힘) — false-positive (generic `permission` 단어)
+ false-negative (wording 부분 변경) 양쪽 risk 동시 감소해요.

ADR-0010 과 관계: ADR-0010 = axhub binary stderr graceful degradation, ADR-0011 = Claude Code 권한 게이트
stderr fallback — strict-anchor 철학 정합이지만 다른 layer 예요. iteration 4 Major (b) 의 unrecognized
stderr passthrough 분기로 ADR-0010 "raw stderr 가 chat 으로 흘러요" 정책과 깊은 정합이에요.

Step 0.7 결과 인식 ✓ 일 때는 `.claude-plugin/plugin.json` 에 wildcard permissions entry 도 추가해서
A+B 혼합 (defense-in-depth) — A happy path TTFD=0, B fallback graceful degradation.

대안 검토:
- A 단독) wildcard / placeholder spec 검증 없이 채택 시 운영 risk — Step 0.7 binary outcome 으로 검증.
- C) SessionStart hook 사전승인 토큰 → SKILL preprocessing 영향도 불확실 + new consent infra short-mode 범위 초과.

## Consequences

### + 긍정

- 9 SKILL + 1 template 모두 첫 실행 raw 영문 거부 메시지 0 회.
- **deploy:101 production 결함 동시 해결 (iteration 4 Minor (d))** — 단일 fix path 로 정합.
- docs/HOOKS.md §3 fail-open 계약과 동형 — SKILL 레이어로 확장.
- codegen single-source + variant-aware byte-identical manifest test 가 10 곳 drift 자동 탐지.
- regex strict-anchor 가 ADR-0010 정책 정합 + false-positive/negative 양쪽 mitigate.
- **iteration 4 Major (b) 의 unrecognized stderr passthrough 가 silent black hole 방지** —
  ADR-0010 "raw stderr 가 chat 으로 흘러요" 정합, helper informational stderr (Rust panic,
  deprecation warning) 가 사용자에게 보여요.
- (Step 0.7 ✓ 시) TTFD=0 달성.

### − 부정

- (Step 0.7 ✗ 시) TTFD=1 잔존 — Phase 27.y follow-up RFC 일정 명시.
- Claude Code 가 영문 prefix 자체를 통째로 바꿀 가능성 — 가능성 낮음, silent skip + 미매칭
  passthrough 분기로 catastrophic 아님.
- Node runner stdio[2]='pipe' 가정 깨지면 silent no-op — Step 0.5 e2e probe 가 binary 측정.
- deploy:101 의 child stderr 가 denialRegex 매칭 시에만 swallow (Korean systemMessage 로 surface).
  미매칭 unrecognized stderr 는 parent passthrough 로 표시. 의도된 trade-off 예요.
- 8 곳 신규 Node runner wrap 으로 raw shell substitution 대비 첫 실행 SKILL preprocessing ~50-100ms 미세 증가.
- **PR #99 review M2 trade-off**: `result.error` 분기 (ENOENT / EACCES 같은 helper binary
  부재) 가 권한 거부 분기 (denialRegex 매칭) 와 동일한 systemMessage 를 출력. 사용자 mental model 은
  "권한 prompt" 가 아니라 "binary 부재" 라 "허용 클릭" 안내가 inaccurate — 첫 클릭 해도 다음에도
  같은 메시지를 봐요. 본 PR scope 에서는 의도된 trade-off (Case F 회귀 test 로 lock).
  Phase 27.y RFC 또는 별도 systemMessage 분기는 follow-up.
- **PR #99 security M2 trade-off**: stderr passthrough sink 가 chat surface (Claude Code
  대화 / telemetry / 사용자 screenshot / 공유 transcript) 라 ADR-0010 (axhub binary stderr)
  대비 secret leak risk 큼. helper 가 token / credential 을 stderr 에 emit 하지 않는 invariant
  깨질 시 production 노출 가능. mitigations: (1) codegen `redactRe` 가 `sk-` / `gho_` /
  `axhub_` / `Bearer` 4 패턴 redact (Case G 회귀 test 로 lock), (2) helper 의 stderr 출력
  policy 는 `crates/axhub-helpers/src/lib.rs` 의 logging surface 검토 follow-up.

## Follow-ups

- (Step 0.7 ✗ 시) Phase 27.y RFC: Claude Code 플러그인 매니페스트 `permissions` 필드 wildcard / placeholder 인식.
- Phase 28.x: Claude Code SKILL `!command` injection 의 systemMessage JSON surface 표시 동작 production trace 장기 관찰.
- Phase 28+: SKILL-specific preflight variant 요구 (예: trace 의 extra env var injection) 발생 시
  codegen-preflight-injection.ts 의 frontmatter flag 인식 확장 가능 (현재는 `needs-preflight: true` boolean
  + SKILL 이름 기반 deploy variant 매핑).

## 관련

- [docs/HOOKS.md §3 fail-open](../HOOKS.md)
- [ADR-0010 stderr graceful degradation](0010-stderr-filter-graceful-degradation.md) — axhub binary layer, 본 ADR 과 다른 layer 지만 정합 (unrecognized stderr passthrough 분기)
- 9 SKILL + 1 template `!command` injection 라인 (variant 매핑):
  - **deploy variant**: `skills/deploy/SKILL.md:101` (PowerShell setup + lite body)
  - **lite variant**: `skills/routing-stats/SKILL.md:29` / `skills/apis/SKILL.md:30` / `skills/trace/SKILL.md:36` / `skills/verify/SKILL.md:35` / `skills/env/SKILL.md:29` / `skills/github/SKILL.md:29` / `skills/recover/SKILL.md:32` / `skills/apps/SKILL.md:28` + `skills/_template/SKILL.md.tmpl:26`
  - (deploy:101 만 본문 교체, 나머지 8 SKILL + 1 template 은 raw shell substitution → lite variant Node runner envelope 신규 wrap)
