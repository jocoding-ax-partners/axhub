# Init Bootstrap And Local Reference

Load this reference when dry-run/execute/watch, GitHub device-code handling, clone/current-dir safety, manifest slug correction, or local preview dependency handling needs detail.

All Codex-visible commands in this reference follow the top-level bootstrap contract: one direct `axhub ...` command per tool call, Korean title starting with Korean text, and no developer-only wrappers. Never prefix with `rtk`, never run `rtk ls -la`, and never use generic `pwd`/`ls`/`find` probes to decide the bootstrap path.

## Dry-Run Preview

Run only after template, app name, tenant, and GitHub owner gate are settled:

Codex-visible tool titles and progress text must stay user-facing and start with Korean text. Use titles like `만들기 전 확인` or `미리보기 확인`; do not write `dry-run`, `Bootstrapped dry-run`, `Bootstraping dry-run`, `axhub bootstrap`, `saga`, or other internal execution labels in chat or tool descriptions.

Use the command shape below by replacing the sample literals with the values already confirmed in the conversation. Do not run a Desktop-visible command that contains `export`, value-assembly `TEMPLATE=...`/`APP_NAME=...`, `$TEMPLATE`, `$APP_SLUG`, `$AXHUB_TENANT`, command substitution, or semicolon-chained shell glue. Execute and resume commands carry no env prefix — `AXHUB_DEVICE_FLOW_AUTO_OPEN=1` there makes the CLI block-poll instead of returning the device code, so the tool call never ends and the code stays invisible. That prefix belongs only on the short `github link` fast path.

```bash
axhub apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --github-owner realitsyourman --tenant test --dry-run --json
```

Show preview fields in Korean: template, slug, subdomain, repo name, visibility. Do not show raw JSON. Do not execute before the user confirms:

Even if the initial natural-language request says "바로 인터넷에 올려줘" or "배포까지 해줘", treat that as the user's goal, not as execute approval. The preview-confirm card below is still required before any `--execute` call.

```json
{
  "question": "지금 만들고 배포까지 진행할까요?",
  "header": "앱 만들기",
  "options": [
    {"label": "진행", "value": "execute", "description": "앱 생성, 저장소 생성, 첫 배포를 진행해요"},
    {"label": "취소", "value": "cancel", "description": "지금은 만들지 않아요"}
  ]
}
```

Use exactly this question, labels, values, and descriptions. Do not paraphrase the question or invent new Korean option labels/descriptions.

Subprocess/no TTY safe default is `취소`.

## Execute And Watch

Before execute, write resume state and let `axhub plugin-support init-resume put` generate the idempotency key:

Use a user-facing tool title such as `앱 생성 진행` or `첫 배포 진행`. Never expose idempotency, saga, route, or skill names in the title or surrounding prose; those are implementation details.

Do not run an OS UUID generator as a separate Desktop-visible command. Run one direct `axhub plugin-support init-resume put ... --json` command, read `.state.idempotency_key` from its output internally, then pass that literal UUID to execute/resume commands. Keep each Desktop-visible tool call to one direct command.

```bash
axhub plugin-support init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --json
```

```bash
axhub --no-input apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --github-owner realitsyourman --tenant test --execute --idempotency-key 00000000-0000-4000-8000-000000000000
```

Do not add `--json` to Desktop-visible execute or device-flow resume commands. Do not set an intentionally short tool timeout to force Codex backgrounding. Device-flow code display must come from the visible `axhub ...` command output, not from a background watcher or a generated output file. When GitHub device flow starts, surface the `verification_uri` plus `user_code` in normal chat text before any confirmation/status command.

Do not attach `--watch` or `--watch-timeout` to the first execute. If execute returns `device_flow_required_user_action`, show the plain URL/code and then continue with a separate direct resume or account-confirmation command. If execute returns `bootstrap_id` or `deployment_id`, continue with separate direct status commands. Narrate about every 30s with short Korean progress lines like "앱 만들고 있어요", "GitHub repo 만들고 있어요", "첫 배포 중이에요. 거의 다 왔어요".

Use `axhub --no-input apps bootstrap` for execute and resume. Codex may run shell tools through a PTY, so stdout can look interactive to the CLI; `--no-input` forces the agent device-flow branch that prints the code envelope and exits instead of waiting invisibly.

If execute returns before deployment reaches a terminal state, the known `deployment_id` becomes an owned watch. preflight 의 `capabilities.import.verify_wait` 가 true 면 `배포 상태 확인` 제목으로 `axhub deploy verify <deployment-id> --app <app-slug> --wait --wait-interval 20s --wait-timeout 10m --json` 를 정확히 한 번 호출해 CLI 안에서 기다리고, 같은 명령을 연달아 재호출하지 않아요 — 대기 수단 없이 같은 exit 6 을 화면에 쌓는 연타 폴링은 UX 실패예요. capability 가 없는 구 CLI 에서만 아래 status fallback 을 써요. Never poll deployment status with a shell loop or any background watcher. Run one direct `axhub deploy status <deployment-id> --tenant <tenant> --json` command per check, with a user-facing title like `배포 상태 확인`. If it is still running, say it is still building and issue another separate direct status command in the same Desktop response when the UI can run another tool call; do not express the wait as `sleep`, `until`, `while`, a pipe, or a compound shell block. A Desktop permission request containing `until axhub deploy status`, `while`, `sleep`, `grep`, `head`, or a pipe is a failed watch UX and must be replaced with standalone status calls. Do not end the response by telling the user to check later. Do not claim "I'll automatically check again in 90 seconds" unless you actually run a follow-up tool call in this same Desktop session. Do not parse JSON with `grep`, `head`, `tail`, `cut`, `awk`, `sed`, or `jq` in Desktop-visible commands; read the tool output JSON directly.

Do not report final success while the deployment status is building/running/pending. Phrases such as `완벽해요`, `앱이 생성되었습니다`, `배포 완료`, or URL-later handoffs are final-result language and are only allowed after terminal deploy success plus verify success. Once status is terminal success, run one direct `axhub deploy verify <deployment-id> --app <app-slug> --json` command. If verify says still building, continue the same watch. If terminal failure or verify failure is returned, summarize briefly and route to diagnosis; do not call the app complete.

If the watch cannot finish in the current Desktop response, fetch bootstrap id with the same idempotency key and then use one direct status command. Do not use CLI watch flags from Codex:

```bash
axhub apps bootstrap-status 11111111-1111-4111-8111-111111111111 --tenant test --json
```

Use the real `bootstrap_id` from the previous JSON/status output instead of the sample UUID.

## Device-Code Event

A linked GitHub account makes execute finish with no device flow at all, so nothing in this section runs on that path. Everything below is the fallback for the not-linked or expired-link case that the GitHub App gate already surfaced.

If stdout contains:

```json
{"event":"device_code_issued","data":{"verification_uri":"https://github.com/login/device","verification_uri_complete":null,"user_code":"XXXX-XXXX","auto_poll":true,"browser_opened":true,"expires_in":899}}
```

write pending state:

```bash
axhub plugin-support init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --idempotency-key 00000000-0000-4000-8000-000000000000 --pending-device-flow true --json
```

When `auto_poll:true` and `browser_opened:true`, still surface the `user_code` immediately in chat. Never leave the user staring at an empty GitHub code-entry screen with no code. Do not use background-task output as the device-flow control plane. After the code is visible, use one direct `axhub github accounts list --tenant <tenant> --json` check or a watch-flag-free resume command to observe approval. Do not ask the user to say an approval phrase, and do not write wording that asks them to report back after approval. Say only that this screen will keep checking automatically.

If `browser_opened:false` or the command exits with `device_flow_required_user_action`, show exactly one fallback prompt:

```text
GitHub 연결이 필요해요. 다음 단계로 진행해 주세요:
1. 브라우저에서 열기: <verification_uri_complete 우선, 없으면 verification_uri>
2. 코드 입력: <user_code>
3. axhub GitHub App 설치 승인

브라우저에서 승인하면 제가 이 화면에서 자동으로 계속 확인할게요. 따로 `승인했어`라고 말하지 않아도 돼요.
```

In fallback mode, resume the cached flow yourself after `retry_after_secs` or a short bounded delay; do not wait for a manual approval phrase and do not end the response asking the user to report approval. Pending messages must say that approval will be detected automatically, not that the user should tell the agent after approving. Do not follow old wording like "Prefer the emitted `resume_command` literally"; use it only as a base argv, never verbatim: strip `--watch --watch-timeout <value>` and `--json` from the first Desktop resume after a device code so the result surfaces. Otherwise use:

```bash
axhub --no-input apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --github-owner realitsyourman --tenant test --execute --resume-last --idempotency-key 00000000-0000-4000-8000-000000000000
```

While an outstanding code exists, never run fresh `bootstrap --execute` without `--resume-last`; it can issue a new code and invalidate the user's approved one. If response remains `device_code_pending`, respect `retry_after_secs` and retry `--resume-last` until success or expiry. If code expired, start Step 7 execute again to issue a fresh challenge.

A `device_code_pending` payload carries `user_code`, `verification_uri`, and `expires_at` whenever the resume came from the local cache. Re-show the two body lines (`인증 URL:` / `입력 코드:`) from those fields on every pending retry, not only on the first `device_code_issued`. The user approving is the only thing that ends this loop, so a retry that reports "still pending" without the code leaves them nothing to act on. Older CLIs omit those fields; then keep retrying without re-showing.

If resume says `no pending github device flow`, follow `resume-and-tenant.md` recovery: re-check `axhub github accounts list --json`, confirm owner installation, then run one same-idempotency recovery execute without `--resume-last`.

## Clone Current Directory

After saga reaches terminal success, read repo from status and fill current directory. Do not create a subdirectory.

Codex may create `.omc/` in a newly added folder before the app code is cloned. Treat that as Desktop metadata, not as a user app. Do not run `git clone ... .`, because it fails in metadata-only folders and leads to extra `rtk ls -la` probes. Fill the current folder with `git init` + `fetch` + `reset --hard` so `.omc/` stays in place:

Keep the clone/hydrate command raw-git only. Never prefix any segment with rtk and do not add grep/cut/awk/sed probes to the same Desktop-visible command.

```bash
REPO=$(axhub apps bootstrap-status "$BOOTSTRAP_ID" --tenant "$AXHUB_TENANT" --field-expr '.data.repo_full_name // .data.status.repo_full_name // empty' 2>/dev/null || true)
if [ -z "$REPO" ]; then
  echo '{"systemMessage":"GitHub repo 정보가 응답에 없어요. 설치 상태 진단해줘라고 말하면 이어서 점검할 수 있어요."}'
  exit 65
fi
if [ -d .git ]; then
  echo "{\"systemMessage\":\"현재 dir 에 이미 .git 이 있어요. 자동 clone 건너뛸게요. 수동으로 origin 을 붙이려면: git remote add origin https://github.com/${REPO}.git && git fetch origin && git checkout -b main origin/main\"}"
else
  git init -q -b main
  git remote add origin "https://github.com/${REPO}.git"
  git fetch origin --quiet --depth=1
  DEFAULT_BRANCH=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo main)
  git reset --hard "origin/$DEFAULT_BRANCH"
  git branch --set-upstream-to="origin/$DEFAULT_BRANCH" "$DEFAULT_BRANCH" 2>/dev/null || true
fi
```

If `.git` already exists, skip automatic clone and show manual remote commands. If clone fails, show only the real `repo_full_name` and manual guidance. Do not invent URLs.

## Manifest Slug Correction

After clone, make sure `axhub.yaml` points to the newly created app slug, not the template default:

```bash
if [ -f axhub.yaml ]; then
  if grep -Eq '^name:[[:space:]]*' axhub.yaml; then
    APP_SLUG="$APP_SLUG" perl -0pi -e 's/(^name:[[:space:]]*).*$/${1}$ENV{APP_SLUG}/m' axhub.yaml
  else
    TMP_MANIFEST="$(mktemp)"
    { printf 'name: %s\n' "$APP_SLUG"; cat axhub.yaml; } > "$TMP_MANIFEST"
    mv "$TMP_MANIFEST" axhub.yaml
  fi
  axhub deploy --explain --json >/dev/null
fi
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --repo-full-name "$REPO" --clone-done true --json
axhub plugin-support init-resume clear --json
```

If manifest `name:` differs, edit only that top-level binding to `$APP_SLUG`, then run `deploy --explain` as the parser check. This protects later deploy resolve from targeting the template app.

## Scaffold And Dependency Preview

Detect local app shape:

```bash
axhub plugin-support scaffold-detect --json
```

If `package.json`, lockfile, node, and dev script are present, ask once:

```json
{
  "question": "앱을 바로 실행해 볼까요?",
  "header": "앱 실행",
  "options": [
    {"label": "아니요", "value": "skip", "description": "배포 결과만 확인해요"},
    {"label": "네, 실행까지", "value": "start", "description": "의존성을 설치하고 로컬 미리보기를 띄워요"}
  ]
}
```

Subprocess/no TTY safe default is `아니요`. If user chooses start:

```bash
axhub plugin-support scaffold-dev start --json
```

`scaffold-dev` handles package manager choice. Installs only when a lockfile exists and must use `--ignore-scripts`. Show natural-language outcomes:

- success/already alive: "로컬 미리보기도 떠 있어요." plus URL if present.
- install/dev failure: "미리보기 자동 실행이 잠깐 안 됐어요. '다시 해줘' 하면 재시도할게요."
- no lockfile/package/dev script: skip local preview and continue result guidance.
