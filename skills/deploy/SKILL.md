---
name: deploy
description: '이미 axhub에 연결된 앱의 현재 브랜치/커밋을 다시 배포하거나 배포 상태를 이어서 확인할 때 사용해요. 트리거: "배포", "배포해", "배포해줘", "올려", "공개해", "띄워", "프로덕션", "deploy", "ship", "release", "rollout". 첫 연결/첫 배포는 import, 빈 폴더 새 앱 생성은 bootstrap, 명시적인 실패 원인 진단은 diagnosis 가 담당해요. 이 스킬은 preview-confirm gate, headless dry-run, deployment-record verify, static deploy 예외, terminal failure diagnosis handoff 를 맡아요. 이 트리거들은 axhub 맥락(대화의 axhub 언급·현재 폴더의 axhub 연결·직전 axhub 작업)이 있을 때만 유효해요. axhub 맥락 없는 일반 "배포해" 발화나 Vercel·AWS 같은 다른 배포 대상을 명시한 요청에는 이 스킬을 쓰지 않아요.'
examples:
  - utterance: "paydrop 배포해"
    intent: "deploy current branch to axhub live"
  - utterance: "ship paydrop"
    intent: "deploy current branch to axhub live"
  - utterance: "/axhub:deploy paydrop --commit abc123"
    intent: "deploy specific commit to axhub live"
allows-dependency-execution: false
model: sonnet
---

# Deploy via axhub

Deploy an already-connected axhub app with preview, approval, and verification safety. First-connect/import and new-app/bootstrap flows do not run here.

명시적인 배포 실패 원인 진단 요청(예: "배포 실패 원인 진단해줘", "왜 배포가 죽었어")은 `diagnosis` 에 양보해요. 이 스킬이 실제 배포를 시작한 뒤 `axhub deploy verify` 에서 terminal failure 를 확인한 경우에만 같은 앱 식별자와 실패 근거를 유지해 `diagnosis` 로 읽기 전용 handoff 해요. 이 handoff 는 재배포, 롤백, 새 deploy create 를 실행하지 않아요.

## First Visible Sentence

When the user says a human deployment phrase such as `배포해줘`, `올려줘`, or `프로덕션에 띄워줘`, the first visible chat sentence must be exactly:

`배포 준비를 확인할게요.`

Then run one Bash/tool call with Korean title `배포 준비 확인` from the user-visible app folder:

```bash
axhub plugin-support deploy-preview-summary --user-utterance "<latest user sentence>"
```

Copy the Korean stdout as the preview card and ask for explicit approval. If stdout says `axhub 매니페스트(axhub.yaml)가 없어요.`, do not create files here. axhub 맥락(사용자의 axhub 언급·직전 axhub 작업)이 있으면 기존대로 안내해요: non-empty existing app -> `기존 앱 올려` / `import`; empty directory new template -> `새 앱 만들어줘` / `bootstrap`. axhub 맥락이 없으면 import/bootstrap 으로 넘기지 말고 "이 폴더는 axhub에 연결돼 있지 않아요. axhub로 배포하려는 거예요?" 를 한 번만 묻고, 아니라는 답이면 이 스킬을 종료해요. headless 에서는 묻지 않고 조용히 멈춰요.

For the initial Desktop preview, stop reading after this section unless approval is received. After approval, continue with the canonical workflow below and load `references/workflow-details.md` for branch detail.

## Headless Contract

Headless means `claude -p`, CI, `$CLAUDE_NON_INTERACTIVE`, no TTY, or unavailable/denied AskUserQuestion.

- Headless = AskUserQuestion 0회. Do not call AskUserQuestion and do not render numbered choices then stop.
- Headless safe default is dry-run for deploy preview/create paths. Force `DEPLOY_DECISION=dry_run` and never run `--execute`.
- Headless may run non-mutating CLI/auth/dry-run checks so automated QA sees real behavior.
- If a branch would require a human choice, use the safe default recorded in `references/workflow-details.md`.

## User-Facing Language

Keep chat human and Korean. Do not echo raw ids, raw JSON, schema names, exit numbers, internal command names, or stderr unless `AXHUB_DEPLOY_VERBOSE=1`.

User-visible Bash/tool call titles must be Korean noun phrases only. Do not expose helper-shaped or English labels such as `manifesting`, `manifested`, `gitted`, `pushed`, `Push`, `resumed`, `bootstraped`, `deploy-prep`, `in-flight`, `dry-run`, `token-gate`, `execute`, `production`, `terminal success`, `grep pipe`, `gitignore`, `gitignoring`, `gitting`, `checking`, `Build passed`, `Working tree clean`, or `Not ignored`. Good examples: `배포 준비 확인`, `변경사항 확인`, `커밋 동기화 확인`, `원격 반영 확인`, `진행 중 배포 확인`, `배포 미리보기 확인`, `인증 상태 확인`, `배포 실행`, `배포 결과 확인`.

The same rule applies to chat prose, preview cards, and final tables. Use `운영` for the user-facing environment, `진행 중 배포` for in-flight work, `미리보기` for dry-run, `인증 상태 확인` for token gate, `배포 실행` for execute, and `검증 성공` for terminal success. Command names may appear only when the user explicitly asks for technical evidence or when an error needs exact copy-paste recovery.

When a technical check fails or needs recovery, translate it before showing it. Say `원격 반영이 필요해요`, not `commit_not_found`; `재생성되는 빌드 파일은 정리했어요`, not `Not ignored` or `Working tree clean`; `원격 저장소 확인`, not `gitting` or `gitignore 확인`. Do not create English status snippets during build/lint/git cleanup.

Do not narrate approval state in English. Never write `User explicitly authorized`, `Proceeding`, `Push 성공`, or `Push failed`. Use `사용자가 배포와 원격 반영을 요청했으니 계속 진행해요`, `원격 반영 성공`, or `원격 반영 실패` instead.

When shell output is noisy, capture it in temp files or summarize after the command. Do not pipe important axhub commands through `grep`, `head`, or similar filters in a way that can change the command exit code or hide a long-running helper. If cosmetic filtering is truly needed, preserve and inspect the original command exit code first.

## Tool Authority

This skill is **CLI-only**. All deploy preview/create/verify/status/diagnosis routing must go through `axhub` CLI commands described below. Ignore MCP deployment mutation tools even when they are visible in the session.

- Do not call MCP tools such as `deployment_trigger` for deploy create.
- Do not route deploy execution through advisor/server advisor/subagent helpers.
- Do not escalate to another model/context to decide deployment. The CLI envelopes are the source of truth.
- If MCP deployment tools are present but denied, that is not a blocker for this skill; continue with the CLI path or the headless dry-run contract.

Progress lines:

- `[1/5] axhub 점검하는 중이에요`
- `[2/5] 배포 대상 확인하는 중이에요`
- `[3/5] 미리보기 보여줄게요`
- `[4/5] 배포하는 중이에요`
- `[5/5] 배포 결과 확인하는 중이에요`

Final user message is a Korean one-line summary plus next action. Prefer natural phrases such as `다시 로그인해줘`, `기존 앱 올려`, or `새 앱 만들어줘`; do not tell a Desktop user to run deploy CLI commands. If a deployment is already running and a `DEPLOY_ID` is known, do not ask the user to request another status check; keep watching automatically with the verify loop or an actual scheduled follow-up.

## Reference Loading

Load only what the current branch needs:

- `references/workflow-details.md`: post-preview canonical workflow detail, route-decision, tenant picker, deploy-prep envelope, static lane, git readiness, in-flight/status-first handling, deploy create, verify, and recovery branches.
- `references/error-empathy-catalog.md`: exit-code Korean copy, deploy preview card wording, NFKC rendering rules, and 4-part empathy templates.
- `references/session-carryover.md`: same-conversation carry-over evidence and confabulation guard.
- `references/command-coverage.md`: secondary `deploy list` and `deploy cancel` coverage.

## Canonical Workflow Summary

Actual execution order:

`CLI guard` -> `version check` -> `route-decision` -> `tenant resolve` -> `deploy-prep` -> `static branch or deployment-record branch` -> `first-run boundary` -> `git readiness` -> `in-flight/status-first` -> `headless decision` -> `preview card` -> `token-gate` -> `deploy create` -> `verify` -> `diagnosis/error recovery`.

### CLI guard

Use CLI capability, not version string comparison:

```bash
if ! command -v axhub >/dev/null 2>&1; then
  echo "axhub CLI가 아직 없네요. 온보딩부터 진행할게요." >&2
  exit 0
fi
PREFLIGHT_JSON=$(axhub plugin-support preflight --json 2>/dev/null)
PREFLIGHT_EXIT=$?
if [ "$PREFLIGHT_EXIT" = "2" ] || [ -z "$PREFLIGHT_JSON" ]; then
  echo "axhub CLI가 오래됐어요. `axhub update apply`로 업데이트한 뒤 다시 시도해 주세요." >&2
  exit 0
fi
echo "$PREFLIGHT_JSON"
```

If auth is missing/expired, explain in Korean and ask before starting login flow in interactive mode.

### Routing and resolve

If the user explicitly names another deployment target, stop axhub deploy before `deploy-prep`. Otherwise route with the CLI context gate:

```bash
EXPLICIT_FLAG=""
[ "${EXPLICIT:-0}" = "1" ] && EXPLICIT_FLAG="--explicit"
ROUTE_DECISION=$(axhub plugin-support route-decision --user-utterance "$ARGS" $EXPLICIT_FLAG --field-expr '.decision // "axhub"' 2>/dev/null || echo axhub)
```

Only `axhub` continues to `deploy-prep`. Session carry-over evidence is route gate 통과 후에만 적용해서 다른 타깃으로 배포 의도를 훔치지 않아요. For `ignore`, interactive mode asks whether to deploy to axhub; headless stops safely.

Resolve live deployment inputs with:

```bash
DEPLOY_PREP_JSON=$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json)
```

The `deploy-prep` envelope is authoritative for `profile`, `endpoint`, `app_id`, `app_slug`, `branch`, `commit_sha`, `commit_message`, `eta_sec`, preflight, `bootstrap_plan`, in-flight deploy, GitHub connection, and quality gate. Never infer `app_id` from pwd or git remote alone in the mutation path.

If this skill was invoked as a handoff from another axhub skill after code changes, do **not** reuse the original feature prompt as `--user-utterance`; it may contain display text such as "QA banner" that looks like an app candidate. Use a short deploy utterance like `현재 앱 배포해` and rely on the current folder's axhub.yaml binding. If the current folder has a valid axhub.yaml and the latest deploy phrase does not explicitly name another app, the bound app slug wins over arbitrary words in prior chat.

If `bootstrap_plan` is present, `app_id` is missing, or branch/commit is empty, stop before preview. Existing non-empty app first-connect belongs to `import`; empty new app creation belongs to `bootstrap`.

### Static branch

After resolving an existing app, detect static hosting:

```bash
DEPLOY_METHOD=$(axhub apps get "$APP_ID" --no-input --field-expr '.deploy_method // empty' 2>/dev/null || true)
```

Only `DEPLOY_METHOD=static` enters static lane. Static lane uses `apps static deploy --execute` after its own dry-run preview and approval:

```bash
axhub apps static deploy --app "$APP_ID" --from-dir "$STATIC_DIR" --tenant "$AXHUB_TENANT" --dry-run
axhub apps static deploy --app "$APP_ID" --from-dir "$STATIC_DIR" --tenant "$AXHUB_TENANT" --execute
```

Static success is `active_release_id` from activate plus public URL when available. Do not call `axhub deploy verify` in static lane.

### Deployment-record branch

Deployment-record apps continue through git readiness, in-flight/status-first handling, preview, token gate, fallback create, and verify. Load `references/workflow-details.md` for the branch mechanics.

Preview card is interactive only and must show app, environment, branch, commit, and ETA. Display the environment as `운영`, not `prod`, `production`, or raw profile values, unless the user explicitly asked for exact CLI fields. Use `references/error-empathy-catalog.md` for the deploy-preview card and NFKC warning. Slash invocation does not skip this card.

Before showing the preview, make sure the commit is actually reachable from the remote branch that axhub will build. Normalize any short commit to the full local SHA (`git rev-parse "$COMMIT_SHA^{commit}"` or `git rev-parse HEAD`) and use the full SHA for remote containment and `axhub deploy create`. If the branch has an existing `origin` remote/upstream and local commits are ahead, push with `git push -u origin "HEAD:$BRANCH"` first, refresh `origin/<branch>`, and confirm `git merge-base --is-ancestor "$COMMIT_SHA" "origin/$BRANCH"` (or an equivalent remote containment check) before preview/create. Judge push success by exit code, not by stderr text such as harmless hook warnings. Never deploy a local-only commit SHA; if push or remote containment fails, stop before mutation and say the remote commit is not ready yet.

Before execute, run:

```bash
AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate
```

Claude Desktop often hides stdout/stderr for long-running tool calls until completion. The deploy skill therefore uses the fast inline token check above for Desktop smoothness and never wraps token-gate in `grep`, `head`, or a multi-command pipe. Present this step as `인증 상태 확인`, not as token-gate. Exit 0 continues. Exit 65 routes to auth recovery. `AXHUB_AUTH_BG_REFRESH=0` disables the gate.

On approval, run fallback create only when status-first found no in-flight deployment:

```bash
axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --execute --field-expr '.data.id // .data.deployment_id // .id // .deployment_id // empty'
```

Dry-run path uses the same target fields with `--dry-run` and skips verify:

```bash
axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --dry-run --field-expr '.data.id // .data.deployment_id // .id // .deployment_id // empty'
```

Bind `DEPLOY_ID` only from an in-flight deployment id or public `axhub deploy create --execute --json` / field-expr output. If no deployment id is present, do not declare success; say "배포 시작은 확인했지만 결과 확인 id 를 못 받았어요. 자동으로 지켜볼 id 가 없어 여기서 멈출게요." and stop.

### Verify loop

Deployment-record success is declared only by `axhub deploy verify` with the bound id:

```bash
echo "배포 결과를 확인하고 있어요." >&2
VERIFY_OUT=$(mktemp)
axhub deploy verify "$DEPLOY_ID" > "$VERIFY_OUT" 2>&1
VERIFY_EXIT=$?
```

Do not use latest lookup. Current CLI resolves the exact deployment id to its app scope when needed, so the happy path is `axhub deploy verify "$DEPLOY_ID"` without `--app`. Do not call `axhub deploy watch` or `axhub deploy status --watch` from this skill; Desktop/non-TTY watch paths can degrade or require extra flags. If verify exits `6`, say `아직 빌드 중이에요. 같은 배포를 계속 확인할게요.` and retry the same verify command until terminal success/failure or a bounded timeout. Prefer separate short tool calls or a real ScheduleWakeup when available; do not end the response by asking the user to say `배포 상태 확인해줘`. If the bounded timeout is reached while the deploy is still running, schedule a follow-up check when the host supports it; otherwise say `아직 진행 중이에요. 여기서 실패로 보지 않고, 제가 확인 가능한 범위까지는 같은 배포를 지켜봤어요.` and keep the `DEPLOY_ID` visible enough for a future status request. Do not claim success from deploy-create stdout, status snapshots, watch output, or prose polling; verify 전에는 성공을 선언하지 않아요. If verify returns `url_checked=false`, read `access_url` with `axhub apps get "$APP_ID" --field-expr '.access_url // .data.access_url // empty'` and do a bounded HTTPS HEAD retry before saying the app is openable.

Verify exits:

- `0`: terminal success. Summarize in Korean with verified URL if available.
- `6`: still running. Keep the same `DEPLOY_ID` and continue the bounded verify loop automatically; do not ask the user to request another status check.
- `7`: terminal failure. Say "배포가 실패했어요. 지금부터 원인 진단만 읽기 전용으로 확인할게요. 재배포나 롤백은 하지 않아요." Then hand off to `diagnosis`.
- `5`: unknown deployment id. Stop; do not search latest.
- `4`: auth expired. Use auth recovery copy.

### Deploy failure → diagnosis handoff

For verify exit 7 only, preserve internal `DEPLOY_ID`, app slug/id/name, and classified verify state. Do not expose raw output. If a Skill tool exists, invoke `diagnosis` with app identity and "방금 배포 verify 가 실패했다" context. Otherwise follow diagnosis read-only CLI surfaces: `axhub deploy status <deployment-id> --json`, `axhub deploy logs <deployment-id> --app <앱> --json --limit 100`, then `axhub --json deploy diagnose <앱>`. Do not call MCP deployment diagnosis tools.

## Recovery Summary

Use `axhub plugin-support classify-exit "$EXIT" "$STDOUT"` or `references/error-empathy-catalog.md`.

- exit 64 + `validation.deployment_in_progress`: never retry `axhub deploy create`; monitor the in-flight deploy with the verify loop when an id is available.
- subdomain precondition: `axhub apps update <slug> --subdomain <subdomain> --json` is a separate destructive mutation and needs its own preview/approval before one retry.
- GitHub connection required: do not create repo, first push, or `apps git connect` from deploy; hand off to `import`. This does not prohibit pushing normal ahead commits to an already connected `origin` branch before deploy.
- auth expired: ask before login flow in interactive mode.
- not found/ambiguous: show slug candidates only, no numeric ids.
- rate limit: respect Retry-After.
- transport/read paths: retry at most once; never retry create.

## NEVER

- NEVER let deploy create or initialize first-run app/import state. Missing app/manifest first-connect belongs to `import` or `bootstrap`.
- NEVER run `axhub init`, `axhub apps create`, first GitHub repo creation, first push, or `apps git connect` from deploy. Pushing normal ahead commits to an already connected `origin` branch is allowed and required before deployment-record create.
- NEVER retry `axhub deploy create` on exit 64.
- NEVER drop JSON/field-expr parsing contracts where a command result is parsed.
- NEVER call `axhub deploy create --execute` without the interactive AskUserQuestion preview decision. Headless is exempt only because it must stay dry-run and must not use `--execute`.
- NEVER call `axhub deploy watch` or `axhub deploy status --watch` from this skill. In-flight and webhook-triggered deployments use the bounded `axhub deploy verify "$DEPLOY_ID"` loop.
- NEVER declare deploy success from deploy-create stdout, status snapshots, watch output, or prose polling. Deployment-record success declaration is terminal `axhub deploy verify <deployment-id>`.
- NEVER call `axhub deploy verify` without a deployment id. Latest 재탐색 금지.
- NEVER call `axhub deploy verify` in static lane (`deploy_method=static`). Static is release-based, not deployment-record-based, and success is `apps static deploy --execute` activate with `active_release_id`.
- NEVER send non-static apps to static lane. Empty or unsupported `deploy_method` uses the normal deployment-record pipeline.
- NEVER call `apps static deploy --execute` without static dry-run preview plus interactive approval. Headless static lane is dry-run only.
- NEVER change command semantics after approval by omitting `--execute`, changing the resolved deploy target, changing `--commit`, or changing the resolved tenant/profile. Surface the typed reason in one jargon-free line and stop, or use status-first verify-loop monitoring when appropriate.
- NEVER instruct the user to run `axhub deploy create`, `axhub deploy verify`, `apps static deploy --execute`, or any deploy CLI command themselves. The agent runs deploy and verify in this skill flow.
- NEVER run `deploy create` when status-first already found an in-flight deploy for this app; route to the bounded verify loop instead.
- NEVER call `axhub deploy cancel` without explicit confirmation.
- NEVER infer mutation target from pwd, git remote, cached app id, or old manifest alone; live resolve through `deploy-prep`.
- NEVER bypass the AskUserQuestion preview card on slash invocation. Slash confirms skill invocation, not the destructive operation.
- NEVER insert the old approved-run helper bridge between preview approval and the canonical workflow; approval flows into `deploy-prep`, public `axhub deploy create --execute`, and verify.
- NEVER call MCP deployment mutation tools such as `deployment_trigger`; deploy is CLI-only.
- NEVER use advisor/server advisor/subagent/model escalation to choose or execute deploy; use CLI envelopes only.
- NEVER commit, push, or add `.omc/`, `.claude/`, `.codex/`, `.serena/`, or other local agent/runtime state as part of deploy cleanup. Ignore those paths when deciding whether app code is clean enough to deploy; if they are the only dirty entries, proceed with the tracked app commit and mention local cleanup only after deployment.
- NEVER call `axhub deploy create --execute` for a commit that is only local. AxHub resolves commits from the connected GitHub repo; local-only commits fail in prod with commit-not-found.
- NEVER pipe `axhub plugin-support token-gate`, `axhub deploy create`, or `axhub deploy verify` through `grep`, `head`, or filters that can make a successful command look failed or make a waiting command look hung.
