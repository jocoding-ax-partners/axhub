# PRD — Vibe Coder 5min Smooth Deploy

**Status**: draft (ralplan iter 1 reviewed by Architect + Critic, ITERATE → consolidated PRD)
**Owner**: axhub plugin maintainer
**Target**: 비개발자 vibe coder 가 빈 dir 에서 "현재 프로젝트 배포해줘" 한 번 입력 → live URL 까지 P50 5min / P95 8min
**Created**: 2026-05-06 (Phase 24.11 시작)

---

## 1. 배경 — 실측 세션 로그

사용자 (비개발자 vibe coder persona) 가 빈 dir 에서 "현재 프로젝트 배포해줘" 시도. 결과: **>10분 + 차단 5회 + 사용자 질문 6회**. 목표 5분의 2배+. 실측 로그 (transcript 발췌):

| 시각 | 이벤트 |
|------|--------|
| 0:00 | "현재 프로젝트 배포해줘" 입력 |
| 0:05 | preflight `cli_too_new: true` (0.2.6 보고, 실제 0.2.9) — version 스큐 false positive |
| 0:30 | resolve `apps_list_parse_error` (exit 67) — backend response shape mismatch |
| 1:00 | AskUserQuestion "어떻게 진행?" → 사용자 "axhub init 먼저" |
| 1:30 | init template scaffold 충돌 (.editorconfig 등) → AskUserQuestion → 사용자 "--force" |
| 2:00 | apphub.yaml 생성. 다시 deploy 시도 |
| 2:30 | apps_create 차단 (consent token 부재) |
| 2:30~6:30 | consent mint 시도 4회 모두 차단 (action/slug/source/positional 각 mismatch) |
| 6:30 | apphub.yaml 에 `subdomain`/`domain_id` 누락 → API 거부 |
| 7:00 | AskUserQuestion subdomain → 사용자 "trash5" |
| 7:30 | yaml 수동 편집 + apps_create 재시도 |
| 8:30 | 앱 생성 성공 (id=163) |
| 9:00 | resolve 다시 `apps_list_parse_error` |
| 9:30 | git 저장소 아님 → init + 첫 커밋 |
| **10:00+** | 여전히 deploy 단계 미도달 |

추가 사용자 신규 보고: **Issue #8** — apphub.yaml 있는데 Claude Code exit 후 재진입 시 init 재실행. 즉 manifest 인식이 session 재시작 후 사라짐.

---

## 2. 진단 — Root Cause (empirically verified)

| # | 증상 | Root cause (코드 위치) | 검증 |
|---|------|---------------------|------|
| 1 | `cli_too_new: true` false positive | `crates/axhub-helpers/src/telemetry.rs:resolve_cli_version` Mutex cache, **TTL 없음** | 코드 확인 |
| 2 | `apps_list_parse_error` (exit 67) | `crates/axhub-helpers/src/resolve.rs:125-142` `parse_apps_list` 가 **bare JSON array 만 지원**, backend `{apps:[...],total:N}` envelope 미지원 | 코드 확인 |
| 3 | init scaffold non-empty dir 충돌 | init SKILL `--force` 사용자 결정 떠넘김. CLI 자체가 안전 분기 없음 | SKILL 확인 |
| 4 | apps_create consent 차단 4회 | `crates/axhub-helpers/src/main.rs:184-189 cmd_consent_mint` **binding schema validator 없음**. agent 가 매번 다른 shape 추측 → 매번 mismatch. helper 가 `apphub.yaml` 에서 binding 자동 합성 안 함 | 코드 확인 |
| 5 | apphub.yaml `subdomain`+`domain_id` 누락 | template registry default 없음. backend default 도 없음. client/server 양쪽 source 모호 | template + backend 확인 필요 |
| 6 | git init 단계 늦음 | deploy SKILL Step 1 manifest 가정 + git_init 분기는 Step 1.5. 빈 dir 분기 없음 | SKILL 확인 |
| 7 | non-idempotent flow | bootstrap state file 없음. 부분 진행 후 재시도 시 처음부터 | 부재 |
| **8** | apphub.yaml 있는데 init 재실행 | `preflight.rs:220-223` `current_app` 이 **`AXHUB_APP_SLUG` env + `last-deploy.json` cache 만 읽음**. **`apphub.yaml` 미읽음**. 즉 init 가 yaml 만들어도 helper 가 다시 못 봄 → 매 session fresh | **코드 확인 (b 가설 confirmed)** |

Architect/Critic 추가 우려:
- Helper auto-mint 시 preview card 가 ceremonial 됨 — consent integrity 침식
- "5분 hard budget" 은 backend build 시간 (Next.js ~3min) 무시한 KPI
- bootstrap FSM 구체 spec 부재 시 SKILL prose chain = LLM 추측

---

## 3. Goals & Non-Goals

### Goals
- 빈 dir → live URL **P50 ≤ 5min / P95 ≤ 8min**
- AskUserQuestion **≤ 3개** (template / subdomain (collision 시) / 최종 동의)
- consent 차단 메시지 **0회**
- bug regression 회귀 0 (P1, P2, P8)
- HMAC consent gate 보안 surface 그대로 유지 (preview card 의 "네/아니요" 가 의미 있는 stop point)

### Non-Goals
- backend build 시간 단축 (architecture 가 줄일 수 없는 floor)
- production-grade Next.js 배포 (Demo 수준)
- 사용자 manifest 수동 편집 능력 (프로 개발자 흐름은 별도 PRD)

---

## 4. Principles

1. **Single bootstrap workflow** — 빈 dir → live URL 1 흐름. SKILL chain 자동.
2. **P50/P95 SLA budget** — hard 5min KPI 폐기. P50 5min / P95 8min 측정 기반.
3. **Consent friction zero** — helper 가 binding 자동 capture. agent 추측 X. **단** preview card 가 binding hash echo 해서 사용자가 source 검증 가능.
4. **Backend ↔ helper contract resilience** — response shape drift 자동 catch + dual-format 지원.
5. **CLI version freshness** — cache TTL 30초 또는 mismatch detect 시 즉시 invalidate.
6. **Idempotent + resumable** — bootstrap state file 보존. 부분 완료 후 재실행 안전.
7. **FSM in Rust, not Markdown** — chain 로직은 helper FSM 으로 enforce. SKILL.md prose 추측 금지.
8. **Single source of truth (server-side)** — subdomain/domain_id default 는 backend. client-side fill 폐기.

---

## 5. Architecture — Bootstrap FSM

새 helper subcommand: `axhub-helpers bootstrap`. Rust FSM 으로 빈 dir → deploy_ready 까지 enforce.

### State diagram

```
┌─────────┐  scaffold        ┌─────────────┐  apps_create     ┌────────────────┐
│  empty  │ ───────────────> │ scaffolded  │ ───────────────> │ app_registered │
└─────────┘                  └─────────────┘                  └────────────────┘
    │                              │                                  │
    │                              │ manifest_present?                │ git_init
    │                              ▼                                  ▼
    │                        ┌──────────────┐                 ┌────────────────┐
    │                        │ manifest_ok  │ ───skip────────>│ git_initialized │
    │                        └──────────────┘                 └────────────────┘
    │                                                                  │
    │                                                                  │ first_commit
    │                                                                  ▼
    │                                                         ┌────────────────┐
    │                                                         │  deploy_ready  │
    │                                                         └────────────────┘
    │                                                                  │
    │                                                                  │ deploy_create
    │                                                                  ▼
    │                                                         ┌────────────────┐
    └─── (already in some state, resume) ───────────────────> │   deploying    │
                                                              └────────────────┘
                                                                       │
                                                                       │ build success
                                                                       ▼
                                                              ┌────────────────┐
                                                              │   deployed     │
                                                              └────────────────┘
```

### Failure transitions

- `scaffolded → conflict_existing_files` → AskUserQuestion (skip / new subdir / overwrite)
- `app_registered → 422 subdomain_collision` → AskUserQuestion (suggested alt subdomain from backend response)
- `app_registered → 5xx transient` → retry up to 3x with idempotency key
- `git_initialized → first_commit_fail` → halt + show diagnostic (rare; mostly user permission)
- `deploying → build_fail` → exit code routing via `error-empathy-catalog.generated.md`

### State persistence

- Location: `.axhub/bootstrap.state.json` in cwd (project-local, not user-global)
- Schema:
  ```json
  {
    "version": 1,
    "state": "app_registered",
    "app_id": 163,
    "app_slug": "nextjs-axhub",
    "subdomain": "trash5",
    "domain_id": 1,
    "manifest_path": "./apphub.yaml",
    "git_initialized": false,
    "last_deploy_id": null,
    "updated_at": "2026-05-06T15:30:00Z"
  }
  ```
- gitignore: `.axhub/` 자동 추가 by bootstrap
- Re-entry: bootstrap 시작 시 state file 읽고 적절한 transition 위치로 jump

### Idempotency contract per state

| State | Re-run behavior |
|-------|----------------|
| `empty` | scaffold 다시 시도 OK (template 무 변경) |
| `scaffolded` | manifest 검사만, 새 init 시도 X |
| `manifest_ok` | apps_create 검사 — backend 에 동일 slug 존재 시 skip |
| `app_registered` | git_init 검사 — git repo 존재 시 skip |
| `git_initialized` | first_commit 검사 — HEAD 존재 시 skip |
| `deploy_ready` | deploy_create 만 mint + run |
| `deploying` | last_deploy_id watch 만 (재 mint X — double charge 방지) |
| `deployed` | "이미 배포됐어요. 변경사항 있으면 다시 배포할까요?" AskUserQuestion |

### Apps_create idempotency key

- HTTP `Idempotency-Key: bootstrap-{cwd_hash}-{slug}` header
- backend 가 동일 key 동일 slug 재요청 시 기존 app row 반환 (200 OK, 신규 row 미생성)
- 5xx transient 재시도 시 안전

---

## 6. Phase rollout

각 Sprint = 별도 PR. ralplan 통과 후 머지.

### Sprint 1 — v0.2.11 patch (즉시 ship)

| Phase | Scope | LOC est |
|-------|-------|---------|
| **P1** | `parse_apps_list` envelope dual-format (`[...]` ∨ `{apps:[...],total:N}`) | ~30 |
| **P2** | `resolve_cli_version` cache TTL=30s + version mismatch invalidate | ~50 |
| **P8** | `preflight.current_app` 가 cwd 의 `apphub.yaml`/`axhub.yaml` 도 읽음 (env > yaml > cache 순) | ~80 |

목표: 가장 명확한 helper bug 3개 즉시 fix. 회귀 위험 낮음.

### Sprint 2 — v0.3.0 minor (consent layer 변경)

| Phase | Scope | 비고 |
|-------|-------|------|
| **P4-prereq** | `mint_token` binding schema validator (action whitelist + required fields per action) | helper 에 schema bridge |
| **P3** | apphub.yaml `subdomain`/`domain_id` **server-side default** (backend 협업 필수). client-side fill 폐기 | backend PR + helper response read |
| **Audit** | `synthesized_by_helper: true` flag in JWT claims (helper auto-mint 추적) | telemetry sink |

목표: P4 prereq 단단히. backend 협업으로 source of truth 확립.

### Sprint 3 — v0.4.0 minor (bootstrap command)

| Phase | Scope | 비고 |
|-------|-------|------|
| **P4** | `axhub-helpers bootstrap` 신규 subcommand FSM (Rust) — 6 state diagram | 큰 PR |
| **P5** | deploy SKILL 의 Step 1 이 `bootstrap --auto-chain` 호출. 부재 분기 자동 | SKILL refactor |
| **P6** | init SKILL non-empty dir 안전 분기 (skip / new subdir / overwrite) | SKILL refactor |

목표: 빈 dir → deploy_ready 까지 단일 helper 명령. SKILL.md prose chain 폐기.

### Sprint 4 — v0.4.x patches (e2e + measurement)

| Phase | Scope | 비고 |
|-------|-------|------|
| **P7a** | baseline 측정 — 현재 deploy time P50/P95 instrument | telemetry events `bootstrap_phase_start/end` per stage |
| **P7b** | e2e 실제 backend, 짧은 test app (<30s build), P50/P95 SLA assertion | fixture + CI cost budget |
| **P7c** | release gate on P95 < 8min | CI workflow gate |

목표: SLA decoration 아닌 측정 기반 commitment.

---

## 7. Pre-mortem (5 scenarios)

### X. Helper bug → 100% deploy block — likelihood MED
P4 helper 가 binding 자동 합성. helper bug 시 token 항상 invalid → 전체 fleet block.
- Mitigation 1: `consent-mint --validate-only` dry-run flag 로 binding 검증
- Mitigation 2: helper version mismatch 감지 시 P4 auto-synth 비활성, agent-binding flow fallback
- Mitigation 3: `synthesized_by_helper` audit flag → backend log 30일 retention

### Y. Subdomain collision — likelihood HIGH
slug 기반 client default 가 사내 다른 팀과 충돌. API 거부.
- Mitigation: P3 client-side fill 폐기. backend 가 collision 시 422 + suggestion 응답. SKILL AskUserQuestion 으로 사용자 final 결정.

### Z. apps_create 성공 + git fail → re-run — likelihood HIGH
Bootstrap 중간 실패. State 파일 `app_registered` 상태로 멈춤.
- Mitigation: bootstrap 재진입 시 state file 읽고 git_init transition 으로 점프. apps_create 재시도 X (idempotent skip).

### W. 사용자 build 중 exit → 재실행 — likelihood HIGH (NEW)
Step 5 watch 도중 사용자 laptop 닫음. State `deploying`. last_deploy_id pending.
- Mitigation: bootstrap 재진입 시 state `deploying` + last_deploy_id 존재 → `axhub deploy logs --id <last_deploy_id>` 재 attach. **deploy_create 재 mint 금지** (double charge 방지).
- 추가: bootstrap state 에 `deploy_create_attempted_at` timestamp 저장. 24시간 경과 + status unknown 면 정상 retry 허용.

### V. apps_create 5xx transient — likelihood MED (NEW)
Backend 일시 장애. 첫 시도 5xx, 두번째 동일 slug 로 재시도 → backend 가 이미 row 만든 상태에서 충돌.
- Mitigation: `Idempotency-Key: bootstrap-{cwd_hash}-{slug}` header. backend 가 동일 key + 동일 slug 재요청 시 기존 row 반환. 신규 row 미생성.
- 추가: helper 자동 retry 3x with exponential backoff.

### U. Windows path/CRLF — likelihood LOW (NEW)
apphub.yaml CRLF, git init defaults, path separator.
- Mitigation: bootstrap FSM Rust 가 path 처리 일관 (PathBuf component-by-component join). yaml emit 시 LF 강제. e2e Windows job 추가.

---

## 8. Test plan (deliberate mode)

### Unit
- `parse_apps_list` 두 envelope 형식 (P1)
- `resolve_cli_version` cache TTL + mismatch invalidate (P2)
- `current_app` 가 yaml 우선 + env override + cache fallback 순서 (P8)
- `mint_token` binding schema validator — action whitelist 외 reject, required field 누락 reject (P4-prereq)
- bootstrap FSM transition matrix — 정상 + 실패 transition 모두 (P4)

### Integration
- bootstrap full chain (empty → deployed) mock backend
- bootstrap re-entry from each state (X/Y/Z/W/V scenario coverage)
- apps_create idempotency key collision

### E2E
- P50/P95 timing on real staging backend (test app, <30s build)
- AskUserQuestion count assertion (≤3)
- consent 차단 0 회 verification
- Windows path separator + CRLF (cli_e2e Windows fixture)

### Observability
- `bootstrap_phase_start/end` per state with timestamp (P50/P95 측정 source)
- `consent_synthesized_by_helper` flag emit on auto-mint
- `bootstrap_re_entry_at_state` event

### Contract test (S2 ↔ S3 bridge)
- `tests/fixtures/consent-bindings/{apps_create,deploy_create,...}.json` 공유 fixture
- S2 mint validator + S3 bootstrap synthesizer 동일 fixture 로 검증
- contract drift 즉시 catch

---

## 9. Acceptance criteria

- [ ] **P50 deploy time ≤ 5min** (1주 baseline 측정 후 commitment)
- [ ] **P95 deploy time ≤ 8min** (release gate)
- [ ] AskUserQuestion ≤ 3 (`bootstrap_ask_count` telemetry assertion)
- [ ] consent 차단 메시지 0 (telemetry event count)
- [ ] `apps_list_parse_error` 회귀 0 (P1)
- [ ] `cli_too_new` false positive 회귀 0 (P2)
- [ ] apphub.yaml 존재 시 init 재실행 0 (P8)
- [ ] preview card 에 binding hash echo (consent integrity)
- [ ] `synthesized_by_helper` audit flag 모든 helper auto-mint 에 존재
- [ ] bootstrap re-entry 시 state file 읽고 적절 transition 으로 jump
- [ ] apps_create idempotency key 동작 (5xx scenario V)
- [ ] deploy_create 재 mint 금지 in `deploying` state (scenario W)
- [ ] subdomain collision 시 backend 422 + suggestion (scenario Y, P3)
- [ ] Windows e2e green (scenario U)

---

## 10. ADR

- **Decision**: 7개 confirmed bug 를 4 sprint × 별도 PR 로 점진적 ship. helper bug fix → consent layer hardening → bootstrap FSM 명령 → measurement-based SLA. 단일 hard 5min KPI 폐기, P50/P95 SLA 채택.
- **Drivers**: vibe coder 5min smooth UX, backend↔helper contract resilience, consent gate 보안 유지, FSM Rust enforce (Markdown prose chain 폐기)
- **Alternatives 검토**:
  - **B (helper bug fix only)** — bug 4/6/7/8 미해결, 5min 미달
  - **C (atomic bootstrap helper command 단독)** — Architect 권고. P4-P6 으로 흡수
  - **B+C 분리** — 채택안 = Sprint 1 (B) + Sprint 3 (C) phased rollout
- **Why chosen**: phased rollout 이 (a) 즉시 ship 가능 bug fix 와 (b) backend 협업 필요 항목 분리, (c) bootstrap FSM 의 큰 surface 는 별도 sprint, (d) measurement 가 SLA decoration 안 되도록 마지막 sprint 분리.
- **Consequences**:
  - 4 PR + 4 ralplan 세션 추가 필요 (각 sprint)
  - backend 협업 필요 (P3 server-side subdomain)
  - helper auto-mint 보안 review 필요 (S2 + S3 사이)
  - e2e CI cost 증가 (실제 staging backend hit)
- **Follow-ups**:
  - HTML comment sentinel 패턴 (`<!-- stage-checklist:allow -->`) — Phase 24.10 carry-over
  - dev escape hatch (`AXHUB_PREAUTH_BYPASS` vs `consent-mint --dev-mode`) — ralplan iter 1-2 ADR 보관
  - CLI 자체 (`ax-hub-cli` 별도 repo) 의 init template + apps create 응답 shape 통일 backend 팀 협업
  - production grade Next.js deploy (Demo 수준 넘어선 hardening) — 별도 PRD

---

## 11. Open questions

1. **backend 팀 캐파** — P3 server-side subdomain default 와 envelope 응답 shape 통일은 backend repo 변경 필요. timeline 협의 필요.
2. **bootstrap state 위치 정책** — `.axhub/bootstrap.state.json` 이 multi-tenant CI 환경에서 stale 가능성. CI 마다 cleanup hook 필요?
3. **e2e cost budget** — staging backend hit 의 CI minutes/$ 한도. PR 마다? main 마다? nightly?
4. **Issue #5 (subdomain default)** server-side 결정 후 helper 가 yaml 에 fill 하지 말지 명시 필요 (현재 init template 가 빈 채로 둘 것인지 결정).
5. **dev escape hatch (Phase 24.9 ralplan iter 1-2 carry-over)** — maintainer 가 본인 repo 에서 raw bash 로 destructive 호출할 때의 흐름. 이 PRD scope 외이지만 vibe coder UX 와 별개로 결정 필요.

---

## 12. References

- 실측 세션 transcript (Phase 24.11 시작 시점, 사용자 메시지)
- `crates/axhub-helpers/src/preflight.rs:120-239` — current_app detection (Issue #8 root cause)
- `crates/axhub-helpers/src/resolve.rs:125-142` — parse_apps_list bare-array (Issue #2)
- `crates/axhub-helpers/src/resolve.rs:281-292` — apps_list_parse_error exit 67
- `crates/axhub-helpers/src/telemetry.rs:24-32` — resolve_cli_version no TTL (Issue #1)
- `crates/axhub-helpers/src/main.rs:184-189` — cmd_consent_mint no validator (Issue #4)
- `crates/axhub-helpers/src/main.rs:197-275` — cmd_preauth_check verify path
- `skills/deploy/SKILL.md` — Step 1 manifest assumption + Step 1.5 git_init
- `skills/init/SKILL.md` — non-empty dir 분기 부재 (Issue #3)
- `skills/apps/SKILL.md:75-90` — apps_create binding requirement (Issue #4)
- ralplan iter 1 transcripts (Architect REVISE + Critic ITERATE)
- Phase 24.9 ralplan iter 1-2 ADR (dev escape hatch carry-over)

---

## 13. 변경 이력

- **2026-05-06 (Phase 24.11 draft)**: 초안 작성. ralplan iter 1 Architect/Critic feedback 반영. Issue #8 empirical confirmed. 4 sprint phased rollout. P50/P95 SLA 채택.
- **2026-05-06 (Sprint 4 ralplan)**: Sprint 4는 helper telemetry 를 SLA 근거로 쓰지 않고, full-chain measurement harness wall-clock 을 P50/P95 source of truth 로 둔다. telemetry 는 `bootstrap_phase_start/end`, `bootstrap_re_entry_at_state`, `consent_synthesized_by_helper` marker 로만 사용한다. destructive measurement 는 explicit staging endpoint + token + `AXHUB_E2E_DESTRUCTIVE=1` + cost/run budget + cleanup/TTL contract 가 있을 때만 advisory/manual/nightly 로 실행한다. blocking release gate 는 N≥20 sample, backend P3 readiness, cleanup ownership, pre-publication workflow placement 후에만 켠다.
