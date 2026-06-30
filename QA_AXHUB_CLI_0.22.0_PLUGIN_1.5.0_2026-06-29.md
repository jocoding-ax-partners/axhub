# axhub CLI 0.22.0 / Claude Code Plugin 1.5.0 QA Report

Date: 2026-06-29
Endpoint: `https://api.axhub.ai`
QA workspace: `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z`

## Verdict

**PASS after CLI fixes and final review blocker fix.** Plugin 1.5.0 exposes all 8 skills and CLI 0.22.0 can import and deploy non-`axhub bootstrap` apps to the live prod API.

Import was the main risk area. I tested four real import/deploy paths against `api.axhub.ai`:

| Case | App slug | Repo | Result evidence |
|---|---|---|---|
| Next.js default app, no axhub bootstrap, Docker deploy | `qa-next-poll-0629155808` | `demodev-lab/axhub-qa-next-poll-0629155808` | deployment `0081ede0-649a-48e3-9735-e015811a5649`, verify `success`, URL `https://qa-next-poll-0629155808.test.axhub.ai` |
| Vite React default app, no explicit deploy method | `qa-vite-static-0629155808` | `demodev-lab/axhub-qa-vite-static-0629155808` | static release `3b83f22c-49cc-4273-8602-e72cf25502af`, `verified: true`, URL `https://qa-vite-static-0629155808.test.axhub.page` |
| Local-only static app, no git remote at start | `qa-local-static-0629155808` | `demodev-lab/axhub-qa-local-static-0629155808` | static release `bc4172b4-6530-40aa-bee6-84c75e5250df`, `verified: true`, URL `https://qa-local-static-0629155808.test.axhub.page` |
| Local-only static app after final approval-contract fix | `qa-local-review-0630015914` | `demodev-lab/axhub-qa-local-review-0630015914` | static release `2aab9a27-0989-4153-913f-0e8e1475736c`, `verified: true`, URL `https://qa-local-review-0630015914.test.axhub.page` |
| Existing axhub app repair/re-import | `qa-vite-static-0629155808` | `demodev-lab/axhub-qa-vite-static-0629155808` | static release `b43be770-3c8b-4da6-b6ac-c3e4813ff603`, `verified: true` |

The public URLs currently redirect to auth pages, but deployment/release verification succeeded through the product verification surfaces.

## Version And Capability Evidence

| Check | Evidence |
|---|---|
| CLI version | `/Users/wongil/.axhub/bin/axhub --version` -> `axhub 0.22.0` |
| Plugin version | `claude plugin details axhub@axhub` -> `axhub 1.5.0` |
| Plugin skill inventory | 8 skills: `clarity`, `deploy`, `development`, `diagnosis`, `import`, `init`, `onboarding`, `update` |
| Plugin state | `claude plugin list` -> `axhub@axhub`, version `1.5.0`, local scope, enabled |
| Preflight | `auth_ok: true`, `cli_version: 0.22.0`, endpoint `https://api.axhub.ai`, import `supported: true`, schema `import/v1`, `commit_manifest: true` |

## Import QA Coverage

### 1. Next.js Default App Import

- Started from a newly created non-axhub-bootstrap Next.js app.
- Import inferred Docker deploy.
- CLI generated and committed `axhub.yaml` plus a minimal Dockerfile.
- GitHub was connected with direct installation-id flow.
- Deployment was created and `axhub deploy verify` was polled until terminal success.
- Evidence file: `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/next-poll-execute.json`

### 2. Vite React Default App Import

- Started from a newly created Vite React app with no axhub bootstrap.
- Import inferred `static` from Vite/build-script shape without requiring `--deploy-method`.
- Manifest was committed and pushed.
- Static release was finalized and activated.
- Evidence file: `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/vite-default-static-execute.json`

### 3. Local-Only Static App Import

- Started from a local app with no useful remote setup.
- Import initialized git, committed app files, created/pushed a GitHub repo, connected GitHub, and deployed.
- This covers the "not created by axhub bootstrap" plus "no existing GitHub remote" path.
- Evidence file: `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/local-only-static-execute.json`

### 3b. Local-Only Approval Contract Regression

- Independent final review found that `--repo` with no origin could execute repo creation/full source push while preview under-reported the external mutation.
- Fixed detection so `--repo` is not treated as an already-connected origin. Preview now reports `starting_state: local_only`, `remote_repo: null`, and includes `github_repo_create`.
- Preview safety notes now disclose local git initialization, full source commit, GitHub repo creation, and remote push.
- Re-tested against prod with a new local-only app after the fix.
- Evidence files:
  - `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/local-review-preview.json`
  - `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/local-review-execute.json`

### 4. Existing App Repair/Re-import

- Re-ran import against the already imported Vite app.
- Import recognized `existing_axhub_app_repair`, selected the app, connected GitHub, and created a new static release.
- Evidence file: `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/vite-existing-repair-execute.json`

## 8 Skill QA

All skills were invoked one by one through Claude Code skill routing. Headless was enough for all observed paths, so computer-use was not needed.

| Skill | Result | Evidence |
|---|---|---|
| `onboarding` | PASS | Detected CLI 0.22.0, auth OK, GitHub App accounts present, MCP connected, no mutation |
| `init` | PASS with observation | Empty directory dry-run succeeded; headless tenant ambiguity defaults to first candidate |
| `deploy` | PASS after CLI resolver fix | Initial run exposed stale slug resolution; after fix, static lane resolved `qa-vite-static-0629155808` and dry-run preview passed |
| `import` | PASS | Headless preview envelope obeyed `import/v1`, enum, approval, and no-mutation contracts |
| `development` | PARTIAL PASS | App/data discovery worked and connector schema loaded; `connector_query` failed even for `SELECT 1`, so live row smoke is blocked |
| `diagnosis` | PASS | Read-only deployment diagnosis ran and did not redeploy or roll back |
| `clarity` | PASS | Live help/schema exploration found `axhub apps mine`, ran read-only, returned app list |
| `update` | PASS | CLI and plugin both already latest; apply/update commands were not executed |

Skill logs:

- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/onboarding.txt`
- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/init.txt`
- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/deploy-after-fix.txt`
- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/import.txt`
- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/development.txt`
- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/diagnosis.txt`
- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/clarity.txt`
- `/Users/wongil/Desktop/work/jocoding/axhub-qa-runs/live-plugin-1.5.0-cli-0.22.0-20260629T145025Z/skill-logs/update.txt`

## Bugs Found And Fixed In CLI

Modified repo: `/Users/wongil/Desktop/work/jocoding/ax-hub-cli`

| Problem | Fix |
|---|---|
| Import with `--installation-id` still entered browser/callback GitHub connect flow | `connect_git` now passes `--direct` when installation id is supplied |
| Next.js default app had no Dockerfile, so first Docker deployment could fail | Import now generates a minimal Node Dockerfile when Docker deploy is inferred and no Dockerfile exists |
| Docker/compose import could return before deployment verification was truly successful | Import now creates deployment with an idempotency key and polls `axhub deploy verify` until terminal success/failure or timeout |
| Vite/default static apps required too much manual deploy-method knowledge | Import now infers static for Vite/static package shape and `dist/index.html` |
| Local-only apps without a useful GitHub remote could not complete import | Import can initialize git, commit, create/push the target GitHub repo, then continue app connect/deploy |
| Local-only `--repo` preview could under-report repo creation/source publication before approval | Detection now keeps missing-origin projects as `local_only`, required mutations include `github_repo_create`, and preview safety notes disclose git init/full source commit/GitHub create/push |
| Deploy skill could resolve stale `current_app`/remote slug instead of the imported app slug | `deploy-prep` now uses manifest app slug/name fallback, including slugifying top-level manifest `name`, before treating the app as first-run |

## Remaining Observations

- `development` skill data-query smoke is blocked by `connector_query` failing for connector `qa-ext-rnacentral` even when `connector_resources` works. This looks like a backend/runtime connector execution issue, not a plugin routing issue.
- `init` headless mode does not ask for tenant selection and defaults to the first tenant when multiple candidates exist. This is acceptable for headless QA but worth revisiting for non-interactive safety semantics.
- Static and Docker public URLs redirected to auth pages during curl checks. Product verification still reported success, so this is not a deploy failure.

## Verification Commands

CLI repo:

```bash
rtk cargo fmt --all -- --check
rtk cargo test -p axhub --bin axhub plugin_support::import --locked
rtk cargo test -p axhub --test plugin_support_deploy_prep_cli --locked
rtk cargo build --release -p axhub --locked
```

Latest fresh results in this session:

- `cargo fmt --check`: exit 0
- import tests: 47 passed
- deploy-prep CLI tests: 7 passed
- release build: passed after the CLI fixes and final approval-contract fix; installed to `/Users/wongil/.axhub/bin/axhub`
- GitNexus `detect_changes`: 8 changed files, 20 changed symbols, 3 affected processes, medium risk; affected surface is plugin-support import/resolve

Plugin repo:

```bash
rtk bun test
rtk bun run typecheck
rtk bun run lint:tone
```

Latest fresh results:

- `bun test`: 57 passed
- `tsc --noEmit`: exit 0
- `lint:tone`: 0 errors, 0 warnings across 8 skill files

## Computer Use

Not used. The user requested computer-use only for parts that cannot be handled headlessly; every required skill invocation, import, deploy, and verification path completed through Claude Code skill/headless logs plus CLI/API evidence.
