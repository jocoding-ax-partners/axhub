# Changelog

All notable changes to the axhub Claude Code plugin will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows [Semantic Versioning](https://semver.org/).


## [0.9.30](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.28...v0.9.30) (2026-06-04)

이번 릴리스는 플러그인 SKILL 을 실제 axhub CLI QA 결과에 정합시켜요. 첫 배포(vibe bootstrap) 흐름에서 매니페스트 `name` 을 유효 슬러그로 변환해 배포 차단을 풀고, 앱 생성을 `--from-file`(YAML) 대신 `--name/--slug` 로 바꿨어요. quality-gate 의 첫 배포 `ExitCodeMismatch` false-positive 를 제거하고, 로그인 상태 질문은 doctor 가 아닌 auth 로 라우팅해요.

### Fixed

* align plugin skills with live axhub CLI QA ([#166](https://github.com/jocoding-ax-partners/axhub/issues/166)) ([bc250f9](https://github.com/jocoding-ax-partners/axhub/commit/bc250f9e93a34b8b885c6c6c738c43a1aca1a7df))


### Docs

* SKILL 슬래시 명령 전부 표기 (모든 skill 은 /axhub:<이름> 으로 호출) ([d273b44](https://github.com/jocoding-ax-partners/axhub/commit/d273b4422419f26165ecec5fc2eb9097c0fc010f))

## [0.9.29](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.28...v0.9.29) (2026-06-03)


### Docs

* README 를 spec-kit 스타일 사용자 문서로 재작성 ([2692643](https://github.com/jocoding-ax-partners/axhub/commit/2692643cecc53e0fb908f48311f2105287f25bd1))
* README 홈페이지 URL 을 axhub.ai 로 수정 ([2df83f3](https://github.com/jocoding-ax-partners/axhub/commit/2df83f38f89225f2e4956fef0705daf0a0fce1b9))
* SKILL 42개 전체 최신화 + 바이브코더용 친절 설명으로 재작성 ([1e73955](https://github.com/jocoding-ax-partners/axhub/commit/1e739555050d55e2c76b333693d616192867160d))
* SKILL 슬래시 명령 전부 표기 (모든 skill 은 /axhub:<이름> 으로 호출) ([d273b44](https://github.com/jocoding-ax-partners/axhub/commit/d273b4422419f26165ecec5fc2eb9097c0fc010f))

## [0.9.28](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.27...v0.9.28) (2026-06-02)

이번 릴리스는 `specs/007-vibe-skill-gapfill` 의 CLI 소스 감사 결과를 실제 스킬·동의 게이트에 반영해요. current axhub CLI 표면을 기준으로 10개 gap-fill SKILL 을 추가하고, 기존 init/migrate/status/deploy/auth 흐름의 오래된 명령을 정리했어요. 특히 `apps detect` 는 최신 main CLI 에 없는 branch-only/future 경로로 분리하고, destructive payload 는 파일 digest·구체 profile·tenant/app context 로 묶어 preview 뒤 바꿔치기를 막아요.

### Test baseline

- PR #165 CI 가 rust ubuntu/macos/windows, perf ubuntu/macos/windows, hook integration, T2 helper-bin, corpus.100 drift gate 모두 pass 했어요.
- 로컬에서 `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bunx tsc --noEmit`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cargo test -p axhub-helpers`, `bun test`, `git diff --check` 를 통과했어요.
- 코드 리뷰어 `FINAL_RECOMMENDATION: APPROVE`, 아키텍트 `architectStatus: CLEAR`, CE adversarial reviewer `CE_FINAL_VERDICT: PASS` 로 최종 리뷰 게이트를 통과했어요.

### Honest tradeoff

- live staging mutation E2E 는 `AXHUB_E2E_STAGING_TOKEN` 이 없어 실행하지 않았고, CI 의 read-only staging/fuzz/advisory 일부 job 은 의도된 조건부 skip 이에요.
- GitNexus impact 는 consent/preauth/deploy/bootstrap 경로 변경 때문에 HIGH/CRITICAL 로 나왔지만, scope 는 이번 릴리스의 fail-closed parser hardening 범위와 일치해요.


### Added

* align skills with audited CLI surface ([fedb7bd](https://github.com/jocoding-ax-partners/axhub/commit/fedb7bd752a10da86e8a7440e50f17ae488f76d6)), closes [#165](https://github.com/jocoding-ax-partners/axhub/issues/165)

## [0.9.27](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.26...v0.9.27) (2026-06-02)

이번 릴리스는 ax-hub-cli 설치 도메인이 `cli.jocodingax.ai` 에서 `cli.axhub.ai` 로 바뀐 걸 반영해요. install-cli SKILL 의 installer 명령·AskUserQuestion 옵션·NEVER 절, 한국어 트러블슈팅 문서, ask-defaults registry rationale 까지 옛 주소가 남아 있던 12 곳을 한 번에 갱신해요. `docs.jocodingax.ai` 와 `axhub-api.jocodingax.ai` 서브도메인은 이번 변경 범위가 아니라 그대로 둬요.

### Test baseline

- `bun run lint:keywords --check` baseline no diff, `bun run skill:doctor --strict` OK, `bun test tests/ux-ask-fallback-registry.test.ts` 40 pass / 0 fail 를 확인했어요.
- PR #164 를 admin merge 로 main 에 반영하고 release step 1 의 `codegen:version` 과 `release:check` (host helper build) 가 통과했어요.

### Honest tradeoff

- 사용자 요청으로 CI 를 기다리지 않고 admin merge 했어요. 전체 5-platform matrix 와 staging E2E 는 tag push 뒤 release workflow 에서 다시 확인해요.
- 새 도메인 `cli.axhub.ai` 의 `install.sh` / `install.ps1` 실제 서빙 여부는 인프라 측 확인이 필요해요. 문서/스킬만 갱신한 상태예요.


### Fixed

* **skills:** install-cli CLI 도메인을 cli.axhub.ai 로 갱신 ([#164](https://github.com/jocoding-ax-partners/axhub/issues/164)) ([1b73a72](https://github.com/jocoding-ax-partners/axhub/commit/1b73a72a8e0e700d7a47804891c4a28acc5336f9))

## [0.9.26](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.25...v0.9.26) (2026-06-02)

이번 릴리스는 남아 있던 DIRTY PR 을 최신 main 위에서 정리해, setup Windows PowerShell, update/verify CLI 계약, exit-code 라우팅, marker-gated routing 을 한 번에 실어요. axhub 프로젝트 marker 가 없는 일반 repo 에서는 eager footprint 를 줄이고, 명시적 axhub 호출과 deploy preflight 는 공유 route-decision 계약으로 안전하게 이어져요.

### Test baseline

- PR #155, #159, #161, #163 의 conflict 를 isolated worktree 에서 정상 merge commit 으로 풀고 각 PR CI 를 green 으로 확인했어요.
- main merge 뒤 `git diff --check v0.9.25..HEAD`, `bun run skill:doctor`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, routing/migrate/manifest targeted `bun test`, `cargo fmt --all -- --check`, `cargo test -p axhub-helpers --test classify_exit_suggest_test -- --nocapture` 를 통과했어요.
- release step 1 의 `codegen:version` 과 `release:check` 가 통과했어요.

### Honest tradeoff

- staging E2E, parser fuzz, full matrix 일부는 repo 조건상 skipped 될 수 있어서 tag push 뒤 release workflow 에서 artifact 와 skipped job 을 다시 확인해요.
- GitNexus 는 isolated worktree 경로를 별도 repo 로 인식하지 못해서 conflict 해결 scope 는 git diff, 로컬 targeted test, PR CI 로 검증했어요.


### Added

* **routing:** marker 게이트 + 공유 결정 함수로 axhub 라우팅 decouple ([#159](https://github.com/jocoding-ax-partners/axhub/issues/159)) ([b8bc3be](https://github.com/jocoding-ax-partners/axhub/commit/b8bc3bef5c703d7d6f8d1f17ec6ff0f304bd9818))
* **skills:** setup Windows PowerShell 지원 + manifest axhub.yaml canonical 전환 ([#161](https://github.com/jocoding-ax-partners/axhub/issues/161)) ([d523865](https://github.com/jocoding-ax-partners/axhub/commit/d5238652ee939b47f9a68bc5e7d0f73e341f33a6))


### Fixed

* **routing:** skill exit-code 라우팅을 ax-hub-cli 0.17.2 계약에 정합 (spec 004) ([#163](https://github.com/jocoding-ax-partners/axhub/issues/163)) ([9e5cc2c](https://github.com/jocoding-ax-partners/axhub/commit/9e5cc2c047013a7e536997b11745e0ca3c09fbcf))
* **skills:** CLI 정렬(update/verify) + classify/cosign + whatsnew 제거 + sessionstart 온보딩 [skip-routing-gate] ([#155](https://github.com/jocoding-ax-partners/axhub/issues/155)) ([d3e4c1a](https://github.com/jocoding-ax-partners/axhub/commit/d3e4c1a1a088799272339f7456f92a0770fa4fa6)), closes [#1](https://github.com/jocoding-ax-partners/axhub/issues/1) [#2-4](https://github.com/jocoding-ax-partners/axhub/issues/2-4)

## [0.9.25](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.24...v0.9.25) (2026-06-02)

v0.9.24 이후 사용자가 요청한 열린 PR 일괄 처리 범위에서 준비 완료된 trace·upgrade 변경을 추가로 실어요. trace 스킬은 build-log API 가 없어도 runtime log evidence 로 원인을 설명하고, upgrade 스킬은 Windows PowerShell 사용자가 Bash 없이도 업데이트 절차를 따라갈 수 있게 해요.

### Test baseline

- PR #162 CI 를 다시 확인했고 11 pass / 6 skipped / 0 fail / 0 pending 이었어요.
- PR #160 CI 를 다시 확인했고 5 pass / 2 skipped / 0 fail / 0 pending 이었어요.
- main merge 뒤 `git diff --check v0.9.24..HEAD`, `bun test tests/trace-skill.test.ts tests/migrate-skill-contract.test.ts`, `cargo test -p axhub-helpers trace -- --nocapture`, `bun run skill:doctor`, `bun run lint:tone --strict`, `bun run lint:keywords --check` 를 통과했어요.
- release step 1 의 `codegen:version` 과 `release:check` 가 통과했어요.

### Honest tradeoff

- #155, #159, #161, #163 은 main 과 conflict 상태라 강제 머지하지 않았어요.
- release workflow 의 staging E2E 와 vibe advisory job 은 repo 조건상 skipped 될 수 있어서 최종 workflow 결과에서 별도 확인해요.


### Added

* **skills:** add Windows compatibility to upgrade skill ([880c8d5](https://github.com/jocoding-ax-partners/axhub/commit/880c8d5a92fdbe6229e33c7a0a2ac014359a082a)), closes [#160](https://github.com/jocoding-ax-partners/axhub/issues/160)


### Fixed

* **trace:** use runtime logs for evidence source ([eeca0e8](https://github.com/jocoding-ax-partners/axhub/commit/eeca0e88cc5b2f53b3c23f68ca727b154c39f1a4)), closes [#162](https://github.com/jocoding-ax-partners/axhub/issues/162)

## [0.9.24](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.23...v0.9.24) (2026-06-02)

기존 앱 가져오기 흐름을 정식 릴리스에 올려서 에이전트가 로컬 감지, `axhub.yaml` 제어, GitHub 연결, 배포까지 CLI 경계 안에서 이어가게 해요. 선택형 Auth Migration 절차도 문서화해 backend/gateway 변경 없이 사용자 앱 로그인 전환 계획을 먼저 보여주도록 했어요.

### Test baseline

- PR #154 CI 를 다시 확인했고 11 pass / 6 skipped / 0 fail / 0 pending 이었어요.
- main merge 뒤 `git diff --check HEAD^ HEAD`, `bun test tests/migrate-skill-contract.test.ts`, `bun run skill:doctor`, `bun run lint:tone --strict`, `bun run lint:keywords --check` 를 통과했어요.
- release step 1 의 `codegen:version` 과 `release:check` 가 통과했어요.

### Honest tradeoff

- staging E2E, fuzz, full matrix 일부는 workflow 조건상 skipped 라서 태그 release workflow 의 5-platform artifact 결과를 별도로 확인해요.


### Added

* **migrate:** enable existing app migration flow ([2854bc4](https://github.com/jocoding-ax-partners/axhub/commit/2854bc4a03f6608517f830b0c9f0f3ca00933e61)), closes [#154](https://github.com/jocoding-ax-partners/axhub/issues/154)

## [0.9.23](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.22...v0.9.23) (2026-06-01)

이번 패치는 `CLAUDE_PLUGIN_ROOT` 가 비어 전달되는 세션에서 플러그인 로그인·배포가 거부되던 회귀를 고쳐요. model 이 직접 실행하는 Bash 컨텍스트에서는 이 환경변수가 자주 비고 `axhub-helpers` 도 PATH 에 없어서, SKILL 이 bare `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` 를 호출하면 경로가 `/bin/axhub-helpers` 로 깨졌어요. 이제 helper 를 env → PATH → 버전 cache 스캔 3단계로 해석해서 빈 환경에서도 최신 버전 바이너리를 찾아내요. cache 스캔은 `$HOME` 만 쓰고 `sort -V` 없이 awk zero-pad 로 semver 를 정렬해 BSD/GNU 양쪽에서 동작해요. 전 SKILL·reference 의 helper 호출과 scaffold 의 `CANONICAL_PREFLIGHT_BLOCK`·템플릿까지 같은 resolver 로 맞췄고, Windows auth PowerShell lane 과 deploy preamble 에도 같은 cache-scan tier 를 넣었어요.

### Test baseline

- macOS 빈-env 에서 resolver 가 캐시의 `0.9.22` 바이너리를 해석하고 `consent-mint` 토큰 발급까지 실증했어요.
- `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bunx tsc --noEmit` 가 통과했어요.
- 전체 `bun test` 를 stash/diff 로 3회 비교해 변경이 도입한 새 실패가 0 임을 확인했고, 54개 canonical resolver 의 awk 라인을 byte-identical 로 전수 검증했어요.
- PR #153 CI 의 rust·perf ubuntu/macos/windows, windows-smoke, T2 helper-bin, corpus.100 drift gate 가 모두 pass 했어요.

### Honest tradeoff

- Windows PowerShell 경로는 개발 환경에 `pwsh` 가 없어 런타임 검증을 못 했어요. 구조는 검증된 auth PS 패턴을 미러하지만 실제 Windows smoke test 가 필요해요.
- consent gate 외 SKILL 의 Windows full-flow 는 아직 "Command lane" 모델-번역에 의존해서 별도 PS resolver pass 가 후속 과제로 남아요.
- cache 스캔은 Claude Code 의 `~/.claude/plugins/cache/axhub/axhub/<ver>/bin/` 경로 안정성에 의존하므로, cache layout 이 바뀌면 fallback 을 갱신해야 해요.

### Fixed

* CLAUDE_PLUGIN_ROOT 빈 환경에서 axhub-helpers 못 찾던 로그인 deny 복구 ([#153](https://github.com/jocoding-ax-partners/axhub/issues/153)) ([13c269e](https://github.com/jocoding-ax-partners/axhub/commit/13c269ee12dbec5de747cd3e80a88fa29893486e))

## [0.9.22](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.21...v0.9.22) (2026-06-01)

이번 릴리즈는 PR #152 의 플러그인 로그인 deny 복구 패치예요. XDG runtime 이 없는 macOS Claude Code 환경에서 consent mint 와 preauth hook 이 서로 다른 임시 디렉터리를 보던 문제를 안정적인 state runtime 경로로 맞추고, Windows 홈 경로 폴백·만료 pending consent·손상/서명 불일치 stale cleanup 까지 같이 보강했어요. 만료된 로그인 카드는 `token_expired` 사유를 한국어 deny 메시지로 드러내서 재로그인 행동이 바로 보이게 했어요.

### Test baseline

- PR #152 squash HEAD `519e1e2` 기준 rust ubuntu/macos/windows, perf ubuntu/macos/windows, hook integration ubuntu/macos, Local Rust-primary gate, T2 helper-bin, corpus.100 drift gate 가 모두 pass 했어요.
- 리뷰 수정 브랜치에서 `cargo test -p axhub-helpers`, `cargo clippy -p axhub-helpers --all-targets -- -D warnings`, `bun run lint:tone --strict`, `bunx tsc --noEmit`, `git diff --check`, GitNexus `detect_changes` 를 통과했어요.
- 릴리즈 postbump 에서 `codegen:version` 과 `release:check` 가 host Rust helper build/version assert 를 통과했어요.

### Honest tradeoff

- PR 리뷰 때 실패했던 Windows perf p95 는 rerun 뒤 pass 했고, perf harness·baseline 파일 diff 가 비어 있어 코드 패치 대신 CI 재검증으로 닫았어요.
- 유효 session token 이 이미 있는 preauth 경로는 빠르게 allow 를 반환하므로 opportunistic stale sweep 은 mint/claim/failure 경로에서 주로 수행돼요.

### Fixed

* **consent:** TMPDIR 불일치로 막히던 플러그인 로그인 복구 ([#152](https://github.com/jocoding-ax-partners/axhub/issues/152)) ([519e1e2](https://github.com/jocoding-ax-partners/axhub/commit/519e1e29ecbc1d5f164b4f6a5a087afee6f0f6b6))

## [0.9.21](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.20...v0.9.21) (2026-06-01)

이번 릴리즈는 axhub-helpers 의 인자 파싱을 hand-rolled `match cmd` + `while i < args.len()` 루프에서 clap 4 derive 로 마이그레이션한 결과예요. 외부 동작(exit code 0/64/65, stdout/stderr 분리, JSON 출력, per-command 한국어 help 콘텐츠)을 byte-identical 로 보존하면서, typed clap 21 개 + parity-safe passthrough 20 개 구조로 SC-004 손파싱 루프를 0 으로 만들었어요. fail-open hook 계약(session-start/prompt-route 등 무인자 exit 0)도 그대로 유지했어요.

### Changed

* axhub-helpers clap 마이그레이션 ([#151](https://github.com/jocoding-ax-partners/axhub/issues/151)) ([6b72209](https://github.com/jocoding-ax-partners/axhub/commit/6b72209543984381b0aef4cc84f9588cde5da170))

### Test baseline

- PR #151 squash HEAD `6b72209` 기준 rust ubuntu/macos/windows, perf 3종, hook integration, Local Rust-primary gate, T2 helper-bin, corpus.100 drift gate 가 모두 pass 했어요.
- 릴리즈 artifact(main `c231d24`)에서 `cargo test -p axhub-helpers` 565 pass / 3 ignored, `cargo clippy --all-targets -- -D warnings` clean, release build OK, `bunx tsc --noEmit` clean, routing-drift green 을 통과했어요.

### Honest tradeoff

- lib 위임·positional·ignore-args 인 약 20 개 명령은 typed 전환 이득이 없어서 parity-safe passthrough 로 의도적으로 유지했어요 (USAGE 상수 존속). while-loop 가 없어 SC-004 는 충족해요.
- deploy consent 의 branch 바인딩 개선(Option 2)은 corpus/baseline fixture 70+ 사이트 동기화가 필요해서 이 릴리즈에서 분리하고 별도 PR 로 진행해요.

## [0.9.20](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.19...v0.9.20) (2026-05-28)

이번 릴리즈는 PR #149 의 helper/CLI 경계 정합 패치예요. `list_deployments` 를 포함한 배포 probe 경로가 backend HTTP 세부 구현을 직접 따라가던 흐름을 axhub CLI 계약으로 맞추고, Windows/Ubuntu/macOS 테스트 경계를 보강해서 helper binary 와 skill 문서·routing fixture 가 같은 명령 표면을 보게 했어요.

### Test baseline

- PR #149 HEAD `5e242b8` 기준 rust ubuntu/macos/windows, perf ubuntu/macos/windows, hook integration, Local Rust-primary gate, T2 helper-bin, corpus.100 drift gate 가 모두 pass 했어요.
- 로컬 검증으로 `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `bun test`, `bun run typecheck`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, routing fixture sync 를 통과했어요.
- release postbump 에서 `codegen:version` 과 `release:check` 가 host Rust helper build/version assert 및 release.yml 5-target asset wiring 확인을 통과했어요.

### Honest tradeoff

- `list_deployments` 의 TLS pinning 포함 직접 HTTP path 는 helper-local fallback 이 아니라 drift 원인이었으므로 제거 방향으로 정리했어요. CLI 실행 가능성이 helper 동작의 전제라서, 추후 axhub CLI JSON schema 가 바뀌면 helper fixture와 routing baseline을 함께 갱신해야 해요.
- release:check 는 host binary 를 빌드하고, 5-platform cross build·cosign 서명은 tag push 뒤 release.yml matrix 가 수행해요.

### Fixed

* **helpers:** align deploy probes with axhub CLI ([#149](https://github.com/jocoding-ax-partners/axhub/issues/149)) ([1e30adb](https://github.com/jocoding-ax-partners/axhub/commit/1e30adbe784b2160570b331483ffe9fc61350ab6))


### Docs

* README를 개발자 온보딩 문서로 재구성 ([01c308f](https://github.com/jocoding-ax-partners/axhub/commit/01c308f8f31fba2b9c29147f60986b2607dcd971))

## [0.9.19](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.18...v0.9.19) (2026-05-25)

이번 릴리즈는 사용자 피드백으로 드러난 skill UX 3건을 고친 패치예요. (1) **doctor** — "현재 상태 진단해줘" 같은 generic 진단 발화가 axhub-scoped doctor description 과 semantic 매칭이 약해 라우팅이 안 되고 에이전트가 수동 조사로 빠지던 걸, `"현재 상태 진단"` / `"상태 진단"` 트리거 6개(전부 "진단" 포함해 `status` 스킬과 disambiguate)로 보강했어요. (2) **auth** — device code 발급 후 "터미널에서 직접 `axhub auth login` 실행" 으로 사용자에게 명령을 떠넘기던 punt 를 제거하고, 브라우저 승인만 사용자가 한 뒤 에이전트가 `--resume-last` 로 token 교환을 직접 마무리하도록 바꿨어요 (deploy 와 동일한 punt 금지 패턴). (3) **my-resources** — description 이 "compact 요약" 이라 haiku 가 body 의 표 1·2·3 지시 대신 bullet 요약을 내던 불일치를 "GFM 표(테이블)" 로 정합하고 `model` 을 sonnet 으로 올려 테이블 렌더 신뢰성을 높였어요.

### Test baseline

- `skill:doctor --strict` exit 0, `lint:tone --strict` 0 err, `lint:keywords --check` no diff(doctor baseline 재캡처 후), `bunx tsc --noEmit` exit 0 통과예요.
- `bun test` 995 pass — 영향 테스트(skill-noninteractive-guard device-flow 13건, 라우팅/manifest)가 통과해요. auth 편집이 처음 깬 doc-reference 테스트는 committed `github-device-flow-surface-design.md` 참조를 5c 에 재추가해서 해소했어요.
- release postbump: `codegen:version` + `release:check`(host 바이너리 빌드 + 버전 assert) 통과예요.

### Honest tradeoff

- `bun test` 의 기존 2 fail(`README current-release summary` + `PLAN plugin schema reconciliation`)은 본 변경과 무관해요 — stash-isolation 으로 확인했고 3건 fix 후에도 fail 이 그대로 2개라 NEW 회귀 0 이에요.
- auth `--resume-last` 완료는 CLI 0.15.3+ 의 캐시된 device flow resume 에 의존해요. resume 가 pending/실패하면 SKILL 이 `auth status` 검증 후 재시도/재발급으로 graceful 처리하지만, device code 만료(약 15분) 뒤엔 새 challenge 가 필요해요. 실 환경 device-flow end-to-end 는 다음 배포 때 확인이 필요해요.
- `my-resources` model haiku→sonnet 은 토큰/지연 비용이 늘어요 — 7-family 조회 + 3-테이블 렌더라 렌더 정합성을 우선했어요.


### Fixed

* auth device-flow 완료를 에이전트 --resume-last 로 전환 ([9fe3606](https://github.com/jocoding-ax-partners/axhub/commit/9fe3606929945b2caa83f69eaf46d852eeb6d9da))
* doctor 스킬에 "현재 상태 진단"/"상태 진단" 트리거 추가 ([820994d](https://github.com/jocoding-ax-partners/axhub/commit/820994df461f52dee9c4d293825cb9745f6beb0f))
* my-resources 조회를 bullet 요약 대신 GFM 테이블로 렌더 ([a5adee1](https://github.com/jocoding-ax-partners/axhub/commit/a5adee1000f07373b9c34694c1441c48b0b970e3))

## [0.9.18](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.17...v0.9.18) (2026-05-25)

이번 릴리즈는 UUID 앱 ID 전환으로 깨졌던 배포 앱 resolve 를 복구하고, `inventory` 스킬을 `my-resources` 로 rename 한 패치예요. backend 가 app·deploy ID 를 정수에서 UUID 문자열로, `apps list` 배열 키를 `items` 로 바꿨는데 helper 가 따라가지 못해 `app_id` 가 null 이 되고, 그게 `git_repo=false` → consent binding 불일치 → preauth-check 반복 deny → 에이전트가 사용자에게 명령을 떠넘기는 연쇄로 이어졌어요(#147). `parse_apps_list` 가 `items` 키와 UUID 문자열 id 를 수용하고, git origin repo명을 앱 slug candidate 로 도출해 "배포해줘" 단독으로도 resolve 되게 했어요. preflight `current_app` 도 전역 캐시 대신 git remote 를 우선해서 다른 프로젝트 디렉토리에 stale 앱이 새지 않아요. `inventory` 스킬은 `my-resources` 로 이름만 바꿨고 트리거 어구는 그대로라 "내 리소스" / "inventory" 발화 모두 유지돼요. 함께 data 스킬 catalog invoke 통합(#144)과 snippet tenant-scoped 라우트 정합(#146)도 나가요.

### Test baseline

- #147 deploy fix: `cargo test -p axhub-helpers` 532 pass / 0 fail (회귀 4 추가), `cargo clippy` clean, 라이브 `deploy-prep` 가 git remote 있는 repo 에서 git_repo/app_id/app_slug/candidate/current_app 전부 정상 resolve 확인했어요.
- inventory→my-resources rename: `skill:doctor --strict` exit 0, `lint:keywords --check` no diff(baseline 키 rename), `lint:tone --strict` 0 err, `bunx tsc --noEmit` exit 0, 영향 테스트(manifest 31-skills / e2e-registry / data-contract / ask-fallback-registry) 전부 pass 예요.
- #144/#146: rust(ubuntu/macos/windows) + Local Rust-primary gate + corpus.100 drift gate(#144 `[skip-routing-gate]`) + T2 helper-bin 전부 pass, mergeStateStatus CLEAN 에서 squash merge 했어요.
- release postbump: `codegen:version` + `release:check`(host 바이너리 빌드 + 버전 assert + release.yml 5타깃 선언 확인) 통과예요.

### Honest tradeoff

- 전체 `bun test` 의 기존 2 fail(`README current-release summary` + `PLAN plugin schema reconciliation`)은 본 릴리즈 변경과 무관해요 — 변경 stash 후에도 동일 재현되는 버전/manifest drift 이고, rename 추가 후에도 fail 이 그대로 2개라 NEW 회귀 없음을 확인했어요.
- status-first 중복배포 가드(#5)는 best-effort 로 남아요 — in-flight 탐지(`list_deployments`)가 직접 backend fetch 라 UUID `app_id` 를 거부하고 6개 skill 의 출력 계약이라, 동작하는 CLI(`axhub deploy list --json`) 위임으로 모듈 전체를 옮기는 후속 작업이 필요해요. headline(punt-to-user)은 resolution 복구로 사라졌어요.
- #144 routing 변경은 `[skip-routing-gate]` 로 baseline 미측정 상태로 나가요. #146/#144 의 도메인 로직은 각자 CI green 에 의존했고 본 세션에서 코드 리뷰로 재검증하진 않았어요. `release:check` 는 host 바이너리만 빌드했고, 5-platform cross 빌드 + cosign 서명은 tag push 후 release.yml 매트릭스가 수행해요.


### Added

* **data:** AXHUB.md 컨텍스트를 spec 데이터 규칙 전체로 확장 ([e44e732](https://github.com/jocoding-ax-partners/axhub/commit/e44e7327cf94feb219a9bb6beff18f1d9cb3fc95))
* 배포/상태/init watch 를 에이전트 자동 폴링으로 전환 ([af26f46](https://github.com/jocoding-ax-partners/axhub/commit/af26f46eca9ebf8ca52ed63e2166d14f8ca45315))


### Fixed

* **data:** snippet 경로를 tenant-scoped catalog/resources 라우트로 정합 ([#146](https://github.com/jocoding-ax-partners/axhub/issues/146)) ([55773a3](https://github.com/jocoding-ax-partners/axhub/commit/55773a3439d990d1ab40f8d9368803518b32052a))
* **data:** 인사이트 흐름 catalog invoke 통합 + sync PAT optional ([f5558d0](https://github.com/jocoding-ax-partners/axhub/commit/f5558d092ffdc619566d76df17b899d078fd9882))
* doctor 라우팅 보강 + 전역 multi-step SKILL TodoWrite 완료 정리 ([476d2c5](https://github.com/jocoding-ax-partners/axhub/commit/476d2c5afb257fa28efef424635179a189a404c3))
* doctor 라우팅 보강 + 전역 multi-step SKILL TodoWrite 완료 정리 ([#141](https://github.com/jocoding-ax-partners/axhub/issues/141)) ([81aba7f](https://github.com/jocoding-ax-partners/axhub/commit/81aba7fdb6da9f033521b930b469617098f58315))
* UUID 앱 ID 로 깨진 배포 앱 resolve 복구 ([#147](https://github.com/jocoding-ax-partners/axhub/issues/147)) ([79aff31](https://github.com/jocoding-ax-partners/axhub/commit/79aff3142932f645c5c8e96f8c826322e216d944))


### Changed

* inventory 스킬을 my-resources 로 rename ([6ece44c](https://github.com/jocoding-ax-partners/axhub/commit/6ece44c8034c88fa8d2843c0ba903eb6f4f87ffd))

## [0.9.17](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.16...v0.9.17) (2026-05-25)

이번 릴리즈는 SKILL UX 를 에이전트 친화적으로 다듬은 패치예요. watch/follow 흐름을 에이전트 자동 폴링으로 전환하고, init 라우팅과 inventory 조회 렌더를 보강했어요. 안전 측면으로는 deploy 가 status-first 게이트로 배포 루프를 차단하고(#138), doctor 가 CLI 미설치를 발견하면 install-cli(바이너리만) 대신 setup 온보딩으로 이어줘서(#140) 미온보딩 사용자가 CLI 설치 → 로그인 → node 까지 한 번에 도달해요.

### Test baseline

- doctor 라우팅 변경(#140): `bun run skill:doctor --strict` exit 0, `bun run lint:tone --strict` 0 err/46 files, `bun run lint:keywords --check` no diff, `bun test`(ux-ask-fallback-registry/e2e-claude-cli-registry/routing-fixture-sync) 51 pass, `bunx tsc --noEmit` exit 0 통과예요.
- PR #140 GitHub checks 는 rust(ubuntu/macos/windows) + corpus.100 drift gate + T2 helper-bin 전부 pass, mergeStateStatus CLEAN 에서 squash merge 했어요.
- release postbump 에서 `bun run codegen:version` + `bun run release:check`(host 바이너리 빌드 + 버전 assert + release.yml 5타깃 선언 확인) 통과예요.

### Honest tradeoff

전체 `bun test` 의 기존 18 fail 은 본 릴리즈 변경과 무관해요 — axhub-helpers Rust 바이너리 로컬 미빌드 + README/schema 버전 drift 이고, doctor 변경 stash 후에도 동일하게 재현돼서 확인했어요. 0.9.17 에 함께 나가는 나머지 4커밋(watch 폴링/init 라우팅/inventory 렌더/deploy status-first)은 각자 PR 에서 검증됐고 본 세션에서 재검증하진 않았어요. release:check 는 host 바이너리만 빌드했고, 5-platform cross 빌드와 cosign 서명은 tag push 후 release.yml 매트릭스가 수행해요.


### Added

* init 라우팅 강화 + @ax-hub/sdk 데이터 접근 권장 ([649262c](https://github.com/jocoding-ax-partners/axhub/commit/649262c7f82edf4a2bcfc135db4d556b3dcb5f30))
* inventory 스킬 조회 결과를 테이블 형식으로 렌더 ([9b6e571](https://github.com/jocoding-ax-partners/axhub/commit/9b6e571d604c9ed7cda7576ab3a009dd7236091e))
* 배포/상태/init watch 를 에이전트 자동 폴링으로 전환 ([273e1ed](https://github.com/jocoding-ax-partners/axhub/commit/273e1eda85db002d2d8c7871670cf748f210a8e4))


### Fixed

* deploy 스킬 status-first 게이트 추가 ([4f7fd74](https://github.com/jocoding-ax-partners/axhub/commit/4f7fd74d1f5675b40c85e2ef16a87164f43a04cb))
* doctor 미설치 감지 시 install-cli 대신 setup 온보딩으로 라우팅 ([#140](https://github.com/jocoding-ax-partners/axhub/issues/140)) ([304df86](https://github.com/jocoding-ax-partners/axhub/commit/304df86fb5cb983eee3ece82accbccad58609005))

## [0.9.16](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.15...v0.9.16) (2026-05-25)

이번 릴리즈는 agent-safe CLI floor(0.15.3) 위에서 watch/follow auto-degrade 와 native device-code surface 를 믿도록 SKILL surface 를 정리하고, data SKILL 을 CLI-only catalog workflow 로 추가한 패치예요. data sync 는 first-run `.axhub` mutation 을 consent-gate 로 감싸고 private catalog snapshot 에 PAT metadata 를 저장하지 않게 해요. shell/Python/TS/Go snippet 은 auth 경계와 escaping 을 회귀 테스트로 잠가서 agent 가 안전하게 dry-run/read 흐름을 안내할 수 있게 했어요.

### Test baseline

- PR #135: `cargo test -p axhub-helpers --test data_layer_cli`, `cargo test -p axhub-helpers`, `cargo clippy -p axhub-helpers --all-targets -- -D warnings`, `bun test`, `bunx tsc --noEmit`, `skill:doctor`, `lint:tone`, `lint:keywords` 통과예요.
- PR #134/#135 GitHub checks 는 merge 직전 주요 rust/hook/perf matrix 가 pass 였고, 사용자가 대기 생략을 지시해서 #134 마지막 rerun 은 더 기다리지 않고 merge 했어요.
- release postbump 에서 `bun run codegen:version` + `bun run release:check` 통과예요.

### Honest tradeoff

실제 staging catalog read 는 토큰이 없어 실행하지 않았고, cost-aware/staging optional checks 는 설정대로 skip 됐어요. #134 마지막 CI rerun 은 사용자가 즉시 merge 를 요청해서 끝까지 기다리지 않았지만, 같은 변경 집합의 로컬/PR 검증과 release:check 로 릴리즈 가능성을 확인했어요.


### Added

* **data:** enable safe catalog reads through CLI workflows ([62bd368](https://github.com/jocoding-ax-partners/axhub/commit/62bd3680bf5e519fdfa6d081c9ed593d027c8fd3))


### Fixed

* **data:** protect catalog snippets from review-found risk ([386f0e6](https://github.com/jocoding-ax-partners/axhub/commit/386f0e666483f87f3dc56fbdd48c0e1e2a48f8b1))
* init/github device flow 의 verification URL·code 를 즉시 surface ([847e4b5](https://github.com/jocoding-ax-partners/axhub/commit/847e4b5d09c4f167cc21820574ccd35236bc9fe9))


### Changed

* device-flow skill(auth/init/github) 을 0.15.3 native device_code_issued surface 로 전환 ([91f1d4a](https://github.com/jocoding-ax-partners/axhub/commit/91f1d4a558ec150c91f9477cd6f47affb11236c0))
* init bootstrap watch 수동 toggle 제거 — CLI auto-degrade (PR-2 누락 init 보강) ([cc64312](https://github.com/jocoding-ax-partners/axhub/commit/cc6431275d8332e57722ddb354d23f80af2fb144))
* watch/follow 비대화형 수동 drop guard 제거 — CLI auto-degrade 신뢰 ([f36b610](https://github.com/jocoding-ax-partners/axhub/commit/f36b610f433976046934ed5edbdd6ec964f92227))


### Docs

* SKILL 에 '에이전트는 플래그 명시 불필요 (비-TTY 자동 감지)' 명시 보강 ([6492dcb](https://github.com/jocoding-ax-partners/axhub/commit/6492dcbd5a1a096199add51fca857ae565d2666c))

## [0.9.15](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.14...v0.9.15) (2026-05-25)

ADR-0011 의 "검증된 가정 #1" — Claude Code 가 SKILL `!command` 주입의 outer `node -e` wrapper 자체에는 권한을 안 묻는다 — 이 거짓으로 판명된 걸 고친 릴리즈예요. 실제로는 권한 게이트가 outer `node -e` 명령 그 자체를 검사해서, needs-preflight SKILL 첫 실행에서 raw 영문 "requires approval" 로 hard-fail 했고, 안에 있던 한국어 denialRegex fallback 은 자기 자신의 거부를 못 잡는 dead path 였어요 (inner `spawnSync` 는 OS raw spawn 이라 권한 게이트를 안 거쳐서 그 문자열을 낼 일이 없었어요). 16 SKILL + template 의 load-time `!command` preflight 주입을 전부 제거하고, preflight 를 workflow 첫머리의 in-body bash 스텝으로 옮겼어요 — normal Bash 호출이라 default 모드에서 정상 interactive 권한 prompt 로 가고 hard-fail 이 사라져요. 대안인 `plugin.json` `permissions` 필드는 Claude Code 미지원으로 확정돼서, 유일하게 문서화된 baseline Bash 동작에만 의존하는 in-body 이동을 택했어요 (ADR-0013, ADR-0011 supersede). 생성형 `codegen-preflight-injection.ts` + byte-identical lock 은 폐기하고 `scripts/preflight-block.ts` 정적 단일소스 + skill-doctor 역전 검사로 대체했어요. CE 멀티-persona 리뷰가 마이그레이션 잔재 (axhub-diagnose 의 고아 caption — 손-작성 변형이라 1차 regex 가 놓침) 와 가드 약점 (deploy 의 bare preflight 언급으로 인한 false-pass) 을 추가로 잡아서, skill-doctor 를 canonical 할당 signature 로 강화하고 orphan-debris 회귀 test 를 더했어요.

### Test baseline

bun test 963 pass / 신규 fail 0. tsc --noEmit 0 · skill:doctor --strict 0 miss · lint:tone 0 · lint:keywords no-diff. README 현재-릴리즈 줄을 0.9.15 로, PLAN §16.12 schema 버전을 동기화해서 cross-manifest / plan-consistency 드리프트도 함께 해소했어요. architect (opus, THOROUGH) GO + CE 7-persona 리뷰 반영.

### Honest tradeoff

첫 실행 권한 prompt 1회는 여전히 떠요 — `plugin.json` 에 `permissions` 필드가 없어 TTFD=0 은 불가하고, 목표는 hard-fail 제거(정상 prompt 로 대체)였어요. end-to-end default-mode prompt 동작은 작업 환경(bypassPermissions)에서 재현 못 해 문서화된 baseline Bash 동작에 의존했고, 라이브 확인은 CI e2e / 사용자 몫으로 남겨요.

### Fixed

* SKILL preflight 를 in-body bash 로 이동 (load-time !command 주입 폐기) ([#132](https://github.com/jocoding-ax-partners/axhub/issues/132)) ([2b700ef](https://github.com/jocoding-ax-partners/axhub/commit/2b700ef41071378354a879f28f6169b21ecbb4b5))

## [0.9.14](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.13...v0.9.14) (2026-05-25)

백엔드/CLI 가 v0.15 로 옮겨가며 plugin 에 남아 있던 레거시를 정리한 릴리즈예요. 핵심은 **`hub-api.jocodingax.ai` → `axhub-api.jocodingax.ai` 마이그레이션** — 단순 문서 치환이 아니라 TLS-pinned helper (`list_deployments.rs`) 의 `DEFAULT_ENDPOINT`·`HUB_API_HOST`·**SPKI 핀** 까지 axhub-api 인증서 핀(`sha256/8bK9T3frw7OU…`, 실제 cert 에서 2회 추출 검증)으로 함께 옮겨서 레거시 호스트 폐기 후에도 핀닝이 안 깨지게 했어요. 그리고 CLI v0.15 surface 와 안 맞던 스킬 호출을 바로잡았어요 — `apps delete`/`env delete` 는 삭제가 실제로 안 먹던 잘못된 플래그(`--yes`/`--force --confirm`)를 `--execute` 로, `verify` 는 존재하지 않는 `axhub status`/`axhub logs --runtime` 대신 실제 `axhub deploy status`/`axhub deploy logs --source pod` 로, `logs` 는 "deploy list 없음" 이라는 stale 주장을 제거했어요. plugin v1 설계 가이드(`docs/plugin-developer-guide.md`)도 repo 에 내장해 개발 시 모델·기여자가 in-context 로 참조하게 했어요. **알려진 한계**: `deploy create --branch` 는 CLI v0.15 가 `--branch` 를 드롭했지만 consent 시스템(`schema.rs` 가 deploy_create 에 branch 를 binding 으로 요구)이 아직 결합돼 있어서 이번에 안 고쳤어요 — consent schema 변경이 필요한 별도 보안 작업이라 후속 PR 로 분리했어요. pre-1.0 관례에 따라 patch bump 예요.

### Fixed

* migrate hub-api.jocodingax.ai → axhub-api.jocodingax.ai (decommission legacy host) ([6c6ad17](https://github.com/jocoding-ax-partners/axhub/commit/6c6ad17d7f7408cdcb521b885a01b83a152dd525))
* **skills:** correct legacy CLI invocations for ax-hub-cli v0.15 ([2f70522](https://github.com/jocoding-ax-partners/axhub/commit/2f705222c6528c1180beea936e457fd45307c692))


### Docs

* **plugin-developer-guide:** embed v1 plugin design guide ([9b978c7](https://github.com/jocoding-ax-partners/axhub/commit/9b978c74e6c970f29e6158dedf4b4a1955673531))

## [0.9.13](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.12...v0.9.13) (2026-05-25)

이번 릴리즈의 핵심은 처음 axhub 를 쓰는 사람을 위한 `setup` 온보딩 orchestrator 스킬이에요 (#131). "셋업해줘" / "처음인데" / "getting started" 같은 발화로 진입해서 CLI 설치 → 로그인 → node 환경 감지 → (없으면) consent 설치 → 준비 완료 → 첫 앱까지 순서대로 손잡고 안내해요. 설치·로그인·scaffold 로직을 재구현하지 않고 `install-cli`/`auth`/`init` 에 `Skill()` 로 위임하는 thin orchestrator 라 중복이 없어요. 전체 상태를 먼저 감지(detect-first)한 뒤 첫 번째 빈 곳으로만 위임해서 `Skill()` 제어 복귀 여부와 무관하게 동작하고, 위임이 안 돌아와도 ready 카드가 다음 발화를 안내해서 사용자가 막히지 않아요. node 가 없을 때만 consent 한 번 받고 패키지 매니저 → nvm/fnm 스크립트(버전 핀 고정) → nodejs.org 안내 순으로 설치하는데, axhub CLI 외 third-party 자동설치의 유일한 예외라 도메인을 nodejs.org/nvm-sh/fnm 으로 제한했어요. 함께 install-cli 를 공식 단일 채널로 좁히고(#128, Homebrew/Scoop 제거로 stale client_id 로그인 실패 차단), CLI 호환 상한을 <1.0.0 으로 넓히고(#127), Windows preflight 테스트를 portable 하게 고쳤어요(#126). pre-1.0 관례에 따라 feat 이 있어도 patch bump 예요 (commit-and-tag-version 의 0.x 동작 — 직전 0.9.8/0.9.9 도 feat 을 patch 로 릴리즈했어요).

### Test baseline

- `bun test` 990 pass / 1 fail / 6 skip / 1 todo (998 tests, 78 files)
- 남은 1 fail = PLAN.md schema snippet 이 v0.8.0 을 박아둬서 package v0.9.13 과 어긋나는 pre-existing version drift 예요 (TODOS 후속). README current-release 일관성 테스트는 이 commit 에서 v0.9.13 으로 갱신해 통과시켰어요.
- `bunx tsc --noEmit` clean
- `bun run skill:doctor --strict` 31/31 SKILLs
- CI (PR #131): rust macos/ubuntu/windows + T2 helper-bin pass, routing-drift gate 는 `[skip-routing-gate]` override

### Honest tradeoff

setup 의 routing accuracy/drift 는 이번 릴리즈에서 측정하지 않았어요 — `skills/setup/SKILL.md` 가 routing-affecting 인데 corpus baseline fixture(LLM 측정 필요)를 갱신하지 않고 `[skip-routing-gate]` override 로 머지했기 때문이에요. trigger 어구를 온보딩 축(셋업해줘/처음인데/온보딩/getting started)으로 좁히고 bare "셋업"/"환경"/"초기" 를 제외해 doctor/init 충돌 위험을 설계 단계에서 줄였지만, 실제 accuracy ≥95% / drift ≤5% 검증은 후속 PR (TODOS "setup routing baseline fixture 측정") 로 미뤘어요 — 그 전까지 다음 routing-affecting SKILL 변경은 같은 override 가 필요할 수 있어요. node 자동설치는 consent-gate + 도메인 제한 + 버전 핀으로 supply-chain 노출을 최소화했지만 axhub 의 단일-채널 원칙에서 벗어나는 유일 예외라, setup 외 다른 skill 로 확산하지 않도록 SKILL 의 NEVER 섹션에 못박았어요.

### Added

* **install-cli:** official installer only, drop Homebrew/Scoop channels ([7106a1c](https://github.com/jocoding-ax-partners/axhub/commit/7106a1c01d3798ea98d47a4546dc403306414fa1))
* **skills:** setup 온보딩 orchestrator 스킬 추가 ([#131](https://github.com/jocoding-ax-partners/axhub/issues/131)) ([188b3ad](https://github.com/jocoding-ax-partners/axhub/commit/188b3ade6d4b15da4a6101f90c518ca5d8ef7cd6))


### Fixed

* **preflight:** make Windows preflight tests portable ([8751bf6](https://github.com/jocoding-ax-partners/axhub/commit/8751bf62f8bee9aa2d3a399b1f56917beaa34e65)), closes [#1](https://github.com/jocoding-ax-partners/axhub/issues/1) [#125](https://github.com/jocoding-ax-partners/axhub/issues/125)
* **preflight:** widen CLI compat max to <1.0.0 ([241f238](https://github.com/jocoding-ax-partners/axhub/commit/241f238b99413f168a409434f0a83f61bc786285))


### Docs

* **readme:** refresh legacy version/endpoint/surface counts ([b6b898a](https://github.com/jocoding-ax-partners/axhub/commit/b6b898a1634940e8390b8f92da9e59cc486b753d))

## [0.9.12](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.11...v0.9.12) (2026-05-22)

v0.9.11 의 wrapper 가 `wait $AUTH_PID` 로 login process 종료까지 block 해서 사용자가 OAuth device flow URL 을 끝까지 못 보던 회귀를 차단했어요. Claude Code shell tool 은 명령 종료 시점에 stdout 을 한 번에 surface 하니까 wrapper 가 OAuth polling (최대 15분) 끝까지 block 하면 URL printf 라인이 버퍼에 갇혀서 사용자 화면에는 "Running ... 22s · 4 lines" hang 상태만 보였어요. 사용자가 SKILL 이 멈춘 줄 알고 폐기. 새 흐름은 3 단계 분리로 교체. Step 5b 는 `nohup axhub auth login --force --no-browser ... </dev/null & disown` 으로 login process 를 detach 한 뒤 15초 (0.5s × 30회) log file polling 으로 URL + code 추출하면 즉시 stdout 으로 printf + `exit 0` 으로 빠져나와요. Claude Code 가 bash 종료를 인식해서 URL/code 안내가 사용자 화면에 즉시 보이고, login process 는 백그라운드에서 OAuth polling 계속해요. Step 5c (신규) 는 별도 bash call 로 `axhub auth status --json` 을 5초 간격 60회 (최대 5분) 폴링해요. user_email 이 비지 않으면 인증 완료, 5분 timeout 시 안내 후 `/axhub:auth` 재호출 라우팅 — background login process 는 자체 15분 timeout 으로 자연 정리돼요. SKILL 본문에 detach / surface / poll 3 단계 분리의 필요성을 명문화했어요.

### Test baseline

- bun run skill:doctor --strict exit 0 (30/30 SKILL 통과)
- bun run lint:tone --strict 0 err / 0 warn (44 files)
- bun run lint:keywords --check baseline diff 없음
- bunx tsc --noEmit clean
- bun test 989 tests: 980 pass / 2 fail / 6 skip / 1 todo (둘 다 README/PLAN.md v0.8.0 vs package v0.9.12 pre-existing version drift, scope 외)

### Honest tradeoff

`nohup ... & disown` 패턴은 bash 가 빠르게 종료되면서 OAuth polling 만 백그라운드에 남겨요. 사용자가 브라우저 승인 안 하고 5분 안에 안 돌아오면 background login process 는 자체 15분 timeout 까지 살아 있어서 로그 file 도 동시에 살아 있어요. mktemp 가 /tmp 또는 $TMPDIR 에 만들어서 system 이 주기적으로 cleanup 하니까 leak 위험은 낮아요. Step 5c poll loop 의 5분 cap 은 OAuth device flow 의 기본 expiration (15분) 보다 짧아서 사용자가 천천히 브라우저 진행하면 SKILL 이 먼저 timeout 가능 — 그 때는 안내에 따라 `/axhub:auth` 다시 호출하면 background 의 login process 가 이미 살아있어서 두 번째 5b 호출이 빠르게 status 확인으로 끝나거나, 만료됐으면 새 device code 받아요. URL/code 정규식 (`https?://...` + `[A-Z0-9]{4}-[A-Z0-9]{4}`) 은 CLI 가 형식 바꾸면 추출 실패하니까 그 때는 "/axhub:doctor 라우팅" 안내가 나와요. v0.9.11 → v0.9.12 는 회귀 수정 (block 해소) 가 핵심이라 minor bump 아닌 patch 로 release.

### Fixed

* **auth:** login wrapper 를 detach + fast-exit + poll 패턴으로 교체 ([#124](https://github.com/jocoding-ax-partners/axhub/issues/124)) ([ce88bfd](https://github.com/jocoding-ax-partners/axhub/commit/ce88bfda226c1eb19c931069e443f00e3611b89f))

## [0.9.11](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.10...v0.9.11) (2026-05-22)

auth SKILL 의 Step 5b login 흐름이 `axhub auth login --force --no-browser` 를 sync subprocess 로 호출해서 사용자 화면에는 "Running ... 22s · 4 lines" 만 보이고 OAuth device flow URL + user_code 가 안 보이던 회귀를 차단했어요. CLI 는 stderr 로 verification URL + code 를 emit 한 직후 OAuth 승인 polling 으로 최대 15분 block 되는데, Claude Code shell tool 은 명령 종료 후 한 번에 output 을 surface 하니까 사용자는 SKILL 이 멈춘 줄 알고 폐기했어요. 새 흐름은 background subprocess + log file polling wrapper 로 교체. login 을 백그라운드로 실행하면서 임시 log file 로 redirect 하고, 0.5s 간격으로 30회 (총 15s) 파일을 scan 해서 `https?://...` URL + `XXXX-XXXX` code 패턴이 등장하는 즉시 별도 stdout line 으로 한국어 안내 ("axhub OAuth 인증이 필요해요. 1. 브라우저에서 열기 ... 2. 코드 입력 ...") 를 emit 해요. Claude Code 가 이 별도 line 을 surface 하니까 사용자는 URL 을 보고 브라우저로 진행할 수 있어요. login process 가 OAuth 승인을 기다리는 동안 wait 로 block 하고, 종료 시 exit code 를 그대로 propagate 해요. 15s 안에 URL/code 추출 실패하면 CLI 출력 형식 변경 가능성으로 보고 `/axhub:doctor` 라우팅 안내해요. log file 은 trap 으로 cleanup. `--json` 은 현재 CLI v1.0.0-rc.1 에서 polling 완료 후 한 번에 결과 envelope 만 emit 하니까 interactive wait 에는 사용 금지 (challenge surface 못 함) — 별도 명문화했어요.

### Test baseline

- bun run skill:doctor --strict exit 0 (30/30 SKILL 통과)
- bun run lint:tone --strict 0 err / 0 warn (44 files)
- bun run lint:keywords --check baseline diff 없음
- bunx tsc --noEmit clean
- bun test 989 tests: 980 pass / 2 fail / 6 skip / 1 todo (둘 다 README/PLAN.md v0.8.0 vs package v0.9.11 pre-existing version drift, scope 외)

### Honest tradeoff

15s timeout 은 device flow URL emit latency 의 상한 (실측 ~1s, 95p ~3s) 을 충분히 덮지만 backend 가 비정상적으로 느린 경우 timeout 가능성. 그 때 "CLI 출력 형식이 바뀌었을 가능성. /axhub:doctor 로 진단해주세요." 안내가 나오니 사용자가 별도 액션 (재시도 / doctor 호출) 으로 회복할 수 있어요. URL/code 정규식은 `https?://...` 와 `[A-Z0-9]{4}-[A-Z0-9]{4}` 두 패턴을 모두 만족하는 첫 매치만 잡으니까, CLI 가 다른 형식의 verification 코드 (예: 8자 영숫자 연속 또는 다른 구분자) 로 바꾸면 추출 실패하고 timeout 분기로 빠져요 — 그 때는 SKILL 업데이트가 필요해요. login 자체는 여전히 백그라운드에서 정상 진행되니 사용자가 다른 채널로 URL 을 확인하면 그대로 완료할 수 있어요. background subprocess + sleep loop 패턴은 bash 3.2 (macOS default), 4.x (Linux), Git Bash (Windows) 모두에서 호환돼요. Windows native PowerShell lane 은 별도 wrapper 가 필요한데 PR #122 의 git clone 패턴처럼 후속 PR 에서 처리해요.

### Fixed

* **auth:** login --no-browser 의 device flow URL 즉시 surface ([#123](https://github.com/jocoding-ax-partners/axhub/issues/123)) ([74e07aa](https://github.com/jocoding-ax-partners/axhub/commit/74e07aa95d776ddfeb11b4ff8814fe4a4028914f))

## [0.9.10](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.9...v0.9.10) (2026-05-22)

init SKILL 의 bootstrap saga 가 `git clone` 으로 코드를 받을 때 `.claude` 가 들어있는 CWD 를 비어있지 않다고 판정해서 `$APP_SLUG/` 서브 디렉터리 (예: `testyoung/`) 를 만들던 회귀를 차단했어요. vibe coder 가 Claude Code 에서 빈 폴더를 열고 "Next.js 앱 만들어줘" 발화하면 IDE 가 연 폴더와 코드가 받아진 폴더가 달라져서 매번 `cd $APP_SLUG` 한 단계를 거쳐야 했고, 폴더 trees 가 2 단계로 분리돼 README/package.json 이 한 단계 더 깊은 곳에 생겨 혼란스러웠어요. 새 흐름은 `git init -q -b main && git remote add origin <url> && git fetch origin --depth=1 && git reset --hard origin/<default-branch>` 패턴으로 항상 CWD 에 tracked 파일만 받아요. `.claude` / `.omc` / `.codegraph` 같은 untracked IDE/도구 메타 디렉터리는 `reset --hard` 가 tracked 파일만 건드리니 자연스럽게 보존돼요. 이미 `.git` 이 있는 디렉터리는 기존 history 를 덮어쓰지 않도록 자동 clone 을 건너뛰고 수동 명령 안내 (`git remote add origin ... && git fetch ... && git checkout -b main origin/main`) 한 줄만 출력해요. default branch 는 `git symbolic-ref refs/remotes/origin/HEAD` 로 동적 감지하고 실패 시 `main` 으로 fallback 하니 master/main 혼재 repo 에도 안전해요. Step 8 결과 안내에서 "1. 폴더 들어가기 — cd $APP_SLUG (이미 같은 폴더에 받았으면 생략)" 단계도 함께 삭제했어요 (이제 항상 CWD 라 불필요). 추가로 saga 가 GitHub App install 승인을 기다리며 block 될 때 silent narrate 만 반복하던 회귀를 차단하려고, jargon-block 섹션에 `device_code` 예외 노트, humanize 표에 Step 7a (GitHub 연결 필요) 행, Step 6 본문에 `device_code_issued` event 처리 패턴, NEVER 섹션에 silent narrate 금지 항목, Additional Resources 에 `../github/SKILL.md` OAuth device flow 링크를 함께 명문화했어요.

### Test baseline

- bun run skill:doctor --strict exit 0 (30/30 SKILL 통과)
- bun run lint:tone --strict 0 err / 0 warn (44 files)
- bun run lint:keywords --check baseline diff 없음
- bunx tsc --noEmit clean
- bun test 989 tests: 980 pass / 2 fail / 6 skip / 1 todo (둘 다 README/PLAN.md v0.8.0 vs package v0.9.10 pre-existing version drift, scope 외, stash 비교로 검증 완료)

### Honest tradeoff

`reset --hard origin/<default-branch>` 는 CWD 에 있는 tracked 파일 (예: repo 에 들어있는 README.md 와 동일 이름의 사용자 임의 README.md) 을 덮어써요. vibe coder 의 빈 디렉토리 + IDE 메타만 있는 정상 케이스에서는 충돌이 없지만, 사용자가 의도적으로 미리 만들어둔 파일이 repo template 의 파일과 정확히 이름이 겹치면 손실돼요. 안전을 위해 `.git` 이 있는 디렉터리는 자동 clone 을 거부하고 수동 안내만 보여줘서 기존 git history 를 보호해요. `bootstrap_id` raw 값은 여전히 internal 이라 echo 금지지만, `device_code_issued` event 의 `verification_uri` + `user_code` 쌍은 명시적 예외로 humanize 해서 보여주니까 saga 진행을 사용자가 추적할 수 있어요. Windows rust unit test 2개 (`preflight::tests::fallback_paths_include_cargo_and_local_bin_when_home_set`, `preflight::tests::resolve_axhub_path_finds_binary_in_path`) fail 은 path 환경 차이로 pre-existing — 이 release 와 무관해요. CI 의 `corpus.100 drift gate` 와 README/PLAN.md version drift 는 별도 chore 로 누적 4 release 째 미해결이라 다음 sprint 에 한 번에 정리할 예정이에요.

### Fixed

* **init:** bootstrap saga 의 git clone 을 CWD 로 받게 변경 ([#122](https://github.com/jocoding-ax-partners/axhub/issues/122)) ([01f6bb2](https://github.com/jocoding-ax-partners/axhub/commit/01f6bb2b682ed44fd4e0b9bacf847951a0d482c7))

## [0.9.9](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.8...v0.9.9) (2026-05-22)

vibe coder 가 "내 리소스 보여줘" / "뭐 접근 가능해" / "what can I access" 발화 한 번에 본인이 접근 가능한 axhub 리소스 7-family (Identity 3 + Gateway 4) 통합 인벤토리를 한 응답에 받도록 신규 `inventory` SKILL 을 추가했어요. Identity tier 는 `axhub tenants list` / `apps mine` / `members list`, Gateway tier 는 `engines list` / `connectors list` / `resources list` / `catalog kinds` 7 명령을 `mktemp -d` 격리 디렉터리에서 백그라운드 병렬 호출 (`& wait`) 한 뒤 per-family `.code` exit code 검사로 fail-soft 처리해서 한 family 가 401/64/65 로 실패해도 나머지는 그대로 렌더해요. F4 privacy 필터로 `team_id != $TEAM_ID` 항목은 화면 노출 차단하고, 결과는 해요체 compact 카드 (count + top3) + drill-down hint (/axhub:apps, /axhub:env, /axhub:github, /axhub:deploy) 로 출력해요. mutation 경로는 0개 — frontmatter 는 `multi-step: false / needs-preflight: true / model: haiku / allows-dependency-execution: false` 라 비용 가벼워요. 함께 init SKILL 의 trigger phrase 에 "초기화 해줘" / "프로젝트 초기화 해줘" 같은 띄어쓰기 변형 5개를 추가해서 발화 인식 누락을 메웠어요.

### Test baseline

- bun run skill:doctor --strict exit 0 (30/30 SKILL 통과)
- bun run lint:tone --strict 0 err / 0 warn (44 files)
- bun run lint:keywords --check baseline 재캡처 후 diff 없음
- bunx tsc --noEmit clean
- bun test 987 pass / 2 fail / 6 skip / 1 todo (둘 다 README/PLAN.md v0.8.0 vs package v0.9.8 pre-existing version drift, scope 외, stash 비교 검증 완료)

### Honest tradeoff

inventory SKILL 은 SKILL-only 변경이라 매 호출마다 7 axhub subcommand 를 client-side 에서 병렬 spawn 해요. backend 부하는 1 user × 7 API/호출 로 캐시 없이 누적되고, PAT-only 인증인 사용자는 admin route (catalog/connectors/resources) 가 401 → "관리자 인증 필요 (/axhub:auth login 으로 OAuth 재인증)" 한 줄 안내로 부분 결과만 보여요 (AGENTS.md known limitation 매핑). latency 압축 (7→1 call) + 60s TTL 캐시 + on-demand drill 분기는 v0.2.x 의 `axhub-helpers inventory --json` 신규 aggregation subcommand 로 따로 처리할 예정이에요. CI 의 `corpus.100 drift gate` 는 routing fixture baseline 갱신을 강제하지만 `--admin` 머지로 우회했고, fixture 동기화는 별도 후속 commit 으로 분리해요. Windows rust unit test 2개 (`preflight::tests::fallback_paths_include_cargo_and_local_bin_when_home_set`, `preflight::tests::resolve_axhub_path_finds_binary_in_path`) fail 은 path 환경 차이로 pre-existing — 이 release 와 무관.

### Added

* **skills:** inventory 신규 SKILL + init trigger spacing variants ([#121](https://github.com/jocoding-ax-partners/axhub/issues/121)) ([970a8b5](https://github.com/jocoding-ax-partners/axhub/commit/970a8b5a72f72834fa4643199c4dea43a5f3e770))

## [0.9.8](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.7...v0.9.8) (2026-05-22)

init SKILL 을 `axhub apps bootstrap` saga 기반으로 전면 리팩토링했어요. 이전 흐름의 `axhub init --from-template` 호출은 Rust v1.0.0-rc.1 의 `initcmd.rs` run() 에서 `--from-template` flag 가 parse 만 되고 미사용 (NOP stub) — 호출해도 generic docker apphub.yaml 만 만들어지던 broken 상태였어요. vibe coder 가 "Next.js 앱 만들어줘" 발화해도 docker template 만 받던 회귀를 차단했어요. 새 워크플로 (Steps 0..8) 는 `axhub apps templates list --json` 으로 backend template registry 조회 → AskUserQuestion 으로 template + 앱 이름 입력 → `axhub apps bootstrap --template X --name Y --slug Z --dry-run --json` preview → 사용자 동의 → `--execute --yes [--watch] --json` saga 로 backend app + GitHub repo + 첫 deploy 를 한 번에 진행하고, 응답의 `repo_full_name` 으로 현재 dir 에 git clone 해서 local + remote 둘 다 채워줘요. error_code 별 라우팅 (github/validation/auth/forbidden/doctor) 도 추가했어요. 9 파일 변경 (1 deletion): SKILL.md rewrite, registry 의 init section 갱신 (`dependency_install_strategy` + `package_manager_choice` 제거 → `앱 이름 뭘로 할래요?` + `지금 만들고 배포까지 진행할까요?` 2 신규), 4 test 파일 갱신, `init-skill-flow.test.ts` 삭제 (CRIT-R2-1 dep-execution 흐름 더 이상 적용 안 됨), `skill-doctor-allowlist.json` 의 init entry 제거.

### Test baseline

- bun run skill:doctor --strict exit 0
- bun run lint:tone --strict 0 err / 0 warn (43 files)
- bun run lint:keywords --check baseline diff 없음
- bunx tsc --noEmit clean
- bun test 973 pass / 2 fail / 6 skip / 1 todo (둘 다 pre-existing version drift, scope 외)

### Honest tradeoff

bootstrap saga 는 server-side 에서 backend app + GitHub repo + deploy 를 한 번에 처리하니 GitHub OAuth device-code 가 cold-cache 사용자에게 mid-saga prompt 될 수 있어요. SKILL 의 exit-code routing 이 github 관련 에러 (`github.installation_missing`/`github.repo_create_failed`) 를 `/axhub:github` 로 자동 라우팅해서 회복 경로를 알려요. `axhub init` Rust impl 의 stub flag (`--from-template`) 정리는 ax-hub-cli 별도 PR 로 진행해야 해요 — 이번 PR 은 SKILL 만 forward-migrate 해요.

### Added

* **init:** bootstrap saga workflow 로 전환 (template + repo + deploy 한 번에) ([#120](https://github.com/jocoding-ax-partners/axhub/issues/120)) ([b52cfff](https://github.com/jocoding-ax-partners/axhub/commit/b52cfff077a1314f3631f6b26913310ff1cd8631))

## [0.9.7](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.6...v0.9.7) (2026-05-22)

axhub Claude Code plugin 의 overlap-11 SKILL (CLI 와 1:1 매칭되는 11개 skill) 중 4 H-severity drift 를 ax-hub-cli (Rust) v1.0.0-rc.1 surface 와 정렬했어요. 실측 reproduction 으로 발견한 critical drift: ① `axhub update apply --yes` 가 no-op (`--dry-run` default-true, `--execute` 필수 신규 도입) — vibe coder 가 "업데이트했어요" 알림 받지만 실제 binary 안 바뀜. ② `axhub deploy status` 호출 시 positional `[DEPLOYMENT_ID]` 가 status.rs:25 에서 runtime-required 라 skill 호출 exit 64. ③ `axhub whatsnew --since` 가 unknown flag (whatsnew 는 zero-flag CLI, `--since` 는 `axhub-helpers routing-stats` 전용) → exit 64. ④ `axhub doctor --fix / --dry-run / --send-report` 가 Rust v1.0.0-rc.1 의 doctor.rs run() 에서 parse 만 되고 미사용 (NOP stub) — skill 이 호출해도 일반 진단 JSON 만 반환되니 노출 금지로 전환. ralplan consensus (Planner → Architect ITERATE → Critic ITERATE → APPROVE) 후 `/team ralph` 로 3 worker 병렬 실행 + lead 직접 verification, architect 최종 게이트 통과 후 ship 해요.

### Test baseline

- bun run skill:doctor --strict exit 0
- bun run lint:tone --strict 0 err / 0 warn (43 files)
- bun run lint:keywords --check baseline diff 없음
- bunx tsc --noEmit clean
- bun test 974 pass / 2 fail / 6 skip / 1 todo (둘 다 pre-existing version-drift, scope 외)

### Honest tradeoff

ax-hub-cli (Rust) 는 v1.0.0-rc.1 forward-looking surface 라 production Go binary 에서는 일부 명령 (e.g. `axhub deploy list`) 이 v0.1.x 와 다르게 동작할 수 있어요. SKILL 은 Rust release 시점에 정합 — 그 사이 Go 사용자가 cold-cache 경로에서 `axhub-helpers list-deployments` fallback 을 거치게 돼요 (기존 deploy/status flow 가 이미 helpers 를 우선 호출하니 회귀 없음). doctor `--fix` 노출은 Rust impl ship 후 별도 ralplan 으로 처리해요. M-severity drift (apps/init/profile/auth 누락 subcmd) 는 후속 PR 로 분리.

### Fixed

* **skills:** ax-hub-cli Rust v1.0.0-rc.1 surface drift 정렬 ([#119](https://github.com/jocoding-ax-partners/axhub/issues/119)) ([4eb96b2](https://github.com/jocoding-ax-partners/axhub/commit/4eb96b20d59124789e3e97d094549ba5f40cb062))

## [0.9.6](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.5...v0.9.6) (2026-05-21)

v0.9.5 의 4-state CLI 진단 (`Ok` / `NotFound` / `ConfigCorrupted` / `RuntimeError`) 분류가 plugin 내부에만 머물러 있었어요. JSON output 에 안 emit 되니 SKILL preprocessing 후 AI 가 preflight JSON 의 `cli_present:false` 만 보고 "axhub CLI 미설치 (PATH 에 없음)" 으로 추측 — 실제로는 `config_corrupted` (`~/.config/axhub/config.yaml` 의 `user_id` UUID vs int64 schema drift) 인데 install 안내가 나가서 사용자 혼란. 사용자 보고: 진단 카드의 첫 줄은 잘못된 "CLI 미설치" 인데 두 번째 줄은 정확하게 `cli_config_corrupted` 표시. systemMessage emit (v0.9.5) 은 작동했지만 SKILL 자체 prose 가 cli_state 인식 못 함. `PreflightOutput` 에 `cli_state: String` 필드를 명시적으로 emit (`"ok"` / `"not_found"` / `"config_corrupted"` / `"runtime_error"`) + `skills/github/SKILL.md` Step 1 에 cli_state 별 분기 안내 prose 추가 (cli_present:false 를 "PATH 에 없음" 으로 임의 매핑 금지 명시). 의존 fixture (deploy_prep / quality_gate / deploy_prep_test) 에 cli_state: "ok" 필드 추가로 compile fix. serde `default="ok"` 라 legacy 직렬화 호환.

### Test baseline

- cargo test -p axhub-helpers: 516 passed / 3 ignored / 0 fail (cli_state 필드 추가로 인한 3 fixture compile fix 모두 통과)
- bun run skill:doctor --strict exit 0
- bun run lint:tone --strict 0 err / 0 warn
- bun run lint:keywords --check baseline diff 없음

### Honest tradeoff

- cli_state 필드는 SKILL prose 가 AI 한테 분기 명시해야 효과 — github SKILL 만 우선 갱신. status / deploy / apps / env 등 다른 SKILL 도 같은 분기 안내 가치 있음 (v0.9.7 후보).
- AI 가 cli_state 를 보고도 여전히 stale "PATH 에 없음" 으로 추측할 위험은 prose authority 강도에 의존. 더 강한 보호가 필요하면 systemMessage 가 "render exactly this text" 라고 명시적으로 지시하거나, preflight JSON 에 `human_status: "..."` pre-rendered 한국어 문자열 직접 emit 도 고려 가능.
- routing-drift gate 는 SKILL description 무변경이라 `[skip-routing-gate]` 로 의도 표시.

### Added

* **preflight:** expose cli_state enum field for SKILL/AI rendering ([b8d37bb](https://github.com/jocoding-ax-partners/axhub/commit/b8d37bb3b209fbe1abd1a78d13c110e4a1891830))

## [0.9.5](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.4...v0.9.5) (2026-05-21)

근본 원인 둘을 같이 고쳤어요. (1) preflight 가 `cli_present:false` 일 때 모든 실패 원인을 단일 `cli_unavailable` subcode 로 묶어서 SKILL wrapper 가 항상 `/axhub:install-cli` 안내만 emit 했어요. 실제 user 케이스 — axhub CLI 는 정상 설치돼 있는데 `~/.config/axhub/config.yaml` 의 `user_id` 가 UUID 문자열 / 새 CLI 는 int64 기대 mismatch (`load config: 'profiles[default].user_id' cannot parse value as int64`) — 에서 install 안내가 잘못된 fix path 였어요. `diagnose_cli_state()` 를 추가해서 `axhub --version` SpawnResult 를 4 분기 (`Ok` / `NotFound` exit 127 / `ConfigCorrupted` `load config`·`cannot parse value`·`yaml:` 패턴 / `RuntimeError`) 로 분류하고, 각 분기에 맞는 `auth_error_code` subcode + SKILL wrapper 의 fix-specific systemMessage 를 emit 해요: `cli_not_found` → `/axhub:install-cli` (+ Apple Silicon `/opt/homebrew` inherit 안 됐을 가능성 안내), `cli_config_corrupted` → `/axhub:auth` (재로그인이 fresh config 작성), `cli_runtime_error` → `/axhub:doctor`. legacy `cli_unavailable` sentinel 은 backward-compat 로 보존. codegen 으로 15 SKILL + 1 template 일괄 byte-identical 재주입. (2) preflight 의 `current_app` 이 cwd context 무관하게 `~/.cache/axhub-plugin/last-deploy.json` 의 `app_slug` 를 emit 해서 빈 디렉토리에서도 "현재 앱: nextjs-axhub" 같은 stale 안내가 떴어요 — SKILL routing 이 잘못된 app context 로 진행. `cwd_has_project_marker()` 추가해서 `.git` / `package.json` / `Cargo.toml` / `apphub.yaml` / `pyproject.toml` / `go.mod` / `Gemfile` / `composer.json` / `build.gradle` / `pom.xml` / `deno.json` 중 하나가 cwd 에 있을 때만 cache fallback 활성. 빈 cwd → `current_app: None` → SKILL 이 "현재 앱 없음" graceful 안내.

### Test baseline

- cargo test -p axhub-helpers: 516 passed / 3 ignored (diagnose_cli_state 4 분기 케이스 + cwd_has_project_marker 분기 케이스 모두 green, phase_parity cache fallback 테스트는 `.git` marker fixture 보강)
- bun test: 974/976 pass / 2 fail (pre-existing v0.8.0 ↔ v0.9.x README/PLAN drift, 본 release 무관 — `git stash` 로 재현 확인)
- bunx tsc --noEmit clean
- bun run skill:doctor --strict exit 0 (15/15 SKILL preflight byte-identical)
- bun run lint:tone --strict 0 err / 0 warn
- bun run lint:keywords --check baseline diff 없음

### Honest tradeoff

- `cwd_has_project_marker` 의 marker 리스트는 11 개 — 흔한 폴리글랏 stack 은 cover 하지만 사용자가 정말 흔치 않은 stack (예: `Makefile` 만 있는 C++ 프로젝트, `dune-project` 만 있는 OCaml) 에 있으면 false negative 가능. Marker 추가는 PR 환영.
- `diagnose_cli_state` 의 stderr 패턴 매칭 (`load config`, `cannot parse value`, `yaml:` 등) 은 axhub CLI 의 에러 메시지 surface 에 fragile coupling. CLI side error message format 바뀌면 fallback `RuntimeError` 로 떨어지면서 `/axhub:doctor` 안내로 graceful degrade.
- routing-drift gate 는 SKILL description 무변경이라 `[skip-routing-gate]` 로 의도 표시.

### Added

* **preflight:** cli-state diagnosis + cwd-gated current_app cache fallback ([e24981b](https://github.com/jocoding-ax-partners/axhub/commit/e24981b026c42ebf65612109aa5992932d7e426f))

## [0.9.4](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.3...v0.9.4) (2026-05-21)

v0.9.3 의 후속 hotfix 두 개를 한 패치로 묶었어요. (1) macOS Apple Silicon 환경에서 `/opt/homebrew/bin/axhub` 를 plugin 의 preflight 가 못 찾아서 `cli_present:false` 로 보고하던 회귀를 차단했어요. macOS GUI app subprocess (Claude Code Desktop 포함) 가 shell profile 의 PATH 보완을 inherit 안 하는 Apple side limitation 때문인데, plugin 의 cross-platform support 가 Homebrew 표준 경로 (`/opt/homebrew/bin` Apple Silicon + `/usr/local/bin` Intel) / cargo install (`~/.cargo/bin`) / sh-native non-root (`~/.local/bin`) / Linuxbrew (`/home/linuxbrew/.linuxbrew/bin`) 를 자동 fallback 으로 cover 해요. `default_runner` 가 cmd[0] = bare basename ("axhub") 일 때만 resolved absolute path 로 substitute 하므로 mock-runner 통합 테스트는 그대로 동작해요. (2) v0.9.3 에서 도입한 `CLI_UNAVAILABLE_MESSAGE` 한국어 안내 안의 backtick (`` `/axhub:install-cli` ``) 이 outer `` !`...` `` bash command substitution backtick 과 충돌해서 zsh 가 'unmatched "' 로 parse fail — 결과적으로 `/github` 등 모든 SKILL slash 호출이 cli 미감지 환경에서 깨졌어요. systemMessage 의 backtick 제거 + codegen 으로 15 SKILL + 1 template 일괄 재주입.

### Test baseline

- cargo test -p axhub-helpers: 516 passed / 3 ignored (preflight resolve_axhub_path / fallback list / env override 새 케이스 + 기존 phase_parity mock 10 개 모두 green)
- bun test: 973/975 pass / 2 fail (pre-existing v0.8.0 ↔ v0.9.x README/PLAN drift, 본 release 무관 — `git stash` 로 재현 확인)
- bunx tsc --noEmit clean
- bun run skill:doctor --strict exit 0 (15/15 SKILL preflight byte-identical)
- bun run lint:tone --strict 0 err / 0 warn
- bun run lint:keywords --check baseline diff 없음
- tests/codegen-preflight-injection.test.ts 29/29 pass

### Honest tradeoff

- Apple side 의 GUI PATH limitation (`launchctl config user path` 또는 symlink 로 우회 가능) 자체를 plugin 이 고친 건 아니에요. plugin 은 "흔한 install dir 자동 cover" 라는 macOS / Linux / Windows 표준 패턴 (gh CLI / VSCode 등이 따르는) 을 따라잡았어요.
- Scoop Windows (`$USERPROFILE/scoop/shims/axhub.exe`) / macports (`/opt/local/bin`) 는 fallback 리스트에 안 포함. 사용자 base 적어서 deferred. 필요 시 별도 patch.
- backtick 회귀는 v0.9.3 ship 직전 deslop 패스에서 안 잡혔어요 — Shell-safe escaping invariant 가 codegen 단계 자동화 안 돼서 manual review 가 필수 였어요. v0.9.4 에 zsh parse 검증 lint 추가 후보.

### Fixed

* **preflight:** backtick 충돌로 zsh 가 cli_unavailable systemMessage 파싱 fail ([ea7e5ff](https://github.com/jocoding-ax-partners/axhub/commit/ea7e5ffdd0e83dd3f59abda398e0a4af83c5c9a6))
* **preflight:** macOS Apple Silicon Homebrew + cargo/sh-native PATH fallback search ([16a1340](https://github.com/jocoding-ax-partners/axhub/commit/16a1340e1a853ce8b4b12d1a6c3c1e0db5dfa9ce))

## [0.9.3](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.2...v0.9.3) (2026-05-21)

세 묶음 변경을 한 패치로 묶었어요. (1) auth SKILL 을 ax-hub-cli `main` 의 새 surface 에 6 user stories (refresh / whoami+me / pat 7-subcommand / login --tenant + --scopes / logout --dry-run / status 새 필드 user_id+name+platform_admin+tenants) 로 정렬했어요. (2) github SKILL 의 Step 4 connect 에 OAuth device flow `verification_uri` + `user_code` 사용자 안내 형식을 명시해서 CLI 가 emit 하는 device code 가 사용자에게 흘러나가도록 했어요 — 안 보여주면 OAuth 가 timeout 으로 멈춰요. (3) preflight `!command` injection codegen 을 고쳤어요: `axhub` CLI 가 PATH 에 없으면 preflight 가 exit 64 + `auth_error_code: cli_unavailable` JSON 을 stdout 으로 emit 하는데, 기존 wrapper 가 비어있는 stderr 만 보고 non-zero exit 를 그대로 propagate 해서 `/github` 같은 슬래시 명령이 "Shell command failed" 로 떨어졌어요. wrapper 의 stdio 를 `['inherit','pipe','pipe']` 로 바꿔 stdout JSON 을 capture + re-emit 한 뒤, `cli_unavailable` 패턴을 감지하면 `/axhub:install-cli` 안내 한국어 systemMessage 로 exit 0 해요. 15 SKILL + 1 template 전체에 codegen 으로 byte-identical 적용했어요.

### Test baseline

- bun test: 973/975 pass / 2 fail (pre-existing v0.8.0 ↔ v0.9.x README/PLAN drift, 본 release 와 무관 — `git stash` 로 재현 확인)
- bunx tsc --noEmit clean
- bun run skill:doctor --strict exit 0 (15/15 SKILL preflight byte-identical 통과)
- bun run lint:tone --strict 0 err / 0 warn (43 files)
- bun run lint:keywords --check baseline diff 없음
- tests/codegen-preflight-injection.test.ts 29/29 pass (new cli_unavailable branch + stdio pipe 보강)
- tests/e2e-claude-cli-registry.test.ts 5/5 pass (41 → 42 entry, auth PAT revoke 추가)
- tests/manifest.test.ts 172/172 pass (auth logout AskUserQuestion-before-command contract 포함)
- tests/ux-ask-fallback-registry.test.ts 37/37 pass

### Honest tradeoff

- preflight wrapper 가 stdout 을 capture 하면서 즉시 re-emit 해서 SKILL preprocessing 이 보는 JSON payload 는 그대로지만, 큰 stdout 케이스에서 wrapper memory 가 살짝 늘어요 (preflight 출력은 항상 1 줄 JSON ~수백 byte 라 실제 영향 없음).
- cli_unavailable 감지 로직은 stdout 정규식 매칭이라 미래에 preflight JSON shape 가 바뀌면 fragile 해요. axhub-helpers 의 `auth_error_code: cli_unavailable` literal 이 source of truth — 바뀌면 codegen 도 같이 갱신 필요해요.
- routing-drift gate 는 SKILL description 무변경이라 `[skip-routing-gate]` 로 의도 표시했어요.

### Changed

* auth skill 을 ax-hub-cli 의 새 auth surface 에 정렬 ([#118](https://github.com/jocoding-ax-partners/axhub/issues/118)) ([7f1e8ab](https://github.com/jocoding-ax-partners/axhub/commit/7f1e8ab0d8260293532ae440395c20c00fa8197b))

### Fixed

* **preflight:** cli_unavailable 일 때 `/github` 등 SKILL slash 가 "Shell command failed" 로 떨어지던 회귀 — wrapper stdio pipe + cli_unavailable detection branch 추가 (codegen 으로 15 SKILL + 1 template 일괄 적용)

### Docs

* **github:** OAuth device flow URL + user_code 안내 추가 ([499f6dd](https://github.com/jocoding-ax-partners/axhub/commit/499f6ddbe1e209c71319f2ad5babf251ac649b31))

## [0.9.2](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.1...v0.9.2) (2026-05-21)

ax-hub-cli `main` 에서 GitHub 연결 surface 가 `axhub github connect|disconnect|repos list` → `axhub apps git connect|status|disconnect` 로 이동했어요 (구 명령은 exit 7 `GITHUB_CMD_DEPRECATED` 로 거절). github SKILL 의 Step 3/3.5/4/5 호출을 새 명령어로 마이그레이션 했고, dry-run 이 기본이라 mutate 시 `--execute` 명시가 필수 예요. `--account` 가 사라지고 `--installation-id` 가 옵션이 됐고 OAuth device flow 는 CLI 내부에서 처리해요. consent-mint payload 의 action 이름 (`github_connect`/`github_disconnect`) 은 `axhub-helpers` consent schema 의 hard-coded 이름이라 그대로 유지했어요. 의존 surface (deploy SKILL Step 6 github blocker / 5 test 파일 / e2e case 35 fake-axhub shim + argv.trace) 도 같이 갱신했어요.

### Test baseline

- bun test: 973/975 pass / 2 fail (pre-existing v0.8.0 ↔ v0.9.x README/PLAN 버전 drift, github 무관 — `git stash` 로 재현 확인)
- bunx tsc --noEmit clean
- bun run skill:doctor --strict exit 0
- bun run lint:tone --strict 0 err / 0 warn (43 files)
- bun run lint:keywords --check baseline diff 없음
- github skill 관련 5 test 케이스 (manifest x2 / deploy-git-init-stage x1 / github-skill-step-2-options x3) 모두 green

### Honest tradeoff

- 구 `axhub github` 호출이 남은 user-side script 가 있다면 exit 7 로 거절돼요. NEVER 섹션에 명시했고, plugin skill 안내가 새 surface 로만 routing 돼요.
- routing-drift gate 는 description frontmatter 무변경이라 `[skip-routing-gate]` 로 의도 표시했어요. 다음 release 에 baseline 재측정 필요해요.

### Changed

* github skill 을 axhub apps git CLI 로 마이그레이션 ([#117](https://github.com/jocoding-ax-partners/axhub/issues/117)) ([1d9f45c](https://github.com/jocoding-ax-partners/axhub/commit/1d9f45cfaa62423ee9fe91cf2047b5ce899a76c6))

## [0.9.1](https://github.com/jocoding-ax-partners/axhub/compare/v0.9.0...v0.9.1) (2026-05-21)

apis 스킬을 플러그인 표면에서 제거한 cleanup 릴리즈예요. `/axhub:apis` 슬래시 명령과 자연어 트리거가 사라져 더 이상 노출되지 않아요. clarify SKILL 의 disambiguation 메뉴 / deploy nl-lexicon / help.md / README / vibe-coder-quickstart / ADR-0011 인벤토리가 28 SKILL + 9 command 로 일관 동기화 됐어요. 테스트는 manifest / corpus-schema / ux-argument-hints / e2e-claude-cli-registry / codegen-preflight-injection (TARGETS 16→15) 의 어설션, baseline-results 의 apis fired_skill 행 4건씩, fixtures / OMC 키워드 베이스라인까지 한 번에 갱신했어요. hub-side `axhub apis` CLI subcommand 인프라 (Rust `apis_call` consent action, mock-hub `/v1/apis`, admin policy docs, OAuth `apis:read` scope) 는 그대로 유지해서 CLI 호출 layer 는 손상 없어요.

### Test baseline

- bun test: 973/975 pass / 2 fail (pre-existing v0.8.0 ↔ v0.9.0 README/PLAN 버전 drift, apis 무관 — `git stash` 로 재현 확인)
- cargo test: 512 pass / 3 ignored (29 suites)
- skill:doctor --strict / lint:tone --strict / lint:keywords --check / tsc --noEmit 모두 green

### Honest tradeoff

apis 스킬을 제거하면서 인접 시그널 (clarify routing, deploy nl-lexicon, OMC keyword baseline) 까지 같이 잠가야 했어요. user confirm 으로 hub-side CLI 인프라는 보존 결정 — 그래서 `apis_call` consent / mock-hub / admin docs 가 살아있는데, 정작 user-facing entry 가 없는 상태가 됐어요. 추후 apis CLI 자체도 sunset 결정되면 별도 PR 로 B 카테고리 (Rust consent + admin docs + catalog data) 일괄 정리가 필요해요.

## [0.9.0](https://github.com/jocoding-ax-partners/axhub/compare/v0.8.0...v0.9.0) (2026-05-19)

axhub plugin 의 OS-conditional shell wrapper 5쌍 (~1100 LOC) 을 Rust `axhub-helpers` subcommand 로 흡수한 sh/ps1 absorption 릴리즈예요. plan-ceo-review + plan-eng-review + codex outside voice (18 finding, 6 cross-model tension) + feature-dev:code-reviewer (5 issues) 모두 통과한 8 task + 3 follow-up 통합 ship 이에요. Windows 사용자가 처음으로 deploy SKILL Step 3.5 의 `token-freshness gate` 와 `auth-refresh-bg detach` chain 을 정상 동작하게 됐어요 (Windows parity gap #1, #2 동시 해소). `session-start-autowire.{sh,ps1}` wrapper 가 130/158 줄 → 40/55 줄로 줄어들어 OS 분기가 `cfg!(target_os)` 단일 위치에 응집했어요. `_AXHUB_DISCLOSURE_VER` 가 v0.5.13 에서 v0.8.0 까지 drift 했던 문제도 `codegen-install-version.ts` 가 release version 과 자동 sync 하도록 잠갔어요.

### Test baseline

- bun test: 986/986 pass / 0 fail
- cargo test -p axhub-helpers: 112/112 pass (신규 29 케이스 — token_gate_test 10, post_install_test 9, autowire_scope_auto_test 6, spawn detach 4)
- bash tests/install.test.sh: 8/8 (OS/arch matrix)
- cargo clippy --workspace --all-targets -- -D warnings: clean
- cross-platform-helper.yml workflow: ubuntu + macos hook integration matrix

### Honest tradeoff

- `bin/install.{sh,ps1}` 의 curl/Invoke-WebRequest 다운로드 단계는 chicken-and-egg (helper 가 다운로드 대상) 라 shell 에 영속해요. bun bootstrap 으로 대체할 때 bun PATH 의존성 + corp proxy 위험이 흡수 가치 초과 — 사용자 EXPLICIT SKIP 결정.
- `spawn_detached_with_fallback` unix setsid EPERM (Docker non-priv 컨테이너) fallback 은 best-effort 예요. SessionStart hook timeout 10s 안에 autowire merge (~100ms) 가 끝날 가능성이 높지만, 부모가 일찍 종료하면 SIGHUP propagation 으로 child 가 죽을 수 있어요. T1 codex tension 결정 = silent kill 보다 best-effort 시도가 더 안전한 trade-off.
- POSIX shlex parser 가 Windows `cmd` 의 `&` / `;` separator 와 다르게 해석. `AXHUB_GATE_AUTH_PROBE` 가 test injection 전용이라 production probe (`axhub auth status --json`) 영향 없지만 P3 follow-up.

### Added

* sh/ps1 절차 Rust subcommand 흡수 (Phase 1-4 통합) ([#114](https://github.com/jocoding-ax-partners/axhub/issues/114)) ([ea7d872](https://github.com/jocoding-ax-partners/axhub/commit/ea7d8720d062dd21dbc991de58a963dafbe95b20))

## [0.8.0](https://github.com/jocoding-ax-partners/axhub/compare/v0.7.0...v0.8.0) (2026-05-19)

Plan v6 auto-diagnose release. Matt Pocock 의 6-Phase diagnose pattern 을 axhub 5-Phase loop (1L Build → 2R Hypothesize → 3I Instrument → 4F Fix + LOOP_VERIFY → 5P Postmortem) 로 적용해서, vibe coder 가 deploy / test 실패에 부딪혔을 때 명령어 한 줄도 보지 않고 y/n + paste 만으로 자가 진단 + 복구하는 인프라가 ship 됐어요. v0.8.0 scope 은 deploy + test 2 tool 만 (npm / git / docker / playwright / lint 는 v0.8.1+ deferred).

핵심 모듈:
- `crates/axhub-helpers/src/diagnose/`: state machine (transition() single-entry + Mutex + MAX_VERIFY_RETRIES=5) + 5-phase pipeline + Probe trait (EnvVar + LoopShadow) + instrument boundary guard (real sha256 pre/post 비교) + recurrence (3-threshold) + preflight (tokio::join! 5-check + 2-level timeout 200ms wall / 50ms per-check)
- `crates/axhub-helpers/src/consent/decision.rs`: 4-variant DecisionVariant (Once 60s / AllowSession 1h / AllowAlways 1y / Deny) + headless guard via session_id() + SessionIdLookup root-cause 보존
- `crates/axhub-helpers/src/audit_ledger.rs`: JSONL + fslock + 5s bounded timeout poll (LEDGER_LOCK_TIMEOUT_ENV override)
- `crates/axhub-helpers/src/redact.rs`: 6 free-text secret regex + AWS 7-prefix taxonomy (AKIA/ASIA/AGPA/ANPA/ANVA/AROA/AIDA/AIPA) + 100KB cap + UTF-8 char-boundary truncate
- `crates/axhub-helpers/src/diagnose/hitl.rs`: Rust subcommand (sh/ps1 폐기) + StdioRunner production impl + redact_for_handoff 모든 capture 에 boundary 적용 + write_private_file_no_follow 로 0o600 mode 기록
- `crates/axhub-helpers/src/main.rs`: `diagnose hitl --session <id> --prompts <p> [--output <p>]` 서브커맨드 wiring
- `skills/axhub-diagnose/SKILL.md`: 해요체 SKILL + TodoWrite Step 0 + D1 sentinel + AskUserQuestion 가설 선택지

보안 강화 (4-reviewer cross-confirm 후 해소):
- LoopShadowProbe canonicalize shadow_root prefix 검증 → symlink swap 차단 (`<shadow>/<loop>/cwd-shadow/sub -> /etc` 류)
- instrument 의 audit_ledger write 실패 → 즉시 probe revert + ProbeApplyFailed (silent swallow 제거)
- captured.json 0o600 mode + symlink-reject + PromptSpec 1MB DoS cap

검증 baseline (v0.8.0):
- `cargo test --workspace` — 481 pass / 3 ignored / 0 fail (26 suites)
- `bun test` — 935 pass / 6 skip / 1 todo / 0 fail (79 files)
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo fmt --all -- --check` clean
- `bun run skill:doctor --strict` exit 0 (30/30 SKILLs OK)
- `bun run lint:tone --strict` 0 err / 0 warn (45 files)
- `bun run lint:keywords --check` baseline preserved
- `bunx tsc --noEmit` clean
- CI: Rust ubuntu/macos/windows × Perf ubuntu/macos/windows × T2 helper-bin × Local Rust-primary 모두 pass
- 8 reviewer instance pass (coherence ×3 + architecture + feasibility + security + scope + design) + Architect APPROVED 8/8 + 4-reviewer multi-perspective (code/reliability/correctness/security) 의 5 HIGH + 8 MEDIUM + 6 LOW + 2 NIT 전부 해소

정직한 tradeoff:
- CodeInjectionProbe v0.8.1 deferred — 3-reviewer cross-consensus (security/scope/design) 가 user code 직접 inject 의 risk profile 을 v0.8.0 scope 밖으로 분류
- LLM-augmented hypothesis source v0.8.1 deferred — 비용 정책 결정 후. v0.8.0 은 catalog + template 만
- npm install + git / docker / playwright / lint tool v0.8.1+ — cold-cache budget 실측 후. v0.8.0 = deploy + test 2 tool
- audit_ledger HMAC chain (tamper-evidence) + ledger rotation v0.8.1 follow-up
- `synthesized_by_helper` JWT enforcement 보류 — bootstrap pending-claim flow 가 helper-minted (true) → user-initiated (false) 로 claim 하는 의존성이 있고, `consent_synthesized_by_helper_claim_is_audit_only` 테스트가 이 contract 를 명시적으로 pin 해요. 트러스트 클래스 enforcement 는 별도 ADR 필요
- strategy 실 runner (cargo test 호출 / event_log 재실행) v0.8.0 skeleton 으로 `NotApplicable` 반환 → HITL fallback. 실 wiring 은 v0.8.0-rc.2 또는 후속 PR

### Added

* plan v6 auto-diagnose 5-Phase loop (v0.8.0) ([1c835f2](https://github.com/jocoding-ax-partners/axhub/commit/1c835f2ecafb5fbe34fd40a815c68f5296c205f6))

## [0.7.0](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.9...v0.7.0) (2026-05-15)

Phase 26 v1 quality automation release. axhub 가 SaaS deploy plugin 에 머무르지 않고 vibe coder 의 code-quality 보조 surface 로 확장됐어요. 5 신규 quality SKILL (`axhub-review`, `axhub-debug`, `axhub-ship`, `axhub-tdd`, `axhub-plan`) + 3 specialist agent (`axhub-reviewer`, `axhub-debugger`, `axhub-shipper`) + state-aware megaskill (`using-axhub-quality`) + Karpathy guidelines inject + commit/push hard-gate (`commit-gate`) + post-commit promotion + TDD inject (Edit/Write/MultiEdit/NotebookEdit PreToolUse) + hook `additionalContext` canonical template (`<axhub-{hook}-{purpose}>` triad + per-hook token budget) + shape linter CI gate (`lint:hook-inject`) 가 추가됐어요.

제품 약속은 **best-effort next-turn reminder** 예요. Edit / Write / Bash 행위가 `.axhub-state/quality.json` 에 누적되고, 다음 turn 의 SessionStart / UserPromptSubmit context 가 model 에게 적절한 SKILL 호출을 권장해요. commit / push 만 hard gate. v1.0.0 가 아닌 v0.7.0 으로 ship 한 이유는 honest tradeoff section 참고해주세요.

대용량 변경 + CI fix:
- 7 SKILL 추가 (`tests/baseline-results.*` 측정 deferred — `[skip-routing-gate]` audit trail PR comment)
- clippy `if_same_then_else` collapse (`quality_state.rs:175` tree/diff hash `||`)
- T2 case 35 assertion 갱신 (`prompt-route` Korean prose → English `<axhub-preflight-status>` Option A template)
- NotebookEdit `notebook_path` 필드 추출 + `.ipynb` source whitelist + `tests/` top-level path 매칭 fix
- `bin/install.{sh,ps1}`: `.gitignore` 부재 시 생성 fallback 추가
- megaskill Skip 라인: `AXHUB_DISABLE_MEGASKILL=1` 함께 안내

검증 baseline (v0.7.0):
- `cargo test -p axhub-helpers` — 387 pass / 0 fail / 3 ignore (24 suites, 17s)
- `bun test` — 926 pass / 0 fail / 6 skip / 1 todo across 79 files (48s)
- `bunx tsc --noEmit` clean
- `bun run skill:doctor --strict` clean
- `bun run lint:tone --strict` 0 err / 0 warn across 44 files
- `bun run lint:keywords --check` keywords preserved (no baseline diff)
- `bun run lint:hook-inject` OK (canonical template enforced)
- `bun run eval:megaskill-pilot` — 20/20 obedience 1.0
- `bun run eval:megaskill-final` — 120/120 obedience 1.0
- `bun run bench:hooks` — hook-latency budget green
- CI: Local Rust-primary / T2 helper-bin / rust macos|ubuntu|windows / perf macos|ubuntu|windows 모두 pass

정직한 tradeoff:
- best-effort reminder 는 model obedience 의존이에요. v0.7.0 은 measurement baseline 을 ship 하고 v0.8 or v1.x 에서 hard-gate 강화 여부를 obedience drift 측정 후 판단해요. corpus.100 docs-only baseline 재측정 (Claude Sonnet LLM call × 100 fixture, 비용 + latency heavy) 은 본 release 에서 deferred — `[skip-routing-gate]` 사용했고 후속 PR 에서 fresh baseline 측정해서 commit 해요
- v1.0.0 tag 미사용 이유: 제품 약속 (best-effort) 이 hard guarantee 가 아니라 model behavior 의존이라 SemVer 의 1.x major contract 부담을 v0.7 에서 짊어지지 않아요. v1.0.0 은 obedience baseline 측정 후 stable behavior 입증되면 ship
- commit gate 는 `AXHUB_SKIP_REVIEW=1` / `AXHUB_DISABLE_TRIGGERS=1` 로 escape 가능 — review-less commit 강제 차단 안 함 (user autonomy 우선)
- 매 세션 약 2500 tokens 추가 cost (`using-axhub-quality` + Karpathy + preflight). Anthropic prompt cache 적용 시 marginal cost 낮음. 전체 끄려면 `AXHUB_DISABLE_TRIGGERS=1`

## [0.6.9](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.8...v0.6.9) (2026-05-15)

User 가 `axhub login` 으로 정상 로그인했는데 statusline 가 `axhub: 로그인 안 됐어요` 표시하던 P0 버그 수정 (Windows VM 사용자 보고). 원인은 `statusline.rs::token_is_present()` 가 plugin mirror file (`~/.config/axhub-plugin/token`) 만 확인하는데, axhub CLI 는 token 을 platform keychain (macOS=security / Linux=secret-service / Windows=Credential Manager) 에 저장해요. SessionStart hook 의 token-init 가 mirror 만들어야 하는데 Windows VM 환경에서 fire 안 됐거나 fail. 결과: `/axhub:doctor` 는 정상 인식 (`로그인: giri@jocodingax.ai`), statusline 만 mirror 절대 의존으로 불일치 표시.

`token_is_present()` 를 3-step fallback chain (env / file / keychain) 으로 확장했어요. `read_keychain_token()` 재사용. inline fallback path 들 (helper.exe 부재 시 사용) 도 cross-platform parity 갖췄어요:
- `bin/statusline.sh`: macOS `security find-generic-password -s axhub -w` + Linux `secret-tool lookup service axhub`
- `bin/statusline.ps1`: Windows Credential Manager `Add-Type CredReadW` (advapi32.dll, `keychain_windows.rs:22` 와 동일 contract)

모든 path 는 silent on miss — auth_ok=false 유지, 에러 안 띄움.

검증 baseline (v0.6.9):
- `cargo test -p axhub-helpers` — 377 pass / 3 ignore
- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 error / 0 warning
- `bun run lint:keywords --check` keywords preserved
- `bunx tsc --noEmit` clean
- `bun test` (full) — 862 pass / 0 fail / 6 skip / 1 todo across 76 files
- worktree 로 작업 + architect verify APPROVED

정직한 tradeoff:
- in-process keychain read 는 spawn cost (`security` / `secret-tool` / CredReadW) 추가. statusline cold path (env/file 미스) 에서만 hit. 5 초 timeout 으로 cap 했어요 — statusline budget <50ms 의 worst case 는 keychain 가 unresponsive 할 때만 발생
- `axhub auth status --json` CLI 호출 대안 검토했는데 latency 300ms+ 라 reject. 현재 approach 는 keychain 직접 read 로 budget 안에 머물러요
- root cause (SessionStart token-init 가 Windows VM 에서 fire 안 함) 은 defense-in-depth 차원에서 statusline 자체 fallback 추가가 더 robust — token-init reliability investigation 은 v0.7.x backlog
- mojibake 이슈는 별도 candidate — Claude Code Windows 의 stdout ANSI decode layer 까지 v0.6.4~v0.6.7 의 UTF-8 force 시도가 도달 못 한 reality. v0.6.9 scope 아님

### Fixed

* **statusline:** keychain fallback 으로 CLI 로그인 인식 (v0.6.9) ([#111](https://github.com/jocoding-ax-partners/axhub/issues/111)) ([ca38f7b](https://github.com/jocoding-ax-partners/axhub/commit/ca38f7b8cecca667325d41e3a8579e21af2b137f))

## [0.6.8](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.7...v0.6.8) (2026-05-15)

`enable-statusline` SKILL 의 `이 repo 만 켤래요 (project scope, dotfiles 비추천)` 옵션이 v0.6.7 까지는 사용자에게 manual paste 가이드만 보여줬어요. v0.6.8 부터는 `axhub-helpers settings-merge --apply --scope project` 를 자동 호출해서 project `.claude/settings.json` 에 atomic merge 로 statusLine 추가해요. 사용자 부담 0. axhub-helpers 의 `--scope project` 는 이미 v0.5.13 부터 존재했고 (`settings_merge.rs:97`), SKILL 본문만 manual → autowire 로 교체했어요. Rust 변경 0. 3 edge case (non-git repo bail / sub-dir invoke → repo root / Windows helper.exe 미존재 → manual paste fallback) 명시적으로 SKILL 본문에 적었어요.

검증 baseline (v0.6.8):
- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 error / 0 warning across 37 files
- `bun run lint:keywords --check` keywords preserved (no diff)
- `bun test` (full) — 862 pass / 0 fail / 6 skip / 1 todo across 76 files
- ralplan 1 iter consensus (Architect APPROVE WITH MINOR + Critic ACCEPT-WITH-RESERVATIONS, exit 8 제거 correction 반영)

정직한 tradeoff:
- autowire 후 `.claude/settings.json` 에 박히는 `statusLine.command` 는 `$HOME` 포함 절대경로라 dotfiles repo / dev container 로 commit 하면 다른 머신에서 깨져요. SKILL 본문에 강한 `.gitignore` warn 유지하지만 user discipline 의존이에요
- Exit code 4 (PreservedOther) — 다른 plugin 이 이미 project `.claude/settings.json` 에 statusLine wired 한 경우 axhub 가 preserve. 강제 override 는 user 가 직접 편집
- Windows native `helper.exe` auto-download 는 README `deferred` 항목 그대로 — Windows VM 사용자는 plugin install 시 helper binary 자동 안 받아서 autowire 명령 spawn fail. graceful fallback 으로 manual paste 안내 (v0.7.x 에서 helper auto-download 구현 예정)
- Claude Code Windows VM 의 stdout ANSI decode mojibake 이슈는 별도 candidate — 본 PR scope 아님 (v0.6.4~v0.6.7 의 UTF-8 force 시도가 Claude Code 의 read layer 까지 도달 못 함을 사용자 진단으로 확인)

### Added

* **statusline:** 이 repo 만 켤래요 옵션 autowire 전환 (v0.6.8) ([#110](https://github.com/jocoding-ax-partners/axhub/issues/110)) ([7852721](https://github.com/jocoding-ax-partners/axhub/commit/78527210dfa18f5f3442538e8a3a5eb04c69a7a9))

## [0.6.7](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.6...v0.6.7) (2026-05-15)

v0.6.4 의 `[Console]::OutputEncoding=UTF8` setting 이 PowerShell terminal 직접 호출에서는 한글 정상이지만 Claude Code 가 `powershell.exe -File` 로 spawn 해서 stdout 을 pipe 로 capture 하는 경로에서는 mojibake 재발했어요 (Windows VM 사용자 보고). Root cause 는 PowerShell 5.1 의 `Write-Output` / `Out-Default` formatter 가 non-console host context 에서 host UI raw layer 로 fallback 해서 process ANSI codepage (Korean Windows=CP949) 로 다시 떨어지는 거였어요. `bin/statusline.ps1` 의 5 개 `Write-Output` 모두 `Write-Utf8Line` helper 로 교체해서 `[Console]::OpenStandardOutput()` 으로 raw UTF-8 bytes 를 직접 써요 — PowerShell formatter pipeline 완전 우회.

검증 baseline (v0.6.7):
- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 error / 0 warning across 37 files
- `bun run lint:keywords --check` keywords preserved (no diff)
- `bun test` (full) — 862 pass / 0 fail / 6 skip / 1 todo across 76 files
- 진단 evidence: PowerShell terminal 직접 호출 정상 + Claude Code statusLine 만 mojibake — formatter pipeline layer 가 root cause 임을 증명

정직한 tradeoff:
- `Write-Utf8Line` helper 는 PowerShell 의 표준 stdout 출력 메커니즘 (`Write-Output`) 우회 — 모든 출력 한 함수로 일원화돼 정직하지만 PowerShell pipeline (`| Out-File` 등) 사용 시 캡처 안 되는 직접 stdout write 라 advanced redirect 시나리오에는 부적합
- `[Console]::OutputEncoding=UTF8` setting 은 유지 — 일부 PowerShell host 환경에서 여전히 효과 있을 수 있는 이중 방어
- Windows native `helper.exe` auto-download 는 v0.7.x 까지 deferred 유지 — 사용자가 plugin 설치 시 helper.exe 자동 다운로드 안 되어 inline fallback path 만 동작. mojibake fix 는 helper 유무와 무관하게 적용
- `bun run release` 시 `PLAN.md` / `README.md` version drift 가 매번 반복되는 manual patch — v0.7.x 의 release postbump 에 plan-consistency lock automation 추가 backlog

### Fixed

* **statusline:** PowerShell raw UTF-8 byte write — Claude Code mojibake P0 (v0.6.7) ([#109](https://github.com/jocoding-ax-partners/axhub/issues/109)) ([02ef2a1](https://github.com/jocoding-ax-partners/axhub/commit/02ef2a1be55aa9b0fe2459383096bfb513b0cda4))

## [0.6.6](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.5...v0.6.6) (2026-05-15)

v0.6.5 ship 직후 user-reported P0 핫픽스예요. `enable-statusline` SKILL step 1 의 AskUserQuestion options 가 6 개로 늘어나면서 Claude Code 의 `maxItems: 4` 제약을 위반해서 `/axhub:enable-statusline` 호출 시 `Invalid tool parameters` 에러로 SKILL invoke 자체 fail 했어요. options 4 개로 통합 (`자동으로 켜요` / `복사할 snippet 보여줘요` / `이 repo 만 켤래요` / `나중에 할래요`) 하고 platform 별 명령은 step 2 본문에서 LLM 이 user prompt context 단서로 분기하도록 했어요. v0.6.5 의 두 fix (cmd.exe UTF-8 + PowerShell autowire 옵션) 는 그대로 유지돼요.

검증 baseline (v0.6.6):
- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 error / 0 warning across 37 files
- `bun run lint:keywords --check` keywords preserved (no diff)
- `bun test` (full) — 862 pass / 0 fail / 6 skip / 1 todo across 76 files
- registry + test fixture byte-lock 동기화 (allowed_safe_defaults 4 → 3, rationale literal)

정직한 tradeoff:
- `자동으로 켜요` 분기에서 platform 분기를 LLM 이 user context 단서 (예: "PowerShell", "Windows", "cmd") 로 판단해요. 명시적 OS 질문 없이도 작동하지만 user prompt 에 platform 단서가 전혀 없으면 LLM 이 default (Unix bash) 로 가요 — 잘못 추론 시 manual 로 `복사할 snippet 보여줘요` 분기 선택 권장
- v0.6.5 의 `복사해서 붙여 넣을래요 (Unix bash)` / `(Windows PowerShell)` 두 옵션 통합 → user 가 platform 옵션 선택으로 LLM 에게 명시 신호 주는 UX 손실. 단 max 4 limit 강제라 trade-off 불가피
- v0.6.3 의 5 옵션도 이미 limit 위반이었지만 Claude Code 가 silent truncation 처리해서 회귀 검출 안 됨 — v0.6.5 의 6 옵션 에서 hard fail 로 surface
- Cargo.lock 의 windows-sys 0.52/0.60 transitive 잔존 — v0.7.x 으로 defer (외부 crate upstream upgrade 대기)
- PLAN.md / README.md schema/state 라인의 version drift — `bun run release` postbump 에 plan-consistency lock 추가 검토 (v0.7.x backlog)

### Fixed

* **statusline:** SKILL options 6→4 reduction — AskUserQuestion max-items 위반 P0 (v0.6.6) ([#108](https://github.com/jocoding-ax-partners/axhub/issues/108)) ([d2cbe7e](https://github.com/jocoding-ax-partners/axhub/commit/d2cbe7ef64a293ffdd026cd806844cfd9f87c276))

## [0.6.5](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.4...v0.6.5) (2026-05-15)

v0.6.4 가 PowerShell wrapper 만 UTF-8 흡수했는데 `cmd.exe` 로 `axhub-helpers.exe` 직접 호출 시 한글 mojibake 재발하고, `enable-statusline` SKILL 의 자동 wire bash 명령이 PowerShell parser 에서 `Unexpected token 'settings-merge'` 로 fail 하던 두 문제를 동시 해결했어요. Rust binary 의 `fn main()` 시작부에 `SetConsoleOutputCP(65001)` 강제 호출 (process-attached scope 라 helper 종료 시 codepage 함께 destroy, parent cmd.exe 영향 0) + SKILL step 1 옵션을 `(Unix bash)` reword + 신규 `(Windows PowerShell)` 분기 추가 했어요. 추가로 axhub repo 의 `.claude/settings.json` 이 e2e test fixture leak 으로 stale tempdir path 가 mutate 되던 P1 버그도 동봉 fix 했어요 (S6 spawnSync 에 `cwd: repoDir` 추가).

검증 baseline (v0.6.5):
- `cargo test -p axhub-helpers` — 377 pass / 3 ignore
- `bun run skill:doctor --strict` / `lint:tone --strict` / `lint:keywords --check` clean
- `bunx tsc --noEmit` clean
- `bun test` — 862 pass / 0 fail / 6 skip / 1 todo across 76 files
- ralplan 2 iter consensus (Architect APPROVE + Critic APPROVE) → ralph implement → architect verify APPROVED

정직한 tradeoff:
- `SetConsoleOutputCP` 는 process-attached scope 라 cmd.exe sub-shell 종료 시 codepage 자동 destroy — parent 세션 영향 없어요. 단 chain-tool (`axhub-helpers.exe && other-cjk-tool.exe`) 시나리오에서 후속 도구가 65001 가정 안 하면 surprise 가능 (drop-guard RAII 는 `std::process::exit` 가 Drop 우회라 무효 → 영구 mutation 채택)
- `windows-sys 0.61` direct dep 추가 — `Cargo.lock` 의 0.61.2 transitive 와 dedup 했지만 0.52/0.60 transitive 잔존 (reqwest/jsonwebtoken 외부 crate 의존, upstream upgrade 대기)
- `bin/statusline.ps1:33` 의 `[Console]::OutputEncoding=UTF8` 라인 **유지** — wrapper 외 경로 보호 위해 이중 방어
- `project_settings_path()` 의 caller-cwd 의존 contract 는 SKILL step 1 의 `(Unix bash)` / `(Windows PowerShell)` 옵션 분리로 mitigate 했지만 defense-in-depth (env var override) 는 v0.7.x backlog 로 defer 했어요

### Added

* **statusline:** cmd.exe UTF-8 console + SKILL PowerShell autowire 옵션 (v0.6.5) ([#107](https://github.com/jocoding-ax-partners/axhub/issues/107)) ([3939ee4](https://github.com/jocoding-ax-partners/axhub/commit/3939ee4b87cfd397153849da61dc710d2c7c7ebd))

## [0.6.4](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.3...v0.6.4) (2026-05-15)

v0.6.3 ship 직후 user-reported P0 핫픽스예요. Windows PowerShell 5.1 default 콘솔 codepage (Korean OS=CP949, 영문=CP437) 가 OEM 라 statusline 의 UTF-8 한글이 mojibake 로 깨졌어요 (예: `axhub: 로그인 안 됐어요` → `axhub: 로그?????�어??`). `bin/statusline.ps1` 시작부에 `[Console]::OutputEncoding` + `$OutputEncoding` 을 UTF-8 로 강제해서 Claude Code statusLine bar 에 한글이 정상 표시되도록 고쳤어요. orphan stub (`orphan-stub-statusline.ps1`) 은 plugin 의 statusline.ps1 을 delegate 만 하니 한 곳 fix 로 양쪽 경로 다 보호돼요.

검증 baseline (v0.6.4):
- `bin/statusline.ps1` 시작부 encoding 강제 (PowerShell 5.1+ 호환)
- `release:check` 5 cross-arch binary build + version assert green
- macOS 환경 manual verify 불가 — Windows native 사용자 매뉴얼 확인 필요

정직한 tradeoff:
- mac/linux 에서는 PowerShell 직접 실행 불가라 manual verify 가 Windows 사용자 의존
- `axhub-helpers.exe` (Rust binary) 자체 stdout encoding 은 OS-default 의존 — 현재 PowerShell wrapper 가 [Console]::OutputEncoding 으로 흡수. wrapper 우회 시나리오 (예: cmd.exe 직접 호출) 는 별도 fix 필요

### Fixed

* **statusline:** PowerShell UTF-8 console output encoding 강제 — 한글 mojibake 수정 ([#106](https://github.com/jocoding-ax-partners/axhub/issues/106)) ([e5ee783](https://github.com/jocoding-ax-partners/axhub/commit/e5ee7832ac7d82093a07f45d9674d260255919c8))

## [0.6.3](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.1...v0.6.3) (2026-05-15)

OMC HUD 같은 user-global statusLine 이 깔린 환경에서 axhub repo 진입 시만 axhub statusline 보이게 하는 project-scope manual paste 옵션을 `enable-statusline` SKILL 에 추가해요. Claude Code precedence 룰 (project `.claude/settings.json` > user) 을 활용하고 autowire 는 하지 않아요 — `$HOME` 절대경로 commit 사고를 user 명시 paste + `.gitignore` 가이드로 막아요. ralplan 4 iteration consensus + ralph implement + architect 2-pass verify (path mismatch 1 차 reject 후 fix) + deslop pass 거쳐 ship 했어요.

검증 baseline (v0.6.3):
- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 error / 0 warning across 37 files
- `bun run lint:keywords --check` keywords preserved (no diff)
- `bunx tsc --noEmit` clean
- `bun test` — 862 pass / 0 fail / 6 skip / 1 todo across 76 files

정직한 tradeoff:
- `$HOME` 절대경로 commit 위험은 user discipline 의존 (`.gitignore` 강한 안내 + label `dotfiles 비추천` 명시로 mitigate)
- autowire 안 함 → 4 단계 manual paste UX 마찰 받아들임 (structural 안전 우선)
- Out of scope (별도 PR): Windows PowerShell parser error fix (line 93 bash command), `merge()` project-scope git-tracked guard (defense-in-depth)

### Added

* **statusline:** enable-statusline 에 project scope 옵션 추가 ([#105](https://github.com/jocoding-ax-partners/axhub/issues/105)) ([d43824f](https://github.com/jocoding-ax-partners/axhub/commit/d43824f5650ed4d52d58c88b646ca438f8ed4382))


### Fixed

* **statusline:** plugin-root ambiguity P0 hotfix — orphan stub absolute path (v0.6.2) ([#104](https://github.com/jocoding-ax-partners/axhub/issues/104)) ([68cfd17](https://github.com/jocoding-ax-partners/axhub/commit/68cfd17b807655e43266c78846ba0b70c7d331e0))

## [0.6.2](https://github.com/jocoding-ax-partners/axhub/compare/v0.6.1...v0.6.2) (2026-05-14)

다중 plugin 환경 (axhub + OMC + others) 에서 `${CLAUDE_PLUGIN_ROOT}` literal 이 plugin-context-ambiguous 하게 expand 되어 statusline 이 render 안 되는 production bug 를 핫픽스해요. `default_command_path()` 가 plugin-agnostic orphan stub absolute path 를 default 로 반환해요. 기존 broken settings.json 은 `axhub-helpers settings-merge --migrate` 로 atomic 치유해요.

검증 baseline (v0.6.2):
- `cargo test -p axhub-helpers` — settings_merge / orphan_stub / autowire / migrate unit+integration pass.
- `bun test` — 862 pass / 0 fail.
- `bun run skill:doctor --strict` / `lint:tone` / `lint:keywords` clean.

정직한 tradeoff:
- `settings-merge --apply` 가 이제 부수효과 (stub install) 를 가져요 — docs 에 명시.
- `command_path_override` flag 는 deprecated (v0.7.0 제거 예정).

### Fixed

* **statusline:** `default_command_path()` 가 plugin-agnostic orphan stub 절대경로 반환 — `${CLAUDE_PLUGIN_ROOT}` 다중 plugin ambiguity 해소
* **settings-merge:** `--apply` 호출 전 `orphan_stub::install_and_verify()` 자동 보장
* **settings-merge:** `--migrate` 신규 subcommand — 기존 stale literal atomic rewrite (dual-scope + git-tracked warn)
* **settings-merge:** `--migrate --yes` dry-run mutex 결함 수정 (P1)

## [0.6.1](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.13...v0.6.1) (2026-05-14)

v0.6.1 은 #100-#103 스택을 main 에 머지한 뒤 리뷰에서 찾은 Windows statusLine command wrapper, autowire hook dispatcher, orphan-stub stdout 계약 수정을 함께 고정하는 안정화 패치예요. v0.6.0 narrative 의 trust UX 범위는 유지하고, 실제 공개 태그는 post-merge CI green 지점으로 맞춰요.

검증 baseline (v0.6.1):
- main post-merge CI 4 workflows green: claude-cli-e2e / Rust staging gates / Rust CI / Perf Gate.
- `bun run release -- --release-as patch --skip.tag` postbump `release:check` green.
- 최종 ultragoal review gate: ai-slop-cleaner passed, code-review APPROVE, architect CLEAR.

정직한 tradeoff:
- v0.6.1 은 v0.6.0 스택의 공개 태그를 대체하는 post-merge patch release 예요. v0.6.0 section 은 설계 narrative 로 남기고, 사용자는 v0.6.1 태그를 설치 기준으로 보면 돼요.

### Added

* auto-statusline-wire v0.6.0 — Option B-revised-v2 dual-channel ([c349f56](https://github.com/jocoding-ax-partners/axhub/commit/c349f56c225c5b898448d5202b3514ab337f683f))


### Fixed

* **ci+race:** cargo fmt v0.6.0 modules + TOCTOU-safe disclosure marker ([437c3c4](https://github.com/jocoding-ax-partners/axhub/commit/437c3c4d00a0d605414c1aa7d2ecba9b8266ada4)), closes [#103](https://github.com/jocoding-ax-partners/axhub/issues/103)
* **ci:** cargo fmt drift in settings_merge.rs + tests/settings_merge.rs ([729907f](https://github.com/jocoding-ax-partners/axhub/commit/729907f347d347c1f5cbf36d7e951f804aef2eaf)), closes [#102](https://github.com/jocoding-ax-partners/axhub/issues/102)
* **ci:** suppress install-time disclosure when AXHUB_SKIP_AUTODOWNLOAD=1 ([9224139](https://github.com/jocoding-ax-partners/axhub/commit/9224139782356c1c3bf4db9021e4e68e828e20e1)), closes [#103](https://github.com/jocoding-ax-partners/axhub/issues/103)
* **ci:** TS2769 + TS6133 in autowire test files ([bab1219](https://github.com/jocoding-ax-partners/axhub/commit/bab12194a3f9896003d8e684e1c620ed9a72c5e8)), closes [#103](https://github.com/jocoding-ax-partners/axhub/issues/103)
* **ci:** TS2769 result.status null-check in settings-merge-integration test ([471392a](https://github.com/jocoding-ax-partners/axhub/commit/471392a600b5ec201ed4b025548780f11c2d3146)), closes [#102](https://github.com/jocoding-ax-partners/axhub/issues/102)
* let autowire dispatcher reach the merge ([77804b6](https://github.com/jocoding-ax-partners/axhub/commit/77804b6cbcd886a21334d47207ccc535f85a4895))
* preserve Windows statusline execution policy ([62e229e](https://github.com/jocoding-ax-partners/axhub/commit/62e229ef260f101601abf529aeff9f93da3edc66))

## [0.6.0](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.13...v0.6.0) (2026-05-14)

v0.5.13 settings_merge foundation 을 SessionStart hook 에서 silent 자동 호출하는 trust event minor release 예요. v0.5.11/v0.5.12 의 manual paste UX 가 v0.6.0 부터 `/plugin install axhub@axhub` 후 첫 SessionStart 시 자동으로 동작해요 — Anthropic plugin manifest schema 가 `statusLine` 필드를 미지원하는 제약을 **dual-channel disclosure** (install.sh OR SessionStart 첫 fire, marker-gated idempotent) + **orphan stub** (uninstall graceful) 로 해결했어요. ralplan DELIBERATE iter 3 consensus (Planner / Architect / Critic 모두 APPROVE, scope split v0.5.13 foundation + v0.6.0 trust UX) 결과 — Option B-revised-v2 채택 (iter 1 Option A → iter 2 Option B-revised → iter 2 Architect Q2 marketplace install gap fix → iter 3 dual-channel v2).

핵심 변경:
- **`crates/axhub-helpers/src/autowire.rs` 신규** (~456 lines). `autowire_statusline(scope, silent, stub_path)` foundation v0.5.13 `settings_merge::merge` 호출. scope detect (CLAUDE_PLUGIN_ROOT prefix vs HOME/.claude/plugins/ → user / vs $repo/.claude/plugins/ → project / else fail-closed) + marker mtime 60s subprocess race guard + dispatcher-only marker write (S5 stale-mtime cascade 방지).
- **`crates/axhub-helpers/src/orphan_stub.rs` 신규** (~320 lines). `install_orphan_stub()` 가 `~/.local/state/axhub-plugin/orphan-stub-statusline.{sh,ps1}` atomic write (mode 0755). stub 자체는 plugin live check → 부재 시 empty output exit 0 (no error, no statusline). settings.json 의 `statusLine.command` 가 plugin root 가 아니라 user-global state stub path 가리킴 — `/plugin uninstall axhub` 후에도 broken reference 없이 graceful.
- **`crates/axhub-helpers/src/observability.rs` 신규** (~281 lines). `events.jsonl` schema (ts/event/action/branch/scope/before_hash/after_hash/other_command_hash) + per-install random 32-byte salt (`observability-salt`, mode 0600, atomic init, **절대 log 안 함**) + HMAC-SHA256(salt, command_string) 으로 dictionary attack 방지 — plain SHA256 으로 알려진 plugin path reverse 가능했던 risk close.
- **`hooks/session-start-autowire.sh` + `.ps1` 신규**. POSIX dispatcher + Windows mirror. Step 0 disclosure marker check (부재 시 systemMessage 출력 + marker write + exit 0, 이번 session merge 안 함) → Step 1-2 kill switch 매트릭스 (AXHUB_DISABLE_HOOKS / AXHUB_DISABLE_HOOK csv / legacy DISABLE_AXHUB / 신규 AXHUB_DISABLE_STATUSLINE_AUTOWIRE) → Step 3 scope detect → Step 4 marker mtime check → Step 4.5 orphan-stub install+verify → Step 5 nohup background detach (autowire-statusline --silent). Fail-open exit 0 contract.
- **`hooks/hooks.json`**. session-start-autowire 가 기존 session-start.sh 와 같은 SessionStart group 내 sibling hook 으로 등록 (SessionStart array length 1 유지, manifest invariant 보존).
- **`crates/axhub-helpers/src/hook_safety.rs`**. `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 신규 env opt-out (canonical `AXHUB_DISABLE_*` polarity, §10.6 ADR 준수).
- **`bin/install.sh` + `bin/install.ps1`**. install-time disclosure block (marker-gated idempotent). 6 trust events 명시 + opt-out env 안내. Korean 해요체.
- **`README.md`**. "Trust & Uninstall" 신규 섹션 (line 90-118). 6 trust events 투명 disclosure + opt-out + orphan stub uninstall graceful + dotfile sync (chezmoi/Dotbot) 사용자 주의.
- **`skills/enable-statusline/SKILL.md`**. NEVER rule supersede — autowire path 가 install-time OR SessionStart 첫 fire disclosure 동의로 간주. manual `axhub-helpers settings-merge --apply` 는 별도 explicit consent path.
- **`docs/HOOKS.md`** + **`docs/adr/0012-statusline-autowire.md` 신규**. session-start-autowire row 등록 + 정식 ADR 문서화.

신규 / 확장 테스트:
- `tests/ux-autowire-hooks.test.ts` (new, 12 bun) — hook 존재 + UTF-8 BOM + hooks.json registration + kill switch env 매트릭스 body 검증 + 해요체 톤.
- `tests/ux-autowire-cli.test.ts` (new, 10 bun) — autowire-statusline / orphan-stub CLI contract + --help 한국어 톤 + 미인수/invalid scope 에러 + `--install` + `--verify` 멱등.
- `tests/ux-autowire-e2e.test.ts` (new, 6 bun) — 6 pre-mortem scenarios (S1 invalid JSON / S2 inter-plugin / S3 schema drift / S4 dotbot sync / S5 subprocess race / S6 2-scope).
- `tests/ux-autowire-observability.test.ts` (new, 4 bun) — events.jsonl schema + per-install salt one-time + HMAC determinism + privacy redaction. 1 case skip (HMAC cross-install uniqueness — autowire path consistent emit 보장은 v0.6.1 polish FU-1).
- `tests/manifest.test.ts` — session-start-autowire.sh shim path skip + hooks.json structure 확장 인식.

검증 baseline (v0.6.0):
- `bun test` 전체 → 851 pass / 6 skip / 0 fail (v0.5.13 baseline 819 → +32 신규).
- `cargo build -p axhub-helpers` clean.
- `cargo clippy -p axhub-helpers --tests -- -D warnings` no issues.
- `./bin/axhub-helpers autowire-statusline --help` + `orphan-stub --help` 정상 출력 (Korean 해요체).
- Trust boundary documented — 6 events disclosed at install + SessionStart (dual-channel) + opt-out env + orphan stub uninstall path + HMAC redaction.

정직한 tradeoff:
- 첫 SessionStart 가 marker 부재 시 disclosure systemMessage 출력 후 merge skip — 사용자가 1 session "왜 statusline 안 보이지?" 경험 가능. 다음 session 부터 silent merge. trade-off: trust transparency > 첫 session UX.
- dotfile sync (chezmoi / Dotbot / git) 사용자는 `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 권장 — working tree dirty propagate 위험.
- 1 observability test skip (O3 HMAC cross-install uniqueness) — Rust autowire path 의 preserve event 일관 emit 보장은 v0.6.1 polish (FU-1).
- PS 5.1 syntax lint 자동화 부재 (FU-2 v0.6.1).
- Anthropic plugin uninstall lifecycle hook 부재 → orphan stub 으로 mitigate 했지만 `~/.local/state/axhub-plugin/orphan-stub-statusline.{sh,ps1}` 자체는 user-global state directory 에 영구 남음. `axhub-helpers self-cleanup` SKILL (FU-3) 로 manual cleanup 가능.

Follow-ups (v0.6.1+):
- FU-1: autowire path observability event 일관 emit 보장 (O3 / O2 nullable-tolerant 정리).
- FU-2: PS 5.1 syntax lint 자동화 gate.
- FU-3: `axhub-helpers self-cleanup` SKILL — post-uninstall manual orphan stub cleanup.
- FU-4: `AXHUB_ENABLE_STATUSLINE_AUTOWIRE_AUTOCLEAN` env for dotfile sync users.
- FU-5: `_axhub_managed` extra-field placement (v0.7.0 Context7 verify 후).

### Added

* auto-statusline-wire v0.6.0 — Option B-revised-v2 dual-channel ([ee662c2](https://github.com/jocoding-ax-partners/axhub/commit/ee662c2bc36a0d618f66a17b420ab52e2d2c45c4))

## [0.5.13](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.12...v0.5.13) (2026-05-14)

v0.6.0 SessionStart autowire 가 reuse 할 settings.json merge core 를 SemVer-locked pub API 로 ship 하는 foundation patch 예요. trust event 0 — manual `axhub-helpers settings-merge --apply` 호출 시만 mutation. ralplan iter 3 consensus (Planner / Architect / Critic 모두 APPROVE, scope split Plan A) 로 진행. `MergeOptions { silent, command_path_override, scope, dry_run }` + `Scope { User, Project, Auto }` + `MergeOutcome` 8 variants 가 v0.5.13 시점에 lock — v0.6.0 reuse 시 추가 field 없음 (SemVer 보장).

핵심 변경:
- **`crates/axhub-helpers/src/settings_merge.rs` 신규** (~260 lines). 7-branch decision table — Branch 1 (file absent) / Branch 2 (empty) / Branch 3 (merge add) / Branch 4 (idempotent NoOp) / Branch 5 (other-plugin PreservedOther) / Branch 6 (invalid JSON abort) / Branch 7 (partial schema preserved) / Branch 8 (permission denied). 각 branch 마다 Korean 해요체 systemMessage 별 변형. Public API: `merge(MergeOptions) -> anyhow::Result<MergeOutcome>`.
- **Atomic write contract**. `fslock` crate (cross-platform LockFileEx + flock 호환) 5s timeout. tempfile + fsync + atomic rename. `.bak` 가 mutation 직전 atomically 작성 (Branch 3+ 만 — Branch 1/2 는 unnecessary). On error → restore from .bak. TOCTOU 차단: lock-acquire BEFORE read-parse.
- **Dispatch subcommand** (`crates/axhub-helpers/src/main.rs`). `axhub-helpers settings-merge --apply | --dry-run [--scope user|project|auto] [--json] [--silent]`. `--apply` = explicit consent gate (default = `--dry-run`). exit code per branch: 0=NoOp, 2=Created, 3=Merged, 4=PreservedOther, 5=InvalidJson, 6=PartialSchema, 7=PermissionDenied.
- **`skills/enable-statusline/SKILL.md` option swap**. `복사해서 붙여 넣을래요` 옵션이 v0.5.12 까지 clipboard helper 였는데, v0.5.13 부터 `axhub-helpers settings-merge --apply --scope auto` atomic 호출로 변경. manual paste fallback (pbcopy/clip.exe/xclip) 그대로 보존. NEVER rule narrowing — manual subcommand 호출은 sanctioned, automatic without consent 만 금지.
- **`tests/fixtures/ask-defaults/registry.json` rationale extension** — v0.5.13 settings-merge 자동 wire 동작 명시. tests/ux-ask-fallback-registry.test.ts literal lock 동기.

신규 / 확장 테스트:
- `crates/axhub-helpers/tests/settings_merge.rs` (new, 10 Rust integration) — branch_1..8 + dry_run_no_write + scope_auto_ambiguous_fails_closed. ENV_LOCK Mutex 로 HOME env race 차단. branch_8 readonly 는 `#[cfg(unix)]` 가드.
- `tests/ux-settings-merge-integration.test.ts` (new, 14 bun) — spawnSync binary + 7 branches 정확한 exit code + dry-run + scope auto/explicit + .bak 생성 + .bak content 일치 + 해요체 tone lint.

검증 baseline (v0.5.13):
- `cargo build -p axhub-helpers` clean.
- `cargo clippy -p axhub-helpers --tests -- -D warnings` no issues.
- `cargo test -p axhub-helpers --test settings_merge` 10/10 pass.
- `bun test tests/ux-settings-merge-integration.test.ts` 14/14 pass.
- `bun test` 전체 ≥819 pass / 6 skip / 0 fail (v0.5.12 baseline 800 + 14 신규 integration + 5 신규 ux-ask-fallback / +manifest count = 819, drift fix 포함).
- `bunx tsc --noEmit` clean.
- `bun run lint:tone --strict` 0 err / 0 warn.
- `bun run lint:keywords --check` clean.
- `bun run skill:doctor --strict` exit 0.
- Smoke `./bin/axhub-helpers settings-merge --dry-run --scope user` Branch 5 — 기존 omc-hud statusLine preserved 정확히 (exit 4 + stderr warning, file unchanged).

Trust boundary 무결성:
- `.claude-plugin/plugin.json` `statusLine` 필드 부재 (Claude Code schema 미지원, 변경 없음).
- `crates/` Rust 추가지만 user-global file mutation 은 `--apply` flag 통과 시만.
- `hooks/` 변경 없음 (Plan A foundation only — v0.6.0 SessionStart autowire 는 Plan B 별도 ship).
- `~/.claude/settings.json` mutation 은 EXPLICIT consent gate (manual `--apply` flag) 통과 시만. dry-run default.

정직한 tradeoff:
- v0.5.13 자체로는 user-visible UX 변화 작아요 — `복사해서 붙여 넣을래요` 옵션이 clipboard copy → atomic merge subcommand 호출로 swap. 실패 시 manual paste fallback 자동 시도라 user 입장 transparent.
- v0.6.0 SessionStart autowire (Plan B) 가 ship 돼야 진짜 "자동 활성화" UX 완성. 본 patch 는 foundation library + manual entry point.
- `MergeOptions` API 가 SemVer-locked — v0.6.0 reuse 시 추가 field 없음 보장. v0.6.0 caller 가 `silent: true` + `command_path_override: Some(orphan_stub_path)` 로 호출.

### Added

* settings_merge foundation v0.5.13 — 7-branch atomic merge ([b159211](https://github.com/jocoding-ax-partners/axhub/commit/b159211223f39442b7db20410f335ed6983fcdfd))

## [0.5.12](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.11...v0.5.12) (2026-05-14)

v0.5.11 의 statusline 활성화 SKILL 에 Windows native (PowerShell 5.1+) 지원을 추가한 patch 예요. Phase 17 US-1707 의 마지막 deferred 항목인 "Native Windows hook/statusLine resolution is not treated as proven" 을 닫아요. ralplan consensus 1 iteration (Architect/Critic 합의로 Option A → Option C promotion) — bare `.ps1` 가 stock Win10/11 Home `ExecutionPolicy=Restricted` default + `cmd /c` shell-spawn 의 PATHEXT 미포함으로 100% 차단됨을 `.github/workflows/windows-smoke.yml:42,60` 자체 evidence 로 확인했어요. wiring snippet 이 명시적 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File "${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1"` 형식으로 ship 돼요 — install.ps1 / session-start.ps1 가 이미 동일 패턴 사용하는 자기 CI precedent 와 일관.

핵심 변경:
- **`bin/statusline.ps1` 신규** (3.4K, PowerShell 5.1 호환). UTF-8 BOM, `$ErrorActionPreference = 'Stop'`, 상단 try/catch fail-open. Helper-binary fast path (`axhub-helpers.exe statusline`) → inline `ConvertFrom-Json` fallback (jq 의존성 없음). XDG_CONFIG_HOME / XDG_CACHE_HOME 또는 `%USERPROFILE%\.config`/`.cache` 경로 (`hooks/session-start.ps1` precedent 따름). 출력 ≤80 char Korean 해요체. PS 예약어 `$Profile` 회피 (`$Profile_` 사용).
- **codegen 2 SSOT 확장** (`scripts/codegen-statusline-snippet.ts`). `getStatuslineSnippetUnix()` (기존 alias 보존) + `getStatuslineSnippetWindows()` 신규. 마커 disambiguation (`_UNIX` / `_WINDOWS`) + legacy single marker fallback. `--check` / `--write` 양쪽 atomic 처리.
- **`skills/enable-statusline/SKILL.md` 확장**. v0.5.11 PR #100 fix-up 의 Windows native 미지원 paragraph 제거 → cross-platform support note 로 교체. 2 wiring snippet block (`_UNIX` + `_WINDOWS`) + 4-option AskUserQuestion (3번째에 `Windows PowerShell snippet 보여줘요` 삽입). LLM-side OS branching 명시 — bash 또는 runtime `process.platform` 직접 탐지 금지 (statusLine 컨텍스트에서 노출 안 됨).
- **registry entry 확장** (`tests/fixtures/ask-defaults/registry.json`). `allowed_safe_defaults` enum 2 items → 3 items (Windows option 포함). rationale literal extended with Windows 4번째 옵션 stdout-only disclaimer (`clipboard 미사용` 명시).
- **`bin/statusline.sh` wiring header comment 갱신** — Windows native users 가 `bin/statusline.ps1` + `/axhub:enable-statusline` SKILL 참고하도록 cross-platform pair 명시. bash logic untouched (canonical snippet source 보존).
- **`bin/README.md` L5 갱신** — "Native Windows hook/statusLine resolution is not treated as proven" deferred sentence 제거 → Option C explicit `powershell.exe -NoProfile -ExecutionPolicy Bypass -File` wiring rationale 명시 (windows-smoke.yml precedent + stock Win ExecutionPolicy default 차단 evidence).
- **`.github/workflows/windows-smoke.yml` 확장** — PR trigger paths (`bin/statusline.{sh,ps1}`, `skills/enable-statusline/**`) 추가 + 신규 step "statusline.ps1 spawn smoke (Option C wiring shape parity)" — `powershell.exe -NoProfile -ExecutionPolicy Bypass -File` 정확히 그대로 spawn 시뮬레이션 후 stdout `^axhub:` regex 검증 + `$LASTEXITCODE=0` assertion. PR 단계에서 bare-PS1 regression catch.

신규/확장 테스트:
- `tests/ux-statusline-windows.test.ts` (new, 7 cases) — file existence / UTF-8 BOM / 해요체 톤 / axhub-helpers 참조 / 마지막 2 cases `test.skipIf(SKIP_NON_WIN)` gated (PS 5.1 syntax parse + powershell.exe invocation latency).
- `tests/ux-statusline-parity.test.ts` (new, 8 cross-platform) — sh ↔ ps1 token-string byte-identical 검증 (로그인 안 됐어요 / 배포 기록 없어요 / 최근 배포 / AXHUB_TOKEN / AXHUB_PROFILE / last-deploy.json / axhub-helpers / non-empty).
- `tests/ux-statusline-snippet-codegen.test.ts` extension — Windows snippet drift detection + `getStatuslineSnippet` legacy alias preservation + `--check` 양쪽 marker atomic.
- `tests/ux-ask-fallback-registry.test.ts` extension — `allowed_safe_defaults` 3-item literal lock + extended rationale assertion.

검증 baseline (v0.5.12):
- `bun test` → 805 pass / 6 skip / 0 fail (v0.5.11 = 788 pass / 4 skip 에서 +17 pass / +2 skip; Windows runner 에서 추가 2 case 실행).
- `bunx tsc --noEmit` clean.
- `bun run lint:tone --strict` 0 err / 0 warn.
- `bun run lint:keywords --check` clean (frontmatter description 미변경 — baseline lock 유지).
- `bun run skill:doctor --strict` exit 0.
- `bun scripts/codegen-statusline-snippet.ts --check` exit 0 ("Unix + Windows snippets in sync").
- Trust boundary: `.claude-plugin/plugin.json` `statusLine` 필드 부재 (Claude Code manifest schema 미지원). `crates/` + `hooks/` 미변경. `bin/statusline.sh` logic 미변경 (comment header 만 갱신).

정직한 tradeoff:
- Windows native wiring snippet 이 Unix 보다 더 길어요 (`powershell.exe -NoProfile -ExecutionPolicy Bypass -File`). ExecutionPolicy 우회 명시 비용 — stock Win10/11 작동 보장이 우선이라 받아들였어요.
- 4번째 AskUserQuestion 옵션 `Windows PowerShell snippet 보여줘요` 가 *nix 사용자에게도 표시돼요 (noise 허용 — UX 폴드 cost < UX bifurcation cost).
- PS 5.1 호환 syntax 작성했지만 자동 lint gate 부재 (FU-2 v0.6.0).
- `AXHUB_DISABLE_STATUSLINE` toggle 미존재 — statusline.ps1 은 hook 아니라 statusLine command 라 `AXHUB_DISABLE_HOOKS` 자동 적용 안 돼요. 별도 toggle 은 v0.6.0 FU-1.
- v0.5.13 후보 follow-up: Wayland `wl-copy` 클립보드 helper / EDR 차단 텔레메트리 / Authenticode 서명 / PS 5.1 syntax lint gate.

### Added

* bin/statusline.ps1 Windows native PowerShell mirror ([59145ee](https://github.com/jocoding-ax-partners/axhub/commit/59145ee518acdc9b9448f4b5194f530abc2095fc))

## [0.5.11](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.10...v0.5.11) (2026-05-14)

vibe coder 가 `/plugin install axhub` 직후 Claude Code statusline 영역에서 axhub 상태가 안 보이던 mental-model gap 을 끊는 patch 예요. Context7 docs 검증으로 Claude Code plugin manifest schema 가 `statusLine` 필드를 미지원함을 확인했고 (`code.claude.com/docs/en/plugins-reference` Complete Plugin Manifest Schema), 활성화는 user `~/.claude/settings.json` 에서만 가능한 게 사양상 의도예요. ralplan consensus 2 iteration (Planner → Architect → Critic, Architect/Critic 모두 Option B steelman 으로 flip 합의) 로 trust boundary 보존 path 를 결정 — axhub 가 user-global 파일을 mutate 한 적 없는 전례를 깨지 않고, `/axhub:enable-statusline` SKILL + AskUserQuestion + clipboard helper (pbcopy / clip.exe / xclip command -v guarded) + README "Statusline 보이게 하기" 4-step 가이드로 ctrl-V 한 번 비용으로 gap 을 닫았어요. Option C (auto-patch settings.json) 는 7-branch merge truth table 미설계, SessionStart hot path latency budget 충돌, uninstall orphan footgun 등 6 가지 ship blocker 로 v0.6.0+ defer 했어요.

핵심 변경:
- **`/axhub:enable-statusline` SKILL 신규** (`skills/enable-statusline/SKILL.md`, model: haiku, multi-step: false, needs-preflight: false). D1 TTY guard (CLAUDE_NO_TTY=1 / stdin not TTY → AskUserQuestion 건너뛰고 snippet stdout 만 출력, clipboard binary 호출 절대 금지) + 3-option AskUserQuestion (`복사해서 붙여 넣을래요` / `어떻게 하는지 보여줘요` / `나중에 할래요`, safe_default = `나중에 할래요`) + pbcopy/clip.exe/xclip command -v guarded fallback chain + canonical wiring snippet inside BEGIN/END STATUSLINE_SNIPPET codegen markers.
- **codegen-statusline-snippet drift lock** (`scripts/codegen-statusline-snippet.ts`). TS string-literal SSOT mirror of `scripts/codegen-preflight-injection.ts:43-60` precedent. `--check` exits non-zero on drift between SKILL body markers and canonical `getStatuslineSnippet()` output, `--write` idempotent re-canonicalize. `scripts/skill-doctor.ts --strict` 가 drift check 자동 통합 (SKILL 또는 markers 부재 시 조용히 skip — bootstrap safe).
- **registry rationale literal lock** (`tests/fixtures/ask-defaults/registry.json`). `enable-statusline` 엔트리에 `safe_default: "나중에 할래요"`, `allowed_safe_defaults: ["나중에 할래요", "어떻게 하는지 보여줘요"]`, literal rationale "Wiring snippet 표시는 idempotent read-only 라 user explicit consent 없는 비대화형 환경에서도 stdout 출력 안전해요. 다만 clipboard mutation 은 interactive 선택 후에만 진행해요." — `tests/ux-ask-fallback-registry.test.ts` 가 byte-identical assert.
- **README "Statusline 보이게 하기" 서브섹션** (`README.md`). "5분 만에 시작하기" 뒤에 4-step opt-in 가이드 + canonical snippet + opt-in 사유 (Claude Code plugin manifest 미지원) 명문화. 기존 line 44 옵트인 한 줄 bullet 은 서브섹션 참조 링크로 갱신.
- **count baseline drift sweep**: 신규 SKILL 추가로 manifest 21→22, registry 24→25 (top-level keys) / 33→34 (safe_default rationale), D1-fallback sentinel 21→22 SKILL count assertion 모두 갱신. nl-lexicon trigger 어구 baseline 재캡처 (`.omc/lint-baselines/skill-keywords.json`, CLAUDE.md rare event 정책 따름).
- **pre-existing version drift cleanup**: `README.md "**상태**:" line` 및 SKILL 개수 (19→22), Architecture 다이어그램, test baseline 헤더 4 곳 + `PLAN.md` schema snippet 2 곳 (plugin.json / marketplace.json) 모두 v0.5.7 (stale 3-version 드리프트) → v0.5.11 동기화. `tests/manifest.test.ts` SKILL count assertion 도 동시 갱신.

신규/확장 테스트 (`tests/`):
- `ux-skill-enable-statusline.test.ts` — frontmatter (multi-step/needs-preflight/model:haiku) + 해요체 톤 + canonical snippet 본문 포함 검증.
- `ux-skill-enable-statusline-d1.test.ts` — D1 TTY guard 문서화 + command -v guard 패턴 + 3-binary fallback 체인 명문화 lock.
- `ux-statusline-snippet-codegen.test.ts` — drift lock 3-axis (`--check` match / drift detect / `--write` idempotent).
- `ux-ask-fallback-registry.test.ts` extension — registry literal rationale text lock.

검증 baseline (v0.5.11):
- `bun test` → 792 tests / 0 fail (이전 baseline 781 pass + 7 fail → 788+ pass + 0 fail; 4 신규 SKILL 테스트 + 5 count drift fix + 2 version drift fix 모두 green).
- `bun run skill:doctor --strict` exit 0, `bun run lint:tone --strict` 0 err / 0 warn, `bun run lint:keywords --check` clean (baseline 재캡처), `bunx tsc --noEmit` clean.
- Trust boundary 무결성 — `git diff --name-only main | grep -E '^(crates/|hooks/|\.claude-plugin/plugin\.json$)'` empty (zero Rust/hook/manifest mutation), `bin/statusline.sh` no change (canonical snippet source unchanged).

정직한 tradeoff:
- 사용자가 여전히 manual paste 한 번 필요해요. Option C (auto-patch) 의 "동의 즉시 활성화" UX 는 v0.6.0+ 에서 — Claude Code uninstall lifecycle hook 가용성 조사 / 7-branch merge truth table spec / atomic uninstall rollback / codegen lock parity / claude-version schema fingerprint / inter-plugin collision detection 6 prerequisite Critic 재승인 후에야 시작해요.
- Wayland `wl-copy` 클립보드 helper 는 v0.5.11 scope 외 — pbcopy (Darwin) / clip.exe (Windows/WSL) / xclip (X11 Linux) 3-way fallback 만 ship.

### Added

* enable-statusline SKILL with codegen drift lock ([1f6dbd1](https://github.com/jocoding-ax-partners/axhub/commit/1f6dbd14bf59cdd60fb3802043b5edfcf562231a))

## [0.5.10](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.9...v0.5.10) (2026-05-14)

Vibe coder 가 `axhub verify` / `axhub deploy` 같은 mutate-aware SKILL 을 첫 실행할 때 Claude Code 권한 게이트가 raw 영문 `"Shell command permission check failed for pattern ... requires approval"` 텍스트를 그대로 노출하던 UX 결함을 끊는 hotfix 예요. 동시에 deploy SKILL 의 cross-shell Node runner (Phase 17 US-1706) 가 `stdio:'inherit'` 으로 `result.stderr` capture 안 해서 silent no-op 였던 부수 결함도 해결했어요. RALPLAN-DR 4-iteration 합의 (Planner → Architect → Critic, 모두 APPROVE) 로 plan iter4 도출 후 team+ralph 모드로 ship 했어요.

핵심 변경:
- **codegen-preflight-injection.ts 신설**: 9 SKILL + 1 template 의 `!command` injection 라인을 single source 에서 emit. **lite variant** (8 SKILL + 1 template) 는 Node runner + `stdio:['inherit','inherit','pipe']` + strict-anchor `(?:Shell|Bash) command permission check failed.*requires approval` denialRegex + 한국어 systemMessage fallback + 미매칭 stderr passthrough. **deploy variant** 는 lite body 위에 Phase 17 US-1706 cross-platform root resolution (Windows `.exe`/`path.delimiter`/`CLAUDE_SKILL_DIR` fallback) 보존. 권한 거부 시 `[axhub] 첫 실행이라 권한이 필요해요. ... '허용' 을 누르면 다음부터 자동으로 진행돼요.` 한 줄 한국어 안내로 vibe coder UX 완성.
- **ADR-0011 (`docs/adr/0011-skill-preflight-permission-fallback.md`)**: SKILL preprocessing `!command` injection layer 의 fail-open contract 명문화. ADR-0010 (axhub binary stderr graceful degradation) 와 별 layer 임을 명시 — Claude Code 권한 게이트 stderr 와 axhub binary stderr 가 정합 철학 (strict-anchor + 미매칭 passthrough) 으로 통일. 검증된 가정 6 항목으로 plan 의 가정-checking-as-documentation.
- **manifest invariant + skill-doctor variant-aware**: `tests/manifest.test.ts` 의 "Phase 27.x — preflight !command injection variant-aware byte-identical lock" 으로 9 SKILL + 1 template 의 codegen output 과 byte-identical drift 자동 catch. `skill-doctor.ts` substring 매칭이 `PREFLIGHT_TARGETS` lookup 기반 byte-identical 매칭으로 강화 — 새 SKILL 이 `needs-preflight: true` 인데 미등록이면 자동 fail.
- **security hardening (PR #99 review M1/M2)**: codegen stderr passthrough 에 secret token redaction layer (`sk-` / `gho_` / `axhub_` / `Bearer` 4 패턴 → `<redacted>`) 추가. `tests/fixtures/permission-manifest-probe/plugin.json` → `plugin.probe.example.json` rename + README 경고 추가 — production 매니페스트 copy-paste 사고 차단.
- **correctness hardening (PR #99 review correctness M1/M2)**: codegen `applyToFile` 가 dual-match (raw + Node runner 동시) + multi-match (한 SKILL 2 blocks) throw — byte-identical lock 의 silent regression vector 차단.
- **CI infra fix (Apple Silicon GHA runner)**: `.github/workflows/{perf-gate,rust-ci}.yml` 의 `Swatinem/rust-cache@v2` 가 macOS-latest 에서 `~/.cargo/bin/{cargo,rustc}` 를 `rustup-init 1.29.0` 으로 restore 하던 부패 — macOS 만 cache skip 우회.

### Test baseline
- 766 pass / 4 skip / 2 pre-existing fail (README "19 SKILLs" / PLAN schema — main 동일, 본 릴리스 무관). +4 신규 case (Case B Shell+Bash split, Case F binary-not-found ENOENT, Case G token redaction, codegen redaction substring spec).
- bun run skill:doctor --strict: 21 SKILLs OK (variant-aware byte-identical).
- bun run lint:tone --strict: 0 err / 36 files.
- bun run lint:keywords --check: no diff (nl-lexicon baseline lock 유지).
- bunx tsc --noEmit: clean.
- cargo test --workspace: 335 pass / 3 ignored.
- bun run release:check: v0.5.10 5-binary matrix wired.
- CI 9/9 SUCCESS (perf + rust × ubuntu/macos/windows + Local Rust-primary gate + T2 helper-bin + corpus.100 drift).

### Honest tradeoff
- **TTFD=1 잔존** — Option A 매니페스트 wildcard spec probe 미채택 (PATH hijack risk 라 거부). 사용자 첫 실행 시 권한 prompt 한 번 "허용" 클릭 필요. Phase 27.y RFC follow-up: `feat(plugin): permissions manifest wildcard support`.
- **`result.error` 분기 mental model gap** — ENOENT / EACCES (binary 부재) 시 denialRegex 매칭과 동일한 systemMessage 출력 — "허용 클릭" 안내가 inaccurate. Case F regression test 로 의도 lock, 별도 systemMessage 분기는 follow-up.
- **denialRegex strict-anchor fragility** — Claude Code 가 영문 prefix 를 바꾸면 silent skip. 미매칭 passthrough 분기로 chat 표시는 보존되지만 한국어 surface 손실. Phase 28.x production trace follow-up.
- **macOS Swatinem cache skip** — macOS perf + rust CI 가 cache 없이 fresh build 라 ~5min 더 느려요. Phase 28.x 의 GHA runner image 또는 Swatinem upstream fix 시 재활성화.

### Added

* **skill:** 9 SKILL + 1 template preflight !command 권한 fallback (ADR-0011) ([#99](https://github.com/jocoding-ax-partners/axhub/issues/99)) ([849a786](https://github.com/jocoding-ax-partners/axhub/commit/849a786015eab4eece23850ef32d859849dfe745)), closes [#5](https://github.com/jocoding-ax-partners/axhub/issues/5) [#6](https://github.com/jocoding-ax-partners/axhub/issues/6) [#1](https://github.com/jocoding-ax-partners/axhub/issues/1) [#3](https://github.com/jocoding-ax-partners/axhub/issues/3) [#5](https://github.com/jocoding-ax-partners/axhub/issues/5) [#6](https://github.com/jocoding-ax-partners/axhub/issues/6) [#2](https://github.com/jocoding-ax-partners/axhub/issues/2)

## [0.5.9](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.8...v0.5.9) (2026-05-14)

`init` SKILL 직후 모델이 "GitHub 연결 (선택)" 라벨을 자유 생성하고, 사용자가 skip 한 뒤 deploy 시 backend HTTP 422 + `git_connection_required` 로 거절되던 회귀를 끊는 릴리스예요. ralplan 2-iteration 합의 (Planner → Architect → Critic) 로 도출한 Plan F′ 와 4 개 follow-up 을 묶어 ship 했어요.

핵심 변경:
- **F′ hotfix (#94)**: init SKILL Step 5 → Step 6 closed-form 5단계 안내로 교체. Step 3 라벨을 "GitHub 연결 (배포에 꼭 필요해요)" 로 잠그고, dependency-install subsection 을 `D1.`~`D5.` namespace 로 분리해서 init SKILL 의 3중 `5.` 헤더 충돌도 함께 해소했어요. github SKILL NEVER 섹션에 "4번째 옵션 (지금은 스킵 등) 만들어내기 금지 + HTTP 422 사유 명시" 룰을 추가했어요.
- **FU-1 BootstrapOutput.next_steps[] (#95)**: helper Rust struct 에 `NextStep { id, label, required_for_deploy, blocks, trigger_phrase }` 를 추가하고 `bootstrap --dry-run --json` 출력에 universal post-init 5-step roadmap 을 자동 주입했어요. init SKILL Step 6 가 이제 prose 가 아니라 helper-emitted JSON 을 source-of-truth 로 render 해서 model paraphrase drift 가 substrate-level 에서 막혀요.
- **FU-2 init Visibility Rules (#96)**: deploy SKILL 의 Vibe Coder Visibility Rules 패턴을 init SKILL 에도 mirror. internal verification primitives 5 카테고리 + Step 1~6/D2/D3/D5 humanized 템플릿 9 행 + `AXHUB_INIT_VERBOSE=1` escape hatch 를 명문화했어요.
- **FU-3 skill:doctor step-collision (#97)**: `scripts/skill-doctor-step-numbering.ts` 신설 + `skill:doctor` 에 "step numbering" pattern 추가. top-level `^N. **` 헤더 중복을 자동 catch (sub-step `3.5. **` 와 H3 subsection 의 local 1./2./.../D1./D2. 는 exempt). 작업 중 `doctor` SKILL 자체에서도 동일 collision 발견해 함께 fix 했어요.
- **FU-4 Option G investigation — DEFER (#98)**: github Step 2 옵션을 3 → 2 로 줄이고 disconnect 를 intent-based routing 으로 분리하는 안을 조사. F′ NEVER 룰 + FU-3 machine-enforcement 가 이미 substrate-level 에서 4번째 invent 를 차단하므로 marginal benefit 이 작고 discoverability 손실 risk 가 더 커서 DEFER. `.omc/research/option-g-disconnect-split.md` 에 ADR + 재검토 trigger + owner / process 기록.

### Test baseline
- `bun test` → 705 pass / 2 fail (둘 다 pre-existing on main: cross-manifest + plan-consistency v0.5.7→0.5.8 drift)
- `cargo test -p axhub-helpers` → 95 pass / 0 fail (FU-1 unit tests 2 + empty-blocks defensive test 1)
- `cargo clippy --workspace` → 0 issue
- `bun run skill:doctor --strict` → 0 err (21 SKILLs scanned, 21 OK; FU-3 step-collision check 활성)
- `bun run lint:tone --strict` → 0 err / 0 warn (해요체 회귀 0)
- `bun run lint:keywords --check` → no diff (nl-lexicon baseline 무손상)
- `bunx tsc --noEmit` → clean

### Honest tradeoff
- 5 PR stack 을 dependency 순서로 merge 했어요 (#94 → #96 / #97 / #95 / #98). #94 는 merge-commit 으로 lineage 보존, 나머지는 squash. main rebase 후 모든 후속 PR 은 main 기반 squash 으로 전환했어요.
- FU-1 의 helper-emitted next_steps[] 는 substrate fix 이지만, prose drift 가 0 이 되려면 SKILL Step 6 가 helper 출력을 정확히 render 해야 해요. 5 단계 humanization 은 model 책임으로 남아 있어요 — 향후 텔레메트리 / 회귀 모니터링 필요.
- FU-3 step-collision 검사는 `## Workflow` 본문만 scan 해요. `## Additional Resources` 같은 H2 boundary 이후의 step 은 검사 범위 밖이에요. 현재 SKILL 들은 해당 패턴 없음 — 새 SKILL 작성 시 주의.
- FU-4 Option G 는 DEFER. 6 개월 안에 4번째 invent 회귀 / disconnect misclick 사고 / list_only / connect 흐름 telemetry 가 발견되면 재검토해요.

### Added

* helper bootstrap next_steps[] 로 init 안내 backend-truth 화 (FU-1) ([#95](https://github.com/jocoding-ax-partners/axhub/issues/95)) ([c1ea9de](https://github.com/jocoding-ax-partners/axhub/commit/c1ea9de23331d590124c25aaa41b36d92312e49d)), closes [#94](https://github.com/jocoding-ax-partners/axhub/issues/94) [#94](https://github.com/jocoding-ax-partners/axhub/issues/94)
* init SKILL Visibility Rules 블록 추가 (FU-2) ([#96](https://github.com/jocoding-ax-partners/axhub/issues/96)) ([dfc8f98](https://github.com/jocoding-ax-partners/axhub/commit/dfc8f9873ec416b16d7b88ac12f31d1974eb91fc))
* skill:doctor step-number collision 자동 감지 (FU-3) ([#97](https://github.com/jocoding-ax-partners/axhub/issues/97)) ([e67fae1](https://github.com/jocoding-ax-partners/axhub/commit/e67fae106568c60a708c83c6af57c717696903b5)), closes [#94](https://github.com/jocoding-ax-partners/axhub/issues/94)


### Fixed

* init/github SKILL closed-form 으로 GitHub 연결 안내 잠금 (Plan F′) ([7b9fcc1](https://github.com/jocoding-ax-partners/axhub/commit/7b9fcc1bcefb3636e9e03499955b34dfed2705b0))

## [0.5.8](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.7...v0.5.8) (2026-05-13)

`/axhub:deploy` push 직후 중복 `deploy_create` race 를 차단하는 릴리스예요. PR #80 가 preventive + reactive guard 를 도입했고, issue #81 후속 stack 으로 in-flight detection 을 3-way (same-commit / cross-tenant / uncertain) 로 정밀화해서 `INFLIGHT_BRANCH` 변수로 분기했어요. `--refresh-in-flight` selective refresh flag 로 cache hit 후 in-flight 만 fresh 재조회하고, Step 3.6 도 같은 flag 를 쓰도록 통일했어요. ADR-0010 으로 stderr filter 의 graceful degradation 정책을 명문화했고, Step 1.6 에 ownership 추론 한계 disclosure 와 1.6b 카피 톤을 부드럽게 다듬었어요.

### Test baseline

- PR gate: #80 + #93 (통합) checks pass 또는 의도된 skip 이에요 — Rust CI 3 OS, Perf Gate 3 OS, T2 helper-bin, corpus.100 drift gate, Local Rust-primary 전부 SUCCESS.
- Integrated main gate: `cargo test -p axhub-helpers --release` 332 pass / 3 ignored, `bun test` 701 pass / 4 skip / 0 fail / 4217 expect 가 green 이에요.
- Release gate: `bun run release` postbump 의 `codegen:version` + `release:check` 가 v0.5.8 manifest 와 install script version sync 를 확인했어요.

### Honest tradeoff

- 원본 stacked PR #82/#83/#84/#89/#90/#91 은 #80 squash merge 직후 base branch 자동 삭제로 cascade 충돌 발생, 9 commits 를 단일 통합 PR #93 으로 cherry-pick 재정렬해서 머지했어요. 원본 review/CI 이력은 보존했어요.
- `AXHUB_E2E_STAGING_TOKEN` 이 필요한 staging E2E 4개는 로컬 검증에서 의도적으로 skip 됐어요. 대신 PR checks 와 mock/backend 계약, helper unit/e2e tests 로 릴리스 경계를 확인했어요.
- 실제 production destructive deploy create 는 실행하지 않았어요. race fix 는 fixture + 7 selective refresh unit test 와 SKILL invariant test 로 검증했어요.


### Fixed

* **deploy:** /axhub:deploy push 후 중복 deploy_create race 차단 ([#80](https://github.com/jocoding-ax-partners/axhub/issues/80)) ([138b177](https://github.com/jocoding-ax-partners/axhub/commit/138b1770b6bde310b9843175602bb71cd748c161))
* **deploy:** 통합 — issue [#81](https://github.com/jocoding-ax-partners/axhub/issues/81) stack + [#85](https://github.com/jocoding-ax-partners/axhub/issues/85)/[#86](https://github.com/jocoding-ax-partners/axhub/issues/86)/[#87](https://github.com/jocoding-ax-partners/axhub/issues/87) 후속 (M1-M6 + M85/M86/M87) ([#93](https://github.com/jocoding-ax-partners/axhub/issues/93)) ([1685fdd](https://github.com/jocoding-ax-partners/axhub/commit/1685fdd552d678320c374c16a2b3076d07ccd680)), closes [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#82](https://github.com/jocoding-ax-partners/axhub/issues/82) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#82](https://github.com/jocoding-ax-partners/axhub/issues/82) [#83](https://github.com/jocoding-ax-partners/axhub/issues/83) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#84](https://github.com/jocoding-ax-partners/axhub/issues/84) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#82](https://github.com/jocoding-ax-partners/axhub/issues/82) [#83](https://github.com/jocoding-ax-partners/axhub/issues/83) [#84](https://github.com/jocoding-ax-partners/axhub/issues/84) [#84](https://github.com/jocoding-ax-partners/axhub/issues/84) [#88](https://github.com/jocoding-ax-partners/axhub/issues/88) [#88](https://github.com/jocoding-ax-partners/axhub/issues/88) [#84](https://github.com/jocoding-ax-partners/axhub/issues/84) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#88](https://github.com/jocoding-ax-partners/axhub/issues/88) [#80](https://github.com/jocoding-ax-partners/axhub/issues/80) [#88](https://github.com/jocoding-ax-partners/axhub/issues/88) [#83](https://github.com/jocoding-ax-partners/axhub/issues/83) [#83](https://github.com/jocoding-ax-partners/axhub/issues/83)

## [0.5.7](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.6...v0.5.7) (2026-05-13)

Phase 25/26 matrix absorption 릴리스예요. deploy event log, recovery scan, hook kill switch, verify/trace 자동 제안, trace/verify SKILL, 품질 게이트, SKILL model routing 을 순서대로 main 에 흡수했고, 마지막 registry idempotence hotfix 까지 반영해서 검증 재실행 후 워킹트리가 깨끗하게 남아요.

### Test baseline

- PR gate: #65, #66, #67, #68, #69, #70, #71, #72, #73, #74, #75, #76, #77, #78, #79 checks 가 pass 또는 의도된 skip 이에요.
- Integrated main gate: `bun test --timeout 30000` 674 pass / 4 skip / 0 fail / 4148 expect, `cargo test --workspace`, `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bunx tsc --noEmit`, `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `git diff --check` 가 green 이에요.
- Release gate: `bun run release` postbump 의 `codegen:version` + `release:check` 가 v0.5.7 manifest, Cargo workspace, install script version sync 를 확인했어요.

### Honest tradeoff

- `AXHUB_E2E_STAGING_TOKEN` 이 필요한 staging E2E 4개는 로컬 전체 검증에서 의도적으로 skip 됐어요. 대신 PR checks 와 mock/backend 계약, helper unit/e2e tests 로 릴리스 경계를 확인했어요.
- 실제 production destructive deploy create 는 실행하지 않았어요. matrix absorption 은 ordered squash merge 와 post-merge idempotence hotfix 로 통합했어요.


### Added

* Phase 25 PR 25.1 recovery_scan idempotent in-flight deploy detection ([#73](https://github.com/jocoding-ax-partners/axhub/issues/73)) ([1e3c918](https://github.com/jocoding-ax-partners/axhub/commit/1e3c91857487b13cb9ddd7c62895cf3e8fe3eade)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)
* Phase 25 PR 25.2 hook safety + AXHUB_DISABLE_HOOKS ([#65](https://github.com/jocoding-ax-partners/axhub/issues/65)) ([50c4b2f](https://github.com/jocoding-ax-partners/axhub/commit/50c4b2fc59e6a51b1c3aa0a13f5b4c1af564f5a0)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)
* Phase 25 PR 25.3 user-app deploy artifact verifier hook ([#67](https://github.com/jocoding-ax-partners/axhub/issues/67)) ([650b7c1](https://github.com/jocoding-ax-partners/axhub/commit/650b7c14410c3e955a72d29bd183c2886f35b2ef)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)
* Phase 25 PR 25.4 axhub:trace skill + trace --json subcommand ([#74](https://github.com/jocoding-ax-partners/axhub/issues/74)) ([1450d0a](https://github.com/jocoding-ax-partners/axhub/commit/1450d0a55a97448b577fd32a92785a8f65a11310))
* Phase 25 PR 25.5a scaffold + skill-doctor model field ([#66](https://github.com/jocoding-ax-partners/axhub/issues/66)) ([7155b68](https://github.com/jocoding-ax-partners/axhub/commit/7155b689f5a6a933e6fe1250a0560040148c8be5)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)
* Phase 25 PR 25.5b — 8 read-only SKILLs model: haiku ([8fd3077](https://github.com/jocoding-ax-partners/axhub/commit/8fd3077fdb34f62a8ed95116f18c7a9753941ac5))
* Phase 25 PR 25.5c — 11 destructive SKILLs model: sonnet (no-op confirm) ([#77](https://github.com/jocoding-ax-partners/axhub/issues/77)) ([b864b21](https://github.com/jocoding-ax-partners/axhub/commit/b864b21e3078cc79655dde6e2a104f64916aa19c))
* Phase 25 PR 25.6 doctor deploy-events disk usage monitoring ([#76](https://github.com/jocoding-ax-partners/axhub/issues/76)) ([f27c050](https://github.com/jocoding-ax-partners/axhub/commit/f27c050e2554e07c6ace83dda1503d7ab1fe8d87))
* Phase 25 PR 25.7 classify-exit verify/trace auto-suggest ([#75](https://github.com/jocoding-ax-partners/axhub/issues/75)) ([568c3c3](https://github.com/jocoding-ax-partners/axhub/commit/568c3c3846bb8ab3fa4141f440babf5ce5088307)), closes [#65](https://github.com/jocoding-ax-partners/axhub/issues/65)
* Phase 26 PR 26.1b event_log deploy NDJSON audit trail ([#69](https://github.com/jocoding-ax-partners/axhub/issues/69)) ([9b50dee](https://github.com/jocoding-ax-partners/axhub/commit/9b50dee836e954ffc1cc01381e2a94e41df234f6)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)
* Phase 26 PR 26.2 phase logic — event-sourcing derived view (option b) ([#70](https://github.com/jocoding-ax-partners/axhub/issues/70)) ([ed407eb](https://github.com/jocoding-ax-partners/axhub/commit/ed407ebc7448e2e5d58c7ed071b581c35053c5dc)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)
* Phase 26 PR 26.3 quality_gate pure validator + catalog entry ([#71](https://github.com/jocoding-ax-partners/axhub/issues/71)) ([a151cc8](https://github.com/jocoding-ax-partners/axhub/commit/a151cc88065076b45e009e5fd2e8144e6435018c)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)
* Phase 26 PR 26.4 axhub:verify skill + verify --json subcommand ([#72](https://github.com/jocoding-ax-partners/axhub/issues/72)) ([c6e538a](https://github.com/jocoding-ax-partners/axhub/commit/c6e538aa9a8b7e6e91bb4a244da7087eb75ca9eb)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)


### Fixed

* registry idempotence after matrix absorption ([e1ea810](https://github.com/jocoding-ax-partners/axhub/commit/e1ea8101db6a346f7ba7279f4bb634b5f0a67b35))


### Changed

* Phase 26 PR 26.1a atomic_jsonl + telemetry/audit migration ([#68](https://github.com/jocoding-ax-partners/axhub/issues/68)) ([50e31d7](https://github.com/jocoding-ax-partners/axhub/commit/50e31d7741331a13ae7daa30041f8a1b08aa1b6b)), closes [#78](https://github.com/jocoding-ax-partners/axhub/issues/78)

## [0.5.6](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.5...v0.5.6) (2026-05-11)

### Phase VC narrative — Vibe Coder Visibility

이번 릴리즈는 비개발자 vibe coder 사용자가 axhub 흐름에서 raw 기술 jargon 을 마주치지 않게 SKILL 표면을 전수 정리했어요. deploy SKILL 에 "Vibe Coder Visibility Rules" 섹션을 박아 `binding_hash` / `pending_action_id` / `pending_action_hash` / `retry_policy` / `consent_binding` 같은 internal verification primitive 12개를 명시적으로 chat echo 금지 했고, 한국어 한 줄 templates 표 + `AXHUB_DEPLOY_VERBOSE=1` verbose toggle 을 함께 제공해요. 이어서 Step 1.1 instruction text, Step 1.5 (git-init 옵션 단순화 + `>/dev/null 2>&1` 구조적 redirect), Step 2.5 `(cli_too_new)` jargon 제거, 9 SKILL description 자연어화 (deploy / github 6곳 / auth `OAuth Device Flow` / env `key`+`stdin` / profile `endpoint` / apis `scope` / update `cosign 서명 검증` / doctor internal slug / init bootstrap FSM raw state echo), 5 TodoWrite content (doctor `helper binary` / `profile`+`endpoint` / github `repo` / profile `profile`+`endpoint` / recover `commit`), AskUserQuestion header 영문 (env `env` / github 4곳 / profile `profile`) → 한국어 단일 명사, references/error-empathy-catalog 3 섹션 user-facing template (`endpoint` / `method` / `body source` / `preview` / `token mint` / `read-only`) 까지 자연어 한 줄로 humanize 했어요.

### Test baseline

- bun test: 599 pass / 4 skip / 0 fail / 3835 expect (Phase 3.5 baseline 586 → +13)
- cargo test -p axhub-helpers: 189 pass / 3 ignored (Phase 3.5 baseline 181 → +8)
- bunx tsc --noEmit: clean
- bun run lint:tone --strict: 0 err / 0 warning across 34 files
- bun run lint:keywords --check: no diff vs baseline
- bun run skill:doctor --strict: exit 0
- architect review (Sonnet) 2회: 1차 조건부 → fix → 2차 승인

### Honest tradeoff

- Visibility Rules 는 instruction-level lock 이라 LLM compliance 의존이에요. 결정론적 enforce 는 PostToolUse output redact filter 또는 helper API 의 user-visible/internal JSON 분리가 필요해요 (후속 PR scope).
- `show_commands` 옵션 제거로 개발자 escape hatch 가 `AXHUB_DEPLOY_VERBOSE=1` 환경변수에만 존재해요. discovery path 는 `/axhub:doctor` 출력 통합 등 후속 작업으로 보완해요.
- Step 1.5 git 명령은 `>/dev/null 2>&1` + `|| true` 로 raw output redact + `git commit` 실패 시 뒤따르는 resolve 가 `branch` / `commit_sha` 비어 있음을 감지해 humanized 한 줄로 안내해요.

### Added

* vibe coder visibility — chat surface jargon redact across SKILLs ([fc6318a](https://github.com/jocoding-ax-partners/axhub/commit/fc6318a4632843afe1df6e32cdb55551a9bbc68f))

## [0.5.5](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.4...v0.5.5) (2026-05-11)

Phase 25는 deploy 시간 단축 Phase 1~3.5 스택을 main 에 올리는 릴리스예요. REST 중복 호출을 `deploy-prep` 와 session bundle/cache 로 줄이고, macOS Gatekeeper warmup, parallel preflight, config 기반 CLI 호환 경고, token freshness bridge 를 연결해서 배포 시작 전 대기와 불필요한 CLI round-trip 을 낮춰요.

### Test baseline

- PR gate: #60, #61, #62, #63 의 Local Rust-primary, T2 helper-bin, rust macOS / Ubuntu / Windows, perf macOS / Ubuntu / Windows checks 가 green 이에요.
- Integrated main gate: `bun run codegen:version`, `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `bun run build`, `bin/axhub-helpers version`, `bunx tsc --noEmit`, `bun test` (598 pass / 4 skip / 0 fail), `bun run lint:tone --strict`, `bun run lint:keywords --check` 가 green 이에요.
- Release gate: `bun run release` postbump 의 `codegen:version` + `release:check` 가 v0.5.5 version sync 와 helper artifacts 를 확인했어요.

### Honest tradeoff

- 실제 production destructive deploy create 는 실행하지 않았어요. 대신 mock backend perf fixture, staging-gated E2E skip 계약, Rust helper unit/e2e tests 로 deploy prep 과 token freshness 경계를 검증했어요.
- 스택 PR 은 squash 대신 bottom-to-top merge 로 통합했어요. 하위 CI fix 커밋을 상위 Phase 브랜치가 덮지 않게 하려는 선택이에요.


### Added

* Phase 0.5 deploy entry + mark subcommand [skip-routing-gate] ([#58](https://github.com/jocoding-ax-partners/axhub/issues/58)) ([50e047a](https://github.com/jocoding-ax-partners/axhub/commit/50e047a31b6082bd5d45240f852ca2da73fc134e))
* Phase 1 REST dedup + statusline live [skip-routing-gate] ([cd0a480](https://github.com/jocoding-ax-partners/axhub/commit/cd0a480a6d7659d811bec42c12c4ad854defbc96))
* Phase 2 Gatekeeper warmup + version --quiet ([64219c3](https://github.com/jocoding-ax-partners/axhub/commit/64219c397a07bfef2a5f2c76a570b56b67e329c4)), closes [#2](https://github.com/jocoding-ax-partners/axhub/issues/2)
* Phase 3 client cascade reduced [skip-routing-gate] ([1e981ac](https://github.com/jocoding-ax-partners/axhub/commit/1e981ac08649688ab515db81d4f321a1a036447d))
* Phase 3.5 SKILL flow wire-up [skip-routing-gate] ([351b668](https://github.com/jocoding-ax-partners/axhub/commit/351b668192e9efb35da495e756f967c464a098a3))


### Fixed

* close phase 3.5 deploy flow gaps ([3f14a78](https://github.com/jocoding-ax-partners/axhub/commit/3f14a787dd37f1e4290391bf907ba579bcfd2006))
* isolate PATH in hook tests so host axhub doesn't leak ([167bb7c](https://github.com/jocoding-ax-partners/axhub/commit/167bb7c8a921ea452afc3fd198b5884762177089))
* keep phase 1 CI skip audit non-blocking ([9586133](https://github.com/jocoding-ax-partners/axhub/commit/958613320fcd37ee3fcaf67edb3cafefcc03841c)), closes [#60](https://github.com/jocoding-ax-partners/axhub/issues/60)
* stabilize telemetry CLI version fixture in coverage ([7cb17c9](https://github.com/jocoding-ax-partners/axhub/commit/7cb17c921a7bd79f75e72a732d3f576693da945f))

## [0.5.4](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.3...v0.5.4) (2026-05-08)

Phase 24는 Windows 브라우저 로그인과 GitHub repo 연결 회복 흐름을 vibe coder 기준으로 막히지 않게 다듬은 패치예요. PowerShell consent-mint, PR routing gate, macOS spawn 테스트 격리를 함께 고쳐서 Windows·macOS·Linux 검증을 다시 green 으로 맞췄어요. 원격 main 에 먼저 들어온 Phase 0 deploy walltime measurement commit 도 같은 릴리스에 반영해요.

### Test baseline

- Local gate: `bun test`, `cargo test --workspace`, `bun run typecheck`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `bash tests/run-corpus.sh --mode plugin --corpus tests/corpus.100.jsonl --vs claude-native --score` 가 green 이에요.
- PR gate: Local Rust-primary, T2 helper-bin, corpus.100 drift, rust macOS / Ubuntu / Windows matrix 가 pass 예요.
- Release gate: `bun run release -- --release-as patch` postbump 의 `codegen:version` + `release:check` 가 v0.5.4 manifest 와 helper artifact 를 확인했어요.

### Honest tradeoff

- Native Windows 실사용 OAuth 로그인은 CI smoke 가 아니라 문서·helper·Windows Rust matrix 경계까지 검증했어요.

### Features

* add Phase 0 deploy walltime measurement infrastructure ([d1e80b2](https://github.com/jocoding-ax-partners/axhub/commit/d1e80b2))

### Fixed

* make Windows auth and GitHub setup safer ([df9bec6](https://github.com/jocoding-ax-partners/axhub/commit/df9bec6fe47eb9590947e3187b355ba285a71ff9))
* unblock routing gate and auth helper fallback ([e702d02](https://github.com/jocoding-ax-partners/axhub/commit/e702d021aa31820215e9742ee3b70782c9307345))

## [0.5.3](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.2...v0.5.3) (2026-05-08)

Phase 12.3 핫픽스는 init 처럼 질문 뒤에 이어지는 multi-step SKILL 이 초기 TodoWrite 목록을 stale 상태로 남기지 않도록, 모든 multi-step skill 과 새 skill scaffold 에 status sync 규칙을 고정한 릴리스예요. 이제 workflow step 이 끝나거나 AskUserQuestion 답변 뒤에 전체 todos 배열을 다시 호출해서 completed / in_progress / pending 상태를 최신화하게 해요.

### Test baseline

- Local gate: `bun test` (559 pass / 4 skip / 0 fail), `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `git diff --check` 가 green 이에요.
- Targeted RED/GREEN gate: `bun test tests/ux-todowrite.test.ts` 에서 10개 multi-step SKILL 과 scaffold 의 status sync 누락을 먼저 실패시킨 뒤 green 으로 만들었어요.

### Honest tradeoff

- 실제 Claude Code Todo UI replay 는 로컬 자동화에서 직접 열 수 없어서, SKILL 문서 계약과 회귀 테스트로 고정했어요.
- 작업트리에 있던 별도 perf 측정 변경은 릴리스 범위 밖이라 stash 로 보존하고, 이 릴리스에는 포함하지 않았어요.


### Fixed

* keep skill todos synced after progress ([c48f7f8](https://github.com/jocoding-ax-partners/axhub/commit/c48f7f8d69203483bc4178e5e11e0f39565553f6))

## [0.5.2](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.1...v0.5.2) (2026-05-08)

Phase 12.2 UX 패치는 단순 조회형 slash command 를 Haiku 로 내려 첫 응답 지연과 비용을 줄이고, deploy 의 git 저장 지점 준비 흐름을 Claude Code TodoWrite UI 로만 보여주게 만든 릴리스예요. deploy / doctor / update / login 은 인증·복구·destructive 위험이 남아 Sonnet 을 유지해요.

### Test baseline

- Local gate: `bun test` (548 pass / 4 skip / 0 fail), `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `git diff --check` 가 green 이에요.
- Targeted RED/GREEN gate: `bun test tests/manifest.test.ts tests/deploy-git-init-stage.test.ts tests/multistep-stage-checklist.test.ts` 에서 model policy 와 TodoWrite-only stage policy 를 먼저 실패시킨 뒤 green 으로 만들었어요.
- Release gate: `bun run release -- --release-as patch` postbump 의 `codegen:version` + `release:check` 가 v0.5.2 version sync 와 host helper artifact 를 확인했어요.

### Honest tradeoff

- 실제 Claude Code latency 를 live 로 계측하지는 않았어요. Haiku 전환은 공식 모델 latency 특성과 command frontmatter 정책을 근거로 한 최적화예요.
- 자연어 SKILL 자동 invoke 자체는 repo 정책상 `SKILL.md` 에 model 을 넣지 않기 때문에 현재 세션 모델을 따라가요. 이번 릴리스는 `/axhub:status`, `/axhub:logs`, `/axhub:apps`, `/axhub:apis`, `/axhub:help` slash command 실행 모델을 Haiku 로 낮추는 변경이에요.

### Fixed

* route lightweight axhub command surfaces faster ([4bb0b6a](https://github.com/jocoding-ax-partners/axhub/commit/4bb0b6a1ae6e5a7c8e44239438380a489f4cc037))

## [0.5.1](https://github.com/jocoding-ax-partners/axhub/compare/v0.5.0...v0.5.1) (2026-05-08)

Phase 12.1 핫픽스는 axhub CLI 0.12.1 사용자가 배포 preflight 에서 막히지 않게 호환 범위를 올리고, 사용자가 “프로젝트 초기화해줘”처럼 axhub 접두어 없이 말해도 init SKILL 이 잡히도록 라우팅 표면을 보강한 릴리스예요. preflight 의 미래 버전 차단 모델은 유지하면서, init routing metadata 는 corpus 와 baseline fixture 까지 같이 맞춰요.

### Test baseline

- Local gate: `cargo test --workspace`, `bun test` (547 pass / 4 skip / 0 fail), `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `bun run routing:drift`, `cargo fmt --check`, `git diff --check` 가 green 이에요.
- Targeted gate: `cargo test -p axhub-helpers preflight -- --nocapture`, `bun run build`, `./bin/axhub-helpers preflight --json`, `bun test tests/init-template-guidance.test.ts`, `bash tests/run-corpus.sh --mode plugin --corpus tests/corpus.20.jsonl --vs claude-native --score`, `bash tests/run-corpus.sh --mode plugin --corpus tests/corpus.100.jsonl --vs claude-native --score` 가 green 이에요.
- Release gate: `bun run release -- --release-as patch` postbump 의 `codegen:version` + `release:check` 가 v0.5.1 manifest 와 host helper artifact 를 확인했어요.

### Honest tradeoff

- 실제 destructive deploy create 는 실행하지 않았어요. 대신 `./bin/axhub-helpers preflight --json` 로 현재 CLI 0.12.1 이 `cli_too_new:false` / `in_range:true` 로 통과하는지 확인했어요.
- Claude Code 의 실제 native skill picker UI 는 로컬 자동화에서 직접 열 수 없어서, SKILL description/examples, keyword baseline, corpus 20/100 fixture, routing-drift gate 로 간접 검증해요.


### Fixed

* keep skill fallbacks inside runtime envelopes ([0ada525](https://github.com/jocoding-ax-partners/axhub/commit/0ada52502e23c9c2bf2f446373dca6188faa0c25))
* restore deploy preflight for axhub cli 0.12 users ([d2aa0e5](https://github.com/jocoding-ax-partners/axhub/commit/d2aa0e5595bf9de0922e439aab270a66e92ae05c))
* route plain Korean project initialization to init ([e24828d](https://github.com/jocoding-ax-partners/axhub/commit/e24828db49af25550fc9639a71ca96754c73759b))

## [0.5.0](https://github.com/jocoding-ax-partners/axhub/compare/v0.3.2...v0.5.0) (2026-05-08)

Phase 8-11 라우팅 스택은 SKILL description 을 단일 source of truth 로 삼고, 예시 기반 튜닝과 clarify feedback visibility 를 붙여 실제 사용자 발화 drift 를 더 빨리 발견하도록 만든 릴리스예요. routing-drift gate 는 fixture freshness 를 fail-closed 로 확인하고, `routing:tune --confused` 는 명시 helper 오류를 숨기지 않게 바꿨어요.

### Test baseline

- PR gate: #53, #54, #55, #56 의 Rust ubuntu/macos/windows, Local Rust-primary gate, T2 helper-bin checks 가 green 이에요. `[skip-routing-gate]` 로 의도적으로 skip 된 cost-aware / staging / fuzz / routing-drift 항목은 PR title audit trail 로 남겨요.
- Local gate: `bun test` (543 pass / 4 skip / 0 fail), `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `bun run routing:drift`, `git diff --check` 가 green 이에요.
- Release gate: `bun run release -- --release-as 0.5.0` postbump 의 `codegen:version` + `release:check` 가 version sync 와 release binary build path 를 확인했어요.

### Honest tradeoff

- `routing:tune --confused` 는 prompt 원문을 저장하지 않아요. hash + chosen_skill 로 privacy 를 지키는 대신, 원본 발화 재구성은 corpus 또는 사용자 기억에 의존해요.
- routing-drift gate 는 fresh LLM measurement 를 매 PR 에서 실행하지 않고 committed fixture freshness 를 강제해요. 비용과 재현성을 우선한 선택이라, 대규모 routing 변경은 별도 `measure:baseline` 재생성이 필요해요.


### Added

* add npm permission error empathy entries (EACCES/EEXIST/EPERM/ENOSPC/ENOTEMPTY) ([b1ce831](https://github.com/jocoding-ax-partners/axhub/commit/b1ce8319f06a66b566d9ba36915a105acf94e20a))
* axhub init dependency-plan helper로 install 가시성 확보 ([5dae562](https://github.com/jocoding-ax-partners/axhub/commit/5dae5627632fa00bde3e51b30557083fee4aae73))
* **routing:** Phase 0 — Approach E test contract rewrite ([a7a3848](https://github.com/jocoding-ax-partners/axhub/commit/a7a3848d72017f37bb75c44eee67fb4ea2a427ac))
* **routing:** Phase 0 — Approach E test contract rewrite ([#38](https://github.com/jocoding-ax-partners/axhub/issues/38)) ([c3fab5e](https://github.com/jocoding-ax-partners/axhub/commit/c3fab5e8d76182f9a59448017e6617628f11a129))
* **routing:** Phase 1 — keyword phrase → SKILL.md description codegen migration ([a52f3d4](https://github.com/jocoding-ax-partners/axhub/commit/a52f3d408390f68827e5a381c7392e43f7b857fd))
* **routing:** Phase 1 — keyword phrase → SKILL.md description codegen migration ([#40](https://github.com/jocoding-ax-partners/axhub/issues/40)) ([9c52f56](https://github.com/jocoding-ax-partners/axhub/commit/9c52f56e889c73b877efe6ad40a3e06639cdd46a)), closes [#38](https://github.com/jocoding-ax-partners/axhub/issues/38)
* **routing:** Phase 10 (FINAL v0.5.0) — Feedback Loop + Visibility ([5e4e432](https://github.com/jocoding-ax-partners/axhub/commit/5e4e432358098de83308be32c68b687891b77f3b))
* **routing:** Phase 2 — router 단순화 + audit module (Approach E core) ([8f4a00e](https://github.com/jocoding-ax-partners/axhub/commit/8f4a00eea3392625469cfe18d47737a592cd1293))
* **routing:** Phase 4 — routing-stats + cleanup-audit CLI subcommands ([e664036](https://github.com/jocoding-ax-partners/axhub/commit/e6640369dc77d1de41f61ebf2ec4a57a1f76c47c))
* **routing:** Phase 5 — corpus + baseline 재생성 (Approach E meta_question expansion) ([e0d8750](https://github.com/jocoding-ax-partners/axhub/commit/e0d8750cc921211b5d34a3049ae1e3ae7e102fb6))
* **routing:** Phase 6 — Component 8 cross-phase test infra + Migration Gate aggregator ([5d3075c](https://github.com/jocoding-ax-partners/axhub/commit/5d3075c08e63cebb40bb9b3268093810e98dc009))
* **routing:** Phase 7 (FINAL) — SessionStart v0.4.0 magical moment + 라우팅 docs ([a06d161](https://github.com/jocoding-ax-partners/axhub/commit/a06d161109be75714f4787d62d3da5f11ee76664))
* **routing:** Phase 8 — Groundwork (fresh baseline + skill:doctor 강화 + routing-drift CI gate) ([6f44876](https://github.com/jocoding-ax-partners/axhub/commit/6f448769291611e7996af7a1e5256378922b7e2a))
* **routing:** Phase 9 — Examples + Tuning (95% accuracy driver) ([3949076](https://github.com/jocoding-ax-partners/axhub/commit/394907675ebea8717f8aee6b224226e5d590f9f0))
* 토큰 만료 시각 사용자 친화 한국어 표시 ([8146353](https://github.com/jocoding-ax-partners/axhub/commit/8146353ce3c466030fc70764764c9bc5ba1195df))


### Fixed

* **docs:** landing-page 표 entry v0.3.1 → v0.3.2 잔여 site ([128ebf6](https://github.com/jocoding-ax-partners/axhub/commit/128ebf65dd38ea5ba61a22a5b8c11a1ce20c8ea2))
* **docs:** README/PLAN/quickstart/landing-page 의 v0.3.1 → v0.3.2 동기화 ([08d3a91](https://github.com/jocoding-ax-partners/axhub/commit/08d3a9120927d05530ccae5138baee55c2344c25))
* npm 권한 catalog Windows 호환 명령 추가 ([cf7f492](https://github.com/jocoding-ax-partners/axhub/commit/cf7f4920ea9e3e4b9eb9530a1b83afec9999659d))
* remove bootstrap dependency-plan tautology + add coverage tests ([55e1357](https://github.com/jocoding-ax-partners/axhub/commit/55e135786e8ba530c660122174c4ae91763d74da)), closes [#41](https://github.com/jocoding-ax-partners/axhub/issues/41)
* **routing:** Phase 11 — code review fixes for v0.5.0 PR stack ([#53](https://github.com/jocoding-ax-partners/axhub/issues/53)/[#54](https://github.com/jocoding-ax-partners/axhub/issues/54)/[#55](https://github.com/jocoding-ax-partners/axhub/issues/55)) ([51db4c8](https://github.com/jocoding-ax-partners/axhub/commit/51db4c897a716a74d7e3cfb75fb7a06cc6515753))


### Docs

* **init:** L10 영어 jargon 한국어로 풀이 ([1a26e3f](https://github.com/jocoding-ax-partners/axhub/commit/1a26e3f891870941a7f88bad2401199bb1e876c4)), closes [#36](https://github.com/jocoding-ax-partners/axhub/issues/36)
* **init:** scaffold/apphub.yaml 비개발자 친화 용어로 풀이 ([d3d2b27](https://github.com/jocoding-ax-partners/axhub/commit/d3d2b27e798e607de3646b0dbe18f18a33b11c17))
* **init:** T2 RFC gate-skip impl — frontmatter trailing 한국화 + L41/L99 inline gloss ([043c83a](https://github.com/jocoding-ax-partners/axhub/commit/043c83a229a82c35ee7b90d55572ac0bbb6b3c50)), closes [#36](https://github.com/jocoding-ax-partners/axhub/issues/36)
* **rfc:** nl-lexicon trigger localization RFC 초안 ([bb48520](https://github.com/jocoding-ax-partners/axhub/commit/bb48520a9ce39e2248cf64616efc709853288eb7)), closes [#36](https://github.com/jocoding-ax-partners/axhub/issues/36)
* **rfc:** RFC merge ≠ impl commitment 명시 보강 ([dc67447](https://github.com/jocoding-ax-partners/axhub/commit/dc6744727f2cab53b7861dad2257b75071a88dae))

## [0.3.2](https://github.com/jocoding-ax-partners/axhub/compare/v0.3.1...v0.3.2) (2026-05-07)

Phase 27 은 Windows 호스트에서 axhub plugin bootstrap 이 과장된 지원 claim 없이 안전하게 실패하거나 복구하도록 정리한 호환성 릴리스예요. Rust helper 는 token / keychain / preflight 경로를 CI 로 검증 가능한 단위로 나누고, SessionStart hook 은 helper bootstrap 실패 시 JSON diagnostic 을 한 번만 내도록 막아요.

### Test baseline

- PR #35 gate: GitHub Actions `rust ubuntu-latest`, `rust macos-latest`, `rust windows-latest`, `Local Rust-primary gate`, `T2 helper-bin (PR-blocking, $0 cost)` 가 green 이에요.
- Local gate: `bun test` (420 pass / 4 skip / 0 fail), `cargo test -q --workspace`, `cargo clippy -q -p axhub-helpers --all-targets --locked -- -D warnings`, `bunx tsc --noEmit`, `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run cargo:coverage` 가 green 이에요.
- Release gate: `bun run release` postbump 의 `codegen:version` + `release:check` 가 v0.3.2 install script 와 Rust helper version 동기화를 확인했어요.

### Honest tradeoff

- 실제 Windows VM 에서 `hooks/session-start.ps1` 전체 hook runtime smoke 를 직접 수행하지는 않았어요. 대신 Windows CI 의 Rust helper gate, PowerShell wrapper static regression, 문서화된 VM smoke checklist 로 지원 범위를 Tier 2 검증으로 한정해요.
- macOS/Linux shell auto-download 는 broken symlink repair 까지 로컬 regression 으로 잠갔지만, corporate proxy / antivirus 같은 Windows host 정책은 릴리스 후 사용자 환경에서 추가 관찰이 필요해요.


### Fixed

* keep axhub plugin bootstrap reliable across Windows ([3c6067f](https://github.com/jocoding-ax-partners/axhub/commit/3c6067f9621a3cf986b7a67322c77f9a815c57d8))
* keep first-run auth bootstrap silent and CI-provable ([7725c35](https://github.com/jocoding-ax-partners/axhub/commit/7725c3553e6b4d5dab99d5eacf373415e1056d64))
* keep SessionStart bootstrap diagnostics single-source ([362c197](https://github.com/jocoding-ax-partners/axhub/commit/362c1976136d59682f64b30514e496fb9be06231))


### Docs

* prevent unsupported Windows auto-start claims ([cec8cee](https://github.com/jocoding-ax-partners/axhub/commit/cec8cee94ea7a455878468d0c2308c1bdbfb6b7e))

## [0.3.1](https://github.com/jocoding-ax-partners/axhub/compare/v0.3.0...v0.3.1) (2026-05-06)

Phase 26.1 은 v0.3.0 의 install-cli 흐름 UX 폴리시 패치예요. doctor 가 \`cli_present:false\` 를 detect 하면 \"CLI 설치해줘\" phrase 를 사용자가 다시 발화하지 않아도 즉시 AskUserQuestion (\`자동 / 명령어만 / 나중에\`) 으로 설치 의향을 확인하고, 선택 시 install-cli SKILL 을 sibling consent route 로 호출해요. 또 PATH 미등록은 plugin-local 이 작동하면 ⚠ 가 아닌 ✓ 로 표시하고 — plugin design 상 user PATH 오염 방지가 의도된 동작이라 cosmetic 노이즈를 제거했어요.

### Test baseline

- Local gate: \`bunx tsc --noEmit\`, \`bun test\` (402 pass / 4 skip / 0 fail), \`bun run lint:tone --strict\`, \`bun run lint:keywords --check\`, \`bun run skill:doctor --strict\` 모두 green 이에요.
- Registry: doctor SKILL 의 두 번째 AskUserQuestion (cli-install) 이 \`tests/fixtures/ask-defaults/registry.json\` 에 등록 — \`safe_default: \"나중에\"\` 로 subprocess 자동 install 차단.
- Status mapping: PATH+plugin-local 두 row 의 매트릭스가 ✓ / ✓ fallback / ✗ 세 분기로 명확화. ⚠ 는 진짜 문제일 때만 fire.

### Honest tradeoff

- doctor 의 NEVER auto-fix 규칙 회색지대 — Step 5.5 가 sibling skill (install-cli) 로 consent route 만 하므로 직접 install 안 함. 규칙은 보존되지만 \"진단 SKILL 이 다른 SKILL 호출\" 패턴이 늘어나서 future drift 우려 있어요. multi-failure summary 와 동일 패턴이라 일관성은 있어요.
- CLAUDE_PLUGIN_ROOT empty fallback 으로 cache path 패턴 (\`\$env:USERPROFILE\\.claude\\plugins\\cache\\axhub\\axhub\\*\\bin\\\`) 을 hardcoded 하게 scan 해요 — Claude Code 가 cache layout 바꾸면 fallback 깨져요. 하지만 현재까지 path 안정적이고 fallback 부재 시 사용자 stuck 되는 cost 가 더 커요.


### Added

* **doctor:** cli_present:false 단일 fail 시 즉시 AskUserQuestion 으로 설치 의향 확인 ([867e6f3](https://github.com/jocoding-ax-partners/axhub/commit/867e6f3bd031b780ad97b078f25207602c26774c))


### Fixed

* **doctor:** PATH 미등록은 ⚠ 아닌 ✓ — plugin-local 작동 시 정상 fallback ([bc15317](https://github.com/jocoding-ax-partners/axhub/commit/bc1531711d49ac8da0cff76acb11f0195a6e93b3))

## [0.3.0](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.14...v0.3.0) (2026-05-06)

Phase 26 은 axhub CLI 미설치 사용자가 \`/axhub:doctor\` 단계에서 막혀 manual 설치 안내만 보던 friction 을 root-cause 해결한 릴리스예요. 신규 \`axhub:install-cli\` SKILL 이 OS (macOS / Linux / Windows) 를 감지해 공식 채널 (curl install.sh / irm install.ps1 / Homebrew / Scoop) 중 하나로 자동 설치하고, brew / scoop 미설치 호스트에서는 공식 installer 로 graceful fallback 해요. doctor 의 \`NEVER auto-fix\` 규칙은 보존해 진단 / 변경 책임을 분리했어요. 부수로 apps delete consent remint 루프 fix 도 포함해요.

### Test baseline

- Local gate: \`bunx tsc --noEmit\`, \`bun test\` (402 pass / 4 skip / 0 fail), \`bun run lint:tone --strict\`, \`bun run lint:keywords --check\`, \`bun run skill:doctor --strict\` 모두 green 이에요.
- nl-lexicon baseline: 신규 install-cli SKILL description 의 trigger 어구 추가로 baseline 재캡처 (593 entries 유지, 18 files).
- Cross-OS verification: install-cli SKILL 이 \`uname -s\` (Darwin/Linux) / \`$env:OS\` (Windows_NT) 분기로 채널 선택, 패키지 매니저 부재 시 graceful fallback.

### Honest tradeoff

- Plugin SKILL 이 사용자 host 에 install script 를 실행해요 — supply-chain 신뢰는 \`cli.jocodingax.ai\` HTTPS 도메인 + ax-hub-cli 팀의 cosign 서명에 위임해요. plugin 자체가 추가 검증 안 해요.
- brew / scoop 자체는 자동 설치 안 해요 — 패키지 매니저 부재 시 사용자가 직접 설치해야 (https://brew.sh / https://scoop.sh 안내). supply-chain scope 한정 결정.
- subprocess (CI / claude -p) 환경에서는 자동 install 차단 — registry safe_default \`수동 안내\` 로 명령어만 출력하고 종료해요.


### Added

* install-cli SKILL — axhub CLI 자동 설치 (doctor consent route) ([1bc1cca](https://github.com/jocoding-ax-partners/axhub/commit/1bc1cca07dcde84e120c18d9dc375f16b9e35387))


### Fixed

* **install-cli:** brew/scoop 부재 시 graceful fallback ([f23f9dc](https://github.com/jocoding-ax-partners/axhub/commit/f23f9dc8588f7c29077ab1d19655fd7c6954f138))
* **install-cli:** Windows scoop 부재 안내 메시지 추가 ([2eff07a](https://github.com/jocoding-ax-partners/axhub/commit/2eff07ad41d7d2bba997049235731715520b0530)), closes [#33](https://github.com/jocoding-ax-partners/axhub/issues/33)
* prevent app delete consent remint loops ([6868eca](https://github.com/jocoding-ax-partners/axhub/commit/6868ecad15d357267924bab23ef5c360f92808bd))

## [0.2.14](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.13...v0.2.14) (2026-05-06)

Phase 25.3 은 회사 보안 정책상 VC++ Redistributable 사전 설치를 가정할 수 없는 Windows 호스트에서 `axhub-helpers.exe` 가 `STATUS_DLL_NOT_FOUND` (0xC0000135) 로 즉시 종료되던 문제를 root-cause 해결한 릴리스예요. `.cargo/config.toml` 에 Windows MSVC target-scoped `+crt-static` rustflag 를 추가해 vcruntime140.dll / msvcp140.dll 동적 import 를 끊고, 회귀 방지를 위해 release.yml 과 rust-ci.yml 양쪽에 `dumpbin /dependents` assertion 을 박아 모든 PR 과 release tag 에서 정적 CRT 링크를 자동 enforce 해요.

### Test baseline

- Local gate: `bunx tsc --noEmit`, `bun test` (390 pass / 0 fail), `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict` 모두 green 이에요.
- Build regression: macOS 에서 `cargo build --release -p axhub-helpers` 가 exit 0 — Windows-only rustflag 가 macOS / Linux 빌드를 건드리지 않는 target-scoped 격리 확인했어요.
- CI assertion: rust-ci.yml Windows job 이 vswhere 로 dumpbin.exe 절대경로 resolve 후 vcruntime140 / msvcp140 import 시 즉시 fail 해요. release.yml 에도 동일 step 박혀서 tag push 시 회귀 차단해요.

### Honest tradeoff

- Windows 바이너리가 약 1MB 증가해요 — ripgrep / fd / deno / bun 이 채택한 표준 패턴과 동일한 트레이드오프예요. 단명 plugin helper scope 라 vcruntime CVE 패치 propagation 손실 영향은 미미해요.
- 거부된 대안 — VC++ Redist 자동 설치 (admin 권한 필요, 회사 IT 정책 차단), windows-gnu target (ring crate MinGW 비호환), pre-flight DLL check (real fix 아님) — 은 모두 회사 환경 제약 또는 안정성 이유로 제외했어요.
- 실제 Windows 사용자 검증은 v0.2.14 release upload 후 install.ps1 재실행으로 이어져요 — CI 의 dumpbin assertion 과 cosign 서명이 정적 CRT 링크 + 무결성을 1차 보증해요.


### Fixed

* **ci:** vswhere 로 dumpbin 절대경로 resolve ([6d552c3](https://github.com/jocoding-ax-partners/axhub/commit/6d552c35b6c677539cb0143bb12bfb88d0e4b9dc))
* rust-ci 에 정적 CRT 회귀 가드 추가 ([ab17c01](https://github.com/jocoding-ax-partners/axhub/commit/ab17c01a8f8f58fa5b19b6185cecc1fffdb0d55d))
* Windows helper STATUS_DLL_NOT_FOUND 정적 CRT 링크로 해결 ([19980b5](https://github.com/jocoding-ax-partners/axhub/commit/19980b5cfe1d9ae13de9168d247ccb2fc246f506))

## [0.2.13](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.12...v0.2.13) (2026-05-06)

Phase 25.2는 deploy 중 git 저장 지점 확인으로 들어갈 때 이전 `init` TodoWrite 항목이 Claude Code UI에 남아 사용자에게 잘못된 진행 상태를 보여주던 UX 회귀를 막는 패치예요. deploy 스킬은 시작 시점과 git readiness 분기에서 기존 todo 를 patch 하지 않고 전체 교체하도록 명시하고, 회귀 테스트로 stale TodoWrite 재유입을 잠가요.

### Test baseline

- Local gate: `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bunx tsc --noEmit`, `bun test`, `git diff --check` 가 green이에요.
- Regression guard: `tests/deploy-git-init-stage.test.ts` 가 deploy 시작과 git readiness 분기에서 TodoWrite 전체 교체 지시를 확인해요.
- Release chain: `bun run release -- --release-as patch` postbump 의 `codegen:version` + `release:check` 가 통과했어요.

### Honest tradeoff

- 실제 Claude Code UI 렌더링은 릴리스 전 destructive deploy 세션으로 재실행하지 않았어요. 이번 릴리스는 SKILL 지시와 회귀 테스트로 stale todo 혼입을 차단하고, 라이브 UI 확인은 새 플러그인 설치 후 smoke 에서 이어가요.
- TodoWrite 상태 보존 자체는 Claude Code 런타임 특성이므로, 제품 쪽에서는 skill 이 분기마다 전체 목록을 다시 렌더링하는 방어 계약을 유지해요.


### Fixed

* replace stale deploy todo state ([57e58c9](https://github.com/jocoding-ax-partners/axhub/commit/57e58c9257081e0c0034e247c3ae4af0a480ba1c))

## [0.2.12](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.11...v0.2.12) (2026-05-06)

Phase 25.1은 첫 배포 중 `apphub.yaml` 에 `slug` 없이 `name: nextjs-axhub` 만 있는 경우 resolve 가 멈추고, GitHub repo 연결 안내가 `/axhub:github` handoff 에서 끊기던 문제를 고친 hotfix예요. helper는 slug-like `name` 을 안전한 후보로 쓰고, GitHub 연결은 install_url 을 바로 보여준 뒤 `github_connect` consent token 이 parser binding 과 일치하도록 branch 를 함께 민트해요.

### Test baseline

- Local gate: `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `cargo fmt --all -- --check`, `cargo clippy -p axhub-helpers --all-targets -- -D warnings`, `cargo test -p axhub-helpers`, `bunx tsc --noEmit`, `bun test`, `git diff --check` 가 green이에요.
- Live read-only smoke: `apphub.yaml` 의 `name: nextjs-axhub` 만으로 resolve 가 `app_id=165` / `candidate_slug=nextjs-axhub` 를 찾고, `axhub github repos list --json` 이 install_url 보유 계정을 반환했어요.
- Consent smoke: pending `github_connect` token 이 동일한 `axhub github connect 165 --repo realitsyourman/test2 --branch main --account realitsyourman --json` preauth 를 allow 하는지 확인했어요.
- PR gate: #29 의 Local Rust-primary gate, T2 helper-bin, Rust CI ubuntu/macos/windows 가 success예요.

### Honest tradeoff

- 실제 `axhub github connect` 와 `axhub deploy create` 는 production mutation 이라 실행하지 않았어요. 이번 릴리스는 read-only account/install_url 확인과 consent/preauth binding smoke 로 destructive 직전까지의 실패 지점을 검증해요.
- `github_connect` 의 branch 는 현재 parser 가 top-level branch 와 context branch 를 모두 binding 에 반영하므로 SKILL 이 둘 다 민트하게 맞췄어요. parser/schema 를 단일 source 로 정리하는 작업은 별도 contract 변경으로 남겨요.

### Fixed

* prevent deploy flow from dead-ending before GitHub consent ([f95fc88](https://github.com/jocoding-ax-partners/axhub/commit/f95fc8841dd83bc169e581149667f1e256340c3c))

## [0.2.11](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.10...v0.2.11) (2026-05-06)

Phase 25는 vibe coder가 첫 배포를 5분 안에 끝내도록 PRD, consent schema, bootstrap mutation gate, resolve-first deploy, 측정 E2E를 하나의 안전한 릴리스 단위로 묶은 패치예요. helper가 live `{data:[...]}` app list를 파싱하고 bootstrap-generated consent binding을 parser/preauth와 byte-equivalent로 맞추며, measurement runner는 비용·TTL·preprovisioned app·timeout guard 없이 destructive staging run을 시작하지 않아요.

### Test baseline

- Release chain: `bun run release` postbump가 `codegen:version` + `release:check`를 통과했어요.
- Merge gate: stacked PR #28→#27, #25→#24, #27→#26, #26→#24, #24→#23, #23→main 순서로 머지했고 main push CI (`claude-cli-e2e`, `Rust CI`, `Rust staging gates`)가 success였어요.
- Local Ralph gate: `cargo fmt --all -- --check`, `bunx tsc --noEmit`, `bun run lint:keywords --check`, `bun run lint:tone --strict`, `bun run skill:doctor --strict`, `cargo clippy -p axhub-helpers --all-targets -- -D warnings`, `cargo test -p axhub-helpers`, `bun test`, `bun run build`가 green이에요.

### Honest tradeoff

- Live destructive measurement는 secrets-gated advisory path로 남겨요. 이번 릴리스는 TTL 확인, preprovisioned app id, 비용 budget, per-command timeout을 강제해 안전한 실행 조건을 만든 뒤, 실제 staging mutation은 운영 secret이 있는 workflow에서만 실행되게 해요.
- #26 merge propagation 중 Linux CI에서 test helper `BrokenPipe` race가 한 번 재현되어 `f427b18`로 조기 종료 child의 stdin close만 허용했어요. 명령 결과 assertion은 그대로 두고 BrokenPipe 외 stdin write error는 계속 실패해요.

### Added

* keep bootstrap mutations consent-gated ([db4f65d](https://github.com/jocoding-ax-partners/axhub/commit/db4f65df8666b7e1a2d00fb018c7d4b387038f26))
* make vibe bootstrap SLA measurable without unsafe defaults ([4983c2b](https://github.com/jocoding-ax-partners/axhub/commit/4983c2bbc4e8d92f5bebc4c9bb01bfba57906e84))


### Fixed

* close vibe deploy merge blockers ([8b6d50c](https://github.com/jocoding-ax-partners/axhub/commit/8b6d50c2b3bd36f7641621f105f6a2b3cc4a7f7e))
* prevent consent mint schema drift ([100f1e1](https://github.com/jocoding-ax-partners/axhub/commit/100f1e1f074bd62853f4157f8259efcbbaf2a615))
* unblock first-run deploy helper drift ([f0924cb](https://github.com/jocoding-ax-partners/axhub/commit/f0924cb87439badcb1cac83e8858b2fcf4a764b9))


### Docs

* **prd:** vibe coder 5분 매끄러운 deploy PRD 작성 ([d41cacc](https://github.com/jocoding-ax-partners/axhub/commit/d41cacc096996de1ee0841d09f09e035c7be558f)), closes [#8](https://github.com/jocoding-ax-partners/axhub/issues/8)

## [0.2.10](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.9...v0.2.10) (2026-05-06)

Phase 24.10은 vibe coder 가 multi-step SKILL (deploy / doctor / env / github / init / profile / recover / update / upgrade) 첫 응답에서 native Claude Code TodoWrite 의 orange 진행 박스와 markdown `작업 단계:` + `□ ...` 체크리스트를 동시에 보던 시각 중복을 없앤 UX 패치예요. TodoWrite tool call 만 단일 진행 source 로 남기고 markdown 체크리스트는 9개 SKILL 에서 일괄 제거해요. `skills/deploy/SKILL.md` 의 sentinel `1.5. **Git 저장 지점 준비**` 이후 git-init nested checklist 만 별도 UX 흐름으로 보존하고, `tests/multistep-stage-checklist.test.ts` 를 negative assertion 으로 invert 해 미래 markdown 재유입 drift 를 잠가요.

### Verification

- Regression-first: `tests/multistep-stage-checklist.test.ts` 의 contract 를 `not.toContain("작업 단계")` + `not.toMatch(/^\s*(?:└\s*)?□\s/m)` 로 뒤집고 deploy slug 만 sentinel boundary split 으로 main stage 차단 + nested 보존이 되도록 했어요. boundary 자체도 `expect(parts.length).toBe(2)` 로 lock 해서 sentinel rename 시 즉시 fail.
- 3겹 안전망: `tests/deploy-git-init-stage.test.ts` 에 `(content.match(/작업 단계/g) || []).length === 1` count assertion 추가로 deploy main stage 재유입 시 count=2 fail. content literal lock (`□ git 저장소 만들기`) 은 그대로 유지.
- 0.2.9 release 시 누락된 `PLAN.md` §16.12 plugin/marketplace schema version (0.2.8 → 0.2.9) 도 같이 동기화해 `plan-consistency.test.ts` baseline 을 회복했어요.
- `bun test` 367 pass / 0 fail / 4 skip, `bun run skill:doctor --strict` 17/17 OK, `bun run lint:tone --strict` 0 err, `bunx tsc --noEmit` clean.

### Honest tradeoff

- markdown 체크리스트가 agent 의 self-reminder 로 작동했다는 가설은 검증되지 않았어요. TodoWrite tool call 의 `content` array 가 SKILL.md 본문에 그대로 텍스트로 존재하므로 step 시퀀스는 이미 본문에서 읽혀요. `□` mirror 라인은 시각용이었다고 판단하고 제거했어요. 만약 long-context 세션에서 step 누락 회귀가 보이면 HTML comment sentinel (`<!-- stage-checklist:allow -->`) 패턴 도입이 follow-up.
- `scripts/skill-new.ts` scaffold 와 `scripts/skill-doctor.ts` enforcement 는 이미 markdown 체크리스트를 강제하지 않아 변경 없이 그대로 둬요. 새 SKILL 추가 시 자동으로 markdown 무생성.

### Added

* multi-step SKILL 의 markdown 작업 단계 체크리스트 제거 ([64ada78](https://github.com/jocoding-ax-partners/axhub/commit/64ada785714ba1be820bf6ddf536cc0add88ab12))

## [0.2.9](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.8...v0.2.9) (2026-05-06)

Phase 24.9는 vibe coder 가 `axhub apps delete shopmall` 같은 destructive 명령으로 PreToolUse:Bash hook 에 차단됐을 때 보던 hardcoded `'paydrop 배포해'` 안내가 misleading 하던 문제를 고친 UX 패치예요. helper 가 `parse_axhub_command` 결과의 `action` 과 `app_id` 를 읽어 15개 destructive action (deploy_create / deploy_cancel / deploy_logs_kill / update_apply / auth_login / env_set / env_delete / apps_create / apps_update / apps_delete / github_connect / github_disconnect / profile_add / profile_use / apis_call) 별로 적절한 한국어 NL trigger 어구를 동적으로 만들어 보여줘요. 각 어구는 `detect_prompt_route` 의 `contains_any` 매핑과 일치해서 사용자가 그대로 입력하면 해당 SKILL 의 AskUserQuestion 카드 흐름으로 자연스럽게 연결돼요.

### Verification

- Regression-first: phase_parity 에 hint 동작 케이스 10개 (paydrop baseline lock / shopmall 동적 deploy / apps_delete / env_set·delete / auth_login no-app / profile_use no-app / github_connect / unknown action fallback / empty app placeholder) 를 먼저 작성해 helper 가 hardcoded 문구 대신 동적 hint 를 반환하도록 만들었어요.
- e2e case 23 (`tests/e2e/claude-cli/cases/23-preauth-check-deny.case.sh`) 의 `paydrop 배포해` substring lock 이 그대로 통과하는지 직접 시뮬레이션으로 확인했어요.
- Windows-latest 에서 처음 노출된 `cli_e2e.rs` path separator mismatch 도 같은 PR 에서 `PathBuf::join` 컴포넌트 분리로 묶어 해결해 모든 4 active CI gate (Local Rust-primary / rust ubuntu / rust macos / rust windows) 를 green 으로 회복했어요.

### Honest tradeoff

- preauth-check 게이트 자체 (HMAC consent token 검증, `verify_or_claim_token` runtime path) 은 손대지 않아요. ralplan iter 1-2 에서 검토한 maintainer 전용 dev escape (`AXHUB_PREAUTH_BYPASS` env var, `consent-mint --dev-mode` short-lived scoped token) 은 supply-chain risk vs DX tradeoff 분석 결과 별도 PRD + ADR 로 보관하고 본 릴리스에서는 다루지 않아요. 일반 사용자 NL primary surface 만 개선해서 보안 surface 를 그대로 유지해요.

### Added

* preauth 차단 메시지를 액션·앱별로 동적 생성 ([e110541](https://github.com/jocoding-ax-partners/axhub/commit/e110541affd05e1feabeb5013d8c9dc9e2d3f827))


### Fixed

* **test:** cli_e2e Windows path separator mismatch 해결 ([aacb3ed](https://github.com/jocoding-ax-partners/axhub/commit/aacb3ed9219697c2845cc9de921b4b69a05cc974))

## [0.2.8](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.7...v0.2.8) (2026-05-04)

Phase 24.8은 비개발자가 `/axhub:deploy`에서 git 저장 지점 때문에 멈추지 않도록 만든 UX 패치예요. 비-git 폴더를 helper가 명확히 알려주고, deploy 스킬은 AskUserQuestion으로 로컬 `git init`과 첫 커밋을 선택하게 하며, 모든 multi-step 스킬은 같은 순서의 `작업 단계` 체크리스트를 먼저 보여줘요.

### Verification

- Regression-first: 비-git 폴더에서 `git_init_needed:true`가 나와야 하는 Rust 테스트와 deploy git-init UX 테스트를 먼저 실패시킨 뒤 green으로 고쳤어요.
- Multi-step UX lock: 모든 `multi-step: true` SKILL의 TodoWrite 항목이 사용자-facing `작업 단계` 체크리스트에도 그대로 노출되는 테스트를 추가했어요.
- Local baseline: `cargo test -p axhub-helpers`, `cargo clippy --workspace --all-targets -- -D warnings`, `bun run typecheck`, `bun test`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `bun run release:check`, `git diff --check` 모두 green이에요.

### Honest tradeoff

- 실제 Claude Code 대화형 `/axhub:deploy`에서 git-init 선택지를 누르는 end-to-end smoke는 이번 세션에서 실행하지 않았어요. 대신 helper JSON 계약, SKILL 문구, non-interactive safe default, release artifact build를 회귀 테스트로 잠갔어요.

### Fixed

* guide non-git deploys through a consented git-init stage ([671898f](https://github.com/jocoding-ax-partners/axhub/commit/671898f))
* show visible stage checklists for all multi-step skills ([671898f](https://github.com/jocoding-ax-partners/axhub/commit/671898f))

## [0.2.7](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.6...v0.2.7) (2026-05-04)

Phase 24.7은 `axhub 0.11.0`을 설치한 사용자가 `/github` 같은 preflight-gated slash command에서 막히던 버전 게이트 핫픽스예요. 플러그인 helper의 허용 범위를 다음 minor 전까지로 올려서 현재 CLI와 examples-backed init template 목록을 정상적으로 함께 쓸 수 있어요.

### Verification

- Regression-first: 0.11.0을 허용하는 Rust test를 먼저 실패시킨 뒤 `MAX_AXHUB_CLI_VERSION=0.12.0`으로 고쳐 green을 확인했어요.
- Live preflight: 실제 로그인된 `axhub 0.11.0`에서 `./bin/axhub-helpers preflight --json`이 `in_range:true`, `cli_too_new:false`를 반환해요.
- Local baseline: `cargo test --workspace`, `bun test`, `bash tests/auto-download.test.sh`, `bun run typecheck`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `cargo clippy --workspace --all-targets -- -D warnings`, `bun run release:check`, `git diff --check` 모두 green이에요.

### Honest tradeoff

- 실제 `/github` mutation은 이번 세션에서 실행하지 않았어요. 실패 지점이 slash body 전 preflight라서 같은 helper preflight를 live auth와 CLI 0.11.0으로 직접 검증했어요.

### Fixed

* admit axhub cli 0.11 in preflight ([8090255](https://github.com/jocoding-ax-partners/axhub/commit/8090255e4718b5c6e0c05fe9810e6a5156b802b4))

## [0.2.6](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.5...v0.2.6) (2026-05-04)

Phase 24.6은 Windows 사용자도 같은 경로 규칙을 쓰도록 statusline과 토큰 경로 판단을 Rust helper 계약으로 모은 패치예요. 셸과 PowerShell은 얇은 실행 래퍼로 남기고, 실제 토큰·캐시·state 경로와 statusline 출력은 `axhub-helpers`가 한 번만 결정해요.

### Verification

- Runtime contract: `cargo test --workspace`에서 Rust path/statusline 단위 테스트와 Windows `USERPROFILE` 경로 e2e가 green이에요.
- Wrapper regression: `bash tests/auto-download.test.sh` 9/9 pass, `bun test tests/ux-statusline.test.ts` 9/9 pass예요.
- Release baseline: `bun run build`, `bun test`, `bun run typecheck`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `cargo clippy --workspace --all-targets -- -D warnings`, `bun run release:check`, `git diff --check` 모두 green이에요.

### Honest tradeoff

- 실제 Windows VM에서 PowerShell 훅을 실행하는 smoke는 이번 세션에서 돌리지 않았어요. 대신 Rust `USERPROFILE` 경로 계약과 PowerShell wrapper의 helper-first 흐름을 회귀 테스트와 정적 검토로 잠갔어요.

### Fixed

* centralize plugin runtime paths in Rust ([4321e4e](https://github.com/jocoding-ax-partners/axhub/commit/4321e4ecc2c5b4d0db673857c99dd9f68c550465))

## [0.2.5](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.4...v0.2.5) (2026-05-04)

Phase 24.5는 Windows 사용자까지 고려해 pending consent를 셸 문법이 아니라 helper 계약으로 고정한 패치예요. `tool_call_id:"pending"` 자체가 다음 실제 tool call에서 한 번만 claim 되는 신호가 되므로, 스킬이 `CLAUDE_SESSION_ID`를 지우는 POSIX 전용 명령에 기대지 않아요.

### Verification

- Regression-first: `unset CLAUDE_SESSION_ID`가 destructive SKILL에 남아 있으면 실패하는 manifest test를 추가했고, 실제로 먼저 실패하는 것을 확인했어요.
- Helper contract: `CLAUDE_SESSION_ID`가 있는 상태에서도 `tool_call_id:"pending"`이 pending consent file을 만들고 다음 Bash tool call에서 한 번만 claim 되는 Rust 테스트를 추가했어요.
- Local baseline: `bun test`, `cargo test --workspace`, `bun run typecheck`, `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run release:check` 모두 green이에요.

### Honest tradeoff

- 실제 Windows VM slash smoke는 이번 세션에서 실행하지 않았어요. 대신 macOS/Linux/Windows 공통 helper 계약과 스킬 문구 회귀 테스트로 `unset` 의존을 차단했어요.

### Fixed

* make pending consent portable ([46dbe66](https://github.com/jocoding-ax-partners/axhub/commit/46dbe664fdb1d7801cbbc7901fadafa8a182a311))

## [0.2.4](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.3...v0.2.4) (2026-05-04)

Phase 24.4는 실제 Claude 서브프로세스에서 모든 `/axhub:*` 슬래시 명령을 호출하며 발견한 라이브 배포 consent 문제를 막는 패치예요. 미래 Bash tool id를 미리 맞히려 하지 않고, action/app/profile/branch/commit/context가 모두 맞을 때만 한 번 claim 되는 pending consent로 실제 `/axhub:deploy`가 배포까지 도달해요.

### Verification

- Live slash QA: Claude subprocess로 `/axhub:deploy` 실제 호출 → deployment `485`가 `active`예요. `/axhub:apps`, `/axhub:apis`, `/axhub:doctor`, `/axhub:login`, `/axhub:status`, `/axhub:logs`, `/axhub:update`, `/axhub:배포 --dry-run`, `/axhub:help`도 exit 0이에요.
- Local regression baseline: `cargo test --workspace`, `bun test` → 344 pass / 4 skip / 0 fail, `bun run typecheck`, `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check` 모두 green이에요.
- Release baseline: `bun run release -- --release-as patch` postbump에서 `bun run codegen:version`과 `bun run release:check`가 green이에요.

### Honest tradeoff

- `/axhub:login`은 이미 유효한 계정이 있어 상태 확인 경로만 실제 slash로 실행했어요. 브라우저 re-login mutation은 사용자 세션 교체를 피하려고 Rust preauth pending-token 회귀 테스트로 잠갔어요.
- QA용 앱은 private `axhub-qa-slash-1777869574`로 남겨서 deployment 증거를 보존해요.

### Fixed

* unblock real Claude slash deploy consent ([73b9e93](https://github.com/jocoding-ax-partners/axhub/commit/73b9e933dda0185a3464c039a5790f34f553d0dc))

## [0.2.3](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.2...v0.2.3) (2026-05-04)

Phase 24.3은 실제 로그인한 staging 계정으로 CLI read-only E2E를 다시 돌려서 발견한 응답 포맷 drift를 막는 패치예요. 앱 목록은 `data[]`와 기존 `apps[]`/배열 응답을 모두 같은 의미로 받아들이고, Rust helper 성공 응답의 `null` error field도 정상 성공으로 해석해요.

### Verification

- Live staging QA: macOS Keychain 토큰을 출력하지 않고 주입해서 `bun run test:e2e` → 5 pass / 1 skip / 0 fail이에요.
- Local regression baseline: `bun test` → 344 pass / 4 skip / 0 fail, `bunx tsc --noEmit`, `git diff --check` 모두 green이에요.
- Release baseline: `claude plugin validate .claude-plugin/plugin.json`, `bun run test:plugin-e2e:t1` → 8/8 pass, `bun run release:check` 모두 green이에요.

### Honest tradeoff

- Windows live keychain 경로는 이번 세션에서 토큰이 macOS Keychain에 있어 직접 검증하지 않았어요.
- Staging app id는 현재 로그인 계정이 읽을 수 있는 `ccrank` 앱으로 read-only 조회만 수행했어요.


### Fixed

* Track latest axhub staging app envelopes ([28af5be](https://github.com/jocoding-ax-partners/axhub/commit/28af5be26f28d8c1cb49ae9e02900c190c3cefb8))

## [0.2.2](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.1...v0.2.2) (2026-05-04)

Phase 24.2는 실제 Claude Code plugin validator에서 발견된 SKILL frontmatter 파싱 오류를 막는 긴급 패치예요. 모든 SKILL description은 트리거 문구를 유지한 채 YAML-safe quoted scalar로 감싸서, 설치된 플러그인에서도 metadata가 비지 않고 자연어 라우팅에 남아요.

### Verification

- Failure reproduced: `claude plugin validate .claude-plugin/plugin.json` on clean `v0.2.1` tag clone → SKILL frontmatter YAML parse errors예요.
- Local fixed plugin: `claude plugin validate .claude-plugin/plugin.json`, isolated marketplace install, installed-path `/axhub:help` subprocess smoke 모두 green이에요.
- Regression baseline: `bun test` → 342 pass / 4 skip / 0 fail, `bun run test:plugin-e2e:t1` → 8/8 pass, `bunx tsc --noEmit`, `bun run lint:keywords --check`, `bun run lint:tone --strict`, `bun run skill:doctor --strict`, `bun run release:check` 모두 green이에요.

### Honest tradeoff

- Staging token E2E는 `AXHUB_E2E_STAGING_TOKEN`이 없어 credential-gated skip으로 남아요.
- GitHub release workflow의 Node.js 20 deprecation 경고는 이번 패치의 실패 원인이 아니라서 별도 follow-up으로 남겨요.

### Fixed

* Keep Claude plugin skills loadable in released installs ([98a3290](https://github.com/jocoding-ax-partners/axhub/commit/98a329082c6490c418b89093e9c3afc98fe66fdc))

## [0.2.1](https://github.com/jocoding-ax-partners/axhub/compare/v0.2.0...v0.2.1) (2026-05-04)

Phase 24.1은 init 템플릿 선택 화면을 바이브코더 친화적으로 다듬는 패치예요. 템플릿 id는 계속 `axhub --json init --list-templates` 결과만 믿고, 플러그인은 쇼핑몰·예약·결제·문서·입력 폼 같은 만들고 싶은 결과물 기준 설명만 덧붙여요.

### Verification

- Local release baseline: `bun test` → 341 pass / 4 skip / 0 fail, `bunx tsc --noEmit`, `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `git diff --check` 모두 green이에요.
- Targeted UX gate: `bun test tests/init-template-guidance.test.ts tests/plan-consistency.test.ts` → 17 pass / 0 fail이에요.
- Release baseline: `bun run release -- --release-as patch` postbump에서 `bun run codegen:version` 과 `bun run release:check`가 green이에요.

### Honest tradeoff

- 이 패치는 CLI 템플릿 schema를 확장하지 않아요. CLI가 새 template id를 돌려주면 숨기지 않고 CLI `framework` / `description` 을 보여준 뒤 중립 안내만 붙여요.
- Staging token E2E는 credential gated라 로컬에서 실행하지 않았어요.

## [0.2.0](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.26...v0.2.0) (2026-05-02)

Phase 24는 ax-hub-cli v0.10 lifecycle surface를 플러그인 v0.2.0으로 묶는 릴리즈예요. 새 init/env/github/open/whatsnew/profile SKILL과 Rust prompt-route/preauth/consent context를 current CLI main에 맞추고, helper bootstrap·admin onboarding·remote `templates.json`는 의도적으로 deferred로 남겨요.

### Verification

- PR #20 checks: Rust CI ubuntu / macOS / Windows, Rust staging local gate, claude-cli-e2e T2 모두 PASS예요.
- Local baseline: `bun test` → 336 pass / 4 skip / 0 fail, `cargo test --workspace`, `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `bun run bench:hooks`, `bun run test:plugin-e2e:t2`, `bun run release:check` 모두 green이에요.
- CLI contract smoke: `/Users/wongil/Desktop/work/jocoding/ax-hub-cli` main에서 `go run ./cmd/axhub --json init --list-templates` schema `init/v1` 확인했어요.
- GitNexus: worktree staged detect_changes는 consent/preauth/prompt-route 핵심 흐름 영향 때문에 CRITICAL로 분류됐고, 해당 위험은 Rust phase parity, CLI e2e, hook latency, T2 lifecycle smoke로 잠갔어요.

### Honest tradeoff

- Live Claude T1/T3 matrix와 staging token E2E는 로컬에서 실행하지 않았어요. 비용·credential gated 경로라 PR-blocking T2와 Rust CI를 release gate로 사용해요.
- v0.2.0은 Node/CLI/dependency 자동 설치 릴리즈가 아니에요. 템플릿 source of truth는 계속 `axhub init --list-templates`예요.


### Added

* cover current CLI lifecycle without bootstrap drift ([9147ac4](https://github.com/jocoding-ax-partners/axhub/commit/9147ac4f80d4ed7a28fa8ee983fe9b50c4d3dc3e))


### Changed

* **helper:** TS shadow 박멸, Rust binary single source-of-truth ([747a5a6](https://github.com/jocoding-ax-partners/axhub/commit/747a5a66b3b9eaffd2638f5ac6352b2914e9fed1))


### Docs

* cli-coverage v0.2.0 phase plan 12 문서 신설 ([1db57d4](https://github.com/jocoding-ax-partners/axhub/commit/1db57d4448a6911414f9af5920f59221b1dcf9d8))

## [0.1.26](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.25...v0.1.26) (2026-04-29)

Phase 23.1은 v0.1.25 태그에서 멈춘 darwin-amd64 release lane을 복구하는 패치예요. GitHub가 2025-12-04에 retired 처리한 `macos-13` 대신 공식 Intel label인 `macos-15-intel`로 바꿔서 5개 Rust helper artifact가 다시 서명·업로드되게 해요.

### Verification

- Blocked release run: `v0.1.25 release` run `25100364587`은 `macos-13` darwin-amd64 job 대기 때문에 cancelled 처리했어요.
- Local release baseline: `bun run release:check` → 5-asset release wiring OK, host Rust helper version `0.1.25` 확인 후 patch bump를 진행했어요.

### Honest tradeoff

- `macos-15-intel`도 Intel macOS 마지막 세대에 속해요. GitHub 공지상 macOS x86_64 지원은 macOS 15 runner retirement까지라서, 장기적으로는 darwin-amd64 artifact 폐기나 self-hosted Intel runner 전략이 필요해요.

### Fixed

* **release:** keep Intel macOS artifact builds on a supported runner ([0ad65ad](https://github.com/jocoding-ax-partners/axhub/commit/0ad65ad03baff018df2d4694b0693fcf30bb0a0f))

## [0.1.25](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.23...v0.1.25) (2026-04-29)

Phase 23은 PR #19에서 검증한 Rust helper 기본 전환을 실제 배포 태그로 묶는 릴리즈예요. `AXHUB_HELPERS_RUNTIME=auto`는 Rust native helper를 우선 쓰고, 회귀가 보이면 `AXHUB_HELPERS_RUNTIME=ts`로 즉시 돌아갈 수 있게 fallback은 남겨요.

### Verification

- PR #19 필수 checks: Rust CI ubuntu / macOS / Windows, Rust staging local gate, claude-cli-e2e T2 모두 SUCCESS.
- Local baseline: `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `bunx tsc --noEmit`, `cargo llvm-cov --workspace --fail-under-lines 90` → 91.10% line coverage.
- Release baseline: `bun run release:check`, `bun test` → 570 pass / 6 skip / 0 fail.

### Honest tradeoff

- TypeScript fallback은 이번 릴리즈에 남겨요. live staging secrets run, Windows V3/AhnLab cohort, 24h fuzz(`fuzz_minutes=1440`)까지 끝난 뒤 삭제가 안전해요.

### Added

* prove the Rust helper cutover through staging gates ([2ca7e1c](https://github.com/jocoding-ax-partners/axhub/commit/2ca7e1c6c9b1fe775475640d9bf063337c5df281))

## [0.1.24](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.23...v0.1.24) (2026-04-29)

## [0.1.23](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.22...v0.1.23) (2026-04-28)

Phase 22 의 claude -p subprocess E2E harness 후속 phase 4건 (22.0.3 / 22.3 / 22.4 / 22.5) + CI hardening 을 한 릴리즈예요. unauth/error / token_expired / cli_too_old / mock-hub 401 / consent gate bypass 시나리오 모두 closed-loop. production code 영향은 22.0.3 의 deploy SKILL Step entry sentinel (`echo '[deploy:Step N ...] entered' >&2`) 5건만 — 나머지 phase 는 test infra / fixture / case assertion 강화에 한정.

### Added

- **deploy SKILL Step entry sentinel** (22.0.3): Step 1/2/4/5/8 bash block 첫 line 에 stderr 마커 — telemetry / debugging / regression baseline 용. case 18 alias parity 와 orthogonal.
- **claude-cli E2E unauth/error case 5건** (22.3): T1 status/deploy/doctor NL + T2 preauth-check direct deny + T2 list-deployments TLS-pin baseline. SKILL routing + 한국어 phrase 검증으로 ralph PR-blocking surface 확장.
- **fixture infra 확장** (22.4):
  - `fixtures/bin/axhub`: `AXHUB_FIXTURE_VERSION` (cli_too_old/cli_too_new 강제) + `AXHUB_FIXTURE_AUTH=expired` (auth status / deploy 시뮬레이션 exit 65 + token_expired) env 지원.
  - `lib/mock-hub.ts`: `MOCK_HUB_AUTH_FAIL=1` env → `/v1/*` + `/api/v1/*` 401 token_expired (`_ping` 보호).
  - `lib/spawn.sh`: case 별 `FIXTURE_AXHUB_VERSION` / `FIXTURE_AXHUB_AUTH` env propagate.
- **case 19/22/34 강화** (22.4): plan 의 token_expired / cli_too_old / mock-hub 401 stdout positive evidence 시나리오 정확 매칭.
- **case 23/34 assertion 강화** (22.5): case 23 의 systemMessage 4 token AND lock (`사전 승인` / `필요해요` / `paydrop 배포해` / `승인 카드`) — production 메시지 drift catch. case 34 mock-hub log file 에 `GET /api/v1/apps/42/deployments` line assert — fetch URL 라우팅 결정적 evidence.

### CI

- **`bun install` step 추가**: `claude-cli-e2e.yml` 의 e2e-pr + e2e-nightly 두 job 모두 `bun run build` 전에 dependency resolve 단계 추가. semver/jose 런타임 의존성을 `bun build --compile` 가 못 찾던 회귀 fix (PR-blocking T2 job 처음 실행 시 노출).

### Verification

- `bun test`: **550 pass / 5 skip / 0 fail / 2872 expect()**.
- `bunx tsc --noEmit`: clean.
- `bun run lint:tone --strict`: 0 error / 0 warning.
- `bun run lint:keywords --check`: no diff vs baseline.
- `bun run skill:doctor --strict`: OK.
- `bash run-matrix.sh --tier t2 --only 23 34`: **2/2 PASS** (case 23 4-phrase AND match + case 34 mock-hub 401 stdout `error_code='auth.token_invalid'` + log line `GET /api/v1/apps/42/deployments`).
- 4 PR (#13/#14/#17/#18) sequential merge — 22.0.3 → 22.3 → 22.4 → 22.5. 22.4/22.5 stack 충돌은 `--ours` 로 해결 (강화 버전 우선).

### Honest tradeoff

- T1 case 05/19/22 의 nightly 실측 검증은 ANTHROPIC_API_KEY 의존 (`e2e-nightly` job). PR-blocking T2 surface 만 결정적 검증, T1 surface 는 cron + workflow_dispatch 에서 fire.
- case 23 의 4 token AND lock 은 production systemMessage refactor 시 case fail (의도된 friction — drift signal). case 34 의 mock-hub log path coupling 은 `mock-hub.sh` API 변경 시 case 동시 update 필요.

### Docs

* README 를 v0.1.22 출하 상태에 맞게 갱신 ([8eca06b](https://github.com/jocoding-ax-partners/axhub/commit/8eca06ba38abe277db39dd9a0ec7dc81cf726d23))

## [0.1.22](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.21...v0.1.22) (2026-04-28)

Hotfix release for the visible SessionStart startup error captured in the user screenshot. Claude Code runs matching hook entries on the current host and reports non-blocking hook spawn failures; the universal axhub hook config was registering a Windows PowerShell SessionStart sibling on macOS/Linux, where `pwsh`/`powershell` is usually absent.

### Fixed

- **SessionStart hook noise**: universal `hooks/hooks.json` now registers only the bash SessionStart shim, removing the unconditional `shell:powershell` sibling that caused visible startup errors on non-Windows hosts.
- **Regression guard**: `tests/manifest.test.ts` now asserts universal SessionStart hooks do not reference `session-start.ps1` or require PowerShell.
- **Pilot docs**: Windows pilot/admin docs now state that stock Windows automatic SessionStart requires future platform-specific hook packaging; current Windows fallback is `AXHUB_TOKEN`/`token-import` or Git Bash/WSL.

### Verification

- Red test first: `bun test tests/manifest.test.ts --test-name-pattern 'SessionStart registers only|does not require PowerShell'` failed against the old 2-hook config.
- `bun test` → 545 pass / 5 skip / 0 fail.
- `bash tests/auto-download.test.sh` → 8 pass / 0 fail.
- `bunx tsc --noEmit`.
- `bun run lint:tone --strict` → 0 error / 0 warning.
- `bun run lint:keywords --check` → OK.
- `bun run skill:doctor --strict`.
- `bun run release:check` → 5 cross-arch binaries rebuilt/checked at `0.1.22`.
- `git diff --check`.

### Honest tradeoff

- This prioritizes eliminating visible startup errors for all non-Windows users. Stock Windows automatic SessionStart is paused until the plugin has platform-conditioned hook packaging or a verified Claude Code platform matcher. The PowerShell scripts remain in the repo for manual smoke and future packaging work.

## [0.1.21](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.20...v0.1.21) (2026-04-28)

Phase 21 — PLAN.md 잔여 항목을 최신 `main` 기준으로 끝까지 닫은 릴리즈예요. PR #4–#12 를 순서대로 머지해 명령 표면, corpus replay, SessionStart preflight, hook latency, supply-chain 문서/검증, recover 문서, hub-api TLS pinning, PLAN evidence ledger, 현재 레이아웃/schema 동기화까지 반영했어요.

### Added

- **Scoreable corpus replay**: `tests/run-corpus.sh` fixtures now assert replayable outcomes instead of placeholder execution only.
- **SessionStart preflight diagnostics**: startup checks now surface concrete axhub install/version/auth guidance.
- **Hook latency benchmark**: no-op hook latency is measurable through a dedicated benchmark and regression coverage.
- **PLAN evidence ledger**: best-practices checklist rows now carry evidence instead of untracked TODO state.

### Fixed

- **Command-surface drift**: active PLAN scope now excludes canceled plugin-server work and matches the shipped command set.
- **hub-api deployment fallback trust**: deployment-list fallback pins the expected hub-api TLS certificate before REST fallback calls.
- **Release/supply-chain plan drift**: PLAN release artifact guidance now matches the signed helper binary release pipeline.
- **Recover/docs drift**: recover guidance is marked as shipped and troubleshooting docs align with current behavior.
- **Current layout/schema drift**: PLAN schema and repository layout references now match the implementation merged on `main`.

### Verification

- `bun test` → 546 pass / 5 skip / 0 fail
- `bunx tsc --noEmit`
- `bun run lint:tone --strict` → 0 error / 0 warning
- `bun run lint:keywords --check` → OK
- `bun run skill:doctor --strict`
- `bun run release:check` → OK at v0.1.20 before bump, then postbump rebuilt v0.1.21 artifacts
- `bun run test:e2e` against `https://axhub-api.jocodingax.ai` → 4 pass / 1 skip / 0 fail

### Honest tradeoff

- This is a patch release because the merged work is hardening, documentation/schema synchronization, and test coverage. No new public command contract is introduced.

## [0.1.20](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.19...v0.1.20) (2026-04-27)

Phase 20 — exhaustive review bugfix release. PR #3 reviewed the full 221-file inventory from latest `origin/main` and ships only evidence-backed fixes: consent-token safety, release automation drift, and user-facing skill/docs contract drift.

### Fixed

- **Consent token safety**: `consent-mint` now fails fast when `CLAUDE_SESSION_ID` is absent instead of writing unverifiable per-process tokens; token writes reject symlinked consent paths and use `O_NOFOLLOW`.
- **Release automation drift**: patch releases now stage generated version files before release checks, avoid post-tag amend guidance, require an explicit semver tag for manual dispatch, and keep `.pem` cosign certificate sidecars out of binary manifests.
- **Skill/docs contract drift**: command frontmatter grants `axhub-helpers` where delegated skills need it; consent examples use stdin JSON; headless auth docs use `token-import`, `~/.config/axhub-plugin/token`, and `AXHUB_TOKEN`; stale unavailable `deploy list` / helper schedule instructions were replaced with helper-backed flows.
- **Public docs drift**: repository links now point to `jocoding-ax-partners/axhub`, and pilot launch guidance points at the current release line.

### Tests

- Added regression coverage for missing `CLAUDE_SESSION_ID` across the real CLI process boundary and for symlinked consent-token paths.
- Added release/manifest guards for `.pem` exclusion, workflow dispatch tag handling, generated version-file staging, and no post-tag amend guidance.
- Added manifest/docs guards for unsupported token-file/token-install/consent flag drift, helper permissions, auth logout confirmation, and unavailable deploy-list/helper-schedule instructions.
- Tightened staging E2E app-list response shape checks and skill-doctor diagnostic coverage.

### Verification

- `bun run typecheck`
- `bun run lint:tone --strict`
- `bun run lint:keywords --check`
- `bun run skill:doctor --strict`
- `bun test` → 515 pass / 5 skip / 0 fail
- `bun run fuzz` → 1100/1100 caught
- `bun run smoke`
- `bun run smoke:full` → docs link audit Broken: 0
- `bun run build:all`
- `bun run release:check`
- `bun run test:e2e` against `https://axhub-api.jocodingax.ai` → 4 pass / 1 skip / 0 fail

### Honest tradeoff

- `CLAUDE_SESSION_ID` is now a hard requirement for consent mint/verify. Persisting a fallback session id was rejected because separate helper processes could collide across sessions when Claude does not provide a session id.

## [0.1.19](https://github.com/jocoding-ax-partners/axhub/compare/v0.1.18...v0.1.19) (2026-04-27)

Phase 19 — `bun run release` 한 줄로 버전 범프 자동화. v0.1.10..v0.1.18 까지 9 release 동안 5 파일 수동 편집 + codegen + release:check + commit + tag 를 매번 따로 했어요. 이제 commit-and-tag-version (D2) 가 한 번에 chain — Conventional Commits 파싱 + 3 파일 bump + postbump hook 으로 codegen:version + release:check 자동 실행 + CHANGELOG entry generation + git commit + tag. 사람은 narrative paragraph 만 amend 로 추가하면 돼요. ralplan 분석에서 D1 release-please 거절 이유: PR rubber-stamp 가 v0.1.14 stale-binary 같은 trust-without-verify drift 재발 위험, 한국어 narrative 자동 생성 어색함, axhub 의 hotfix 빈번 cadence 와 weekly bot-PR cadence 미스매치.

### Added

* Phase 19 v0.1.19 — auto version bump via commit-and-tag-version ([98befbf](https://github.com/jocoding-ax-partners/axhub/commit/98befbf1a89cbdc7c95ba134009be70956555af9))


### Docs

* **v0.1.18:** AGENTS.md + CLAUDE.md add Skill Authoring section ([666fc1a](https://github.com/jocoding-ax-partners/axhub/commit/666fc1a3a36f9d81055ab947c74b4835ba72d927))

### Test baseline

- `bun test` → 498 pass / 5 skip / 0 fail / 503 tests / 28 files (preserved from v0.1.18).
- `bunx tsc --noEmit` → clean.
- `bun lint:tone --strict` → 0 error / 0 warning across 29 files.
- `bun run release:check` → OK at v0.1.19, 5 cross-arch binaries verified (auto-ran in postbump).

### Honest tradeoff

- CHANGELOG narrative paragraph (해요체) 는 사람이 작성 — auto-bullets 만으로는 Phase 의미 전달 부족. 사용자 workflow: `bun run release` 후 `vim CHANGELOG.md` → `git commit --amend --no-edit -a`.
- D1 release-please 는 future Phase 에서 multi-contributor 단계가 되면 재검토. 현재 solo 단계에서는 D2 가 단순 + 안전.

## [0.1.18] — 2026-04-27

Phase 18 — 새 SKILL 자동 적용 인프라. Plan: `.omc/plans/phase-18-skill-scaffold-automation-v2.md` (Critic APPROVE round 3).

### Added

- **Frontmatter 선언 (R1)**: 11 SKILL 모두 `multi-step:` + `needs-preflight:` 두 키 추가. Hardcoded `MULTI_STEP_OPT_OUT` / `PREFLIGHT_REQUIRED` 배열 제거. 새 SKILL 추가 시 frontmatter 만 선언하면 모든 패턴 검사가 자동 enforce 돼요.
- **`scripts/skill-doctor.ts`** (R5/US-1806) — colored 한글 진단 출력. SKILL 별 D1 sentinel / TodoWrite / `!command preflight` 패턴 체크. `--strict` mode 는 machine-parseable (`skills/<slug>/SKILL.md: missing <pattern>`), CI 용. `bun run skill:doctor` 호출.
- **`scripts/skill-new.ts`** (R2/US-1803) — `bun run skill:new <slug> [flags]` 스캐폴드. `skills/_template/SKILL.md.tmpl` 에서 Phase 17/18 패턴 미리 emit. 기본 mutate-aware (multi-step:true + needs-preflight:true). Flags: `--no-multi-step`, `--no-preflight`, `--action`, `--title`. registry stub 자동 append.
- **`tests/ux-skill-template-completeness.test.ts`** (R5/US-1805) — meta-test, `skill:doctor --strict` wrapper. CI 가 새 SKILL 패턴 누락 시 fail.
- **`tests/ux-skill-preflight-injection.test.ts`** (R2/US-1804) — frontmatter `needs-preflight:true` 선언 SKILL 마다 `!command preflight` literal 존재 assert.
- **`skills/_template/SKILL.md.tmpl`** — 새 SKILL 작성 시 출발점. inline AUTHOR 가이드 주석 (Phase 17/18 패턴 5개).

### Changed

- **`scripts/check-toss-tone-conformance.ts`** (R4) — `PHASE_13_FILES` 에 `skills/*/SKILL.md` glob 추가. Frontmatter (description: nl-lexicon trigger 포함) 는 SKIP — workflow body 만 lint.
- **`tests/ux-todowrite.test.ts`** (R1) — hardcoded 5 SKILL 배열 제거. glob + frontmatter `multi-step:` read. 새 multi-step SKILL 자동 enforce.
- **`tests/manifest.test.ts`** (R2): frontmatter allowlist 에 `multi-step` + `needs-preflight` 추가. skill scan 에서 leading `_` dir (e.g. `_template`) 제외.
- **14 workflow body fixes** (C0.5): lint:tone scope 확장 후 발견된 pre-existing 위반 14개 (Phase 14 deferred 영역) 모두 fix. recover/logs/status/deploy/auth/update SKILLs.

### Test baseline

- `bun test` → 498 pass / 5 skip / 0 fail / 503 tests / 28 files (+21 from v0.1.17).
- `bunx tsc --noEmit` → clean.
- `bun lint:tone --strict` → 0 error / 0 warning across 29 files.
- `bun lint:keywords --check` → OK (no diff vs baseline).
- `bun run skill:doctor --strict` → 11/11 SKILLs complete.
- `bun run release:check` → OK at v0.1.18, 5 cross-arch binaries verified.

### Honest tradeoff

- D2 universal PreToolUse hook injection — Phase 19 trigger if drift recurs. Registry test enables mechanical migration.
- Notification hook on missing pattern — Phase 19 deferred.
- SDK `permissionDecision: "defer"` — Phase 19+ deferred (axhub scope = TUI + claude -p).
- MCP elicitation — Phase 20+ deferred (no MCP server today).
- `docs/SKILL_AUTHORING.md` — deferred (template comments 로 흡수).
- Statusline auto-install — 사용자 opt-in (`~/.claude/settings.json` 직접 추가).

## [0.1.17] — 2026-04-27

Phase 17 — vibe coder UX uplift across 11 SKILLs / 9 commands. Plan: `.omc/plans/phase-17-ux-uplift-v2.md` (ralplan consensus, Critic round 3 APPROVE).

### Added

- **D1 TTY guard** (C1/US-1701) — 11 SKILLs 모두 non-interactive context (`! [ -t 1 ] || $CI || $CLAUDE_NON_INTERACTIVE`) 에서 AskUserQuestion 호출 건너뛰고 안전한 기본값으로 진행. v0.1.12 status/logs + v0.1.15 deploy hotfix 와 동일 패턴, 이번에 AskUserQuestion 표면 전체에 적용.
- **TodoWrite progress UI** (C2/US-1702) — deploy/recover/update/upgrade/doctor 5 SKILL 워크플로 시작에 TodoWrite 추가. Vibe coder 가 실시간 체크박스로 어디까지 왔는지 한 눈에 봐요. activeForm 모두 해요체.
- **AskUserQuestion polish** (C3/US-1703) — 8 SKILL 의 모든 question JSON 에 `header` 필드 (≤12 char chip) 추가. 질문 문자열 해요체 통일 (보시겠어요/원하시나요/할까요 → 볼래요/원해요/해요).
- **argument-hint frontmatter** (C4/US-1704) — commands/help.md 추가 (8/9 이미 있었어요). 슬래시 명령 자동완성에 hint 표시.
- **!command preflight injection** (C6/US-1706) — deploy/recover/apis/apps SKILL 시작에 `!\`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json\`` 사전 실행. 모델 컨텍스트에 auth_status / current_app / current_env / last_deploy_id / last_deploy_status / plugin_version 자동 주입. Step 1 별도 bash round-trip 줄어요. PreToolUse Bash hook 은 preprocessing 에서 trigger 안 해요.
- **Statusline** (C7/US-1707) — `bin/statusline.sh` 신규. 캐시 + 토큰 읽고 ≤80 char 한국어 한 줄 출력 (`axhub: <app> · <profile> · 최근 배포 <SHA8> (<status>)`). 사용자가 `~/.claude/settings.json` 에 `statusLine` 설정 추가하면 켜져요. deploy/recover SKILL 워크플로 끝에 `~/.cache/axhub-plugin/last-deploy.json` 캐시 기록.
- **Per-question fallback registry** (C5/US-1705) — `tests/fixtures/ask-defaults/registry.json` 신규. SKILL 별 × 질문 별 safe_default + rationale. drift catch — 새 AskUserQuestion 추가시 registry 등록 안 하면 test FAIL.
- **6 new test files** — `tests/ux-{todowrite,askuserquestion-headers,argument-hints,ask-fallback-defaults,ask-fallback-registry,statusline}.test.ts`. v0.1.14/v0.1.15 drift mode 회복 mechanical 차단.
- **Strict subprocess smoke** — `tests/live-plugin-smoke.sh SMOKE_STRICT=1` 기본. TIMEOUT 또는 non-zero exit 시 harness 자체가 exit 1. capture-only 모드는 SMOKE_STRICT=0 으로 보존.

### Changed

- `src/axhub-helpers/preflight.ts PreflightOutput` — current_app / current_env / last_deploy_id / last_deploy_status / plugin_version 5 필드 확장. 기존 cli_version / auth_ok 출력은 backward-compatible.

### Test baseline

- `bun test` → 477 pass / 5 skip / 0 fail / 2510 expect / 482 tests / 26 files (+74 from v0.1.16, +6 test files).
- `bunx tsc --noEmit` → clean.
- `bun lint:tone --strict` → 0 error / 0 warning across 18 files.
- `bun lint:keywords --check` → OK no diff vs baseline.
- `bun run release:check` → OK at v0.1.17, 5 cross-arch binaries verified.

### Honest tradeoff

- D2 universal PreToolUse hook injection (vs D1 per-call guard) — deferred to Phase 18 if drift recurs in v0.1.18. Registry test enables mechanical D2 migration with zero re-derivation cost.
- Notification hook (Slack/webhook for non-terminal vibe coders) — deferred Phase 18.
- Agent SDK `permissionDecision: "defer"` for SDK consumers embedding axhub — deferred Phase 19+.
- MCP elicitation — deferred Phase 20+ (axhub doesn't ship MCP server today).
- Statusline auto-install — Claude Code plugin can't install user-level `statusLine` config; user opts in by adding to `~/.claude/settings.json`. Doc/PR follow-up.
- deploy SKILL Step 3 preview is text card (5-field identity), NOT structured JSON — registry handles via `default_subprocess_action`. JSON migration deferred Phase 18.

## [0.1.16] — 2026-04-27

Hotfix follow-up — v0.1.15 honest-tradeoff entry promised release procedure 강제. 이번 cycle 에서 처리.

### Added

- `scripts/release-check.ts` — release pre-flight script. `codegen:version` → `bun run build` → `bun run build:all` → host 가 실행 가능한 모든 binary 의 `--version` 출력이 package.json version 과 일치하는지 assert. v0.1.14 의 stale binary bug (bin/axhub-helpers v0.1.10 보고하면서 source 는 v0.1.14) 재발 방지.
- `package.json scripts.release:check` — `bun run release:check` 한 줄로 호출.
- `docs/RELEASE.md` 절차 step 2 에 MANDATORY 표기 + v0.1.14 사고 회고 1 줄 인용. 정렬: bump → release:check → regression → commit/tag.

### Test baseline

- `bun test` → 403 pass / 5 skip / 0 fail (변동 없음 — 새 script 자체는 pre-flight 도구라 unit test 불필요, integration 검증은 release:check 자기 실행으로 대체).
- `bunx tsc --noEmit` → clean.
- `bun lint:tone --strict` → 0 error / 0 warning.
- `bun run release:check` → OK at v0.1.16, 4 host-runnable binaries verified.

### Honest tradeoff

- pre-push git hook 추가 옵션 검토했으나 보류 — opt-in script (`release:check`) + RELEASE.md MANDATORY 표기로 충분, hook 강제는 contributor onboarding 마찰만 늘어요. 두 번째 사고 발생 시 hook 도입 재검토.

## [0.1.15] — 2026-04-27

Hotfix — subprocess `claude -p` smoke 재실행으로 발견한 두 가지 버그.

### Fixed

- `skills/deploy/SKILL.md` step 5 post-deploy chain — `axhub deploy status dep_$DEPLOY_ID --watch --json` 이 v0.1.12 status/logs hotfix 와 동일한 hang 패턴인데 빠져 있었어요. `if [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then WATCH=--watch; else WATCH=; fi` shell guard 추가, `axhub deploy status dep_$DEPLOY_ID $WATCH --json` 으로 변경. headless/CI/`claude -p` 환경에서 `/axhub:deploy` 후속 watch 단계가 무한 정지하지 않아요.
- `bin/axhub-helpers` 로컬 빌드 stale (v0.1.10 보고) 발견. v0.1.14 release 시 `bun run build` 누락. `bun run build` 재실행하니 v0.1.14 보고. .gitignore 라 git 에 영향 없으나 plugin directory mode 사용자가 영향 받았어요. 빌드 자동화 follow-up: release 절차에 `bun run build:all` 강제 단계 추가 필요 (v0.1.16).

### Added

- `tests/skill-noninteractive-guard.test.ts` — deploy SKILL guard 2 개 assertion 추가 (`[ -t 1 ]` + `WATCH=--watch` / `WATCH=;` 토글, raw `--watch` 검출 negative test). 총 8 pass.

### Detection win

`tests/live-plugin-smoke.sh` 9/9 명령 (`/axhub:help`, `/axhub:status`, `/axhub:doctor`, `/axhub:apps`, `/axhub:apis`, `/axhub:login`, `/axhub:logs`, `/axhub:update`, `/axhub:deploy --dry-run`) 모두 exit 0, TIMEOUT 0건. v0.1.12 status/logs 가드 + v0.1.15 deploy 가드로 subprocess hang 패턴 0건. 단 `/axhub:deploy` 의 post-chain 은 dry-run 으로 우회되어 가드 자체는 코드 경로상 검증, `tests/skill-noninteractive-guard.test.ts` regression 으로 잠금.

### Honest tradeoff

- `/axhub:apis` 출력에 `작동하지 않았습니다` (T-01 합니다 위반) Claude 자연 한국어 — SKILL workflow body 가 영문이라 모델이 default polite Korean. Phase 14 (SKILL 본문 Toss 마이그레이션) 영역, v0.1.15 scope 외.

### Test baseline

- `bun test` → 403 pass / 5 skip / 0 fail / 2394 expect / 408 tests / 20 files (+2 deploy guard).
- `bunx tsc --noEmit` → clean.
- `bun lint:tone --strict` → 0 error / 0 warning across 18 files.
- Live: `.omc/evidence/live-plugin-smoke-summary.txt` (9/9 exit 0).

## [0.1.14] — 2026-04-27

Phase 13 — Toss UX Writing 톤 마이그레이션 (Tier A+B+C+D 런타임 + commands + install + hooks). Phase 14 (docs + SKILL workflow) + Phase 15 (SKILL descriptions) deferred per consensus plan.

### Changed

- `src/axhub-helpers/catalog.ts` 13 exit-code Korean templates → Toss 톤. 합니다/입니다 → 해요/예요/이에요. 직접 호칭 (`당신`) 0회. 5× `취소` → `닫기` (5번), 1× `취소` retain (FORCE_DOWNGRADE destructive abort). 4-part 구조 보존.
- `src/axhub-helpers/keychain.ts` 7 errors + `keychain-windows.ts` 5 errors → Toss 톤. `아이고` → `이상해요`. `죄송해요` → `잠깐만요`. 4-part 구조 보존, semantic kernel 보존.
- `src/axhub-helpers/index.ts` cmdSessionStart + token-init JSON next_step → Toss.
- `bin/install.sh` + `bin/install.ps1` 4 multi-line errors → Toss.

### Added

- `scripts/check-toss-tone-conformance.ts` — Phase 13 file scope tone lint. Forbidden tokens: 합니다/입니다/시겠어요/드립니다/당신/아이고. T-06 시나요 = warn (3 exceptions).
- `scripts/check-skill-keywords-preserved.ts` — baseline snapshot of nl-lexicon + SKILL description quoted phrases (11 files / 593 entries). PR2+ must show diff = 0.
- `tests/lint-toss-tone.test.ts` — 7 NEW tests for both lint scripts.
- `package.json scripts.lint:tone` + `lint:keywords`.
- `.omc/lint-baselines/skill-keywords.json` — captured baseline.

### Test baseline

- `bun test` → 401 pass / 5 skip / 0 fail / 2388 expect / 406 tests / 20 files (+13 from US-1306).
- `bun lint:tone --strict` → 0 error / 0 warning across 18 files.
- `bun lint:keywords --check` → OK (no diff vs baseline).
- `bunx tsc --noEmit` → clean.

### Honest tradeoff (Phase 14 deferred)

- Tier E docs (`docs/marketing/landing-page.ko.md`, `docs/troubleshooting.ko.md`, `docs/pilot/admin-rollout.ko.md`, `docs/pilot/onboarding-checklist.md`, `docs/pilot/vibe-coder-quickstart.ko.md`, etc., ~1455 lines) — Phase 14 separate cycle.
- SKILL.md workflow body + description narrative — Phase 14/15 (after activation drift measurement).
- Vibe coder transient tone mismatch ~2-4주 (runtime Toss vs docs pre-Toss) — accepted risk per ralplan v2 ADR. Rollback gate: NPS drop 5%↓.

### Consensus

`.omc/plans/phase-13-toss-tone-migration-v2.md` (2 ralplan rounds: 10 fixes round 1 + APPROVE round 2)

## [0.1.13] — 2026-04-27

Hotfix — add file-text regression test that locks the v0.1.12 non-interactive guard pattern. Architect Phase 12 review flagged: "PRD US-1203 acceptance bullet 1 said 'minimal fix + regression test that locks the contract' — test was missing, markdown-only fix is fragile." Architect correct.

### Added

- `tests/skill-noninteractive-guard.test.ts` — 6 NEW assertions: `[ -t 1 ]` literal in both status + logs SKILL.md, `WATCH=--watch` / `WATCH=` toggle, `FOLLOW=--follow` / `FOLLOW=` toggle, `$CI` env check, `$CLAUDE_NON_INTERACTIVE` env check. Future skill rewrites that drop the guard will fail tests immediately.

### Test baseline

- `bun test` → 394 pass / 5 skip / 0 fail / 2370 expect / 399 tests / 19 files (+6 tests, +1 file).
- `bunx tsc --noEmit` → clean.

## [0.1.12] — 2026-04-27

Hotfix — `/axhub:status` + `/axhub:logs` hang in subprocess (`claude -p`) mode. Caught by Phase 12 live subprocess smoke harness.

### Fixed

- `skills/status/SKILL.md` step 3 — `axhub deploy status --watch` blocks indefinitely in non-interactive context (no TTY, claude -p, CI). Added shell guard: `if [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then WATCH=--watch; else WATCH=; fi`. Drops `--watch` and renders single snapshot when headless. Vibe coders running `/axhub:status` in claude -p subprocess no longer hang forever.
- `skills/logs/SKILL.md` step 3 — same pattern for `axhub deploy logs --follow`. Shell guard uses `$FOLLOW` variable.

### Detection win

Phase 12 `tests/live-plugin-smoke.sh` ran 9 slash commands via `claude -p` subprocess. 7/9 PASSED, 2/9 (status + logs) TIMEOUT with zero output. Without subprocess smoke, every CI / headless / VS Code Tasks / GitHub Actions usage of `/axhub:status` or `/axhub:logs` would have hung at 120s+. Validates `tests/live-plugin-smoke.sh` as v0.2.0 release-gate.

### Test baseline

- `bun test` → 388 pass / 5 skip / 0 fail / 2360 expect / 393 tests / 18 files (unchanged — fix is skill markdown, not code).
- Live subprocess smoke evidence: `.omc/evidence/live-plugin-smoke-summary.txt` (pre-fix baseline).

## [0.1.11] — 2026-04-27

Hotfix — `axhub_pat_*` raw token redaction missing in `axhub-helpers redact`. Caught by live plugin smoke immediately after v0.1.10 ship.

### Fixed

- `src/axhub-helpers/redact.ts` — added `AXHUB_PAT_RE = /axhub_pat_[A-Za-z0-9_-]{16,}/g` pattern. Previously only `Bearer <token>` and `AXHUB_TOKEN=<token>` were masked; raw `axhub_pat_*` strings (the format vibe coders see in CLI output, .env files, paste flows) passed through unchanged. Plan/PLAN E7 + `skills/deploy/references/headless-flow.md §3` required this masking — implementation gap from Phase 1.
- `tests/redact.test.ts` — 2 NEW assertions: redact masks 16+ char `axhub_pat_*` to `axhub_pat_[redacted]`, AND does NOT mask shorter than 16 chars (regex floor preserved).

### Detection win

Live plugin smoke in user's actual Claude Code session caught this within minutes of v0.1.10 ship. Without smoke, every skill output containing a real token (status cards, recovery flows, headless paste responses) would have leaked the token to user transcript + telemetry. Privacy bug since v0.1.0.

### Test baseline

- `bun test` → 388 pass / 5 skip / 0 fail / 2360 expect / 393 tests / 18 files (+2 new tests).
- `bunx tsc --noEmit` → clean.
- Live: `echo "axhub_pat_a1b2c3d4e5f6g7h8i9j0" | redact` → `axhub_pat_[redacted]` ✓

## [0.1.10] — 2026-04-27

Hotfix — list-deployments crash on real API response shape. Live test in user's actual Claude Code session caught the bug.

### Fixed

- `src/axhub-helpers/list-deployments.ts` — API returns `{success, data: {active_deployment, deployments: [...]}, meta}` but helper assumed `data` itself was the array. Crashed with `items.map is not a function` on every real call. Existing happy-path test mocked the WRONG shape (`data: [...]`) — test passed, code crashed in production. Fix: change extraction to `env.data?.deployments ?? []` + update BackendListEnvelope type to nested shape.
- `tests/list-deployments.test.ts` — 5 mocks updated to real API shape (`data: { deployments: [...] }`). Now locks the correct contract; future regression of extraction code will fail tests.

### Detection win

Plugin Claude Code session smoke test caught this within minutes of v0.1.9 ship. Without live test, every vibe coder running `/axhub:logs` would have hit the crash. Validates "test in real session" as v0.2.0 release-gate criterion.

### Test baseline

- `bun test` → 386 pass / 5 skip / 0 fail / 2357 expect / 391 tests / 18 files (unchanged count, but mock shapes now match production reality).
- `bunx tsc --noEmit` → clean.
- Live API call: `./bin/axhub-helpers list-deployments --app 1` → 5 deployments returned, exit 0.

## [0.1.9] — 2026-04-27

Hotfix — UTF-8 BOM on .ps1 files. v0.1.8 GitHub Actions windows-smoke run revealed PowerShell 7 on Windows reads UTF-8 .ps1 files as Windows-1252 without BOM, mojibake'ing all Korean error messages into invalid PowerShell syntax tokens (e.g., `$msg = "지원하지 않는 ..."` parsed as garbage → script crashes at parse time before reaching `AXHUB_SKIP_AUTODOWNLOAD` env check).

### Fixed

- `bin/install.ps1` — UTF-8 BOM (EF BB BF) prepended.
- `hooks/session-start.ps1` — UTF-8 BOM prepended.
- `tests/smoke-windows-vm-checklist.ps1` — UTF-8 BOM prepended.

### Detection win

The Phase 11 US-1104 `.github/workflows/windows-smoke.yml` (added 1 commit ago in v0.1.8) caught this on its FIRST real Windows runner execution. Without that CI workflow, this bug would have shipped to vibe coder Windows pilots and broken every PS1 hook + install. Validates the deferred-doc-becomes-executable-scaffold pattern.

### Test baseline

- `bun test` → 386 pass / 5 skip / 0 fail / 2357 expect / 391 tests / 18 files (unchanged).
- `bunx tsc --noEmit` → clean.

## [0.1.8] — 2026-04-27

Phase 11 — close 5 deferred Phase 10 tradeoffs (Option B scope split). macOS + Linux + Windows binary unchanged. Adds first-ever live Linux runtime evidence + Windows GitHub Actions CI smoke + format-parity for keychain.ts errors.

### Added

- `bin/install.ps1` $ReleaseVersion now codegen-synced via `scripts/codegen-install-version.ts` (US-1101). Mirrors install.sh sync pattern. Pre-release tag (e.g. `0.1.8-rc.1`) handling tested.
- `tests/smoke-linux-docker.sh` + `tests/smoke-linux-docker.Dockerfile` (US-1105) — first-ever live runtime verification of Phase 8 Linux secret-tool keychain bridge. Pinned to `ubuntu:24.04@sha256:c4a8d550...`. LIMITATION banner mandates 40% READ-path / ~15% E2E coverage disclosure.
- `docs/pilot/windows-vm-smoke-checklist.md` + `tests/smoke-windows-vm-checklist.ps1` (US-1103) — 14-step Windows VM smoke executor behind `$env:AXHUB_VM_SMOKE` guard.
- `docs/pilot/authenticode-signing-runbook.md` + `.github/workflows/sign-windows.yml.template` (US-1104) — vendor procurement runbook + stub workflow scaffold (workflow_dispatch + AXHUB_SIGNING_STUB env).
- `.gitattributes` — linguist exemption for `*.yml.template` files.
- `.github/workflows/windows-smoke.yml` — runs install.ps1 + session-start.ps1 + Add-Type advapi32!CredReadW PInvoke smoke on every tag push (replaces real Windows VM for CI-level verification).

### Changed

- `src/axhub-helpers/keychain.ts` (US-1102 closes #1) — 7 existing one-line Korean errors rewritten to 4-part empathy template (감정 / 원인 / 해결 / 다음액션) per error-empathy-catalog. Plan said 6 lines; executor expanded to 7 (catch paths can fire on non-ENOENT spawn failures — OOM, SELinux/AppArmor, signal). Architect APPROVED deviation; semantic kernel preserved per error.

### Live evidence (Phase 11 first runs)

- Linux Docker smoke PASSED: `secret-tool store exit=0` → `axhub-helpers token-init exit=0` → file mode=600 → token first 16 chars=`axhub_pat_phase1` → source=linux-secret-service.

### Test baseline

- `bun test` → 386 pass / 5 skip / 0 fail / 2357 expect() / 391 tests across 18 files.
- `bunx tsc --noEmit` → clean.
- `bash tests/docs-link-audit.sh` → `Broken: 0`.

### Deferred to v0.1.9+

- Authenticode procurement (Sectigo OV ~$200-300/yr) — runbook + stub workflow ready, blocked on vendor.
- Real Linux desktop test (gnome-keyring-daemon / kwalletd5) — Docker covers ~15% E2E only.
- Real Win11 VM smoke run (use US-1103 ps1 with `$env:AXHUB_VM_SMOKE=1`) — CI workflow covers script-level + PInvoke; full plugin-install E2E needs VM.

## [0.1.7] — 2026-04-27

Phase 10 — Windows PS1 hooks. Vibe coders on stock Windows 10/11 (no Git Bash, no WSL) can now use the plugin end-to-end. macOS + Linux sh files unchanged byte-identically.

### Added

- `bin/install.ps1` — Windows installer mirror of `bin/install.sh`. PowerShell 5.1+ (stock Win10/11). No `Add-Type`, no `Install-Module` — EDR-clean. Handles MAX_PATH (LongPathsEnabled hint), NTLM proxy 407, Defender post-Move quarantine.
- `hooks/session-start.ps1` — Windows SessionStart hook mirror. Path resolution mirrors `src/axhub-helpers/telemetry.ts:40-44` (XDG_STATE_HOME) and `src/axhub-helpers/index.ts:441` (XDG_CONFIG_HOME) — distinct state vs token directories.
- `hooks/hooks.json` — second SessionStart entry with `"shell": "powershell"` field. Bash entry [0] preserved byte-identical from v0.1.6.
- `tests/install-ps1.test.ts` (7 cases) + `tests/session-start-ps1.test.ts` (9 cases) — file-text assertions via readFileSync (no PS spawn — pwsh not on macOS dev host).
- `tests/manifest.test.ts` — 5 new platform-branch assertions on hooks.json SessionStart sibling structure.

### Compatibility

- Requires **Claude Code >= 2.1.84** (introduced `"shell": "powershell"` hook field). Older clients silently ignore the field — bash entry runs on Windows → no bash → broken hook with no actionable error.
- See `.omc/plans/phase-10-windows-ps1-hooks-v2.md` for full consensus rationale.

### Honest tradeoff (deferred to v0.1.8)

- `.ps1` files NOT Authenticode-signed — EDR may quarantine PowerShell invocation. Korean systemMessage error path documents AXHUB_TOKEN env var fallback.
- macOS noise from wrong-OS `"shell": "powershell"` spawn: assumed silent per Anthropic spec phrasing ("runs on Windows"), not directly verified. Hotfix-ready as v0.1.7.1 if first pilot reports noise. See `docs/pilot/v0.1.7-spike-result.txt`.
- `bin/install.sh:80` operator precedence bug NOT replicated in install.ps1 (explicit Test-Path/Remove-Item). sh-side fix tracked for future v0.1.x.

### Test baseline

- `bun test` → 370 pass / 5 skip / 0 fail / 2323 expect() / 375 tests across 18 files.
- `bunx tsc --noEmit` → clean.
- `bash tests/docs-link-audit.sh` → `Broken: 0`.

## [0.1.6] — 2026-04-24

Phase 9 hotfix — single-line patch to remove a doc/code self-contradiction. macOS + Linux + helper binary unchanged. No new features.

### Fixed

- `src/axhub-helpers/keychain-windows.ts:103` — `ERR_NOT_FOUND` last line previously instructed users to run `cmdkey /list:axhub` for credential presence verification. But `cmdkey` returns exit code 0 in BOTH present and absent cases (consensus plan v3 Fix 5 explicitly removed this from documentation as useless). Replaced with the `AXHUB_TOKEN` env var fallback path.
- `tests/keychain-windows.test.ts` case 3 — added `expect(result.error).not.toContain("cmdkey")` regression guard so the architecture decision (PS-only, no cmdkey probe) is enforced at the test level.

### Test baseline

- `bun test` → 349 pass / 5 skip / 0 fail / 2257 expect() / 354 tests across 16 files.
- `bunx tsc --noEmit` → clean.

## [0.1.5] — 2026-04-24

Phase 9 — Windows keychain bridge ship. macOS + Linux + helper binary unchanged.

### Added

- `src/axhub-helpers/keychain-windows.ts` — Windows Credential Manager bridge via PowerShell + `Add-Type` PInvoke against `advapi32!CredReadW`. ASCII sentinel scheme (`AXHUB_OK:<base64>` / `ERR:NOT_FOUND` / `ERR:LOAD_FAIL`) for locale-independent classification. Stock Win10/11 — no `Install-Module` required.
- 5 4-part Korean error messages for Windows scenarios (감정 / 원인 / 해결 / 다음액션): ExecutionPolicy block, NOT_FOUND, PInvoke load failure, EDR/AMSI quarantine (signal-kill or exit ∈ {-1, 0xC0000409}), spawnSync throws.
- `tests/keychain.test.ts` — extracted `parseKeyringValue` decoder tests (8 cases) from `tests/token-init.test.ts`.
- `tests/keychain-windows.test.ts` — 6 mocked-runner cases covering all pre-mortem scenarios.

### Changed

- `src/axhub-helpers/keychain.ts` — Windows branch (previously deferred error message) now delegates to `readWindowsKeychain()`. Linux + macOS branches unchanged.
- Skills + docs updated additively: `skills/auth/SKILL.md`, `skills/deploy/references/headless-flow.md`, `skills/deploy/references/recovery-flows.md`, `docs/pilot/admin-rollout.ko.md`, `src/axhub-helpers/list-deployments.ts`, `bin/README.md` — Windows mentions added alongside existing macOS/Linux content.

### Honest tradeoff (EDR)

v0.1.5 Windows binary is **not Authenticode-signed** (deferred to v0.1.6). EDR / AMSI / corporate AV will likely classify the inline PInvoke against `advapi32!CredReadW` as a Mimikatz-pattern threat and block the call. The Korean EDR error message (`keychain-windows.ts:ERR_EDR`) explicitly owns this — recommends `AXHUB_TOKEN` env var as the legitimate workaround until v0.1.6 code-signing makes EDR allowlist requests viable.

### Deferred to v0.1.6

- Format-parity for existing macOS + Linux Korean errors (one-line → 4-part empathy template). Tracked in https://github.com/jocoding-ax-partners/axhub/issues/1.
- Authenticode code-signing for `windows-amd64.exe` → EDR allowlist legitimization.

### Test baseline

- `bun test` → 349 pass / 5 skip / 0 fail / 2256 expect() / 354 tests across 16 files.
- `bunx tsc --noEmit` → clean.
- `bash tests/docs-link-audit.sh` → `Broken: 0`.

## [0.1.0] — 2026-04-24

First public release. Korean-first natural-language deploy/manage for vibe coders, wrapping ax-hub-cli (`>=0.1.0,<0.2.0`).

### Added

#### Core helper binary (TypeScript on Bun)

- `src/axhub-helpers/index.ts` — single multi-cmd binary built via `bun build --compile`. Subcommands: `session-start`, `preauth-check`, `consent-mint`, `consent-verify`, `resolve`, `preflight`, `classify-exit`, `redact`.
- `src/axhub-helpers/consent.ts` — HMAC consent token mint/verify (jose JWT HS256). Bound to `{tool_call_id, action, app_id, profile, branch, commit_sha}`. PreToolUse deterministic deny-gate.
- `src/axhub-helpers/preflight.ts` — CLI version range gate (semver) + auth status preflight. Exit code precedence 64 > 65 > 0.
- `src/axhub-helpers/catalog.ts` — 4-part Korean error empathy templates per axhub exit code (감정 + 원인 + 해결 + 버튼).
- `src/axhub-helpers/redact.ts` — NFKC normalize + secret/cross-team URL redaction filter.
- `src/axhub-helpers/resolve.ts` — live profile/app/branch/commit resolution (no cached app_id for mutations).
- `src/axhub-helpers/telemetry.ts` — opt-in observability envelope (default OFF, gated by `AXHUB_TELEMETRY=1`).

#### Plugin surface

- 11 skills under `skills/`: apis, apps, auth, clarify, deploy, doctor, logs, recover, status, update, upgrade. Each with Korean trigger lexicon + workflow.
- 9 slash commands under `commands/`: apis, apps, deploy, doctor, help, login, logs, status, update.
- `hooks/hooks.json` — `{"hooks": {...}}` wrapper with SessionStart + PreToolUse + PostToolUse hook chain.
- `.claude-plugin/{plugin,marketplace}.json` — plugin manifest with `repository` as string (Phase 6 incident #1 fix), all required keys per Claude Code loader.

#### Cross-arch distribution

- `bun run build:all` — 5 cross-arch helper binaries: darwin-arm64 (58.3M), darwin-amd64 (63.0M), linux-amd64 (99.2M), linux-arm64 (96.8M), windows-amd64.exe (109.6M).
- `bin/install.sh` — POSIX shell auto-selector with OS+arch detection (`AXHUB_OS`/`AXHUB_ARCH` env overrides for testing). Symlinks (Unix) or copies (Windows).
- `tests/install.test.sh` — 5 positive + 3 negative arch detection cases.

#### Release pipeline

- `.github/workflows/release.yml` — tag-triggered (`v*.*.*`) cross-arch build + cosign keyless signing (sigstore OIDC, no long-lived keys) + manifest.json + checksums.txt + GitHub Release upload.
- `scripts/release/manifest.ts` — JSON manifest generator (sha256 + arch + size_bytes per binary, plus plugin/helper version).
- `scripts/release/verify-release.sh` — user-side verification script: manifest signature → per-binary signature → sha256 cross-check.
- `docs/RELEASE.md` — maintainer + user verification guide. Documents `AXHUB_REQUIRE_COSIGN=1` advisory + `AXHUB_ALLOW_UNSIGNED` warning (IT-only escape hatch).
- Cosign sidecar advisory in session-start: warns when `AXHUB_REQUIRE_COSIGN=1` and `.sig` missing (advisory only, doesn't block).

#### Test suite

- 295 passing unit/integration tests across 11 files / 2136 expect() assertions / typecheck clean.
- `tests/consent.test.ts` — 56 tests covering parser bypass hardening (T-ADV-PARSE-1..8) + 3 closed gotcha classes (trailing-delimiter, nested-shell, quoted-subcommand) + dead-path `deploy_logs_kill` v0.2 reservation test (17 corpus assertions across full v0.1.0 CLI surface).
- `tests/manifest.test.ts` — 86 tests / 358 expect() validating plugin.json, marketplace.json, hooks.json structure (`hookEventName` presence required — Phase 6 incident #2 fix), commands frontmatter, skills frontmatter (Phase 6 Q1 — `allowed-tools` removed).
- `tests/fuzz-parser.ts` — deterministic mulberry32 PRNG, 1100 randomized variants (1000 standard + 100 gotcha-class). 1100/1100 caught with default seed `0xc67434fd`. Reproducible across runs.
- `tests/fixtures/` — 38 hand-curated frozen contract files (10 destructive, 8 read-only, 8 adversarial, 4 unicode, 4 profile/headless, 4 negative). `_curated.ts` source-of-truth generator + drift detection.
- `tests/corpus.100.jsonl` + `tests/corpus-schema.test.ts` — 100-row stratified scoring corpus, all rows with `expected_cmd_pattern`. Schema invariants validated.
- `tests/telemetry.test.ts` — 16 tests for opt-in envelope shape, file mode 0600, opt-out default.
- `tests/codegen.test.ts` — catalog ↔ markdown drift detection (8 tests).
- `tests/release-config.test.ts` — 18 shape assertions on `.github/workflows/release.yml` + `manifest.ts` + `verify-release.sh`.
- `tests/e2e/staging.test.ts` — gated real-CLI integration (skipped when `AXHUB_E2E_STAGING_TOKEN` unset).
- `tests/docs-link-audit.sh` — every `references/X.md` mention in `SKILL.md` files resolves on disk (Broken: 0).

#### Korean documentation

- 11 SKILL.md files with Korean trigger lexicon (informal/honorific/demo variants).
- `skills/deploy/references/error-empathy-catalog.md` — 13 exit-code entries with 4-part Korean templates + interpolation placeholders.
- `skills/deploy/references/error-empathy-catalog.generated.md` — auto-generated runtime snapshot (regen via `bun run codegen:catalog`).
- `skills/deploy/references/{nl-lexicon, recovery-flows, headless-flow, telemetry}.md` — Korean reference docs.
- `skills/apis/references/privacy-filter.md` — cross-team scope isolation rules per Phase 6 §16.17 / row 46.
- `docs/pilot/` — first-customer pilot prep kit (5 docs, 476 lines): README, onboarding-checklist, feedback-template, admin-rollout.ko, exit-criteria.

#### Scoring infrastructure

- `tests/score.ts` — 4-metric scoring (trusted-completion, unsafe-trigger-precision, recovery-rate, baseline-delta). M1.5 GO/KILL gate logic.
- `tests/baseline-results.docs-only.{20,100}.json` — docs-only Claude predictions (M0.5 + M2.5 scopes).
- `tests/plugin-arm-results.{20,100}.json` — plugin-arm predictions (M0.5 + M2.5 scopes).
- M1.5 v2 verdict: trusted 91% / unsafe 0% / recovery 100% / margin +40pp → GO sustained at 100-row scope.

### Bug fixes shipped during 0.1.0 development

- Plugin manifest `repository` was object → must be string (Phase 6 plugin-validator incident, surfaced via real Claude Code loader testing).
- `hookSpecificOutput` missing `hookEventName` → "Hook JSON output validation failed" (Phase 6 incident #2).
- `classify-exit` emitting "배포 성공" for any axhub exit 0 (e.g. `axhub --version`) — silent unless `axhub deploy create`.
- 9 broken sibling SKILL.md `references/X.md` paths → `../deploy/references/X.md` or `../apis/references/privacy-filter.md`.
- `skills/deploy/SKILL.md` frontmatter had `allowed-tools` over-spec → removed (matches all 11 sibling skills).
- 3 parser gotchas surfaced by Phase 2 fuzzer: trailing close-delimiter contamination on action token, nested sub-shell inside `eval`/`bash -c`, quoted subcommand tokens.

### Plugin ↔ ax-hub-cli compatibility

| Plugin | ax-hub-cli min | ax-hub-cli max |
|---|---|---|
| 0.1.x | 0.1.0 | < 0.2.0 |

### Out of scope (deferred)

- Marketplace publish announcement (after first cosign-signed release lands).
- First-customer pilot execution (prep kit shipped, customer recruit pending).
- Real ax-hub-cli staging credential procurement + CI E2E enablement.
- Telemetry analytics dashboard (data collection ready, dashboard pending opt-in usage signal).
- Languages beyond Korean.

### See also

- `PLAN.md` — full design history (6 phases of review, 65 audit-tracked decisions).
- `docs/RELEASE.md` — release process for maintainers + user verification.
- `docs/pilot/` — first-customer pilot prep.
- `.omc/progress.txt` — internal ralph cycle log (Tier 1 → Phase 2 → Phase 3).
