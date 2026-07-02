# Deploy workflow detail reference

Load this only after the top-level `deploy` skill has passed the activation boundary and the user approved the initial preview, or when a specific branch below is needed.

## Routing and context

- Named target wins. If the user explicitly says another target such as Vercel, Netlify, Fly, Cloudflare Pages, Render, Railway, Heroku, AWS, GCP, Azure, VPS, or GitHub Pages, stop axhub deploy before `deploy-prep` and say that the other target seems intended.
- If no other target is named, use `axhub plugin-support route-decision --user-utterance "$ARGS"` after the CLI/preflight guard. `axhub` continues; `ignore` asks one interactive target question; headless uses the safe default and stops rather than mutating.
- Same-conversation carry-over is allowed only when concrete evidence is visible in the current conversation. Use `references/session-carryover.md`. It can reduce repeated explanation, but it never bypasses auth, GitHub install, tenant, preview, or verify gates.

## Tenant picker

Use `AXHUB_TENANT` if already set. Otherwise resolve once:

```bash
AXHUB_TENANT=$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)
```

If the resolver reports multiple candidates, interactive mode may ask the user to pick and cache `{tenant, source, ts}` in `.axhub/state/tenant.json`. Headless mode uses the resolver's active/default candidate if available; never block on a tenant picker with AskUserQuestion in headless.

## Deploy-prep envelope

Resolve the authoritative app, profile, branch, commit, preflight, bootstrap boundary, in-flight deployment, GitHub connection, and quality gate through `deploy-prep`:

```bash
DEPLOY_PREP_JSON=$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json)
eval "$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --field-expr '"IN_FLIGHT_ID=" + (.in_flight_deploy.id // "" | @sh), "IN_FLIGHT_COMMIT=" + (.in_flight_deploy.commit_sha // "" | @sh), "RESOLVE_COMMIT=" + (.resolve.commit_sha // "" | @sh), "GITHUB_CONNECTED=" + (.github_connected // false | tostring | @sh)' 2>/dev/null)"
```

The same logical envelope must drive in-flight, status-first, and create. Reuse the first JSON when practical; if field extraction is repeated, do not let the values diverge semantically.

Quality gate failure stops by default. Interactive mode may ask whether to force current values; headless safe default is cancel. If `bootstrap_plan` is non-null, or `app_id` cannot be resolved, stop at the first-run boundary and hand off to `import` for a non-empty existing app or `init` for an empty new app. Do not continue to preview while `branch` or `commit_sha` is empty.

## Target reconciliation

Before mutation, reconcile stale manifest risk. If the conversation points at a different app, the utterance names a different app, or the manifest slug looks stale, confirm the target interactively. If the user chooses another app, update `axhub.yaml`, rerun `deploy-prep`, and include that manifest change in the git-readiness step. In headless mode, any target conflict downgrades to dry-run.

## Static lane

Static apps do not use deployment-record verify. After `deploy-prep` resolves an existing `APP_ID`, probe the app:

```bash
DEPLOY_METHOD=$(axhub apps get "$APP_ID" --no-input --field-expr '.deploy_method // empty' 2>/dev/null || true)
```

Only `DEPLOY_METHOD=static` enters this lane. All other values use the deployment-record lane.

1. Capability probe:

   ```bash
   axhub apps static deploy --help >/dev/null 2>&1
   ```

   If unavailable, ask the user to update axhub and stop.

2. Select `--from-dir` from common output folders (`dist`, `build`, `out`, `public`) or ask interactively if ambiguous. Headless chooses the first candidate or stops with dry-run guidance when no candidate exists.

3. Preview first:

   ```bash
   axhub apps static deploy --app "$APP_ID" --from-dir "$STATIC_DIR" --tenant "$AXHUB_TENANT" --dry-run
   ```

   Humanize file count/bytes and process as release create, upload, finalize, activate. Headless stops here. Interactive mode asks for approve, dry-run only, or abort.

4. Execute only after explicit approval:

   ```bash
   axhub apps static deploy --app "$APP_ID" --from-dir "$STATIC_DIR" --tenant "$AXHUB_TENANT" --execute
   ```

   Static success is `active_release_id` from activation plus, when available, `axhub apps get "$APP_ID" --no-input --field-expr '.access_url // empty'`. Never call `axhub deploy verify` in this lane.

## Git readiness

Do not preview an old commit while deploy-affecting local changes are uncommitted. If `deploy-prep` reports `git_init_needed`, no commit, missing branch/commit, or uncommitted deploy-affecting changes, pause before preview.

Interactive mode may ask to create a local save point, then run quiet git commands and rerun `deploy-prep`:

```bash
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git init >/dev/null 2>&1
fi
git add -A >/dev/null 2>&1
git commit -m "init: axhub deploy baseline" >/dev/null 2>&1 || true
git branch -M main >/dev/null 2>&1
axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json
```

Raw git output stays out of chat. Headless safe default is cancel; never run `git init` automatically in subprocess mode.

## In-flight and status-first

If `IN_FLIGHT_ID` exists, compare `IN_FLIGHT_COMMIT` and `RESOLVE_COMMIT`:

- same non-empty commit: likely duplicate of the user's deploy; prefer monitoring.
- different non-empty commit: possible other user or tenant; be conservative.
- either empty: uncertain; be conservative.

Interactive choices are monitor, force new, or cancel. Headless safe default is abort. `monitor` sends `DEPLOY_ID="$IN_FLIGHT_ID"` to verify and never creates a new deployment. `force_new` may proceed to preview, but exit 64 deployment-in-progress is not retried.

For GitHub-connected apps, use status-first before fallback create. If no in-flight deployment is visible yet, interactive mode may wait briefly and refresh with `deploy-prep --refresh-in-flight`; headless does not wait. If a status-first id appears, reuse the same in-flight branch rules and skip `deploy create`.

## Preview and token gate

Headless detection:

```bash
AXHUB_HEADLESS=0
if ! [ -t 1 ] || [ -n "${CI:-}" ] || [ -n "${CLAUDE_NON_INTERACTIVE:-}" ]; then
  AXHUB_HEADLESS=1
fi
if [ "$AXHUB_HEADLESS" = "1" ]; then
  DEPLOY_DECISION="dry_run"
fi
```

Interactive preview shows exactly app, environment, branch, commit, and ETA. Normalize displayed slug with NFKC and warn if normalization changes it. Ask approve, dry-run, or abort. Dry-run natural language such as "리허설", "테스트로", or "진짜 안 올리고" also sets `DEPLOY_DECISION=dry_run`.

Before execute, run:

```bash
axhub plugin-support token-gate
```

Exit 0 continues. Exit 65 routes to auth recovery. `AXHUB_AUTH_BG_REFRESH=0` disables the gate.

## Deployment-record create

This is the fallback path when status-first found no running deployment. Run only after the preview decision:

```bash
if [ -z "${AXHUB_TENANT:-}" ]; then
  AXHUB_TENANT=$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)
fi
PROFILE_FLAG=()
if [ -n "${PROFILE:-}" ] && [ "${PROFILE:-}" != "default" ]; then
  PROFILE_FLAG=(--profile "$PROFILE")
fi
AXHUB_STDERR_TMP=$(mktemp); AXHUB_STDOUT_TMP=$(mktemp)
if [ "${DEPLOY_DECISION:-approve}" = "dry_run" ]; then
  axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --dry-run --field-expr '.id // .deployment_id // empty' >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
elif [ "${DEPLOY_DECISION:-approve}" = "abort" ]; then
  echo "배포를 멈춰요." >&2; rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
else
  axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --execute --field-expr '.id // .deployment_id // empty' >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
fi
AXHUB_EXIT=$?
```

On exit 64 with `validation.deployment_in_progress`, do not retry. Refresh in-flight once and verify that id if available; otherwise tell the user another deploy is in progress and stop. On exit 0, bind `DEPLOY_ID` from stdout. If no id exists, do not claim success; tell the user the start was seen but no result id was received.

## Verify and diagnosis

Deployment-record success is declared only by:

```bash
echo "배포 결과를 확인하고 있어요." >&2
VERIFY_OUT=$(mktemp)
axhub deploy verify "$DEPLOY_ID" > "$VERIFY_OUT" 2>&1
VERIFY_EXIT=$?
```

Exit handling:

- 0: terminal success; summarize in Korean with the verified live URL if available.
- 6: non-terminal; say the build is still running and invite "배포 상태 확인해줘".
- 7: terminal failure; say "배포가 실패했어요. 지금부터 원인 진단만 읽기 전용으로 확인할게요. 재배포나 롤백은 하지 않아요." Then hand off to `diagnosis`.
- 5: unknown deployment id; stop without latest lookup.
- 4: auth expired; use auth recovery.

For the failure handoff, preserve `DEPLOY_ID`, app slug/id/name, and classified verify state internally. Do not expose raw output. If a Skill tool exists, invoke `diagnosis` with the app identity and "방금 배포 verify 가 실패했다" context. Otherwise follow diagnosis' read-only surface: MCP `deployment_diagnosis` if callable, else `axhub --json deploy diagnose <앱>`.

## Error routing

For non-zero commands outside verify, classify with:

```bash
axhub plugin-support classify-exit "$EXIT" "$STDOUT"
```

or use `references/error-empathy-catalog.md`.

- exit 64 + `validation.deployment_in_progress`: explain another deployment is running, never retry create, offer to watch.
- exit 9 + `subdomain_not_configured`: subdomain update is a separate destructive mutation. Preview the proposed 2..32 character subdomain and require approval before `axhub apps update`.
- exit 9/64/67 + GitHub connection required: do not create repo, first push, or `apps git connect` from deploy. Hand off to import.
- exit 4/65: auth expired; ask before login flow in interactive mode.
- exit 5/67: not found; offer did-you-mean from apps list without numeric ids.
- exit 6/68: rate limit; respect Retry-After.
- exit 1: transport; retry read paths at most once, never create.

## Secondary commands

Load `references/command-coverage.md` for read-only deployment browsing and cancel. Cancel remains a mutation: preview the in-progress deployment and require explicit approval before `axhub deploy cancel`.
