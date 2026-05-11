# Changelog

All notable changes to the axhub Claude Code plugin will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows [Semantic Versioning](https://semver.org/).


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
- `bun run test:e2e` against `https://hub-api.jocodingax.ai` → 4 pass / 1 skip / 0 fail

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
- `bun run test:e2e` against `https://hub-api.jocodingax.ai` → 4 pass / 1 skip / 0 fail

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
