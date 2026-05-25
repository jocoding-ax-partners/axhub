# TODOS

이 파일은 향후 작업 후보예요. 각 항목은 별도 PR 에서 다뤄요.

## P2 — STOP_WORDS layer 일관성 + Korean 형태소 분석

**Why:** `crates/axhub-helpers/src/resolve.rs:47-101` 의 `STOP_WORDS` HashSet 가 prompt-route layer 와 같은 단어 (`"배포"`, `"deploy"`) 를 정반대 의미로 다룸 (signal vs noise). 또한 한국어 조사 처리가 50개 손으로 고른 list 에 의존해서 `"checkout-v2를"` 같은 입력은 SLUG_RE (`^[a-z0-9][a-z0-9-]*$`) 매칭 fail 회귀.

**What:**
- `STOP_WORDS` HashSet 폐기
- `lindera-ko-dic` crate (~5MB Rust port) 도입해서 한국어 형태소 분석 → 조사 자동 제거
- 영문/외국어 stop word 처리는 명시적으로 분리 (관사/전치사 mini-list, language-tagged)
- slug NER 두 경로 (a) 따옴표 감싼 명시적 ID detection (b) `axhub apps` catalog fuzzy match (offline 캐시 + 5초 hook budget 안 짧은 fuzzy)

**Pros:**
- 같은 단어가 layer 마다 정반대 의미 갖는 drift 제거
- 일본어/중국어 사용자 silent fail (현재 SLUG_RE 미매칭 → None 반환) 부분 완화
- 한국어 조사 자동 처리 → 발화 변형 robust

**Cons:**
- `lindera-ko-dic` 추가 dependency. binary +5MB.
- `axhub apps` 카탈로그 fuzzy match 가 network 또는 캐시 필요 → offline-first 보장 약간 약화. 캐시 TTL 결정 필요.

**Context:** routing 큰 vision (Approach A — Hybrid embedding) 은 측정 phase 가 분기 결정. 이 항목은 그것과 *별개* 가치 — STOP_WORDS layer drift 자체로 즉시 가치.

**Effort:** human ~3d / CC+gstack ~1h
**Priority:** P2
**Depends on:** 없음 (이번 Skeptic fix PR 와 독립)
**Blocks:** Approach A (만약 채택 시 형태소 분석 prerequisite)

## P2 — Event Log byte-offset cursor

**Why:** `event_log.rs` tail 계열이 매번 전체 JSONL 파일을 읽어요. 배포가 잦은 환경에서 로그가 커지면 trace / recovery scan / verify 호출 비용이 O(N) 으로 커져요.

**What:**
- 마지막으로 읽은 byte offset 을 deploy 별 cursor 파일에 저장해요.
- cursor 손상이나 rotation 감지 시 full-read 로 복구해요.
- 7-day rotation 에 맞춰 cursor cleanup 정책도 같이 둬요.

**Pros:**
- 큰 event log 파일에서 read 비용을 O(델타) 로 낮춰요.
- 향후 dashboard / monitoring loop 에 필요한 기반이에요.
- OMC `runtime-v2.ts` cursor 패턴을 재사용할 수 있어요.

**Cons:**
- `{deploy_id}.jsonl.cursor` 같은 파일이 늘어요.
- cursor 손상과 로그 rotation edge case 테스트가 필요해요.
- 현재 평균 로그가 작으면 효과보다 복잡도가 커요.

**Context:** Phase 26 event_log 도입 시점에는 full-read 가성비가 더 좋아요. PR 25.6 doctor monitoring 의 100MB threshold 관측을 1개월 모은 뒤 실제 필요성을 판단해요.

**Effort:** human ~2d / CC+gstack ~30 min
**Priority:** P2
**Depends on:** phase-26 v2 PR 26.1b event_log merge + PR 25.6 monitoring 1개월 측정
**Blocks:** 없음

## P2 — Stage handoff `.omc/handoffs/<phase>.md`

**Why:** TodoWrite 는 user-facing 진행만 보여주고 machine-readable resume 지점은 남기지 않아요. deploy / verify / trace 같은 장시간 skill 이 interrupt 되면 정확한 stage resume 이 어려워요.

**What:**
- multi-step skill 의 stage 시작/완료를 `.omc/handoffs/{skill}-{stage}.md` 에 atomic 기록해요.
- deploy → verify → trace 처럼 skill 간 handoff 가 필요한 값만 최소 schema 로 남겨요.
- 손상된 handoff 는 fail-soft 로 무시하고 현재 preflight 로 재구성해요.

**Pros:**
- interrupt 후 resume 정확도가 올라가요.
- skill 간 hand-off 가 명시돼요.
- TodoWrite UI 와 machine-readable state 를 분리할 수 있어요.

**Cons:**
- `.omc/handoffs/` cleanup 정책이 필요해요.
- event_log 와 책임이 겹칠 수 있어요.
- skill 별 schema 유지보수 비용이 생겨요.

**Context:** event_log 는 deploy phase audit, handoff 는 skill step resume 에 가까워요. PR 25.1 recovery scan 과 PR 25.7 classify-exit chain 을 1개월 운영한 뒤 둘 다 필요한지 판단해요.

**Effort:** human ~5d / CC+gstack ~2h
**Priority:** P2
**Depends on:** phase-25 v2 PR 25.1 recovery_scan merge + PR 25.7 classify-exit chain 1개월 측정
**Blocks:** 없음

## P3 — Phase deliverables schema `templates/deliverables.json`

**Why:** `.plan/<phase>/91-test-strategy.md` 가 산출물 체크 역할을 하지만 free-form markdown 이라 PR 자동 검증에는 약해요.

**What:**
- phase 별 필수 산출물 schema 를 `templates/deliverables.json` 로 정의해요.
- CI 에서 변경 PR 의 SKILL / Rust module / test 산출물 충족 여부를 기계적으로 점검해요.
- 기존 markdown test strategy 와 중복되지 않게 최소 필드만 둬요.

**Pros:**
- PR 리뷰에서 누락 산출물을 빠르게 잡아요.
- Phase governance 를 자동화할 수 있어요.
- OMC deliverables schema 패턴을 가져올 수 있어요.

**Cons:**
- 기존 `91-test-strategy.md` 와 중복될 수 있어요.
- schema 관리 비용이 생겨요.
- schema 통과가 실제 품질 보장은 아니에요.

**Context:** axhub 는 이미 phase plan / decision log 문화가 강해요. schema 가 실제 리뷰 시간을 줄이는지 작은 phase 에서 먼저 측정해요.

**Effort:** human ~3d / CC+gstack ~1h
**Priority:** P3
**Depends on:** 없음
**Blocks:** 없음

## P2 — `docs/MIGRATION.md` 점진 마이그레이션 가이드

**Why:** 현재 `docs/migrate-rust.md` 는 Rust 포팅 단일 vector 중심이에요. CLI 0.x → 1.0 또는 env alias 제거처럼 사용자 영향이 큰 변경의 점진 migration / fallback 문서가 부족해요.

**What:**
- `docs/migrate-rust.md` 를 OMC `docs/MIGRATION.md` 스타일로 확장해요.
- runtime toggle, deprecation window, rollback, breaking change 섹션을 표준화해요.
- v0.8.0 `DISABLE_AXHUB` alias 제거 또는 v1.0 계획과 맞춰 갱신해요.

**Pros:**
- B2B 고객사 무중단 업그레이드 신뢰도가 올라가요.
- `AXHUB_HELPERS_RUNTIME=ts|rust|auto` 같은 fallback 문서 패턴을 재사용해요.
- 향후 schema_version bump 때 기준 문서가 생겨요.

**Cons:**
- 현재 사용자 베이스가 작으면 즉시 가치는 낮아요.
- 문서 유지보수 비용이 있어요.
- v1.0 일정이 없으면 내용이 추상적일 수 있어요.

**Context:** 첫 적용 후보는 v0.8.0 env alias 제거 또는 v1.0 release 계획 확정 시점이에요. 그 전에는 deferred 로 두는 편이 안전해요.

**Effort:** human ~3d / CC+gstack ~1h
**Priority:** P2
**Depends on:** v1.0 release 계획 확정 또는 v0.8.0 env alias 제거
**Blocks:** v1.0 release 게이트

## P3 — Rust porting quality checklist

**Why:** CLAUDE.md 의 global guideline 은 이미 있지만 Rust 포팅 맥락의 panic 금지, `Result<>` 경계, ownership/lifetime 단순화 같은 체크가 PR 템플릿에 고정돼 있지 않아요.

**What:**
- OMC `templates/rules/karpathy-guidelines.md` 의 Think before / Simplicity / Surgical / Goal-driven 원칙을 Rust 포팅 체크리스트로 축약해요.
- axhub Rust helper PR 템플릿에 panic / unwrap / boundary / regression 항목을 추가해요.
- 중복되는 일반 원칙은 링크만 남겨요.

**Pros:**
- Rust 포팅 quality drift 를 줄여요.
- 리뷰어가 놓치기 쉬운 failure boundary 를 반복 점검해요.
- 작은 체크리스트라 유지 비용이 낮아요.

**Cons:**
- CLAUDE.md 와 중복될 수 있어요.
- manual review 항목이 늘어요.
- 자동 검증이 아니면 강제력이 약해요.

**Context:** bus factor 가 낮은 Rust helper 영역에서 reviewer 보조 장치로 가치가 있어요. 다만 기능 gap 은 아니라 P3 로 유지해요.

**Effort:** human ~1d / CC+gstack ~30 min
**Priority:** P3
**Depends on:** 없음
**Blocks:** 없음

## P3 — `axhub reset` subcommand 검증용 follow-up

**Why:** phase-26 PR 26.2 가 derived view 를 채택하면 last-event-wins 로 stuck state 가 자동 해소돼요. 그래도 FSM 으로 되돌아가는 미래 선택지에서는 explicit reset 경로가 필요할 수 있어요.

**What:**
- `axhub reset --deploy-id=<id>` 의 필요성을 derived view 운영 데이터로 재평가해요.
- FSM 채택으로 방향이 바뀌면 `DeployPhase::Failed → Idle` reset entry point 와 audit event 를 설계해요.
- derived view 유지가 확정되면 이 TODO 를 close 해요.

**Pros:**
- FSM 선택지로 돌아갈 때 사용자 stuck 방지책이 준비돼요.
- recovery scan 의 stale incomplete deploy 정리 경로를 명시할 수 있어요.
- reset event audit trail 을 보존할 수 있어요.

**Cons:**
- derived view 유지 시 실효성이 낮아요.
- subcommand / classify-exit / skill 통합 overhead 가 있어요.
- `recover` 와 `reset` 의미 차이를 사용자에게 설명해야 해요.

**Context:** PR 26.2 spike 결과는 현재 derived view 쪽이라 즉시 구현보다 P3 follow-up 이 맞아요. FSM 으로 정책이 바뀌는 순간 P1 로 승격해요.

**Effort:** human ~2d / CC+gstack ~1h
**Priority:** P3
**Depends on:** phase-26 v2 PR 26.2 spike decision 유지 여부
**Blocks:** FSM 채택 시 phase-26 v2 PR 26.2 merge

## P3 — setup 온보딩에 statusline 자동활성 제안

**Why:** `setup` 스킬 온보딩 끝에 statusline 활성화를 제안하면 첫 사용자가 배포 상태를 항상 보게 돼요. 지금은 `enable-statusline` 을 따로 발견해야 해요.

**What:**
- `setup` 의 init 연결 단계 근처에 `enable-statusline` 호출 제안을 추가해요.
- 사용자가 거절하면 조용히 넘어가요 (온보딩 피로 방지).

**Pros:**
- 온보딩 완성도가 올라가요.
- 기존 `enable-statusline` skill 을 재사용해요.

**Cons:**
- 온보딩 단계가 늘어 첫 사용자 피로가 생길 수 있어요.
- statusline 미선호 사용자에겐 noise 예요.

**Context:** `/plan-ceo-review` 에서 SELECTIVE EXPANSION cherry-pick 후보로 올랐으나 핵심 경로가 아니라 deferred 했어요. setup baseline 이 merge 되고 검증된 뒤 추가해요.

**Effort:** human ~0.5d / CC+gstack ~15min
**Priority:** P3
**Depends on:** setup skill merge
**Blocks:** 없음

## P2 — setup 에 프로젝트 deps consent 설치 (dep-exec 게이트 정식 오픈)

**Why:** `node_modules` 없는 프로젝트에서 `setup` 이 deps 설치까지 해주면 "clone → 셋업 → 바로 dev" 가 한 번에 돼요. 지금은 사용자가 수동 `npm/bun install` 해야 해요.

**What:**
- `setup` 에 `package.json` 감지 + `node_modules` 부재 시 consent-gate `npm/bun install` 을 추가해요.
- `allows-dependency-execution: true` 선언 + `scripts/skill-doctor-allowlist.json` 등록 + rationale ≥50자 필요해요 (CI 게이트).
- pm 선택은 lockfile(`bun.lockb`/`pnpm-lock.yaml`/`package-lock.json`) 우선순위로 결정해요.

**Pros:**
- 첫 dev 환경 마찰을 제거해요.
- dep-exec 게이트를 정식 절차로 여는 첫 사례가 돼요.

**Cons:**
- dep-exec 게이트를 여는 일이라 보안 표면이 늘어요.
- lockfile 없을 때 pm 선택이 모호해요.
- silent 금지 — consent-gate 필수라 단계가 늘어요.

**Context:** dep-exec 는 `skill-doctor.ts` 가 의도적으로 gated. `/plan-ceo-review` cherry-pick 에서 가치는 인정됐으나 CI 작업이 따라와 별 PR 로 deferred 했어요. setup baseline 검증 + 실수요 확인 후 진행해요.

**Effort:** human ~1.5d / CC+gstack ~30min
**Priority:** P2
**Depends on:** setup skill merge
**Blocks:** 없음
