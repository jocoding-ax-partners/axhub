# Research: npx 원커맨드 온보딩

**Date**: 2026-07-10
**Input**: 브레인스토밍 결정(설계 문서 §2) + 웹 리서치(실제 npx 설치 제품 비교, 주장별 3표 적대 검증)

모든 NEEDS CLARIFICATION 을 아래 결정으로 해소했어요. 유일한 실행 전 확인 항목은 R-3 의 npm org 확보예요(결정은 fallback 까지 완료).

## R-1. 배포 수단

- **Decision**: npx 를 메인 진입점으로 해요 — `npx axhub@latest setup`.
- **Rationale**: 한 줄 암기 가능한 크로스 플랫폼 문법, npm 인프라 재사용. 사용자(제품 오너)가 브레인스토밍에서 확정했어요.
- **Alternatives considered**: (A) `curl | sh` install script — 전제 0개지만 미채택, Node 부재가 실측 문제가 되면 재검토(설계 §8). (C) A+npx 병행 — 메시지 분산으로 미채택.

## R-2. npm 패키지 이름

- **Decision**: `axhub` 를 그대로 써요.
- **Rationale**: 팀(Jocoding AX Partners, `jocodingax`)이 2026-04-20 에 이미 선점했어요 — npm registry 에서 maintainer 확인 완료.
- **Alternatives considered**: `create-axhub`, `@axhub/cli` — 둘 다 미등록이라 가능하지만 본명 `axhub` 확보로 불필요.

## R-3. 바이너리 배포 방식

- **Decision**: `optionalDependencies` 플랫폼 패키지(`@axhub/cli-darwin-arm64` 등)에 Rust 바이너리를 직접 담아요 — esbuild/biome 패턴. lifecycle script(postinstall 등)는 금지예요.
- **Rationale**: `--ignore-scripts` 환경(자체 onboarding 안전 규칙이기도 함)에서 동작하고, 바이너리가 npm 레지스트리(사내 미러 포함)로 전달돼요. npx 실행이 코드 실행이라는 공급망 리스크 표면(postinstall)이 줄어요.
- **Alternatives considered**: postinstall 이 GitHub releases 에서 다운로드 — `--ignore-scripts` 에서 깨지고 사내 프록시가 GitHub 를 막을 수 있어 기각. 전 플랫폼 바이너리를 한 패키지에 — 다운로드 크기 문제로 기각.
- **Open action (pre-implementation)**: npm org `axhub` 확보 확인. 실패 시 무스코프 `axhub-cli-<platform>` 네이밍으로 대체해요(설계 §6).

## R-4. 오케스트레이션 로직 위치

- **Decision**: ax-hub-cli 의 새 공개 명령 `axhub setup claude` 가 전부 소유하고, npm 패키지는 바이너리 해석·실행만 하는 얇은 런처예요.
- **Rationale**: diet 철학(DP-1: 판정·실행 로직은 plugin 이 아니라 ax-hub-cli) 정합. JS 로 로직을 두면 감지·안내가 이중 구현돼요.
- **Alternatives considered**: JS 런처가 오케스트레이션 — 기각(이중 구현·테스트 분산).

## R-5. 비대화형 계약

- **Decision**: 터미널 입력 불가·CI·`--yes` 에서 입력 대기 금지·브라우저 열기 금지. hang 은 결함으로 규정해요(스펙 FR-012).
- **Rationale**: create-next-app 이 AI 코딩 도구(Claude Code·Cursor) 대행 실행에서 stdin 대기로 hang 하는 선례(vercel/next.js discussion #91169). axhub 는 에이전트가 명령을 대신 실행하는 제품이라 1급 시나리오예요. clig.dev 규범(TTY 에서만 프롬프트, 모든 프롬프트에 플래그 대안)과 정합.
- **Alternatives considered**: TTY 감지에만 의존 — Storybook `--ci`(프롬프트 skip + 브라우저 억제 결합) 사례를 따라 명시 플래그를 함께 둬요.

## R-6. 실행 전 고지와 dry-run

- **Decision**: 실제 변경 전에 설치·등록·건너뜀 목록을 항상 출력하고(확인 프롬프트 없이 진행), `--dry-run` 은 목록만 출력 후 무변경 종료해요(스펙 FR-013).
- **Rationale**: npx 실행은 코드 실행이에요 — 설치 표면 고지는 신뢰 장치의 핵심(GitHub/OWASP 의 npm 공급망 근거). Homebrew 의 고지-후-진행 패턴이 "한 줄 완주" 목표와 양립해요.
- **Alternatives considered**: 고지 + 확인 프롬프트(Enter) — 한 줄 완주 목표·비대화형 계약과 충돌해 기각. 문서에만 공개 — 실행 시점 고지가 업계 관행이라 보강.

## R-7. 설치 무결성 진단 (doctor)

- **Decision**: `axhub setup doctor` 공개 명령 — 네 산출물(CLI·마켓플레이스·플러그인·MCP)을 읽기 전용으로 검증하고 해결 안내를 출력해요(스펙 FR-014). v1 은 자동 수정 없음.
- **Rationale**: Storybook `doctor`·React Native `doctor` 의 확립된 패턴("자동 수정 제안 + 실패 시 안내 fallback") 중 안전한 절반 채택. 판정 로직은 기존 `plugin-support onboarding-detect` 를 재사용해요.
- **Alternatives considered**: diagnosis 스킬 확장 — 기각(배포 실패 전용 역할과 혼합, AP-4). 자동 수정 포함 — 후속으로 미뤄요.

## R-8. 환경 점검 등급

- **Decision**: 진짜 최소 요건(Node 18) 미만만 중단, 그 외 애매한 상태는 감지값·요구 범위를 명시한 경고 후 진행해요(스펙 FR-015).
- **Rationale**: Angular CLI 의 warn-vs-block 패턴(commit 0883248) — 과잉 차단은 이탈, 무경고는 미스터리 실패를 만들어요.
- **Alternatives considered**: 전부 하드 블록 — 기각. Node 최소를 20 으로 — npx·optionalDependencies 동작에 18 이면 충분해 기각.

## R-9. telemetry

- **Decision**: setup 은 사용 데이터를 보내지 않아요. 도입하게 되면 Homebrew 패턴(첫 실행 고지 전 무전송 + opt-out)을 따라요. AI 활용 기록(axrouter)은 AP-10(명시 opt-in) 그대로예요.
- **Rationale**: create-next-app 의 무고지 수집 반발(vercel/next.js issues #59686/#59688)이 반면교사예요.

## R-10. MCP 등록 방식

- **Decision**: `claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp` — onboarding 스킬(`references/mcp-ready-card.md`)과 동일 명령·동일 scope 예요.
- **Rationale**: 채널 간 결과 상태 수렴(스펙 FR-010). 첫 세션 전 등록이라 세션 중 등록용 restart marker 는 쓰지 않아요 — 신규 사용자 재시작이 1회 줄어요(SC-004).

## R-11. Claude Code 재시작 의미

- **Decision**: 플러그인·MCP 는 다음 Claude Code 실행(세션 시작)에 로드돼요 — setup 은 "열기 또는 재시작"을 핸드오프 카드에 항상 포함해요.
- **Rationale**: `claude plugin install` 은 등록이고 로드는 세션 시작 시점이에요. 기존 auto-update 훅 문서("재시작해야 반영")와 동일한 사실관계예요.

## R-12. repo 경계와 배포 파이프라인

- **Decision**: `setup claude`·`setup doctor` 구현, npm 패키지(런처+플랫폼), npm publish CI 는 전부 ax-hub-cli repo 소관이에요(WS-A). 이 plugin repo(WS-B)는 README 빠른 시작 교체, 대표 여정 회귀·fixture 갱신, 정책 parity 유지만 담당해요.
- **Rationale**: 이 repo 의 명시 원칙 — "실제 ax-hub-cli 구현/schema parity/release 는 이 plugin repo 범위 밖 follow-up". npm 버전은 CLI 바이너리와 lockstep 이어야 해서 publish 는 CLI release CI 에 붙어요.
