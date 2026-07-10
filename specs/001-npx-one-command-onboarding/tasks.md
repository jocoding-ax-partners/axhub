# Tasks: npx 원커맨드 온보딩

**Input**: [spec.md](./spec.md) · [plan.md](./plan.md) · [research.md](./research.md) · [data-model.md](./data-model.md) · [contracts/](./contracts/) · [quickstart.md](./quickstart.md)
**Date**: 2026-07-10

## 유저 스토리 매핑 (spec 수용 시나리오 → 스토리·우선순위)

| 스토리 | 우선순위 | 범위 | 시나리오 | 독립 테스트 기준 |
|---|---|---|---|---|
| US1 신규 원커맨드 셋업 (MVP) | P1 | 고지→4단계→핸드오프, claude 부재, user-scope, warn-vs-block | 1·4·6 | quickstart §1·§4·§8 green |
| US2 멱등·재개·dry-run | P2 | 재실행 skipped, 실패 재개, `--dry-run` 무변경 | 2·3 | quickstart §2·§3 green |
| US3 비대화형/AI 도구 실행 | P3 | hang 금지, `--yes`, 브라우저 억제, `--json` | 7 | quickstart §5 green (timeout 가드) |
| US4 설치 무결성 진단 | P4 | `setup doctor` 읽기 전용 5개 검사 | 8 | quickstart §6 green |
| US5 문서·온보딩 인수인계 | P5 | README 교체, footprint 공개, 온보딩 무중복 인계, 대표 여정 회귀 | 5 | quickstart §7 green + DP-5 게이트 |

**repo 경계**: `ax-hub-cli/...` 경로는 ax-hub-cli repo 루트 기준이에요(모듈 배치는 그쪽 구조에 맞춰 조정 가능, 계약은 contracts/ 가 기준). 그 외 경로는 이 plugin repo 기준이에요.

## Phase 1: Setup (프로젝트 준비)

- [ ] T001 npm org `axhub` 확보를 확인하고, 실패 시 fallback 네이밍(`axhub-cli-<platform>`) 확정을 specs/001-npx-one-command-onboarding/research.md R-3 에 기록해요
- [ ] T002 [P] `setup` 명령 그룹 뼈대(서브커맨드 등록·`--help` 표면)를 ax-hub-cli/src/commands/setup/mod.rs 에 생성해요
- [ ] T003 [P] npm 런처 패키지 뼈대(bin 선언, lifecycle script 0, `engines.node >=18`)를 ax-hub-cli/npm/axhub/package.json 과 ax-hub-cli/npm/axhub/bin/axhub.js 에 생성해요
- [ ] T004 [P] 플랫폼 바이너리 패키지 4종 뼈대(`os`/`cpu` 필드 포함)를 ax-hub-cli/npm/cli-darwin-arm64/ · cli-darwin-x64/ · cli-linux-x64/ · cli-win32-x64/ 에 생성해요
- [ ] T005 release CI 에 npm publish 스텝(런처+플랫폼 lockstep 버전, contracts/npm-launcher.md 계약)을 ax-hub-cli/.github/workflows/release.yml 에 추가해요

## Phase 2: Foundational (모든 스토리의 선행 차단 작업)

- [ ] T006 [P] data-model 타입(SetupStep·SetupRunMode·InstallationFootprint·HandoffCard·EnvironmentCheck·DoctorReport)을 ax-hub-cli/src/commands/setup/model.rs 에 구현해요
- [ ] T007 [P] `claude` CLI 감지 유틸과 `plugin-support onboarding-detect` 재사용 어댑터를 ax-hub-cli/src/commands/setup/detect.rs 에 구현해요
- [ ] T008 비대화형 판정 유틸(TTY 감지·`CI` env·`--yes`, data-model SetupRunMode 매핑)을 ax-hub-cli/src/commands/setup/mode.rs 에 구현해요

**체크포인트**: Phase 2 완료 전에는 어떤 유저 스토리도 시작하지 않아요.

## Phase 3: US1 — 신규 원커맨드 셋업 (P1 · MVP)

**목표**: 깨끗한 머신에서 `npx axhub@latest setup` 한 줄 → 고지 → 4단계 → 핸드오프 카드. **독립 테스트**: quickstart §1(신규)·§4(claude 부재)·§8(무권한) green.

- [ ] T009 [P] [US1] 계약 테스트(실패 우선): 실행 순서 7단계·exit code 3분류·claude 부재 경로를 contracts/setup-claude.md 기준으로 ax-hub-cli/tests/setup_claude_contract.rs 에 작성해요
- [ ] T010 [US1] self-install(npx 캐시 실행 감지 → `~/.axhub/bin` 복사 → repair-path 재사용 PATH 등록)을 ax-hub-cli/src/commands/setup/self_install.rs 에 구현해요
- [ ] T011 [US1] 실행 전 고지 출력(InstallationFootprint install/skip 표시, 설계 §4.3 항목 일치)을 ax-hub-cli/src/commands/setup/disclosure.rs 에 구현해요
- [ ] T012 [US1] 4단계 실행기(claude 감지 → marketplace add → plugin install → mcp add, contracts 의 정확한 명령 문자열)를 ax-hub-cli/src/commands/setup/steps.rs 에 구현해요
- [ ] T013 [US1] 환경 점검 warn-vs-block(EnvironmentCheck 등급, 감지값·요구 범위 명시 메시지, FR-015)을 ax-hub-cli/src/commands/setup/env_check.rs 에 구현해요
- [ ] T014 [US1] 핸드오프 카드(next_phrase "처음인데 셋업해줘"·restart_note·rerun_note 필수)를 ax-hub-cli/src/commands/setup/handoff.rs 에 구현해요
- [ ] T015 [US1] 사용자 노출 출력 규칙(해요체, raw stderr·내부 id 비노출, 평문 URL, 실패 시 원인+다음 행동 1-2문장, FR-011)을 ax-hub-cli/src/commands/setup/output.rs 에 구현해요
- [ ] T016 [P] [US1] npm 런처 동작(플랫폼 해석 → spawn passthrough → exit code·시그널 전파, 미지원 플랫폼·Node<18 안내)을 ax-hub-cli/npm/axhub/bin/axhub.js 에 구현해요
- [ ] T017 [US1] 로컬 e2e(`npm pack` 산출물로 quickstart §1·§4 수행)를 ax-hub-cli/scripts/e2e-setup-local.sh 에 작성하고 통과시켜요

**체크포인트**: US1 green 이면 MVP 배포 가능 상태예요.

## Phase 4: US2 — 멱등·재개·dry-run (P2)

**목표**: 재실행·부분 실패·dry-run 이 전부 안전해요. **독립 테스트**: quickstart §2(멱등)·§3(dry-run) green.

- [ ] T018 [P] [US2] 계약 테스트(실패 우선): 완료 단계 skipped 표시·실패 지점 재개·2회 연속 실행 무변경·`--dry-run` 무변경을 ax-hub-cli/tests/setup_idempotency.rs 에 작성해요
- [ ] T019 [US2] 단계별 기존재 감지 → `skipped` 전이(SetupStep 상태 전이표 준수)를 ax-hub-cli/src/commands/setup/steps.rs 에 확장해요
- [ ] T020 [US2] `--dry-run`(고지만 출력·무변경 종료·exit 0)을 ax-hub-cli/src/commands/setup/mod.rs 에 구현해요
- [ ] T021 [US2] 실패 지점 재개(앞 단계 done/skipped 확인 후 실패 단계부터, FR-003)를 ax-hub-cli/src/commands/setup/steps.rs 에 구현해요

## Phase 5: US3 — 비대화형/AI 도구 실행 (P3)

**목표**: 입력 불가 환경에서 절대 멈추지 않아요. **독립 테스트**: quickstart §5 green (timeout 가드 통과).

- [ ] T022 [P] [US3] 계약 테스트(실패 우선): stdin 닫힘+`CI=1` 에서 timeout 가드 내 종료·`--yes` 강제·브라우저 열기 0회를 ax-hub-cli/tests/setup_noninteractive.rs 에 작성해요
- [ ] T023 [US3] 비대화형 모드 전면 적용(모든 경로에서 입력 대기·브라우저 오픈 금지 감사, FR-012)을 ax-hub-cli/src/commands/setup/mode.rs 통합으로 완성해요
- [ ] T024 [US3] `--json` 출력(SetupStep·Footprint·HandoffCard 직렬화, 스키마 안정성 테스트 포함)을 ax-hub-cli/src/commands/setup/json.rs 에 구현해요

## Phase 6: US4 — 설치 무결성 진단 (P4)

**목표**: `axhub setup doctor` 가 5개 항목을 읽기 전용으로 검증하고 해결을 안내해요. **독립 테스트**: quickstart §6 green.

- [ ] T025 [P] [US4] 계약 테스트(실패 우선): 5개 검사 ok/problem 조합·읽기 전용(실행 전후 diff 0)·exit 0/1 매핑을 contracts/setup-doctor.md 기준으로 ax-hub-cli/tests/setup_doctor.rs 에 작성해요
- [ ] T026 [US4] doctor 구현(detect.rs 재사용, 항목별 fix 한 줄 안내, deploy 검사 비포함)을 ax-hub-cli/src/commands/setup/doctor.rs 에 구현해요

## Phase 7: US5 — 문서·온보딩 인수인계 (P5 · 이 repo, WS-B)

**목표**: 문서가 npx 경로로 바뀌고, setup 완료 상태를 온보딩이 무중복으로 이어받아요. **독립 테스트**: quickstart §7 green + `bun test`·대표 여정 회귀 green. **머지 게이트**: T027 은 npm 패키지가 실제 설치 가능해진 뒤(T017·T032 완료) 머지해요 — 깨진 빠른 시작 노출 금지.

- [ ] T027 [P] [US5] README 빠른 시작을 `/plugin` 2줄에서 `npx axhub@latest setup` 1줄로 교체하고 AP-14 invariant 문구 보존을 확인해요 — README.md
- [ ] T028 [P] [US5] 설치 산출물·미설치 항목·제거 방법(FR-007, 설계 §4.3 미러)을 사용자 문서에 공개해요 — POLICY.md
- [ ] T029 [US5] onboarding 스킬에 npx 설치 채널을 반영(detect-first 루프 불변, install 채널 안내에 npx 추가)해요 — skills/onboarding/references/install-channels-and-auth.md
- [ ] T030 [US5] 대표 여정 회귀·fixture 를 "npx 한 줄 → Claude 열기 → 셋업해줘 → Ready" 로 갱신(DP-5)해요 — tests/ 의 대표 여정 fixture 파일
- [ ] T031 [US5] 인수인계 검증: setup 완료 머신에서 "처음인데 셋업해줘" 가 로그인부터 시작하고 재시작 marker 가 안 뜨는지 quickstart §7 로 확인해 결과를 specs/001-npx-one-command-onboarding/quickstart.md 판정 섹션에 기록해요

## Final Phase: Polish & 교차 관심사

- [ ] T032 [P] 플랫폼 매트릭스 스모크(`npx axhub --version` — macOS arm64/x64·Windows x64·Linux x64)를 ax-hub-cli CI job 으로 추가해요 — ax-hub-cli/.github/workflows/
- [ ] T033 quickstart §1~§8 전체를 실행해 SC-001~007 판정을 specs/001-npx-one-command-onboarding/quickstart.md 에 기록해요
- [ ] T034 [P] 고지 목록 ↔ 설계 §4.3 ↔ POLICY.md 공개본의 항목 일치(parity)를 최종 확인하고 어긋남을 고쳐요 — docs/superpowers/specs/2026-07-10-npx-one-command-onboarding-design.md · POLICY.md

## Dependencies (스토리 완료 순서)

```
Phase 1 Setup ─→ Phase 2 Foundational ─→ US1 (MVP)
                                          ├─→ US2 ─┐
                                          ├─→ US3 ─┼─→ Polish (T032~T034)
                                          ├─→ US4 ─┘
                                          └─→ US5 (T027 머지는 T017·T032 이후)
```

- US2·US3·US4 는 US1 완료 후 서로 독립이라 병렬 진행 가능해요 (서로 다른 파일·테스트).
- US4 는 엄밀히는 Phase 2(T007) 이후 시작 가능하지만, 우선순위대로 US1 뒤에 배치했어요.
- T005(publish CI)는 T003·T004 이후, T032 는 T005 이후예요.

## Parallel 실행 예시

- **Phase 1**: T002 ∥ T003 ∥ T004 (서로 다른 디렉토리) → T005
- **Phase 2**: T006 ∥ T007 동시 → T008
- **US1**: T009(테스트 먼저) ∥ T016(런처) 동시 시작 → T010→T011→T012→T013→T014→T015 → T017
- **US1 완료 후**: US2(T018~) ∥ US3(T022~) ∥ US4(T025~) 세 스토리 병렬
- **US5**: T027 ∥ T028 (다른 파일) → T029 → T030 → T031

## Implementation Strategy

- **MVP 우선**: Phase 1~3(US1)까지가 배포 가능한 최소 단위예요 — 신규 사용자가 한 줄로 셋업을 끝내는 핵심 가치.
- **증분 배포**: US2(안전 재실행) → US3(에이전트 실행) → US4(doctor) 순서로 릴리즈마다 하나씩 얹어요. US5(문서 교체)는 npm 설치 가능 시점 이후에만 머지해요.
- **TDD**: 각 스토리는 계약 테스트(T009·T018·T022·T025)를 먼저 실패 상태로 작성한 뒤 구현으로 통과시켜요 — contracts/ 문서가 테스트의 단일 기준이에요.
