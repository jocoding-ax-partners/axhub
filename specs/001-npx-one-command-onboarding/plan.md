# Implementation Plan: npx 원커맨드 온보딩

**Branch**: `main` (브랜치 훅 없음 — 구현 시 워크스트림별 브랜치 권장)
**Spec**: [spec.md](./spec.md)
**Design**: [docs/superpowers/specs/2026-07-10-npx-one-command-onboarding-design.md](../../docs/superpowers/specs/2026-07-10-npx-one-command-onboarding-design.md)
**Date**: 2026-07-10

## Summary

`npx axhub@latest setup` 한 줄이 비대화형 셋업 전부(CLI 영구 설치 → 마켓플레이스 등록 → 플러그인 설치 → MCP 등록)를 멱등하게 수행하고 핸드오프 카드로 Claude 안 onboarding 에 연결해요. 기술 접근: npm 은 lifecycle-script 없는 얇은 런처 + `optionalDependencies` 플랫폼 바이너리, 오케스트레이션은 ax-hub-cli 의 새 공개 명령 `axhub setup claude`(+ `setup doctor`)가 소유해요. 작업은 두 워크스트림으로 나뉘어요 — **WS-A**(ax-hub-cli repo: 명령·npm 패키지·publish CI)와 **WS-B**(이 plugin repo: README·대표 여정 회귀·정책 parity).

## Technical Context

| 항목 | WS-A (ax-hub-cli) | WS-B (이 repo) |
|---|---|---|
| Language | Rust (기존 CLI 스택) + npm 런처(JS, 의존성 0 지향) | Markdown(SKILL/README) + TypeScript(bun 테스트) |
| 신규 표면 | 공개 명령 `axhub setup claude`, `axhub setup doctor`; npm `axhub` + `@axhub/cli-<platform>` | 없음 (스킬 8개 유지, DP-1) |
| 재사용 | `plugin-support onboarding-detect`(판정), `repair-path`(PATH), 기존 release CI | onboarding 스킬 detect-first 루프(변경 최소) |
| Testing | cargo 계약 테스트(비대화형·dry-run·doctor·멱등), npm 런처 플랫폼 스모크 | `bun test`(policy parity·frontmatter·bundle), lint:tone, 대표 여정 회귀 |
| Platform | macOS(arm64/x64)·Linux(x64)·Windows(x64), user-scope 전용 | 동일 |
| 제약 | Node 18+(npx 전제), admin 권한 금지, auth 무접촉, hang 금지 | AP-14 README invariant 보존 |
| 성능 목표 | 실행→핸드오프 5분 이내(SC-002) | — |
| NEEDS CLARIFICATION | 없음 — [research.md](./research.md) 로 전부 해소. 실행 전 액션 1건: npm org `axhub` 확보 확인(R-3, fallback 결정 완료) | 없음 |

## Constitution Check

`.specify/memory/constitution.md` 가 없어서 이 repo 의 기준 문서(`docs/policy/agent-policy.md`, `docs/policy/dev-policy.md`)로 게이트를 평가했어요.

| 게이트 | 판정 | 근거 |
|---|---|---|
| DP-1 diet(스킬 8개·로직은 CLI) | PASS | 새 스킬 0개. setup/doctor 로직은 ax-hub-cli 소유(R-4) |
| DP-2 해요체 tone | PASS | 신규 사용자 노출 문구(고지·카드·doctor 출력) 전부 해요체 — WS-A 구현 계약에 명시 |
| DP-5 quality gates | PASS(조건) | README 빠른 시작 교체 시 대표 여정 회귀·fixture 를 같은 커밋 계열에서 갱신해야 해요 — Phase 2 태스크로 강제 |
| DP-6 hidden 표면 | PASS | doctor 는 공개 명령이고 `onboarding-detect` 재사용은 CLI 내부 구현이에요. plugin/skill 밖에서 hidden 표면을 새로 노출하지 않아요 |
| DP-7 bundle 규칙 | PASS | `specs/`·`docs/` 는 ROOT_DIRS 밖이라 배포 번들에 안 들어가요 |
| AP-6 preflight 게이트 | PASS | bootstrap·deploy 게이트 불변. setup 은 새 CLI 버전 기능이라 구 CLI 에는 명령 자체가 없어요 |
| AP-8 자동 bootstrap 금지 | PASS | 핸드오프는 "처음인데 셋업해줘" 안내만 — 앱 생성을 실행하지 않아요 |
| AP-10 telemetry 옵트인 | PASS | setup 무수집(R-9). axrouter 옵트인 흐름 불변 |
| AP-13 Windows Git Bash | PASS | AP-13 개정(2026-07-10)으로 npx setup 예외가 명문화됐어요 — setup 은 auth 무접촉·PATH 비의존, setup 이후 로그인·auth 검증부터 Git Bash 계약. 해석이 아니라 정책 문언으로 충족해요 |
| AP-14 README invariant | PASS(조건) | WS-B README 수정 시 invariant 문구(`AXHUB_NO_UPDATE_ROUTER` 등) 보존 — parity 테스트가 감시해요 |

**초기 평가**: 위반 0 → Phase 0 진행. **Post-design 재평가**: Phase 1 산출물(contracts·data-model)이 위 판정을 바꾸지 않아요 — 위반 0 유지.

## Project Structure

### 이 피처의 문서 (specs/001-npx-one-command-onboarding/)

```
spec.md              # 기능 명세 (완료)
plan.md              # 이 파일
research.md          # Phase 0 — 결정 12건 (완료)
data-model.md        # Phase 1 — 엔티티·상태 전이 (완료)
quickstart.md        # Phase 1 — 검증 워크스루 (완료)
contracts/
  setup-claude.md    # `axhub setup claude` 명령 계약
  setup-doctor.md    # `axhub setup doctor` 명령 계약
  npm-launcher.md    # npm 패키지(런처·플랫폼 바이너리) 계약
checklists/
  requirements.md    # 스펙 품질 체크리스트 (통과)
```

### 소스 코드 (repo 별)

```
# WS-A: ax-hub-cli repo (이 plugin repo 밖 — follow-up)
src/commands/setup/         # setup claude / setup doctor
npm/axhub/                  # 런처 패키지 (bin + 플랫폼 해석, lifecycle script 없음)
npm/cli-<platform>/         # optionalDependencies 바이너리 패키지들
.github/workflows/release   # 기존 release CI 에 npm publish 스텝 추가

# WS-B: 이 repo
README.md                   # 빠른 시작 교체 (/plugin 2줄 → npx 1줄)
skills/onboarding/          # detect-first 는 그대로 — npx 경로 언급만 최소 반영
tests/                      # 대표 여정 회귀·fixture 갱신
```

## Phase 0: Research (완료)

[research.md](./research.md) — 결정 12건(R-1~R-12): 수단(npx)·이름(`axhub` 선점 확인)·바이너리 배포(optionalDependencies, lifecycle script 금지)·로직 위치(CLI)·비대화형 계약·고지+dry-run·doctor·환경 점검 등급·telemetry 무수집·MCP 등록 명령·재시작 의미·repo 경계. NEEDS CLARIFICATION 0건.

## Phase 1: Design & Contracts (완료)

- [data-model.md](./data-model.md) — SetupStep(4단계 상태 전이)·SetupRunMode·InstallationFootprint·HandoffCard·DoctorReport.
- [contracts/](./contracts/) — CLI 명령 2건 + npm 런처 계약. 각 계약은 스펙 FR 을 참조해요.
- [quickstart.md](./quickstart.md) — 수용 시나리오 8건을 실행 가능한 검증 절차로 매핑.
- Agent context — `CLAUDE.md` 에 SPECKIT 마커 블록으로 이 plan 경로를 기록했어요.

## Phase 2: Task Generation Approach (/speckit-tasks 가 수행)

- **그룹**: WS-A(CLI 명령 → npm 패키지 → publish CI 순), WS-B(README → 회귀·fixture), 마지막에 quickstart 기반 e2e 확인 태스크.
- **순서 원칙**: 계약 테스트 먼저(TDD) — contracts/ 의 각 계약마다 실패 테스트 → 구현 → 통과. WS-A 의 npm org 확보 확인(R-3)이 최우선 선행 태스크예요.
- **의존**: WS-B 의 README 교체는 WS-A 의 npm 패키지가 실제 설치 가능해진 뒤 머지해요(문서가 먼저 나가면 깨진 빠른 시작이 노출돼요). 대표 여정 회귀(DP-5)는 README 교체와 같은 계열로 묶어요.
- **완료 기준**: quickstart.md 의 검증 절차 전부 green + 스펙 SC-001~007 확인.

## Complexity Tracking

위반·예외 없음 — 표 생략해요.

## Progress Tracking

- [x] Phase 0: research.md 생성, NEEDS CLARIFICATION 0건
- [x] Phase 1: data-model.md · contracts/ 3건 · quickstart.md · agent context 갱신
- [x] Constitution Check: 초기 PASS · post-design 재평가 PASS (위반 0)
- [ ] Phase 2: /speckit-tasks 로 tasks.md 생성 (이 명령 범위 밖)
