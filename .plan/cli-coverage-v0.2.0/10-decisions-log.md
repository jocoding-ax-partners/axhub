# Decisions Log

> CEO + ENG + DX 3-pass codex review = 27 finding. 각 finding 의 결정 history.

---

## CEO Pass (codex 1차, 2026-05-02)

12 finding. 8 auto-applied + 4 user-decided.

### Auto-applied (8)

| # | Finding | Action |
|---|---|---|
| F1 | preflight.ts:27 MAX_AXHUB_CLI_VERSION="0.2.0" → CLI v0.10.2 cli_too_new | Phase A0 #1 (MAX → 0.11.0) |
| F2 | Phase A 직접 minor bump = release contract 위반 | Phase A0 #1 의 manual bump 제거, `bun run release` 가 처리 |
| F3 | prompt-route.ts:19 PromptRouteIntent enum hardcoded | Phase A0 #3/#4 (TS+Rust 6 신규 enum) |
| F4 | consent.ts/parser.rs 9 신규 mutation 미인식 | Phase A0 #6 (6 파일 migration, 11 신규 destructive 등록) |
| F6 | tests/e2e-claude-cli-registry.test.ts:48 hardcoded 13 keys | Phase A0 #9 (13→20 baseline) |
| F8 | env set: argv leak risk, --from-stdin 강제 | env SKILL 가 stdin pipe only |
| F11 | axhub open vs apps open 중복 | top-level open SKILL 만 owner, apps open 위임 |
| F12 | init --force = no-op CLI flag | init SKILL copy "이번 release 에서 overwrite 안 돼요" 명시 |

### User-decided (4)

| # | Finding | User Decision | Rationale |
|---|---|---|---|
| F5 | cmd/agent/install.go:296 NOT atomic write | **agent install SKILL DEFER to v0.2.5+** | CLI 측 atomic write fix 후 재고 |
| F7 | "새 앱 만들어줘" routing ambiguous | init 먼저 + apps create 자동 chain (이후 DX F3 에서 reverse) | vision-critical funnel — init 우선 |
| F9 | profile add --endpoint = arbitrary endpoint 위험 | profile add **endpoint allowlist gate** (`*.jocodingax.ai`, `localhost`) | auth-routing surface 보호 |
| F10 | apis call write scope | apis call **full consent gate** (deploy 동일 패턴) | write scope, deploy-equivalent gate |

---

## ENG Pass (codex 2차, 2026-05-03 00:30)

8 finding. 5 auto-applied + 3 user-decided.

### Auto-applied (5)

| # | Finding | Action |
|---|---|---|
| E1 | Rust helper (build-rust-helper.ts:58-71) is shipped binary, NOT TS. main.rs:258-555 separate router | Phase A0 #4 (Rust router parity 명시) |
| E2 | ConsentBinding migration = 6 파일, plan said 2. "9 destructive" != 11 | Phase A0 #6 (6 파일 명시 + plan text "11" 수정) |
| E3 | In-flight token (60s TTL) backwards compat | ConsentBinding context = optional + verify backfill `{}` |
| E5 | nl-lexicon collision mitigation needs explicit negative tests (TS+Rust) | Phase A0 #8 (5 negative test) |
| E8 | benchmark fake-able (preauth-check + classify-exit only, no Rust prompt-route + needs_preflight) | Phase A0 #5 명시화 (Rust binary + needs_preflight + fake AXHUB_BIN) |

### User-decided (3)

| # | Finding | User Decision | Rationale |
|---|---|---|---|
| E4 | Phase A0 broken intermediate state | **Feature flag `beta_skills:false` default OFF, Phase D 에서 flip** | atomic 큰 PR 회피 + intermediate broken state 방지 |
| E6+E7 | lifecycle E2E 6-step harness 미지원 + registry baseline ≠ free-form preview safety | **e2e harness session-persistence opt-in 확장 + free-form 허용 + ADR** | harness 확장 ~3-4시간, lifecycle 적절 시행 |
| E9 | apps create chain hand-waved | **chain 포기 (init 만, apps create 별도 turn)** | vision 후퇴 대신 안전성 + 투명성 우선 |

---

## DX Pass (codex 3차, 2026-05-03 01:00)

7 finding. 3 auto-applied + 3 user-decided + 1 noop.

### Auto-applied (3)

| # | Finding | Action |
|---|---|---|
| F2 | preflight v0.10.2 cli_too_new | 이미 CEO F1 으로 처리, 동일 |
| F5 | README+GIF maintenance plan 부재 + version drift | Phase D #1 (`codegen:readme-version` + GIF re-render ownership) |
| F6 | telemetry contract 코드와 다름 (`AXHUB_TELEMETRY=1` / `~/.local/state/.../usage.jsonl`) | telemetry plan 텍스트 코드 그대로 인용, 신규 file path X |

### User-decided (3)

| # | Finding | User Decision | Rationale |
|---|---|---|---|
| F1 | TTHW 5분 거짓 (cold = 6분+) | **target = cold start 7-8분** (build 3분 포함) | honest, Competitive tier 마지노선 통과 |
| F3 | chain default YES + 2초 countdown vs plan E9 "chain 포기" 충돌 | **plan E9 결정 유지 — chain 완전 포기, explicit ask** | consistent. DX Pass 1 의 default YES 결정 reverse |
| F7 | persona 이상화 (managed pilot vs self-serve cold customer) | **admin onboarding wizard SKILL 신규** (cold customer 30-50% blocked 해소) | 8 신규 SKILL → 19 SKILL → 18 (init 흡수 후) |

### NOOP (1)

| # | Finding | Action |
|---|---|---|
| F4 | 4-dim preview schema 가 apis call write 에 부족 | apis call SKILL polish 가 4-dim 확장 (payload + side_effect + auth_scope + idempotency) |

---

## Post-DX User Refinement (2026-05-03 01:30)

DX Pass 후 user 가 zero-install bootstrap 요구 추가.

### Finding R-1: vibe coder cold customer 가 init 시 모든 dep 자동 install (node/npm/ax-hub-cli/template)

| # | User Refinement | Decision | Rationale |
|---|---|---|---|
| R-1.1 | "init 하면 examples repo 에서 template 선택 + 의존성 자동 install" | **init SKILL = setup 흡수, single all-in-one** | single mental model, vibe coder UX 자연스러움 |
| R-1.2 | "git 없어도 되는가?" | **tarball download (codeload.github.com)** | git X, 첫 dep 진입장벽 0 |
| R-1.3 | "SKILL 로 할 생각말고 Rust helper 에서 install" | **helper Rust subcommand 4 신규 (bootstrap / fetch-template / install-deps / list-templates)** | single source of truth, multi-arch + cosign + cargo test |
| R-1.4 | OS scope | **Mac + Linux + Windows v0.2.0 모두** | 100% feature parity |
| R-1.5 | init workflow design | **stack 선택 ask → bootstrap → fetch-template → install-deps → apps create ask → github connect ask** | 7-step single SKILL |

---

## Final Scope Summary

**Plugin SKILL**: 11 → **18** (7 신규 + 5 polish)
- 신규 7: `init` (zero-install all-in-one) / `env` / `github` / `open` / `whatsnew` / `profile` / `admin`
- polish 5: `apps` / `apis` / `deploy` / `doctor` / `update`

**helper Rust subcommand**: 11 → **15** (4 신규)
- `bootstrap` / `fetch-template` / `install-deps` / `list-templates`

**examples repo**: `templates.json` manifest 신규 (sibling 작은 PR)

**Total**: 27 codex finding 흡수 (8 + 5 + 3 = 16 auto + 4 + 3 + 3 = 10 user + 1 noop)

**Effort**: ~33시간 CC+gstack / ~5주 human team

**TTHW**: cold ~7분 30초 / warm ~3분 (Competitive tier)

**DX overall**: 8.5/10 (B+) — DX v4 final
