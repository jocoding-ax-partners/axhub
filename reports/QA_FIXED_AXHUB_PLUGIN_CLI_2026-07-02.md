# AxHub 수정 버전 QA 리포트

Date: 2026-07-02 12:45 KST
QA workspace: `/tmp/axhub-fixed-qa-20260702`
Prod endpoint: `https://api.axhub.ai`
CLI under test: `/Users/wongil/Desktop/work/jocoding/ax-hub-cli/target/debug/axhub`

## 대상

- Plugin repo: `jocoding-ax-partners/axhub` PR #250, branch `codex/plugin-154-desktop-qa-smoothness`, commit `af0cb391d9f99b512e15f73cf2b1e36868de6ed2`.
- CLI repo: `jocoding-ax-partners/axhub-cli` PR #413, branch `codex/github-owner-installation-guard`, base commit before this QA `2f8cfadb91fc388d60c2ffa85e77b1a77e07f0f4`.
- Backend prod: backend PR #460 merged as `669f8fe8cd3ba8abc2024c6d51aa6cf1313ca646`; prod deploy workflow run `28561698047` succeeded at 2026-07-02 11:47:40 KST.

Note: repo metadata currently reports plugin `package.json` version `1.5.1` and local CLI workspace version `0.22.2`; installed PATH CLI reports `0.22.4`. For local-build QA, `AXHUB_BIN=/Users/wongil/Desktop/work/jocoding/ax-hub-cli/target/debug/axhub` was set so nested plugin-support shell-outs used the modified local binary.

## Coverage

- All 8 official plugin skills were covered by automated frontmatter and smooth-behavior contract tests: `onboarding`, `init`, `deploy`, `import`, `development`, `diagnosis`, `clarity`, `update`.
- Import skill was tested most deeply from a fresh directory:
  - plain static root with only `index.html`
  - Vite-style static app with `vite build`
  - Express backend app with `node server.js`
  - real static import execute against prod
- Deploy skill regression was tested with a fresh dirty git worktree.
- GitHub installation owner selection was covered by direct CLI unit regressions.

## Findings And Fixes

### 1. Static import success evidence hid private access requirements

Initial prod mutation:

- Slug: `qa-fixed-static-20260702123649`
- Result: `static_release`, `active_release_id=eeba01d5-71fd-485c-bb14-15bb93af3aaf`, `verified=true`, `public_url=https://qa-fixed-static-20260702123649.test.axhub.page`
- `axhub.yaml` correctly omitted `static_output_dir` for root static output.
- `apps static site get` showed `visibility=private`.
- Direct curl to the returned URL redirected to `https://axhub.ai/static-auth?reason=login_required...`, then showed the AxHub auth shell, not the uploaded HTML.

This is not a release activation failure, but the envelope made the URL look like a normally open public URL. For vibe-coder DX this is confusing.

Fix applied:

- CLI: `SuccessEvidence::StaticRelease` now has optional `access_note`.
- CLI: static import execute calls `apps static site get` after activation and sets a Korean note for `private`, `tenant_member`, and `invite_only`.
- CLI: static deploy/site-get child commands now preserve explicit `--tenant`.
- Plugin import skill: success copy now tells Claude Code to surface `access_note` as a natural-language note.

Retest prod mutation:

- Slug: `qa-note-124319`
- Result: `static_release`, `active_release_id=59b56568-4958-4c7b-919c-96a183a1a8ce`, `verified=true`, `public_url=https://qa-note-124319.test.axhub.page`
- Evidence now includes `access_note="이 정적 사이트는 비공개라 axhub 로그인이 필요해요."`
- `apps static site get` confirms `visibility=private`.

### 2. `plugin:bundle` script was documented but missing

`AGENTS.md` said to run `bun run plugin:bundle`, but `package.json` had no such script. This blocks clean local-plugin QA.

Fix applied:

- Added `scripts/bundle-plugin.cjs`.
- Added `plugin:bundle` npm script.
- Verified it regenerates `dist/axhub-plugin` with `skills/`, `hooks/`, `README.md`, and `LICENSE`.

## Passing Evidence

Plugin repo:

- `bun test` => 57 passed.
- `bun run typecheck` => clean.
- `bun run lint:tone --strict` => 0 errors, 0 warnings.
- `bun run plugin:bundle` => generated `dist/axhub-plugin` successfully.

CLI repo:

- `cargo build -p axhub` => passed.
- `cargo check -p axhub` => passed.
- `cargo fmt --check` => passed.
- `cargo clippy -p axhub --all-targets -- -D warnings` => passed.
- `cargo test -p axhub plugin_support::import` => 146 passed.
- `cargo test -p axhub --test plugin_support_import_contract` => 4 passed.
- `cargo test -p axhub installation_resolution` => 6 passed.
- `cargo test -p axhub select_installation` => 16 passed.
- `cargo test -p axhub bootstrap_requested_installation` => 4 passed.
- Targeted import regressions:
  - `static_hints_cover_package_and_plain_html_outputs` => passed.
  - `static_default_detects_root_index_with_package_without_start_script` => passed.
  - `porcelain_dirty_detects_tracked_and_untracked_changes` => passed.
  - `write_minimal_manifest_omits_root_static_output_dir` => passed.
  - `execute_without_slug_or_remote_fails_before_manifest_write` => passed.

Fresh-directory CLI smoke:

- Plain static preview: `deploy_method=static`, `manifest_hints.static_output_dir=null`.
- Vite preview: `deploy_method=static`, `manifest_hints.static_output_dir=dist`, `build_cmd=vite build`.
- Express preview: `deploy_method=docker`, `start_cmd=node server.js`.
- Dirty deploy: `plugin-support deploy-approved-run` exited `64` and printed the Korean saved-change guidance before any deploy.

Change-impact evidence:

- GitNexus impact for CLI `execute_static`: `MEDIUM`, direct caller `execute_import`; no HIGH/CRITICAL warning.
- GitNexus impact for `SuccessEvidence`: `LOW`, no upstream dependents.
- GitNexus `detect_changes` for CLI unstaged changes: risk `low`, affected processes `0`.
- GitNexus `detect_changes` for plugin unstaged changes: risk `low`, affected processes `0`.
- Targeted regression `cargo test -p axhub static_private_site_sets_access_note_and_forwards_tenant` => 2 passed.

## Remaining Concerns

- Version metadata mismatch remains: plugin repo says `1.5.1`, local CLI workspace says `0.22.2`, while the installed CLI reports `0.22.4`. QA used local build intentionally, but release/version bookkeeping should be aligned before publishing another release.
- Direct unauthenticated curl to private static site still returns the auth shell. This is expected for `visibility=private`; the fix is that the plugin/CLI now tells the user that login is required.
