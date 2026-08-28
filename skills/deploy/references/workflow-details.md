# Deploy workflow detail reference

Load this only after the top-level `deploy` skill has passed the activation boundary and the user approved the initial preview, or when a specific branch below is needed.

Claude Desktop 에서는 이 문서의 여러 줄 shell 블록을 실행하지 않아요. 블록은 논리·headless 설명용이에요. 모든 Desktop 권한 카드는 구체적인 값을 넣은 bare 명령 하나만 실행하고, 변수 대입·`eval`·`mktemp`·분기·pipe·redirect·`;`·`&&`·`||`·`echo`·`$?`를 금지해요. 앞선 JSON/tool 결과를 모델이 직접 읽어 다음 단일 명령의 literal 인자로 옮겨요.

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

Resolve the authoritative app, profile, branch, commit, preflight, bootstrap boundary, in-flight deployment, repository connection, and quality gate through `deploy-prep`:

```bash
DEPLOY_PREP_JSON=$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json)
eval "$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --field-expr '"IN_FLIGHT_ID=" + (.in_flight_deploy.id // "" | @sh), "IN_FLIGHT_COMMIT=" + (.in_flight_deploy.commit_sha // "" | @sh), "RESOLVE_COMMIT=" + (.resolve.commit_sha // "" | @sh), "GITHUB_CONNECTED=" + (.github_connected // false | tostring | @sh)' 2>/dev/null)"
```

The same logical envelope must drive in-flight, status-first, and create. Reuse the first JSON when practical; if field extraction is repeated, do not let the values diverge semantically.

Quality gate failure stops by default. If `bootstrap_plan` is non-null or `app_id` cannot be resolved, stop at the first-run boundary and hand off to import/bootstrap. Do not reject empty branch/commit until `apps get` selects the lane: selfhosted clones first, GitHub without a repo uploads, and only an unready GitHub repo path stops.

Read `GITHUB_CONNECTED` only after `git_backend.backend=github` is known. In that branch, false on a resolved app means the app has no repository and routes to the upload lane; true keeps the existing GitHub repo path. It is not a backend detector and must never send a selfhosted app to upload.

## Target reconciliation

Before mutation, reconcile stale manifest risk. If the conversation points at a different app, the utterance names a different app, or the manifest slug looks stale, confirm the target interactively. If the user chooses another app, update `axhub.yaml`, rerun `deploy-prep`, and include that manifest change in the git-readiness step. In headless mode, any target conflict downgrades to dry-run.

## App backend and static lane

After `deploy-prep` resolves an existing `APP_ID`, read the public app JSON once:

```bash
axhub apps get "$APP_ID" --no-input --json
```

Use top-level `deploy_method`, `git_backend.backend`, and `git_backend.source`. Only `deploy_method=static` enters the static lane. Non-static `selfhosted` enters the push branch below; non-static `github` and `legacy_github` preserve the existing GitHub/upload path. Never call C1 or Gitea directly and never infer backend from a remote URL.

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

## Self-hosted push branch

For `git_backend.backend=selfhosted`, do not inspect `github_connected`. If the current folder is not the resolved app's clone, run `axhub repo clone <app> --json` first and require a successful envelope with a non-empty absolute `data.destination`. Use the CLI-resolved HTTPS remote and never synthesize the repository path or destination.

Treat that exact `data.destination` as the working directory for every subsequent repository-local command in this branch. Set the tool `cwd` to it for status/savepoint, `git rev-parse`, branch/commit work, push, fetch, and containment checks; do not run those commands in the original un-cloned folder. An explicit clone destination, when supplied, is authoritative because the CLI returns the same absolute path.

After the common local savepoint and preview approval, `git push -u origin "HEAD:$BRANCH"` from that working directory is the deployment mutation. A successful push starts the webhook deployment, so do not run `deploy create` or the upload lane. Refresh `deploy-prep --refresh-in-flight` within AP-16's 30-check/10-minute budget until the exact deployment id appears, then use the common verify loop. A budget expiry is pending, not failure.

The rest of this reference's containment/create/upload instructions are the GitHub branch unless they explicitly say common.

## Git readiness

Do not preview an old commit while deploy-affecting local changes are uncommitted. If `deploy-prep` reports `git_init_needed`, no commit, missing branch/commit, or uncommitted deploy-affecting changes, pause before preview. Ignore local agent/runtime state such as `.omc/`, `.claude/`, `.codex/`, `.serena/`, `.omx/`, `.omo/`; those paths are not deploy-affecting app changes and must not be committed, pushed, or added to `.gitignore` during deploy cleanup. If deploy-prep or another target check still reports only those runtime paths, treat the app commit as clean enough for deploy and do not mutate `.gitignore`; if the CLI blocks anyway, stop with a CLI-gap note instead of creating a cleanup commit.

Interactive mode may ask to create a local save point, then run quiet git commands and rerun `deploy-prep`:

```bash
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git init >/dev/null 2>&1
fi
git add -A -- . ':(exclude).omc' ':(exclude).claude' ':(exclude).codex' ':(exclude).serena' ':(exclude).omx' ':(exclude).omo' >/dev/null 2>&1
git commit -m "init: axhub deploy baseline" >/dev/null 2>&1 || true
git branch -M main >/dev/null 2>&1
axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json
```

Before push, normalize the resolved commit to a full local SHA. GitHub additionally confirms remote containment before create; selfhosted uses push success plus the webhook deployment id:

```bash
COMMIT_SHA=$(git rev-parse "${COMMIT_SHA:-HEAD}^{commit}")
git push -u origin "HEAD:$BRANCH" >/dev/null 2>"$AXHUB_STDERR_TMP"
PUSH_EXIT=$?
git fetch origin "$BRANCH" >/dev/null 2>&1
git merge-base --is-ancestor "$COMMIT_SHA" "origin/$BRANCH"
```

For selfhosted, this section applies even when `GITHUB_CONNECTED` is false; push success routes to the webhook wait. For GitHub, false routes to upload and push/containment failure stops unless GitHub itself is the cause. Judge push success by `PUSH_EXIT`, not by stderr text. Raw git output stays out of chat. Headless never runs `git init` automatically. Never run `axhub deploy create --execute` for a local-only commit.

## In-flight and status-first

If `IN_FLIGHT_ID` exists, compare `IN_FLIGHT_COMMIT` and `RESOLVE_COMMIT`:

- same non-empty commit: likely duplicate of the user's deploy; prefer monitoring.
- different non-empty commit: possible other user or tenant; be conservative.
- either empty: uncertain; be conservative.

Interactive choices are monitor, force new, or cancel. Headless safe default is abort. `monitor` sends `DEPLOY_ID="$IN_FLIGHT_ID"` to the bounded verify loop and never creates a new deployment. `force_new` may proceed to preview, but exit 64 deployment-in-progress is not retried.

For GitHub-connected apps, use status-first before fallback create. If no in-flight deployment is visible yet, interactive mode may wait briefly and refresh with `deploy-prep --refresh-in-flight`; headless does not wait. If a status-first id appears, reuse the same in-flight branch rules and skip `deploy create`. Do not call `axhub deploy watch` or `axhub deploy status --watch`; Desktop/non-TTY watch can degrade or fail on required app flags. Use the bounded verify loop below.

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

Interactive preview shows exactly app, environment, branch, commit, and ETA. Normalize displayed slug with NFKC and warn if normalization changes it. The visible environment label is `운영`; do not show `prod`, `production`, or raw profile names in the card unless the user explicitly asks for CLI-level evidence. Ask approve, dry-run, or abort. Dry-run natural language such as "리허설", "테스트로", or "진짜 안 올리고" also sets `DEPLOY_DECISION=dry_run`.

User-visible prose and tool titles must translate workflow internals: `진행 중 배포` for in-flight work, `미리보기` for dry-run, `인증 상태 확인` for token-gate, `배포 실행` for execute, and `검증 성공` for terminal success. Do not show `deploy-prep`, `in-flight`, `dry-run`, `token-gate`, `execute`, `production`, `terminal success`, `gitignore`, `gitting`, `checking`, `Build passed`, `Working tree clean`, `Not ignored`, `User explicitly authorized`, `Proceeding`, or `Push 성공` in chat/tool titles/final tables unless the user asked for low-level debugging evidence.

Before execute, run:

```bash
AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate
```

Exit 0 continues. Exit 65 routes to auth recovery. `AXHUB_AUTH_BG_REFRESH=0` disables the gate.
Do not pipe token-gate through `grep`, `head`, or a combined dry-run pipeline. It is an exit-code gate with no user-facing stdout. In Claude Desktop use the fast inline check above so the UI does not sit in a silent 30 second polling window.

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

## GitHub upload lane — deploy the local folder

This lane is for `git_backend.backend=github` only. The source is the local folder instead of a repo commit (backend spec 184). Everything after create is unchanged — `axhub up` prints the same create result, so `DEPLOY_ID` binding, the verify loop, and the diagnosis handoff stay identical.

Two entries, and they are not the same thing:

- **No repo (steady state).** `github_connected` is false in the `deploy-prep` envelope. The app has no repository — normal for an app first deployed from local source. For repo apps a push fires the webhook and deploys without this skill; an app with no repo receives no push webhook, so an explicit deploy through this lane is the only way it ever ships a change. This is not a failure: do not attempt push, do not report recovery, and do not apply the empty branch/commit stop, since no commit is sent. Skip Git readiness entirely and come straight here.
- **Repo blocked (recovery).** `github_connected` is true but the push or containment check failed because of GitHub itself: permission denied, 404 on a private repo, org policy, expired or refused device flow. Only this entry gets the recovery framing.

Two guards, recovery entry only:

- The cause must be GitHub. Network, timeout, and 5xx get one retry of the same step first. Falling through on a transient error silently drops the repo and push auto-deploy the user already had.
- The app must already exist and be resolved (`APP_ID` from `deploy-prep`). This lane never creates apps or repos — that stays `bootstrap` and `import`.

Say one line and continue, not a question. No repo: `이 앱은 저장소 없이 소스를 올려서 배포해요.` Repo blocked: `GitHub 쪽이 막혀서, 지금 폴더의 소스를 그대로 올려서 배포할게요.` The preview card and interactive approval are still required in both — same destructive mutation, different source. What the preview shows is the packed file count, size, and version hash.

```bash
axhub up --app "$APP_ID" --tenant "$AXHUB_TENANT" "${PROFILE_FLAG[@]}" --execute --field-expr '.id // .deployment_id // empty' >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
```

Drop `--execute` for the dry-run decision; `axhub up` previews by default. Always pass `--app` explicitly. `axhub up` needs CLI 0.29.0+ — on unknown-command exit, route to `update` and stop rather than falling back to `deploy create`.

What goes up is the current folder with `.gitignore` honored, so `.git/`, `node_modules/`, and `.env` stay out. No commit is sent, which is why a local-only commit is fine here and only here. A connected repo is not touched or disconnected: the source is chosen per deployment, not per app.

Tell the user after the result, never before, and only what applies. No repo: this app deploys by uploading the folder, so there is no push auto-deploy, and `axhub apps git connect` adds one later without recreating the app. Repo blocked: this one deployment did not come from the repo, so push auto-deploy and the version history do not cover it, and the next deploy goes back to the repo path once GitHub works.

## Verify loop and diagnosis

Deployment-record success is declared only by `axhub deploy verify`. Poll this command for in-flight/webhook deployments:

```bash
axhub deploy verify <deployment-id> --app <app-id>
```

preflight 의 `capabilities.import.verify_wait` 가 true 면 위 명령 대신 **권한 카드 한 번으로 끝나는** 단일 대기 호출을 정확히 한 번만 실행해요:

```bash
axhub deploy verify "$DEPLOY_ID" --app "$APP_ID" --wait --wait-interval 20s --wait-timeout 10m --json
```

호출 직전에 `아직 빌드 중이에요. 같은 배포를 계속 확인할게요.` 한 줄만 남겨요. `--wait` 가 성공·실패·예산 제한까지 책임지므로 같은 verify 를 반복 호출하거나 `axhub apps get`, `deploy list`, `deploy status` 같은 사후 확인을 덧붙이지 않아요. 대기 수단 없이 같은 verify 를 연달아 호출해 같은 exit 6 을 화면에 쌓는 연타 폴링은 UX 실패예요.

`verify_wait` capability 가 없는 구 CLI 에서만 `--wait` 없는 verify 를 폴링 예산(최대 30회 또는 10분, AP-16) 안에서 반복해요. Do not combine polling into one long `while`/`for` shell loop with `MAX_ATTEMPTS`, command substitution, or shell expansion. Also do not collapse polling into one long `while`/`for`/`until` shell loop with `sleep`, `grep`, `head`, pipes, or shell expansion. A Claude Desktop permission request for a long polling shell block is a failed watch UX. Do not substitute `axhub deploy watch` or `axhub deploy status --watch`. Do not end by asking the user to say `배포 상태 확인해줘`; the skill owns the follow-up while a known `DEPLOY_ID` is still running.

Exit handling:

- 0: terminal success; summarize in Korean with the verified live URL if available.
- 6: non-terminal. `--wait` 경로에서는 예산을 다 쓰고도 끝나지 않은 상태라 같은 verify 를 자동 재실행하지 않고 재개 요약으로 끝내며 `DEPLOY_ID` 를 보존해요. fallback 경로에서만 keep the same `DEPLOY_ID` and continue the bounded verify loop automatically. 어느 쪽도 사용자에게 다시 상태 확인을 요청하지 않아요.
- 7: terminal failure; say "배포가 실패했어요. 지금부터 원인 진단만 읽기 전용으로 확인할게요. 재배포나 롤백은 하지 않아요." Then hand off to `diagnosis`.
- 5: unknown deployment id; stop without latest lookup.
- 4: auth expired; use auth recovery.

For the failure handoff, preserve `DEPLOY_ID`, app slug/id/name, and classified verify state internally. Do not expose raw output. If a Skill tool exists, invoke `diagnosis` with the app identity and "방금 배포 verify 가 실패했다" context. Otherwise follow diagnosis' read-only CLI surfaces: `axhub deploy status <deployment-id> --json`, 그 status 시간창으로 좁힌 `axhub deploy logs --app <앱> --since <시작> --until <종료> --json --limit 100`(앱 단위 로그), then `axhub --json deploy diagnose <앱>`. Do not call MCP deployment diagnosis tools.

## Error routing

For non-zero commands outside verify, classify with:

```bash
axhub plugin-support classify-exit "$EXIT" "$STDOUT"
```

or use `references/error-empathy-catalog.md`.

- exit 64 + `validation.deployment_in_progress`: explain another deployment is running, never retry create, and monitor it with the verify loop when an id is available.
- exit 9 + `subdomain_not_configured`: subdomain update is a separate destructive mutation. Preview the proposed 2..32 character subdomain and require approval before `axhub apps update`.
- exit 9/64/67 + GitHub connection required: do not create repo, first push, or `apps git connect` from deploy. Hand off to import.
- exit 4/65: auth expired; ask before login flow in interactive mode.
- exit 5/67: not found; offer did-you-mean from apps list without numeric ids.
- exit 6/68: rate limit; respect Retry-After.
- exit 1: transport; retry read paths at most once, never create.

## Secondary commands

Load `references/command-coverage.md` for read-only deployment browsing and cancel. Cancel remains a mutation: preview the in-progress deployment and require explicit approval before `axhub deploy cancel`.
