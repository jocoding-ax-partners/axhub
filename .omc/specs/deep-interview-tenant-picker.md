# Deep Interview Spec: 공유 tenant-picker 도입 (init/deploy + 전체 tenant-scoped skill)

## Metadata
- Interview ID: di-tenant-picker-2026-06-09
- Rounds: 5
- Final Ambiguity Score: 12.5%
- Type: brownfield (2 repo: axhub skills + ax-hub-cli)
- Generated: 2026-06-09
- Threshold: 0.2
- Threshold Source: default
- Initial Context Summarized: no
- Status: PASSED

## Clarity Breakdown
| Dimension | Score | Weight | Weighted |
|-----------|-------|--------|----------|
| Goal Clarity | 0.90 | 0.35 | 0.315 |
| Constraint Clarity | 0.85 | 0.25 | 0.2125 |
| Success Criteria | 0.85 | 0.25 | 0.2125 |
| Context Clarity | 0.90 | 0.15 | 0.135 |
| **Total Clarity** | | | **0.875** |
| **Ambiguity** | | | **0.125** |

## Topology
| Component | Status | Description | Coverage / Deferral Note |
|-----------|--------|-------------|--------------------------|
| C1 공유 tenant-picker (2계층) | active | **Bash 계층**(canonicalizable, doctor 강제) + **Agent prose 계층**(AskUserQuestion, registry) — preflight 선례는 순수 bash라 AskUserQuestion 불가, 그래서 2계층 | AC1~AC6 |
| C2 init + deploy | active | 사용자 1차 지목. 블록 채택 + resolved tenant threading | AC2, AC7 |
| C3 Class B + 조회 skill | active | env/data/tables/infer-tables-env/publish/rollback/recover/logs/status/migrate 등 tenant 처리 없던 skill | AC2, AC7 |
| C4 Class A 통일 | active | apps/app-lifecycle/connectors/resources/team/my-resources/workspace — 이미 `--tenant` 쓰던 skill을 동일 블록으로 통일 | AC2, AC7 |

## Goal
axhub의 모든 tenant-scoped skill(특히 앱 생성 `init`, 배포 `deploy`)에서, 사용자가 **여러 tenant에 소속**돼 있고 명시적 선택이 없을 때 **하나의 공유 canonical 블록**이 대화형으로 특정 tenant를 고르게 하고, 고른 tenant를 **세션 캐시**에 기록해 같은 세션의 이후 skill이 재질문 없이 상속하게 한다. CLI(ax-hub-cli)는 이미 `--tenant`/`AXHUB_TENANT`/`tenants list`를 지원하므로 **CLI 코드는 변경하지 않고 skill 계층만** 수정한다.

## Constraints
- **CLI 불변**: ax-hub-cli 코드 변경 없음. 기존 `axhub tenants list [--all]`, `tenants whoami`, 글로벌 `--tenant`(env `AXHUB_TENANT`), preflight `current_team_id`만 활용.
- **Mechanism = 2계층 (CRITICAL — advisor 지적)**: `CANONICAL_PREFLIGHT_BLOCK`은 순수 bash(`PREFLIGHT_JSON=$("$HELPER" ...)` + echo)라 **AskUserQuestion 호출 불가**(AskUserQuestion은 agent tool, SKILL.md prose가 구동). 따라서 picker는 2계층:
  - **(L1) Bash 계층** — canonicalizable, preflight와 동일 패턴. `scripts/tenant-picker-block.ts`의 `CANONICAL_TENANT_PICKER_BLOCK`. precedence 해석(명시 env/flag → `.axhub/state/tenant.json` 캐시 → preflight `current_team_id`) + `axhub tenants list --json` + `needs_pick` flag(멤버십 ≥2 & 미해석 & TTY) 계산 + 후보 JSON emit + 캐시 read. *(옵션: bash 로직을 `axhub-helpers tenant-resolve --json` 헬퍼 서브명령으로 빼면 Rust에서 단위 테스트 가능 — planning 결정사항.)*
  - **(L2) Agent prose 계층** — `needs_pick`면 AskUserQuestion(후보 = `tenants list`)으로 고르고, 선택을 `.axhub/state/tenant.json`에 write-back + `AXHUB_TENANT` export. 이 prose는 registry 등록 대상.
  - **양 계층 모두 doctor 강제**: L1은 byte-identical bash 존재(preflight식), L2는 AskUserQuestion + write-back stanza 존재를 skill-doctor가 검사(신규 check). scaffold(`skill-new.ts`)가 둘 다 삽입.
- **Smart trigger**: picker는 `tenants list` 멤버십이 **2개 이상** 이고 명시 선택이 **없을 때만** 뜬다.
  - 명시 `--tenant` / `AXHUB_TENANT` 존재 → 항상 존중, picker 건너뜀.
  - 멤버십 1개 → 자동 선택, 안 물음.
  - non-interactive(D1 TTY guard 비충족) → active tenant fallback, 절대 block 안 함.
- **해석 우선순위** (블록 내부):
  1. 명시 `--tenant` / `AXHUB_TENANT`
  2. 세션 캐시 (유효 시)
  3. preflight `current_team_id` + `tenants list` → 멤버십 ≥2 & TTY면 AskUserQuestion, 1개면 자동, non-TTY면 active fallback
- **지속성 = 세션 캐시 (`.axhub/state/*.json`)**: 고른 tenant를 `.axhub/state/tenant.json`에 JSON으로 기록(사용자 지시). 선례: `.gitignore`가 이미 `.axhub/init-resume.json`, `.axhub-state/` 무시 → `.axhub/state/`도 gitignore에 추가(런타임 state, 커밋 금지). 같은 세션 다음 skill은 상속, 재질문 없음. JSON 스키마(안): `{ "tenant": "<slug|id>", "source": "picker|explicit|preflight", "session_id": "<id>", "ts": "<ISO-8601>" }`.
- **캐시 무효화**: (a) 명시 override가 캐시와 다르면 override 우선 + 캐시 갱신, (b) 재로그인/프로필 변경, (c) 캐시된 tenant가 현재 `tenants list` 멤버십에 없음 → 클리어 후 재해석.
- **톤**: 모든 한글 해요체. `bun run lint:tone --strict` 0 err. 금지 token 회피.
- **trigger 어구**: 새 nl-lexicon trigger 어구는 frontmatter `description:`에만. body에 신규 trigger 추가 금지 (keywords baseline lock).
- **AskUserQuestion 등록**: picker 질문을 `tests/fixtures/ask-defaults/registry.json`에 `safe_default`(= active/single tenant) + `rationale` 등록.

## Non-Goals
- ax-hub-cli(Rust CLI) 코드 변경.
- 새 `tenants set-active` / config `active_tenant` 필드 추가.
- tenant 선택을 profile에 영구 저장 (profile use 경유) — 세션 캐시로 한정.
- 진짜 tenant-scoped 아닌 우발 매칭 skill(browse/github/open/trace/apis/inspect/enable-statusline 등)에 블록 강제 주입 — 정확한 inclusion 규칙으로 제외.

## Acceptance Criteria
- [ ] AC1: **(L1)** `scripts/tenant-picker-block.ts`에 `CANONICAL_TENANT_PICKER_BLOCK`(bash) export, `skill-new.ts`가 import·삽입, `skill-doctor.ts`가 대상 skill에서 byte-identical 존재 강제. **(L2)** AskUserQuestion + write-back prose stanza도 scaffold 삽입 + skill-doctor 신규 check로 존재 강제.
- [ ] AC2: 모든 **대상 skill**(아래 inclusion 규칙) SKILL.md가 블록을 포함하고, 해석된 tenant를 자신의 모든 tenant-scoped `axhub` 호출에 `--tenant`/`AXHUB_TENANT`로 thread.
- [ ] AC3: Smart trigger 동작 — 멤버십 ≥2 & 명시선택 없음 & TTY → AskUserQuestion / 1개 → 자동 / non-TTY → active fallback / 명시 override → 건너뜀.
- [ ] AC4: `.axhub/state/tenant.json`에 세션 캐시 기록 + 같은 세션 후속 skill 상속(재질문 없음) + 무효화 규칙(override 불일치 / 재로그인 / 멤버십 이탈) + `.axhub/state/` gitignore 추가.
- [ ] AC5: picker AskUserQuestion(L2)이 `registry.json`에 `safe_default`(= active/single tenant) + `rationale`로 등록.
- [ ] AC6: 공통 게이트 그린 — `bun run skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`(no diff), `bunx tsc --noEmit`, `bun test`(기존 baseline 유지/초과).
- [ ] AC7: **새 contract test**(migrate-skill-contract.test.ts 패턴)가 (a) 모든 대상 skill에 L1 bash 블록 + L2 prose stanza 존재 (b) Smart trigger 분기 (c) 세션 캐시 상속을 assert.

## Phased Rollout (advisor 권장 — 같은 end-scope, 순서만)
사용자가 "일단" init+deploy 라 했고 19 skill byte-identical 주입은 리뷰 부담 큼. **end-scope는 전체 유지하되 순서**를 둠:
- **Phase A (reference impl)**: L1 블록 + L2 prose + `.axhub/state` 캐시 + contract test 를 **init + deploy 2개에만** 먼저 안착. 패턴·doctor check·test 검증.
- **Phase B (fan-out)**: 검증된 패턴을 나머지 ~17 skill로 확산.
contract test가 Phase A 에서 잠금 기준을 먼저 확보 → Phase B diff 안전.

### Inclusion 규칙 (대상 skill 확정)
"tenant-scoped 자원을 **resolve 또는 mutate**하는 `axhub` 서브명령(apps / deploy / env / data / tables / resources / connectors / members / invitations / catalog / publish / rollback / recover / migrate / app fork 등)을 호출하는 skill" = 대상. 조회 skill(logs / status / routing-stats)도 사용자 결정으로 포함(세션 캐시라 세션당 1회만 물음).
- **확정 코어**: init, deploy, env, data, tables, infer-tables-env, publish, rollback, recover, migrate, apps, app-lifecycle, connectors, resources, team, my-resources, workspace, logs, status.
- **구현 시 관련성 확인 후 결정**: auth(로그인 시 tenant 선택 특수 케이스), routing-stats, onboarding.
- 우발 매칭(browse/github/open/trace/apis/inspect/enable-statusline)은 정밀 규칙으로 **제외**.

## Assumptions Exposed & Resolved
| Assumption | Challenge | Resolution |
|------------|-----------|------------|
| CLI에 tenant 선택 기능이 없다 | 이미지·코드 확인: 글로벌 `--tenant`, `tenants list`, preflight `current_team_id` 존재 | gap은 CLI가 아니라 **skill 계층**. CLI 불변. |
| init/deploy 2개만 | "더 있나 찾아봐줘" | 전체 tenant-scoped skill로 확장 (공유 picker → 전체 도입). |
| 공유 picker를 어떻게? | 코드 선례 = preflight canonical 블록 | canonical 블록 주입 방식 채택. |
| 항상 picker | 매번 물으면 노이즈 | Smart: 멤버십 ≥2 & 미선택일 때만. |
| 고른 tenant run 단위 | 19 skill 가로질러 재질문 짜증 | 세션 캐시 → 세션당 1회. |
| 순수 조회는 picker 불필요 (contrarian) | 틀린 tenant면 조회결과도 헷갈림 | 조회도 포함, 세션 캐시로 비용 상쇄. |
| 공통 게이트면 충분 | 회귀 보호 약함 | 전용 contract test 추가. |

## Technical Context
**ax-hub-cli (불변, 활용만):**
- `axhub/src/cli.rs:34` 글로벌 `--tenant <slug|id>` (env `AXHUB_TENANT`).
- `axhub/src/commands/tenants.rs`: `TenantsCmd::List { all }`, `Whoami`, `Get` — 멤버십 나열·확인.
- `axhub/src/commands/deploy/fleet.rs`: `deploy fleet --tenant` (진단/프리플라이트 용도, 실제 라우팅은 app ID 단위).
- `axhub/src/commands/app_ref.rs`: `deploy create`가 `--tenant`로 app slug를 tenant-scoped 해석.
- config(`crates/axhub-config/src/lib.rs`)에 `active_profile`만, `active_tenant` 없음.

**axhub helpers (이 repo):**
- `crates/axhub-helpers/src/preflight.rs:250,279-284`: preflight가 `current_team_id`(/`current_tenant_id`) 출력 → 블록의 active tenant 소스.
- `crates/axhub-helpers/src/main.rs:2924`: 기존에 `current_team_id`를 `--tenant`로 thread하는 선례 있음.

**skill 인프라:**
- `scripts/preflight-block.ts` → `CANONICAL_PREFLIGHT_BLOCK` (선례 패턴).
- `scripts/skill-new.ts` (scaffold), `scripts/skill-doctor.ts` (계약 강제).
- `tests/fixtures/ask-defaults/registry.json` (AskUserQuestion safe_default).
- `tests/migrate-skill-contract.test.ts` (contract test 패턴 참고).

**현재 skill tenant 처리 2부류:**
- Class A (이미 `--tenant "$TENANT"` 전달): apps, app-lifecycle, connectors, resources, team, my-resources, workspace.
- Class B (tenant-scoped인데 무처리): env, data, tables, infer-tables-env, publish, rollback, recover, logs, status, migrate.

## Ontology (Key Entities)
| Entity | Type | Fields | Relationships |
|--------|------|--------|---------------|
| Tenant | core domain | slug, id, name | User has many Tenant (membership) |
| TenantMembership | core domain | tenant, role | from `tenants list` |
| CANONICAL_TENANT_PICKER_BLOCK | core domain | bash block 텍스트 | injected into Skill; enforced by skill-doctor |
| SessionTenantCache | supporting | sessionId, tenant, source, ts | written by block, read by later Skill |
| PreflightContext | external system | current_team_id | source of active tenant |
| Skill | core domain | slug, frontmatter, body | hosts block; threads tenant into CLI |

## Ontology Convergence
| Round | Entity Count | New | Changed | Stable | Stability Ratio |
|-------|-------------|-----|---------|--------|----------------|
| 1 | 3 (Tenant, Block, Skill) | 3 | - | - | N/A |
| 2 | 4 (+TenantMembership) | 1 | - | 3 | 75% |
| 3 | 5 (+SessionTenantCache) | 1 | - | 4 | 80% |
| 4 | 6 (+PreflightContext) | 1 | - | 5 | 83% |
| 5 | 6 | 0 | - | 6 | 100% |

## Interview Transcript
<details>
<summary>Full Q&A (5 rounds + Round 0 ×2)</summary>

### Round 0 (topology)
**Q:** top-level 컴포넌트가 init/deploy 2개 skill + 공유 picker, CLI 불변 맞아요?
**A:** "일단 skill만인데 저 2개 skill 말고 더 있나 찾아봐줘"
**Q(재):** tenant 선택 기능을 어디까지? (Class A 이미 --tenant / Class B 무처리 / 조회 skill 발견)
**A:** "공유 picker → 전체 도입"

### Round 1
**Q:** 공유 picker 형태 — canonical 블록 주입(선례) vs 새 skill vs 둘 다?
**A:** Canonical 블록 주입 (선례 일치)
**Ambiguity:** 44%

### Round 2
**Q:** picker trigger 조건? (명시 override는 항상 건너뜀 공통)
**A:** Smart (다중 tenant일 때만)
**Ambiguity:** 36%

### Round 3
**Q:** 고른 tenant 지속 범위? (config active_tenant 없음)
**A:** 세션 캐시 (재질문 없음)
**Ambiguity:** 31%

### Round 4 (Contrarian)
**Q:** 순수 조회 skill에도 picker? 아니면 mutate/resolve만?
**A:** 조회도 picker 포함
**Ambiguity:** 26%

### Round 5
**Q:** Done 기준 — 추가 검증 수준?
**A:** 계약 테스트 추가 (권장)
**Ambiguity:** 12.5%
</details>
