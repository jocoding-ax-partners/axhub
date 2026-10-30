# Deploy workflow detail reference

Load this only after the top-level `deploy` skill has passed the activation boundary and `## 배포 전 맞추기` left nothing to ask, or when a specific branch below is needed.

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

### 조직 정책 계약

5개 진입점(onboarding/deploy/up/bootstrap/import)의 공통 계약이에요. 첫 변경 전에 읽어요.
- `deploy-prep`의 top-level `developer_policy`는 매 준비(`--refresh-in-flight` 포함)마다 최신이어야 해요. `outcome:verified`에는 인증된 exact app의 `tenant_id`·`revision`·`state`·`deployment_markdown`이 필요해요. `outcome:not_applicable, reason:platform_app`은 인증된 platform 분류일 때만 허용하고 up은 deploy로 양보해요.
- 명시 `get`도 최신 `deployment_markdown`을 적용해요. `unset`·`withdrawn`은 정상 빈 안내예요. opt-out·이전 prep cache로 조회를 생략하지 않아요. 기존 `context_url`·서버 권한·심사·preview 승인·사용자 확인은 그대로예요. client 안내를 서버 강제로 표시하지 않아요.
- `outcome:blocked`의 `code`, 누락·malformed·구 CLI/server·조회 실패면 env·로컬 수정·commit·push·upload·앱 생성·execute 전에 멈춰요. 대상은 유지하고 앱 없음으로 바꾸지 않아요.
- 선택으로 대상이 바뀌면 `axhub apps get <app ID> --json`의 인증된 `tenant_id`로 `axhub tenants developer-policy get --tenant <tenant_id> --json`을 다시 실행해요. manifest·기억된 작업공간·목록 일치로 UUID를 추측하지 않아요.
- hook 실패 code별 terminal 복구·credential backend·retry 한도·수동 재전송·status 필드·전달 ACK 한계는 [POLICY.md 정책 동기화 복구](../../../POLICY.md#정책-동기화-복구)를 따라요. Codex는 manual-only이며 자동 hook 성공으로 해석하지 않아요. Desktop Code mode도 자동 동기화를 보장하지 않아요.

Quality gate failure stops by default. Only a non-null `bootstrap_plan` is the first-run boundary — the lookup finished and found no app (`resolve.error` `app_not_found` or `no_candidate_slug`): stop and hand off to import/bootstrap. A missing `app_id` without it is not first-run and not "app not found"; it is a login prompt only for a real auth failure (last auth bullet below). Decide it from these envelope fields before using `classify-exit` or the catalog; their plain exit 65/67 copy (login expired, name not found) fits only that real auth failure and a real not-found:

- `app_ambiguous`: several apps matched. Show the `matched_apps` slugs (never numeric ids), ask which app, and continue with that app. Never import/bootstrap.
- `apps_list_parse_error`: the app list could not be read — a slow or failed list call, not a missing app. Rerun `deploy-prep` once; if it repeats, stop with `앱 목록을 확인하지 못했어요. 잠시 뒤 다시 시도해 주세요.` No import, no did-you-mean.
- `preflight.auth_error_code` — or, when it is absent or `cli_runtime_error` (preflight's own `--version` check failed, which says nothing about the login), the `resolve.error` suffix after `auth_` — is `probe_timeout`, `probe_no_output`, or `cli_spawn_failed` (the auth probe did not answer), or `api`, `rate_limited`, `timeout`, or `other` (`auth status` answered without a login verdict: `/me` 429·5xx or no connection): login is unknown, not missing. Show no login or import copy and rerun `deploy-prep` once. If it repeats, `axhub auth status --json` decides: exit 0 is logged in — rerun `deploy-prep` once more and continue; exit 4 (`error.code` `auth`, not logged in or expired) goes to auth recovery; anything else, or the same probe failure on that last rerun, stops with `axhub 서버 응답이 늦어요. 잠시 뒤 다시 시도해 주세요.`
- Any other `auth_error_code` with `auth_ok=false` (or another `auth_<code>` in `resolve.error`, e.g. `auth_auth.token_missing`) is a real logout or expiry: auth recovery — ask before the login flow in interactive mode. `cli_runtime_error` with no `auth_` code in `resolve.error` is not one: only the `--version` check failed while resolve's own auth probe passed, so judge by the other fields.
- `--refresh-in-flight` exit 65 follows the same check. It never means "no in-flight deployment" or "login expired": keep the deployment id already bound and do not start a new deploy because of it.

Do not reject empty branch/commit until `apps get` selects the lane: selfhosted clones first, GitHub without a repo uploads, and only an unready GitHub repo path stops.

Read `GITHUB_CONNECTED` only after `git_backend.backend=github` is known. In that branch, false on a resolved app means the app has no repository and routes to the upload lane; true keeps the existing GitHub repo path. It is not a backend detector and must never send a selfhosted app to upload.

## Target reconciliation

Before mutation, reconcile stale manifest risk. If the conversation points at a different app, the utterance names a different app, or the manifest slug looks stale, confirm the target interactively. If the user chooses another app, update `axhub.yaml`, rerun `deploy-prep`, and include that manifest change in the git-readiness step. In headless mode, any target conflict downgrades to dry-run.

## App backend and static lane

Platform app first: when the `deploy-prep` envelope has `resolve.app_kind` `platform` (a missing `app_kind` means tenant — older CLI), `apps get` is tenant-only and answers not found. Read the platform record once instead:

```bash
axhub admin apps get "$APP_ID" --json
```

Use `.data.deploy_method` and `.data.staging_enabled` (bind `STAGING_ENABLED` from it); there is no `git_backend` or `access_url`, so keep `GIT_BACKEND=github`. A platform app takes only the GitHub lane — Git readiness, status-first, `deploy create --app "$APP_ID"` with the UUID (the CLI routes it to the admin surface), and verify — whatever `deploy_method` says. Never enter the upload lane (`up` skill), the static upload lane, or the self-hosted push branch. `GITHUB_CONNECTED=false` or a push blocked by GitHub has no upload fallback: stop with `이 플랫폼 앱은 연결된 GitHub 저장소로만 배포할 수 있어요. 저장소 연결을 확인해 주세요.` Skip later `apps get` reads (`access_url` included); show a URL only when verify returns one. Production changes only by admin promote, never `publish` (platform apps have no review): after a staging deploy verifies, offer `axhub admin apps promote "$APP_ID" "$DEPLOY_ID" --execute` and run it only when the user asks.

Tenant app (`app_kind` `tenant` or missing): after `deploy-prep` resolves an existing `APP_ID`, read the public app JSON once:

```bash
axhub apps get "$APP_ID" --no-input --json
```

Use top-level `deploy_method`, `git_backend.backend`, and `git_backend.source`; keep `git_backend.backend` as `GIT_BACKEND` for the push step. `selfhosted` — `deploy_method=static` included — enters the push branch below. Only `deploy_method=static` with `git_backend.backend=github` (source `legacy_github` included) enters the static upload lane (steps 1-4); non-static `github` and `legacy_github` preserve the existing GitHub/upload path. Never send a self-hosted static app to `apps static deploy`, never call C1 or Gitea directly, and never infer backend from a remote URL.

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

   Humanize file count/bytes and process as release create, upload, finalize, activate. A failed dry-run stops here. Redeploy does not ask (AP-12); dry-run only when the user asked for a rehearsal.

4. Execute after a successful dry-run:

   ```bash
   axhub apps static deploy --app "$APP_ID" --from-dir "$STATIC_DIR" --tenant "$AXHUB_TENANT" --execute
   ```

   Static success is `active_release_id` from activation plus, when available, `axhub apps get "$APP_ID" --no-input --field-expr '.access_url // empty'`. Never call `axhub deploy verify` in this lane.

   Exit 9 that refuses activation because this workspace reviews releases before use (`error.subcode` `staging_required`; this run has no `--json`, so recognize it by the hint naming `axhub apps static releases stage`): the release was already uploaded and finalized. Do not rerun the deploy — it would upload another release. Run the stage command the hint prints after `next:` (`axhub apps static releases stage --app <id> --release-id <id> … --execute`) once, exactly as printed; staging replaces the staged release and cancels a review still pending for the previous one. Report that the release is on staging and production is unchanged (AP-25), and run the hint's `axhub publish --app <id> --static-release-id <id> --execute` only when the user asks to submit it for review. A hint without `next:` comes from an older CLI: stop, say the release is waiting to be staged, and route to `update` instead of guessing the release id.

## Self-hosted push branch

For `git_backend.backend=selfhosted`, do not inspect `github_connected`. Decide the working folder first (CLI answers only, never a remote URL): a folder holding this app's local source is checked in place (`Local source folder` below); only an empty or unrelated folder runs `axhub repo clone <app> --json` and requires a successful envelope with a non-empty absolute `data.destination`. Use the CLI-resolved HTTPS remote and never synthesize the repository path or destination.

Clone results follow the envelope, not git's (translated) stderr:

- `already_cloned: true` (exit 0): the destination already holds this app's repository — success; continue with the returned `data.destination`.
- exit 9 `repository.credential_missing`: the git helper had no credential. Run the command after the hint's `next:` once, exactly as printed with its quotes — it pins the clone's `--profile`/`--endpoint` (and `--config`/`--no-keychain` when used) so the credential lands where the helper reads it; never assemble a setup command yourself — then the same clone once. If it repeats after that, stop and report it with `axhub feedback` (a setup that does not fix it is not an expected rejection). A hint without `next:` (for example an unreadable credential store) is not fixed by setup: repair what its reason names, then retry the clone.
- exit 9 `repository.destination_exists`: the folder exists and is not this app's clone. Show the absolute path from the hint and ask the user to pick another path or empty that folder; never delete or overwrite it yourself. Headless stops here.

Local source folder: when the current folder holds this app's local project (deploy-prep resolved the app from its `axhub.yaml`, or it has the app's source files), run `axhub repo clone <app> . --json` with the folder itself as the explicit destination. The CLI never overwrites a non-empty folder:

- `already_cloned: true`: this folder is the app's clone; `data.destination` is this folder. Continue here.
- exit 9 `repository.destination_exists`: the local project is not attached to this app's repository — no `origin`, or an `origin` that is not this app's repository (typical for a static app previously deployed by upload, whose repository holds only the platform seed commit). Do not clone into another folder: pushing an empty clone abandons the local source and redeploys the seed commit. Do not edit `origin` yourself either. Hand off to `import` for this existing app with `--app <app>` (static: also `--deploy-method static`). Its CLI self-hosted lane attaches `origin`, joins the seed commit, pushes the local source, and returns the webhook deployment id that import verifies. In headless, stop and report that the folder needs `import`.

Self-hosted static: `deploy_method=static` on `selfhosted` uses this branch and is deployment-record based, never `apps static deploy` or `active_release_id`. It additionally requires preflight `capabilities.import.selfhosted_static === true`; missing, malformed, or false → `axhub CLI를 최신 버전으로 업데이트해 주세요.` and stop — never fall back to the upload lane. The server builds from the pushed source: manifest `build.install`/`build.build`, else a lockfile-based install plus `npm run build` when `package.json` exists, then publishes `build.static_output_dir`. So before the commit and push, `## 배포 전 맞추기` makes the local `axhub.yaml` carry `build.deploy_method: static` and an explicit `build.static_output_dir` — the folder (relative to the app root) that holds `index.html` after the build: `.` when `index.html` sits at the project root, otherwise the real build output such as `dist`, `build`, or `out`. Without both the server does not take the static path. Commit that fix with the other fixes; build output folders need not be committed. Success is the webhook deployment id verified with `axhub deploy verify`.

Treat that exact `data.destination` as the working directory for every subsequent repository-local command in this branch. Set the tool `cwd` to it for status/savepoint, `git rev-parse`, branch/commit work, push, fetch, and containment checks; do not run those commands in the original un-cloned folder. An explicit clone destination, when supplied, is authoritative because the CLI returns the same absolute path.

Before the push, run `axhub --tenant <tenant> git setup --json` to repair the host-scoped helper without rotating a valid PAT. After savepoint and `## 배포 전 맞추기`, capture `git rev-parse HEAD` as the full `PUSHED_SHA`. Read `axhub deploy list --app <app-id> --all --json` using the same profile/tenant and save the IDs in top-level `items` whose `commit_sha` exactly equals `PUSHED_SHA`; this baseline correlates discovery to the push without relying on local/server clock alignment. A failed baseline read stops before mutation. Push `git -c credential.interactive=false push -u origin "HEAD:$BRANCH"` from the clone cwd, with no environment assignment; credential absence fails instead of prompting. Preserve the baseline and SHA across the one permitted credential-recovery retry.

After push succeeds, repeat that all-status, all-pages list within AP-16's 30-check/10-minute budget. Select only a record with exact `commit_sha == PUSHED_SHA` and an ID absent from the baseline. Never select unrelated commits, shortened SHAs, or historical records; never filter out terminal statuses. If multiple records qualify, choose the first one in list order (the list is newest `started_at` first; `deploy list --json` has no `created_at`), then pin its ID. Immediately run `axhub deploy verify <deployment-id> --app <app-id> --json`: success follows the success rules, terminal failure hands off to diagnosis, and non-terminal state uses the common bounded verify loop with the same ID. A failed list is not an empty successful result. No match by budget expiry means pending, never create/upload — but first run `axhub deploy check --app <app-id> --json` once: a failing `challenge_deploy_locked` item means the webhook skipped this push for a contest deploy lock set after the pre-push check. Report that instead of pending: the commit is saved on the remote and deploys about a minute after the contest operators reopen deploys (reconcile replays the default-branch tip); relay the item's message and stop. Do not use `deploy-prep --refresh-in-flight` for webhook discovery because it excludes completed deployments.

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
if [ "$GIT_BACKEND" = "selfhosted" ]; then
  # 자격증명이 없으면 콘솔·Git Credential Manager 창 대신 오류로 바로 실패해야 credential retry 분기가 돌아요.
  git -c credential.interactive=false push -u origin "HEAD:$BRANCH" >/dev/null 2>"$AXHUB_STDERR_TMP"
else
  git push -u origin "HEAD:$BRANCH" >/dev/null 2>"$AXHUB_STDERR_TMP"
fi
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

Interactive choices are monitor, force new, or cancel. Headless safe default is abort. `monitor` sends `DEPLOY_ID="$IN_FLIGHT_ID"` to the bounded verify loop and never creates a new deployment. `force_new` may proceed to preview, but a create refused because a deploy is in progress (exit 9 `already_exists`, older CLIs exit 64 `deployment_in_progress`) is not retried.

For GitHub-connected apps, use status-first before fallback create. If no in-flight deployment is visible yet, interactive mode may wait briefly and refresh with `deploy-prep --refresh-in-flight` (its exit 65 follows `## Deploy-prep envelope`); headless does not wait. If a status-first id appears, reuse the same in-flight branch rules and skip `deploy create`. Do not call `axhub deploy watch` or `axhub deploy status --watch`; Desktop/non-TTY watch can degrade or fail on required app flags. Use the bounded verify loop below.

## Preview and token gate

Headless detection:

```bash
AXHUB_HEADLESS=0
if ! [ -t 1 ] || [ -n "${CI:-}" ] || [ -n "${CLAUDE_NON_INTERACTIVE:-}" ]; then
  AXHUB_HEADLESS=1
fi
```

Headless redeploy runs for real when `## 배포 전 맞추기` has nothing to ask, and stops before any change when it does (AP-12, AP-29).

The progress line shows exactly app, environment, branch, commit, and ETA. Normalize displayed slug with NFKC and warn if normalization changes it. The visible environment label is `운영` when `STAGING_ENABLED` is false and `스테이징` when it is true (AP-25 — a staging opt-in app never builds straight into production); do not show `prod`, `production`, or raw profile names in the card unless the user explicitly asks for CLI-level evidence. Do not ask for approval. Dry-run natural language such as "리허설", "테스트로", or "진짜 안 올리고" also sets `DEPLOY_DECISION=dry_run`.

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
  axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --dry-run --field-expr '.data.id // .data.deployment_id // .id // .deployment_id // empty' >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
elif [ "${DEPLOY_DECISION:-approve}" = "abort" ]; then
  echo "배포를 멈춰요." >&2; rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
else
  axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --execute --field-expr '.data.id // .data.deployment_id // .id // .deployment_id // empty' >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
fi
AXHUB_EXIT=$?
```

The field-expr also reads the `--json`/`AXHUB_JSON=1` envelope (`.data.*`). On exit 9 with `already_exists` (deploy in progress; older CLIs: exit 64 `validation.deployment_in_progress`), do not retry: verify `in_flight_deploy.id` from one `axhub plugin-support deploy-prep --refresh-in-flight --json`, or say another deploy is running and stop. On exit 0, bind `DEPLOY_ID` from stdout. If no id exists, do not claim success; tell the user the start was seen but no result id was received.

## GitHub upload lane — deploy the local folder

This lane is for `git_backend.backend=github` only. The source is the local folder instead of a repo commit (backend spec 184). Everything after create is unchanged — `axhub up` prints the same create result, so `DEPLOY_ID` binding, the verify loop, and the diagnosis handoff stay identical.

Two entries, and they are not the same thing:

- **No repo (steady state).** `github_connected` is false in the `deploy-prep` envelope. The app has no repository — normal for an app first deployed from local source. For repo apps a push fires the webhook and deploys without this skill; an app with no repo receives no push webhook, so an explicit deploy through this lane is the only way it ever ships a change. This is not a failure: do not attempt push, do not report recovery, and do not apply the empty branch/commit stop, since no commit is sent. Skip Git readiness entirely and come straight here.
- **Repo blocked (recovery).** `github_connected` is true but the push or containment check failed because of GitHub itself: permission denied, 404 on a private repo, org policy, expired or refused device flow. Only this entry gets the recovery framing.

Two guards, recovery entry only:

- The cause must be GitHub. Network, timeout, and 5xx get one retry of the same step first. Falling through on a transient error silently drops the repo and push auto-deploy the user already had.
- The app must already exist and be resolved (`APP_ID` from `deploy-prep`). This lane never creates apps or repos — that stays `bootstrap` and `import`.

Say one line and continue, not a question. No repo: `이 앱은 저장소 없이 소스를 올려서 배포해요.` Repo blocked: `GitHub 쪽이 막혀서, 지금 폴더의 소스를 그대로 올려서 배포할게요.` Then hand the lane to the `up` skill.

The handoff happens before any deploy mutation. Invoke `up` through the host's skill-invocation tool when one exists, carrying `APP_ID` and the reason (no repo / repo blocked). When no such tool exists, follow `skills/up/SKILL.md` from its step 2 in place — preview, execute, and `axhub deploy verify`. Either way the deploy finishes inside this request; ending the response on the yield sentence alone leaves the user with nothing deployed. Do not restate the upload command here.

A dirty working tree is committed by `## 배포 전 맞추기` before this point. Uploading the folder as-is opens only when the user calls `up` directly.

## Verify loop and diagnosis

Deployment-record success is declared only by `axhub deploy verify`. Poll this command for in-flight/webhook deployments:

```bash
axhub deploy verify <deployment-id> --app <app-id>
```

preflight 의 `capabilities.import.verify_wait` 가 true 면 위 명령 대신 **권한 카드 한 번으로 끝나는** 단일 대기 호출을 정확히 한 번만 실행해요:

```bash
axhub deploy verify "$DEPLOY_ID" --app "$APP_ID" --wait --wait-interval 20s --wait-timeout 9m --json
```

호출 직전에 `아직 빌드 중이에요. 같은 배포를 계속 확인할게요.` 한 줄만 남기고, 명령 도구 timeout 을 600000ms 로 올려 실행해요(기본 2분에 끊기면 CLI 가 판정 exit 를 못 돌려줘요). `--wait` 가 성공·실패·예산 제한까지 책임지므로 같은 verify 를 반복 호출하거나 `axhub apps get`, `deploy list`, `deploy status` 같은 사후 확인을 덧붙이지 않아요. 대기 수단 없이 같은 verify 를 연달아 호출해 같은 exit 6 을 화면에 쌓는 연타 폴링은 UX 실패예요.

`verify_wait` capability 가 없는 구 CLI 에서만 `--wait` 없는 verify 를 폴링 예산(최대 30회 또는 10분, AP-16) 안에서 반복해요. Do not combine polling into one long `while`/`for` shell loop with `MAX_ATTEMPTS`, command substitution, or shell expansion. Also do not collapse polling into one long `while`/`for`/`until` shell loop with `sleep`, `grep`, `head`, pipes, or shell expansion. A Claude Desktop permission request for a long polling shell block is a failed watch UX. Do not substitute `axhub deploy watch` or `axhub deploy status --watch`. Do not end by asking the user to say `배포 상태 확인해줘`; the skill owns the follow-up while a known `DEPLOY_ID` is still running.

Exit handling:

- 0: terminal success; summarize in Korean with the verified live URL if available. Branch on the verify JSON `environment` first (AP-25): `staging` → 스테이징에만 반영됐고 운영에는 아직 반영되지 않았다고 말하고, 심사 신청 명령 `axhub publish --app "$APP_ID" --deployment-id "$DEPLOY_ID" --execute` 를 같은 블록에 안내해요 (승인은 Console Review, 승인 뒤 promote 자동, 신청은 사용자 요청 시에만 실행). platform app 은 심사가 없어서 `publish` 대신 `axhub admin apps promote "$APP_ID" "$DEPLOY_ID" --execute` 를 안내하고, 실행은 사용자 요청 시에만 해요. `environment` 가 없거나 null 이면 `STAGING_ENABLED` 로 판단하고 운영으로 간주하지 않아요.
- 6, 10(CLI timeout), or the command tool cutting the call at its timeout: non-terminal, not a failure. `--wait` 경로는 자동 재실행 없이 재개 요약과 같은 verify 명령 한 줄로 끝내며 `DEPLOY_ID` 를 보존해요. fallback 경로에서만 keep the same `DEPLOY_ID` and continue the bounded verify loop automatically. 어느 쪽도 사용자에게 다시 상태 확인을 요청하지 않아요.
- 7: terminal failure; say "배포가 실패했어요. 지금부터 원인 진단만 읽기 전용으로 확인할게요. 재배포나 롤백은 하지 않아요." Then hand off to `diagnosis`.
- 5: unknown deployment id; stop without latest lookup.
- 4: auth expired; use auth recovery.

For the failure handoff, preserve `DEPLOY_ID`, app slug/id/name, and classified verify state internally. Do not expose raw output. If a Skill tool exists, invoke `diagnosis` with the app identity and "방금 배포 verify 가 실패했다" context. Otherwise follow diagnosis' read-only CLI surfaces: `axhub deploy status <deployment-id> --json`, 그 status 시간창으로 좁힌 `axhub deploy logs --app <앱> --since <시작> --until <종료> --json --limit 100`(앱 단위 로그, status `target_environment` 가 `staging` 이면 `--environment staging`), then `axhub --json deploy diagnose <앱>`. Do not call MCP deployment diagnosis tools.

## Error routing

For non-zero commands outside verify, classify with:

```bash
axhub plugin-support classify-exit "$EXIT" "$STDOUT"
```

or use `references/error-empathy-catalog.md`.

- exit 9 + `already_exists` (older CLIs: exit 64 + `validation.deployment_in_progress`): explain another deployment is running, never retry create, and monitor the `in_flight_deploy.id` from `axhub plugin-support deploy-prep --refresh-in-flight --json` with the verify loop when an id is available.
- exit 9 + `subdomain_not_configured`, or a failing `deploy check` item with `code` `subdomain_not_configured` in `data.checks[]` (`handling: ask`; the envelope's `error.subcode` is then `deploy.subdomain_not_configured`): the app has no address, so a push starts no deployment and create stops here. Setting it is a separate mutation that cannot change after the first production deploy: preview the proposed 2..32 character address, require approval, run `axhub apps update <app> --subdomain <address> --json` (platform app: `axhub admin apps update <app> --subdomain <address> --execute`, as its hint prints), then retry once. Do not push before it is set. Headless never sets an address — stop and report what to set. Self-hosted retries by pushing, except for a commit that already reached the remote while the address was missing (`git fetch origin "$BRANCH"`, then `git merge-base --is-ancestor "$PUSHED_SHA" "origin/$BRANCH"` exits 0): never push it again. The webhook skipped that push, but the backend's self-hosted reconcile checks every minute whether the repository default branch tip has a deployment and replays the missed push once the address exists, so a deployment appears within about a minute of `apps update`. When `$BRANCH` is the default branch (`git ls-remote --symref origin HEAD` prints `ref: refs/heads/$BRANCH`), find it within AP-16's 30-check/10-minute budget with `axhub deploy list --app <app-id> --all --json`: select `commit_sha` equal to `git rev-parse "origin/$BRANCH"` after the fetch (usually `PUSHED_SHA`; reconcile deploys the tip), with no pre-push baseline exclusion — reconcile may create the row before any baseline read. Pin the newest such id and run `axhub deploy verify <deployment-id> --app <app-id> --json`: success follows the success rules, terminal failure hands off to diagnosis, and non-terminal state uses the common bounded verify loop with the same ID. Budget expiry is pending, never create/upload in this lane. When `$BRANCH` is not the default branch, nothing deploys it: a self-hosted app deploys only its repository default branch, so a push to `$BRANCH` is skipped regardless of the address — say the address is set and that deployments follow the default branch, then stop (never create/upload). If `git ls-remote` fails, do not treat that as a non-default branch: run the `deploy list` lookup above (the worst case is pending).
- exit 9/64/67 + GitHub connection required: do not create repo, first push, or `apps git connect` from deploy. Hand off to import.
- exit 9 + `staging_required` from `deploy create`: production changes only through review and this app's staging is off. (The upload lane never reruns its upload on this refusal — it reports the hint's next steps and stops, per the up skill.) Do not retry unchanged. Follow the hint's next steps, but turning staging on (`axhub deploy staging enable --app <app> --execute`) changes where every later deploy lands, so ask first — Headless stops and reports it. After approval, enable it and rerun the same deploy once; it lands on staging (AP-25). Request review with `axhub publish --app <app> --deployment-id <id> --execute` only when the user asks. A platform app has no review: its hint ends with `axhub admin apps promote <app> <deployment-id> --execute` instead of `publish`, under the same only-when-asked rule.
- exit 4/65: auth expired; ask before login flow in interactive mode — unless `auth_error_code` is `probe_timeout`/`probe_no_output`/`cli_spawn_failed`/`api`/`rate_limited`/`timeout`/`other` (`## Deploy-prep envelope`; classify-exit `4:auth.probe_failed`).
- exit 5/67: not found; offer did-you-mean from apps list without numeric ids — except `resolve.error` `apps_list_parse_error`, which is a rerun (`## Deploy-prep envelope`; classify-exit `5:apps.list_unavailable`).
- exit 6/68: rate limit; respect Retry-After.
- exit 1: transport; retry read paths at most once, never create.

## Secondary commands

Load `references/command-coverage.md` for read-only deployment browsing and cancel. Cancel remains a mutation: preview the in-progress deployment and require explicit approval before `axhub deploy cancel`.
