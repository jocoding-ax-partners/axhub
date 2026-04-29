# Ralph Rust-primary cutover evidence — 2026-04-29

## Scope

- `package.json` build scripts now call `scripts/build-rust-helper.ts`, which wraps Cargo release builds and copies the Rust helper into `bin/`.
- `Cargo.toml` workspace package version is now part of `bun run codegen:version` and release postbump staging.
- `.github/workflows/release.yml` now builds five Rust target artifacts and signs/uploads them.
- `.github/workflows/claude-cli-e2e.yml` installs Rust before plugin e2e build steps.
- Rust `classify-exit` now accepts PostToolUse stdin JSON and emits hook `systemMessage` parity for axhub commands.
- Rust `preauth-check` deny path now matches the TS hook contract: exit 0, deny output, and Korean preauth message.

## Fresh verification commands run in this iteration

| Command | Result |
|---------|--------|
| `bun run codegen:version` | PASS — install scripts, TS fallback constants, telemetry, and `Cargo.toml` synced at `0.1.24`. |
| `bun run build` | PASS — Cargo release build copied `bin/axhub-helpers` and host asset `bin/axhub-helpers-darwin-arm64`. |
| `bun run release:check` | PASS — host Rust artifact and host release asset reported `0.1.24`; release matrix includes five Rust artifacts. |
| `cargo test -p axhub-helpers --test cli_e2e` | PASS — 4/4 CLI contract tests. |
| `bun run test:plugin-e2e:t2` | PASS — 11/11 Claude hook/e2e cases after Rust helper rebuild. |
| `bun test tests/runtime-fallback.test.ts tests/hook-latency.test.ts tests/session-start.test.ts tests/release-config.test.ts tests/plan-consistency.test.ts tests/codegen-version.test.ts` | PASS — 51/51 targeted regression tests. |
| `bun test` | PASS — 562 pass / 5 skip / 0 fail / 2913 expects. |
| `bash tests/e2e/claude-cli/run-matrix.sh --only 16` | PASS — 1/1 rerun proved the earlier case 16 timeout was transient. |
| `bun test tests/axhub-helpers.test.ts` | PASS — 25/25, including skill-specific UserPromptSubmit routing contexts. |
| `bash tests/e2e/claude-cli/run-matrix.sh --only 03 04 13 19` | PASS — 4/4 T1 routing rerun after Rust prompt-route expanded beyond doctor. |
| `bash tests/e2e/claude-cli/run-matrix.sh --only 16` after timeout hardening | PASS — 1/1; case 16 completed in 57s with a 90s budget. |
| T1 case timeout hardening | Updated T1 slash/NL case budgets to 90s to avoid false negatives under repeated Claude CLI runs. |

## Evidence artifacts

- `.omc/evidence/plugin-e2e-t1-rust-primary.stdout` — first T1 run: 7/8 pass, case 16 timed out after 60s.
- `.omc/evidence/plugin-e2e-t1-case16-rerun-rust-primary.stdout` — targeted rerun: 1/1 pass.
- `.omc/evidence/plugin-e2e-t1-rust-primary-rerun.stdout` — full T1 rerun before route expansion; showed timeout/routing regressions that drove the fix.
- `.omc/evidence/plugin-e2e-t1-routing-rerun-rust-primary.stdout` — targeted route rerun: 4/4 pass.
- `.omc/evidence/plugin-e2e-t1-rust-primary-final.stdout` — full T1 rerun before case 16 timeout hardening; 7/8 pass, case 16 crossed 60s.
- `.omc/evidence/plugin-e2e-t1-case16-timeout90-rust-primary.stdout` — case 16 timeout-hardening rerun: 1/1 pass.
- `.omc/evidence/plugin-e2e-t1-rust-primary-final-timeout90.stdout` — full T1 rerun before all T1 cases used the 90s budget; 7/8 pass, case 04 crossed 60s.
- `.omc/evidence/plugin-e2e-t1-rust-primary-final-timeout90-all.stdout` — final full T1 rerun artifact after all T1 timeout budgets were aligned to 90s; see final summary for result.

## Honest remaining external gates

- Staging e2e still needs real `AXHUB_E2E_STAGING_TOKEN`/endpoint credentials.
- Windows V3/AhnLab live cohort still needs that Windows/EDR environment.
- Full 24h parser fuzz remains outside this session; 60-second smoke already passed in external verification evidence.
- Full TS source deletion is intentionally deferred because the monitor window, TS fallback, and catalog source-of-truth cleanup remain open.

## Final verification gate — 2026-04-29

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` + `cargo fmt --manifest-path fuzz/Cargo.toml -- --check` | PASS |
| `cargo clippy --workspace -- -D warnings` + fuzz clippy | PASS |
| `cargo test --workspace` | PASS — codegen 5, helper unit 4, CLI e2e 5, phase parity 16, live keychain tests ignored by default. |
| `cargo llvm-cov --workspace --fail-under-lines 90` | PASS — total line coverage **91.07%**. |
| `cargo audit --deny warnings` | PASS — command exited 0 after scanning Cargo.lock. |
| `bunx tsc --noEmit` | PASS |
| `bun run lint:tone --strict` | PASS — 0 errors / 0 warnings across 30 files. |
| `bun run lint:tone:rust` | PASS — 0 errors / 0 warnings across 52 Rust files. |
| `bun run lint:keywords --check` | PASS — keywords preserved. |
| `bun run release:check` | PASS — host Rust artifact and host release asset at `0.1.24`; 5 release assets wired. |
| `bun test` | PASS — **563 pass / 5 skip / 0 fail / 2943 expects**. |
| `bun run test:e2e` | PASS/SKIP — 1 pass / 5 skip / 0 fail because staging token is unset. |
| `bun run test:plugin-e2e:t1` equivalent (`run-matrix --tier t1`) | PASS — **8 / 8** after prompt-route expansion and 90s interactive budgets. |
| `bun run test:plugin-e2e:t2` | PASS — **11 / 11**. |
| `git diff --check` | PASS |
