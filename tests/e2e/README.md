# E2E Tests — ax-hub-cli Staging Integration

`tests/e2e/staging.test.ts` runs against the real `axhub` CLI binary + a real staging API. **Default: skipped.** Activate by setting two env vars:

```bash
export AXHUB_E2E_STAGING_TOKEN=<token-with-staging-scope>
export AXHUB_E2E_STAGING_ENDPOINT=<staging-api-url>
export AXHUB_E2E_STAGING_APP_ID=<numeric-app-id-with-read-access> # enables Rust helper probe

bun run build
bun run test:e2e
```

## What gets tested (when enabled)

1. `axhub auth status --json` returns valid identity (user_email + scopes present)
2. `axhub apps list --json` returns array (may be empty for a fresh staging account)
3. `parseAxhubCommand` action mapping stays consistent with the real CLI surface
4. `classify-exit` produces 4-part Korean templates for documented exit codes (0, 1, 64, 65, 66, 67, 68)
5. `bin/axhub-helpers list-deployments --app-id <id>` reaches staging through the Rust helper when `AXHUB_E2E_STAGING_APP_ID` is set

The first 2 tests hit staging through the real `axhub` CLI; the Rust helper probe hits staging through `bin/axhub-helpers`. The parser/classifier tests are pure-logic smoke checks that ride along when E2E is enabled (no extra cost).

## How to obtain staging credentials

**For axhub team members**: Internal docs at `https://docs.jocodingax.ai/internal/staging-access` (link example, replace with real). Request via `#axhub-internal` Slack channel.

**For external contributors**: staging access is not publicly available. Run the 294 unit tests (`bun test`) — they cover all parser/catalog/consent/manifest/telemetry behavior with mocked CLI invocations. E2E is supplementary, not required for merging.

## CI configuration

Use `.github/workflows/rust-staging-gates.yml` for Rust-primary staging validation. To enable it in your fork/repo:

1. Add `AXHUB_E2E_STAGING_TOKEN`, `AXHUB_E2E_STAGING_ENDPOINT`, and `AXHUB_E2E_STAGING_APP_ID` as repo secrets.
2. Add `AXHUB_CLI_INSTALL_COMMAND` as a repo secret or repo variable. It must install the real `axhub` CLI and leave `axhub` on `PATH`.
3. Optionally set repo var `E2E_ENABLED=true` to run staging gates on pushes to `main`; otherwise run manually with `workflow_dispatch`.
4. If staging uses a proxy or non-production TLS endpoint, set `AXHUB_E2E_ALLOW_PROXY=1`.

Manual run:

```bash
gh workflow run rust-staging-gates.yml \
  -f run_staging=true \
  -f require_credentials=true \
  -f fuzz_minutes=1 \
  -f run_windows_smoke=false
```

## Why gated, not always-on

- **Credentials**: real staging tokens are scarce + sensitive. Gating prevents leakage on PRs from contributors who can't access them.
- **Network dependency**: E2E hits a live API. CI flakes from staging downtime would cause spurious red builds.
- **Time budget**: real-network calls take 200-2000ms each. Unit tests run in <3s; E2E adds 5-30s.
- **Deletes nothing**: even with creds, the test suite makes only read calls (auth status, apps list). It does NOT call `deploy create` or any destructive op against staging.

## Local development workflow

For active dev that touches consent.ts / catalog.ts / preflight.ts:

```bash
# Run unit tests (always)
bun test

# Run E2E (when investigating a real-CLI integration bug)
export AXHUB_E2E_STAGING_TOKEN=<your-token>
export AXHUB_E2E_STAGING_ENDPOINT=https://staging-api.jocodingax.ai
export AXHUB_E2E_STAGING_APP_ID=<numeric-app-id>
bun run build
bun run test:e2e
```

When done, unset the vars to avoid accidentally running E2E in subsequent sessions:

```bash
unset AXHUB_E2E_STAGING_TOKEN AXHUB_E2E_STAGING_ENDPOINT
```

## Vibe bootstrap measurement gate (Sprint 4)

`tests/e2e/staging.test.ts` stays read-only. The empty-dir → live URL measurement path is separate because it can create real apps/deploys and consume backend build capacity.

Run only with explicit destructive opt-in:

```bash
export AXHUB_E2E_STAGING_TOKEN=<token>
export AXHUB_E2E_STAGING_ENDPOINT=<explicit-staging-api-url>
export AXHUB_E2E_DESTRUCTIVE=1
export AXHUB_E2E_MAX_RUNS=1
export AXHUB_E2E_COST_BUDGET_USD=1
export AXHUB_E2E_CLEANUP_MODE=preprovisioned # or ttl
export AXHUB_E2E_PREPROVISIONED_APP_ID=<dedicated-staging-app-id> # required for preprovisioned
# or, for ttl cleanup:
# export AXHUB_E2E_TTL_CONFIRMED=1
export AXHUB_E2E_COMMAND_TIMEOUT_MS=120000
export AXHUB_E2E_FIXTURE_APP=tests/e2e/fixtures/vibe-static-app

bun run build
bun run measure:vibe-bootstrap --out vibe-bootstrap-measurement-summary.json
bun run check:vibe-sla --summary vibe-bootstrap-measurement-summary.json --mode advisory --min-samples 20 --p95-seconds 480
```

Safety contract:

- `AXHUB_E2E_STAGING_ENDPOINT` is required. The script refuses default endpoint fallback.
- `AXHUB_E2E_DESTRUCTIVE=1` is required.
- Cost and run bounds are enforced: `AXHUB_E2E_MAX_RUNS` is capped and `AXHUB_E2E_COST_BUDGET_USD` must cover the configured runs.
- Cleanup ownership is required: `preprovisioned` requires `AXHUB_E2E_PREPROVISIONED_APP_ID`; `ttl` rewrites the fixture to a unique `s4-measure-*` app and requires `AXHUB_E2E_TTL_CONFIRMED=1`.
- Each external command has `AXHUB_E2E_COMMAND_TIMEOUT_MS` timeout protection.
- Summary artifacts are redacted. They may include phase timings and booleans, but not tokens, emails, app slugs, raw URLs, command argv, stdout/stderr, or backend response bodies.
- The SLA source of truth is harness wall-clock. Bootstrap telemetry markers only corroborate phase/re-entry/consent behavior.
