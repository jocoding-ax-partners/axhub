---
name: deploy
description: "연결된 앱의 현재 코드를 실제 AxHub에 배포하고 성공 여부까지 확인할 때 반드시 사용해요. 트리거: \"배포해\", \"<앱이름> 배포해\", \"ship <앱이름>\", \"실제 AxHub에 배포\", \"성공 여부까지 확인\", \"같은 코드로 강제 재배포\", \"deploy\". preview-confirm과 exact deployment verify를 맡고 apps status/curl로 대신하지 않아요. 첫 연결은 import, 빈 폴더는 bootstrap, 실패 원인 진단은 diagnosis예요. GitHub 없이 지금 폴더의 소스를 그대로 올리는 배포(\"GitHub 없이 배포해\", \"이 폴더 그대로 올려줘\", \"소스 올려서 배포\")는 up 으로 양보해요. axhub 맥락 없거나 다른 배포 대상이면 쓰지 않아요."
allows-dependency-execution: false
model: sonnet
---
> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.


# Deploy via axhub

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 `bin_path` 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

Deploy an already-connected axhub app with preview, approval, and verification safety. First-connect/import and new-app/bootstrap flows do not run here.

명시적인 배포 실패 원인 진단 요청(예: "배포 실패 원인 진단해줘", "왜 배포가 죽었어")은 `diagnosis` 에 양보해요. 이 스킬이 실제 배포를 시작한 뒤 `axhub deploy verify` 에서 terminal failure 를 확인한 경우에만 같은 앱 식별자와 실패 근거를 유지해 `diagnosis` 로 읽기 전용 handoff 해요. 이 handoff 는 재배포, 롤백, 새 deploy create 를 실행하지 않아요.

**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.

**장기 대기.** codex 는 긴 명령을 최대 30초에 yield 하고 백그라운드 터미널로 넘겨요 — yield 는 실패도 완료도 아니에요. `deploy verify --wait` 가 yield 되면 같은 명령을 다시 실행하지 말고 같은 터미널을 빈 입력으로 폴링해 완주를 기다려요. 성공 선언 규칙은 그대로예요.

## 승인 게이트 계약 (요약)

codex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 승인 방식은 명시 텍스트 승인 1회예요 — preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

## First Visible Sentence

When the user says a human deployment phrase such as `배포해줘`, `올려줘`, or `프로덕션에 띄워줘`, the first visible chat sentence must be exactly:

`배포 준비를 확인할게요.`

Then run one Bash/tool call with Korean title `배포 준비 확인` from the user-visible app folder:

```bash
axhub plugin-support deploy-preview-summary --user-utterance "<latest user sentence>"
```

이 첫 명령 전에는 설치·플러그인·앱·git·curl probe를 하지 않아요. 명시적 최신성 요청만 update 뒤 이 스킬로 돌아와요.

정상 preview 면 axhub 프로젝트 확정이에요. Interactive 는 별도 진입 질문 없이 **preview card 하나가 axhub 진입 확인을 겸해요** (AP-12 통합 게이트): Korean stdout 을 preview card 로 보여주고 `axhub로 지금 배포를 진행할까요?` 질문과 기존 `진행`/`취소` 승인을 한 번만 받아요. `취소` 면 종료. (headless 는 명시 텍스트 승인 생략, dry-run) 명시 텍스트 승인 1회만 받아요. 승인 채널 없는 headless 에서는 execute하지 않아요 — 승인을 조용히 건너뛰지 않아요. If stdout says `axhub 매니페스트(axhub.yaml)가 없어요.`, do not create files here. axhub 맥락(사용자의 axhub 언급·직전 axhub 작업)이 있으면 기존대로 안내해요: non-empty existing app -> `기존 앱 올려` / `import`; empty directory new template -> `새 앱 만들어줘` / `bootstrap`. axhub 맥락이 없으면 import/bootstrap 으로 넘기지 말고 "이 폴더는 axhub에 연결돼 있지 않아요. axhub로 배포하려는 거예요?" 를 한 번만 묻고, 아니라는 답이면 이 스킬을 종료해요. headless 에서는 묻지 않고 조용히 멈춰요.

For the initial Desktop preview, stop reading after this section unless approval is received. After approval, continue with the canonical workflow below and load `references/workflow-details.md` for branch detail.

## Headless Contract

Headless means `codex exec`, CI, no TTY, or no user-confirmation channel at all — neither native choice UI nor explicit text approval is available.

- Headless = 명시 텍스트 승인 0회. Do not call 명시 텍스트 승인 and do not render numbered choices then stop.
- Headless safe default is dry-run for deploy preview/create paths. Force `DEPLOY_DECISION=dry_run` and never run `--execute`.
- Headless may run non-mutating CLI/auth/dry-run checks so automated QA sees real behavior.
- If a branch would require a human choice, use the safe default recorded in `references/workflow-details.md`.

## User-Facing Language

사용자-facing 문구·tool 제목을 만들기 전에 [references/user-facing-language.md](references/user-facing-language.md) 를 읽고 그대로 따라요. 핵심: 한국어 명사구 제목만(`배포 준비 확인`, `배포 실행`, `배포 결과 확인` 류), raw id·exit 번호·영어 진행어 금지, URL 은 평문 절대 URL 만, 기술 실패는 한국어로 번역해 보여줘요.

## Tool Authority

This skill is **CLI-only**. All deploy preview/create/verify/status/diagnosis routing must go through `axhub` CLI commands described below. Ignore MCP deployment mutation tools even when they are visible in the session.

- Do not call MCP tools such as `deployment_trigger` for deploy create.
- Do not route deploy execution through advisor/server advisor/subagent helpers.
- Do not escalate to another model/context to decide deployment. The CLI envelopes are the source of truth.
- If MCP deployment tools are present but denied, that is not a blocker for this skill; continue with the CLI path or the headless dry-run contract.

**Codex 단일 명령 계약.** 카드마다 한 bare 명령만 써요. 변수·분기·subshell·shell 연산·pipe·redirect·출력 가공 없이 이전 결과의 ID를 literal 인자로 넘겨요.

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
axhub plugin-support preflight --json
```

이 명령의 tool 결과에서 command-not-found, exit, JSON을 직접 읽어요. command-not-found 는 미설치가 아니에요 — AP-17 대로 `"$HOME/.axhub/bin/axhub"` 로 `plugin-support repair-path --json` 을 실행해 그 절대경로로 이어가고, 디스크에도 없을 때만 온보딩을 안내해요. 별도 설치 probe나 shell 분기를 만들지 않아요. If auth is missing/expired, explain in Korean and ask before starting login flow in interactive mode.

### Routing and resolve

If the user explicitly names another deployment target, stop axhub deploy before `deploy-prep`. Otherwise route with the CLI context gate:

```bash
axhub plugin-support route-decision --user-utterance "<latest user sentence>" --field-expr '.decision // "axhub"'
```

Only `axhub` continues to `deploy-prep`. Session carry-over evidence is route gate 통과 후에만 적용해서 다른 타깃으로 배포 의도를 훔치지 않아요. For `ignore`, interactive mode asks whether to deploy to axhub; headless stops safely.

Resolve live deployment inputs with:

```bash
axhub plugin-support deploy-prep --intent deploy --user-utterance "<latest user sentence>" --json
```

The `deploy-prep` envelope is authoritative for `profile`, `endpoint`, `app_id`, `app_slug`, `branch`, `commit_sha`, `commit_message`, `eta_sec`, preflight, `bootstrap_plan`, in-flight deploy, GitHub connection, and quality gate. Never infer `app_id` from pwd or git remote alone in the mutation path.

If this skill was invoked as a handoff from another axhub skill after code changes, do **not** reuse the original feature prompt as `--user-utterance`; it may contain display text such as "QA banner" that looks like an app candidate. Use a short deploy utterance like `현재 앱 배포해` and rely on the current folder's axhub.yaml binding. If the current folder has a valid axhub.yaml and the latest deploy phrase does not explicitly name another app, the bound app slug wins over arbitrary words in prior chat.

If `bootstrap_plan` is present, `app_id` is missing, or branch/commit is empty, stop before preview. Existing non-empty app first-connect belongs to `import`; empty new app creation belongs to `bootstrap`. 단 `github_connected` 가 false 면 저장소 없는 앱의 정상 상태라 이 정지가 안 걸려요 — 아래 Upload lane 으로 가요.

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

### Upload lane

`github_connected` 가 false 면 저장소가 없는 앱이에요 — 실패가 아니라 정상 상태이고, 받을 push 웹훅이 없어서 명시적 배포가 유일한 출고 경로예요. Git readiness·push·containment·빈 커밋 정지를 건너뛰고 여기로 와요. 저장소가 있는데 GitHub 때문에 push 나 containment 가 막힌 경우도 같은 명령을 쓰되(네트워크·타임아웃·5xx 는 한 번 재시도 먼저) 그때만 복구로 알려요. 이 절차는 본문만으로 완결돼요.

이 lane 의 절차는 `up` 스킬이 소유해요. `deploy-prep` 에서 이 lane 임이 드러나는 시점에는 아직 아무것도 바뀌지 않아서 인계가 안전해요. 첫 카드에서 받은 승인은 **저장소 배포 기준이라 이 lane 의 승인이 아니에요** — 올라갈 내용(파일 수·크기·소스 버전)이 그 카드에 없었으니, `up` 이 실제 업로드 내용으로 카드를 한 번 더 보여주고 승인을 받아요. 사용자에게는 배포 방식이 바뀐 이유를 한 줄로 알려요. Skill 호출 도구가 있으면 `up` 을 호출하고 `APP_ID` 와 사유를 넘겨요. 그런 도구가 없으면 `skills/up/SKILL.md` 의 2단계부터 그대로 수행해요 — 미리보기, 승인, 실행, `axhub deploy verify` 성공 확인까지요. 어느 쪽이든 **배포는 반드시 이 요청 안에서 끝나요.** 양보한다는 문장만 남기고 응답을 끝내면 사용자는 아무것도 배포되지 않은 채로 남아요. 이 스킬의 preview 카드를 승인으로 승계시키지 않고, 업로드 명령을 이 본문에 다시 적지도 않아요 — 절차 원본은 `up` 한 곳이에요.

작업 트리가 dirty 하면 이 스킬은 첫 명령(`deploy-preview-summary`)에서 이미 멈춰 여기까지 오지 못해요. 그 경로는 사용자가 `up` 을 직접 부르는 것으로만 열려요.

Preview card is interactive only and must show app, environment, branch, commit, and ETA. Display the environment as `운영`, not `prod`, `production`, or raw profile values, unless the user explicitly asked for exact CLI fields. Use `references/error-empathy-catalog.md` for the deploy-preview card and NFKC warning. Slash invocation does not skip this card.

Before showing the preview, make sure the commit is actually reachable from the remote branch that axhub will build. Normalize any short commit to the full local SHA (`git rev-parse "$COMMIT_SHA^{commit}"` or `git rev-parse HEAD`) and use the full SHA for remote containment and `axhub deploy create`. If the branch has an existing `origin` remote/upstream and local commits are ahead, push with `git push -u origin "HEAD:$BRANCH"` first, refresh `origin/<branch>`, and confirm `git merge-base --is-ancestor "$COMMIT_SHA" "origin/$BRANCH"` (or an equivalent remote containment check) before preview/create. Judge push success by exit code, not by stderr text such as harmless hook warnings. Never deploy a local-only commit SHA; if push or remote containment fails, stop before mutation and say the remote commit is not ready yet.

Runtime-only dirty entries are a hard exception. `.omc/`, `.claude/`, `.codex/`, `.serena/`, `.omx/`, `.omo/` and similar local agent state are not app code and are not deploy-affecting. If `git status`, `deploy-prep`, or a target check reports only those paths, treat the app commit as clean enough for deploy. Do not add those paths to `.gitignore`, do not create a cleanup commit for them, and do not ask the user to approve that cleanup. If a CLI check still blocks solely because of these runtime paths, stop with a concise CLI-gap note instead of mutating the user's repository.

Before execute, run:

```bash
AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate
```

Codex often hides stdout/stderr for long-running tool calls until completion. The deploy skill therefore uses the fast inline token check above for Desktop smoothness and never wraps token-gate in `grep`, `head`, or a multi-command pipe. Present this step as `인증 상태 확인`, not as token-gate. Exit 0 continues. Exit 65 routes to auth recovery. `AXHUB_AUTH_BG_REFRESH=0` disables the gate.

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
axhub deploy verify <deployment-id> --app <app-id>
```

Do not use latest lookup. Always pass the app scope from the same resolved target: `axhub deploy verify "$DEPLOY_ID" --app "$APP_ID"`. If app scope is missing, stop instead of running a bare verify. Do not call `axhub deploy watch` or `axhub deploy status --watch` from this skill; Desktop/non-TTY watch paths can degrade or require extra flags.

preflight 의 `capabilities.import.verify_wait` 가 true 면 **권한 카드 한 번으로 끝나는** 단일 대기 호출을 정확히 한 번만 실행해요:

```bash
axhub deploy verify "$DEPLOY_ID" --app "$APP_ID" --wait --wait-interval 20s --wait-timeout 10m --json
```

호출 직전에 `아직 빌드 중이에요. 같은 배포를 계속 확인할게요.` 한 줄만 남겨요. 이 한 호출의 내부 폴링 예산은 최대 30회 또는 10분(AP-16)이고, `--wait` 가 성공·실패·예산 제한까지 책임지므로 같은 verify 를 반복 호출하거나 `axhub apps get`, `deploy list`, `deploy status` 같은 사후 확인을 덧붙이지 않아요. 대기 수단 없이 같은 verify 를 연달아 호출해 같은 exit 6 을 화면에 쌓는 연타 폴링은 UX 실패예요 — 사용자에게는 실패한 명령이 반복되는 것처럼 보여요.

`verify_wait` capability 가 없는 구 CLI 에서만 `--wait` 없는 `axhub deploy verify "$DEPLOY_ID" --app "$APP_ID"` 를 폴링 예산(최대 30회 또는 10분, AP-16) 안에서 반복해요. Prefer separate short tool calls. 이 fallback 에서도 do not collapse polling into one long `while`/`for`/`until` shell loop with `sleep`, `grep`, `head`, `MAX_ATTEMPTS`, command substitution, pipes, or shell expansion. A Codex permission request for a long polling shell block is a failed watch UX; replace it with standalone `axhub deploy verify "$DEPLOY_ID" --app "$APP_ID"` calls.

Do not end the response by asking the user to say `배포 상태 확인해줘`. If the bounded budget is reached while the deploy is still running, schedule a follow-up check when the host supports it; otherwise say `아직 진행 중이에요. 여기서 실패로 보지 않고, 제가 확인 가능한 범위까지는 같은 배포를 지켜봤어요.` and keep the `DEPLOY_ID` visible enough for a future status request. Do not claim success from deploy-create stdout, status snapshots, watch output, or prose polling; verify 전에는 성공을 선언하지 않아요. If verify returns `url_checked=false`, read `access_url` with `axhub apps get "$APP_ID" --field-expr '.access_url // .data.access_url // empty'` and do a bounded HTTPS HEAD retry before saying the app is openable.

Verify exits:

- `0`: terminal success. Summarize in Korean with verified URL if available.
- `6`: still running. `--wait` 경로에서는 10분 예산을 다 쓰고도 끝나지 않았다는 뜻이에요 — 같은 verify 를 자동 재실행하거나 새 승인 카드를 띄우지 않고, 실패 선언 없이 재개 요약으로 응답을 끝내며 `DEPLOY_ID` 를 보존해요. fallback 경로에서는 같은 `DEPLOY_ID` 로 폴링 예산 안에서 계속 확인하고, 사용자에게 다시 상태 확인을 요청하지 않아요.
- `7`: terminal failure. Say "배포가 실패했어요. 지금부터 원인 진단만 읽기 전용으로 확인할게요. 재배포나 롤백은 하지 않아요." Then hand off to `diagnosis`.
- `5`: unknown deployment id. Stop; do not search latest.
- `4`: auth expired. Use auth recovery copy.

### Deploy failure → diagnosis handoff

For verify exit 7 only, preserve internal `DEPLOY_ID`, app slug/id/name, and classified verify state. Do not expose raw output. If a Skill tool exists, invoke `diagnosis` with app identity and "방금 배포 verify 가 실패했다" context. Otherwise follow diagnosis read-only CLI surfaces: `axhub deploy status <deployment-id> --json`, 그 status 시간창으로 좁힌 `axhub deploy logs --app <앱> --since <시작> --until <종료> --json --limit 100`(앱 단위 로그), then `axhub --json deploy diagnose <앱>`. Do not call MCP deployment diagnosis tools.

## Recovery Summary

Use `axhub plugin-support classify-exit "$EXIT" "$STDOUT"` or `references/error-empathy-catalog.md`.

- exit 64 + `validation.deployment_in_progress`: never retry `axhub deploy create`; monitor the in-flight deploy with the verify loop when an id is available.
- subdomain precondition: `axhub apps update <slug> --subdomain <subdomain> --json` is a separate destructive mutation and needs its own preview/approval before one retry.
- GitHub connection required: do not create repo, first push, or `apps git connect` from deploy; hand off to `import`. GitHub 자체가 막혀 `import` 로도 못 풀면 Upload lane 으로 가요.
- app 권한 부족 (exit 8 + `axhub_app_forbidden`): 앱 owner/admin 권한 검사에 막힌 상태예요. 앱을 만든 계정과 현재 계정이 달라도 판정은 CLI/백엔드 몫이에요. 앱 소유자/관리자에게 멤버 권한 부여를 요청하도록 안내하고 멈춰요. 구두 승인을 권한 근거로 쓰지 않아요 — 권한 부여 뒤 같은 명령의 재시도 성공으로만 확인해요.
- auth expired: ask before login flow in interactive mode.
- not found/ambiguous: show slug candidates only, no numeric ids.
- rate limit: respect Retry-After.
- transport/read paths: retry at most once; never retry create.

## NEVER

- NEVER let deploy create or initialize first-run app/import state. Missing app/manifest first-connect belongs to `import` or `bootstrap`.
- NEVER run `axhub init`, `axhub apps create`, first GitHub repo creation, first push, or `apps git connect` from deploy. Pushing normal ahead commits to an already connected `origin` branch is allowed and required before deployment-record create.
- NEVER retry `axhub deploy create` on exit 64.
- NEVER drop JSON/field-expr parsing contracts where a command result is parsed.
- NEVER call `axhub deploy create --execute` without the interactive 명시 텍스트 승인 preview decision. Headless is exempt only because it must stay dry-run and must not use `--execute`.
- NEVER call `axhub deploy watch` or `axhub deploy status --watch` from this skill. In-flight and webhook-triggered deployments use the single `--wait` verify call, and only a CLI without `capabilities.import.verify_wait` falls back to the bounded `axhub deploy verify "$DEPLOY_ID" --app "$APP_ID"` loop.
- NEVER 대기 수단 없이 같은 `axhub deploy verify` 를 연달아 호출하지 말아요. `verify_wait` capability 가 있으면 `--wait --wait-interval 20s --wait-timeout 10m --json` 단일 호출이 유일한 경로예요. 같은 exit 6 을 화면에 반복해서 쌓으면 사용자에게는 실패한 명령이 도배되는 것처럼 보여요.
- NEVER declare deploy success from deploy-create stdout, status snapshots, watch output, or prose polling. Deployment-record success declaration is terminal `axhub deploy verify <deployment-id> --app <app>`.
- NEVER call `axhub deploy verify` without both a deployment id and the resolved app scope. Latest 재탐색 금지.
- NEVER call `axhub deploy verify` in static lane (`deploy_method=static`). Static is release-based, not deployment-record-based, and success is `apps static deploy --execute` activate with `active_release_id`.
- NEVER send non-static apps to static lane. Empty or unsupported `deploy_method` uses the normal deployment-record pipeline.
- NEVER call `apps static deploy --execute` without static dry-run preview plus interactive approval. Headless static lane is dry-run only.
- NEVER change command semantics after approval by omitting `--execute`, changing the resolved deploy target, changing `--commit`, or changing the resolved tenant/profile. Surface the typed reason in one jargon-free line and stop, or use status-first verify-loop monitoring when appropriate.
- NEVER instruct the user to run `axhub deploy create`, `axhub deploy verify`, `apps static deploy --execute`, or any deploy CLI command themselves. The agent runs deploy and verify in this skill flow.
- NEVER run `deploy create` when status-first already found an in-flight deploy for this app; route to the bounded verify loop instead.
- NEVER call `axhub deploy cancel` without explicit confirmation.
- NEVER infer mutation target from pwd, git remote, cached app id, or old manifest alone; live resolve through `deploy-prep`.
- NEVER 앱 생성 계정과 현재 로그인 계정이 달라 보인다는 이유로 스킬이 스스로 소유자 확인 게이트("소유자에게 물어보세요")를 만들지 말아요. 인가 판정은 CLI/백엔드(exit 8 `axhub_app_forbidden`)의 몫이에요.
- NEVER bypass the 명시 텍스트 승인 preview card on slash invocation. Slash confirms skill invocation, not the destructive operation.
- NEVER insert the old approved-run helper bridge between preview approval and the canonical workflow; approval flows into `deploy-prep`, public `axhub deploy create --execute`, and verify.
- NEVER call MCP deployment mutation tools such as `deployment_trigger`; deploy is CLI-only.
- NEVER use advisor/server advisor/subagent/model escalation to choose or execute deploy; use CLI envelopes only.
- NEVER commit, push, or add `.omc/`, `.claude/`, `.codex/`, `.serena/`, `.omx/`, `.omo/`, or other local agent/runtime state as part of deploy cleanup. Ignore those paths when deciding whether app code is clean enough to deploy; if they are the only dirty entries, proceed with the tracked app commit and mention local cleanup only after deployment.
- NEVER call `axhub deploy create --execute` for a commit that is only local. AxHub resolves commits from the connected GitHub repo; local-only commits fail in prod with commit-not-found. GitHub 이 막혀 로컬 소스를 올려야 하는 경로는 예외이고, 그 경로는 `up` 스킬로 양보해요.
- NEVER pipe `axhub plugin-support token-gate`, `axhub deploy create`, or `axhub deploy verify` through `grep`, `head`, or filters that can make a successful command look failed or make a waiting command look hung.
- NEVER combine deploy polling into one long shell loop that asks Codex for a broad `run` permission card. Use one scoped verify command per check or a real host wakeup.
