# axhub Local CLI / Local Claude Code Plugin QA Report

Date: 2026-06-30
Endpoint: `https://api.axhub.ai`
CLI under test: `/Users/wongil/Desktop/work/jocoding/ax-hub-cli/target/release/axhub`
Plugin under test: `/Users/wongil/Desktop/work/jocoding/axhub` via Claude Code `--plugin-dir`
QA workspace: `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/latest-cli-0.22-plugin-1.5-20260630T010625Z`

## Verdict

**PASS after local CLI fixes.** The local release build reports `axhub 0.22.0`, the local plugin exposes all 8 skills as `axhub 1.5.0`, and import now handles non-`axhub bootstrap` frontend/backend stacks smoothly enough to preview and deploy representative apps with CLI/plugin-only flows.

The plugin repo itself did not need runtime code changes for this run. QA was executed against the local plugin checkout through `--plugin-dir`, not the installed marketplace/cache copy.

## Local Build And Plugin Evidence

| Check | Evidence |
|---|---|
| CLI binary | `/Users/wongil/Desktop/work/jocoding/ax-hub-cli/target/release/axhub --version` -> `axhub 0.22.0 (darwin/arm64)` |
| API endpoint | `AXHUB_ENDPOINT=https://api.axhub.ai` |
| Plugin source | Claude debug logs show `Loaded inline plugin from path: axhub` |
| Skill inventory | Claude debug logs show `Loaded 8 skills from plugin axhub` |
| Auth | `giri@jocodingax.ai`, scopes `read`, `write`, tenant admin on `test`/`jocodingax` |

## 8 Skill QA

All 8 skills were invoked one by one through Claude Code with the local plugin and the local release CLI first on `PATH`.

| Skill | Result | Fresh evidence |
|---|---|---|
| `update` | PASS | CLI `0.22.0` and plugin `1.5.0` both latest; no mutation |
| `onboarding` | PASS | Read-only environment/preflight checks passed against local CLI/plugin |
| `clarity` | PASS | `auth status`/whoami-style live query returned logged-in account and tenants |
| `init` | PASS | Empty-dir readiness passed; headless stopped before actual app creation |
| `import` | PASS | FastAPI fixture preview emitted `import/v1`, Docker hints, no mutation |
| `deploy` | PASS | Deploy readiness/preflight read-only flow passed; no new deploy mutation in skill call |
| `development` | PASS | Local Node fixture gained `/qa-local-build-check`; `node --check` and local HTTP smoke passed |
| `diagnosis` | PASS | Read-only diagnosis identified recent failed Next build; no redeploy/rollback |

Skill logs:

- `logs/skill-update.stream.jsonl`
- `logs/skill-onboarding.stream.jsonl`
- `logs/skill-clarity.stream.jsonl`
- `logs/skill-init.stream.jsonl`
- `logs/skill-import.stream.jsonl`
- `logs/skill-deploy.stream.jsonl`
- `logs/skill-development.stream.jsonl`
- `logs/skill-diagnosis.stream.jsonl`

## Import Preview Matrix

`axhub plugin-support import --mode preview --headless --json` was run with the local release CLI across 17 non-bootstrap fixtures.

| Case | Result | Method | Key hint |
|---|---|---|---|
| Next.js | PASS | `docker` | `next build`, `next start -p ${PORT:-3000}` |
| Vite | PASS | `static` | `vite build`, `static_dir=dist` |
| Astro | PASS | `static` | `astro build`, `static_dir=dist` |
| Static HTML | PASS | `static` | `static_dir=.` |
| Express TS | PASS | `docker` | `tsc`, `node dist/server.js` |
| FastAPI | PASS | `docker` | `uvicorn main:app --host 0.0.0.0 --port ${PORT:-8080}` |
| Flask | PASS | `docker` | `flask run --host=0.0.0.0 --port=${PORT:-8080}` |
| Django | PASS | `docker` | `manage.py runserver 0.0.0.0:${PORT:-8080}` |
| Go | PASS | `docker` | `go build -o app .`, `./app` |
| Rust | PASS | `docker` | `cargo build --release`, `./target/release/qa-rust-api` |
| Java Maven | PASS | `docker` | `mvn -DskipTests package`, `java -jar target/*.jar` |
| PHP | PASS | `docker` | `php -S 0.0.0.0:${PORT:-8080}` when no `public/` dir exists |
| Ruby Rack | PASS | `docker` | `bundle exec rackup -o 0.0.0.0 -p ${PORT:-8080}` |
| Deno | PASS | `docker` | `deno task start` |
| .NET | PASS | `docker` | `dotnet publish -c Release -o out`, `dotnet out/*.dll` |
| Dockerfile app | PASS | `docker` | Existing Dockerfile respected |
| Compose app | PASS | `compose` | Compose manifest detected |

Matrix artifact: `logs/import-preview-matrix.tsv`

## Prod Execute / Deploy QA

Representative destructive import paths were re-run after the local CLI fixes, without passing `--installation-id`, to prove automatic installation id resolution and static deploy timeout behavior.

| Case | App slug | Repo | Result |
|---|---|---|---|
| Static HTML | `qa-static-html-0630110949` | `demodev-lab/qa-static-html-0630110949` | static release `1d3d370d-413a-4c7a-9377-8ced7bc1fc41`, `verified=true`, URL `https://qa-static-html-0630110949.test.axhub.page` |
| FastAPI | `qa-fastapi-0630110949` | `demodev-lab/qa-fastapi-0630110949` | deployment `5e4b8ab5-6d2b-4f8f-9e13-0402e31dc2e7`, verify `success`, URL `https://qa-fastapi-0630110949.test.axhub.ai` |
| Compose | `qa-compose-app-0630110949` | `demodev-lab/qa-compose-app-0630110949` | deployment `a6d7082f-55f6-417a-92cf-b73f5af4dcc9`, verify `success`, URL `https://qa-compose-app-0630110949.test.axhub.ai` |

Additional verification:

- `axhub deploy verify --app qa-fastapi-0630110949 5e4b8ab5-6d2b-4f8f-9e13-0402e31dc2e7 --json` -> `success=true`, `status=succeeded`
- `axhub deploy verify --app qa-compose-app-0630110949 a6d7082f-55f6-417a-92cf-b73f5af4dcc9 --json` -> `success=true`, `status=succeeded`
- `axhub apps get qa-static-html-0630110949 --tenant test --json` -> `status=deployed`, `deploy_method=static`

Public app URLs redirect to the product login/auth wall because these QA apps are private. That is expected for unauthenticated curl; deployment verification still succeeded through axhub's product verification surfaces.

## Bugs Found And Fixed In Local CLI

Modified repo: `/Users/wongil/Desktop/work/jocoding/ax-hub-cli`

| Problem | Fix |
|---|---|
| Plain `index.html` apps previewed as Docker instead of static | Added plain static HTML detection with `static_output_dir="."` |
| Static package apps did not expose usable output dirs | Added static output hints for Vite/Astro/etc. (`dist`) and React/Gatsby-style builds (`build`) |
| PHP projects without `public/` got an invalid `php -S ... -t public` start command | Only add `-t public` when `public/` exists |
| GitHub connect fell into device flow when repo owner already had an installed GitHub App account | Import execute now resolves `axhub github accounts list --json` and passes matching `--installation-id ... --direct` automatically |
| Static release in import execute was killed by the short child-process timeout | `apps static deploy` child commands now get a 300s timeout instead of the probe timeout |

## Verification Commands

CLI repo:

```bash
CARGO_INCREMENTAL=0 rtk cargo check --locked -p axhub --bin axhub
rtk cargo fmt --all --check
CARGO_INCREMENTAL=0 rtk cargo test --locked -p axhub --bin axhub plugin_support::preflight::tests::static_deploy_child_timeout_is_longer_than_probe_timeout
CARGO_INCREMENTAL=0 rtk cargo test --locked -p axhub --bin axhub plugin_support::import
CARGO_INCREMENTAL=0 rtk cargo build --release -p axhub --locked
```

Results:

- `cargo check`: passed
- `cargo fmt --check`: passed
- preflight timeout test: 1 passed
- import tests: 60 passed
- release build: passed

Plugin repo:

```bash
rtk bun test
rtk bun run lint:tone --strict
rtk bunx tsc --noEmit
claude plugin validate /Users/wongil/Desktop/work/jocoding/axhub
```

Results:

- `bun test`: 57 passed
- tone lint strict: 0 errors/warnings
- TypeScript: passed
- plugin validation: passed

## Computer Use

Not used in the final pass. The requested fallback condition did not occur: auth, plugin loading, skill invocation, import, GitHub connect, static release, Docker deploy, compose deploy, and verification all completed headlessly with the local CLI/plugin.
