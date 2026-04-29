# Ralph Rust staging workflow evidence — 2026-04-29

## Scope

- Added `.github/workflows/rust-staging-gates.yml` so Rust-primary changes have an operational staging gate, not only local code/tests.
- Extended `tests/e2e/staging.test.ts` with a Rust helper staging probe:
  `bin/axhub-helpers list-deployments --app-id "$AXHUB_E2E_STAGING_APP_ID" --limit 1`.
- Added workflow-shape regression tests in `tests/release-config.test.ts`.
- Documented required secrets/vars in `docs/RELEASE.md` and `tests/e2e/README.md`.
- Updated `.plan/10-source-mapping.md` to record the credential-gated workflow.

## Workflow inputs and gates

Required for real staging:

- `AXHUB_E2E_STAGING_TOKEN`
- `AXHUB_E2E_STAGING_ENDPOINT`
- `AXHUB_E2E_STAGING_APP_ID`
- `AXHUB_CLI_INSTALL_COMMAND`

Optional:

- `AXHUB_E2E_ALLOW_PROXY=1` for staging proxy / non-production TLS endpoints.
- `fuzz_minutes=1440` for the 24h parser fuzz gate.
- `run_windows_smoke=true` for GitHub-hosted Windows smoke. V3/AhnLab remains a separate manual cohort.

## Verification run locally

| Command | Result |
|---------|--------|
| `bun test tests/release-config.test.ts` before workflow file | RED — 5 expected failures because `.github/workflows/rust-staging-gates.yml` did not exist. |
| `bun test tests/release-config.test.ts` after workflow file | PASS — 28 pass / 0 fail. |
| `bun test tests/e2e/staging.test.ts` without credentials | PASS/SKIP — 1 pass / 6 skip / 0 fail. |
| `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/rust-staging-gates.yml")'` | PASS — workflow YAML parses locally. |
| `bun test tests/release-config.test.ts tests/e2e/staging.test.ts` | PASS — 29 pass / 6 skip / 0 fail. |
| `bunx tsc --noEmit` | PASS. |
| `bun test` | PASS — 568 pass / 6 skip / 0 fail / 2962 expects. |
| `bun run release:check` | PASS — Rust helper host artifact and host release asset at 0.1.24; release matrix wired. |
| `git diff --check` | PASS. |

## Honest remaining external gates

- The workflow is committed/configured locally, but real staging still requires repository secrets/vars.
- Windows V3/AhnLab cohort cannot be proven by GitHub-hosted Windows alone.
- 24h fuzz is represented by `fuzz_minutes=1440`; only a real manual/CI run can close that gate.
