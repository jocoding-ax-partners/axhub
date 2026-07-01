# Init Bootstrap And Local Reference

Load this reference when dry-run/execute/watch, GitHub device-code handling, clone/current-dir safety, manifest slug correction, or local preview dependency handling needs detail.

## Dry-Run Preview

Run only after template, app name, tenant, and GitHub owner gate are settled:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --dry-run --json
```

Show preview fields in Korean: template, slug, subdomain, repo name, visibility. Do not show raw JSON. Do not execute before the user confirms:

```json
{
  "question": "지금 만들고 배포까지 진행할까요?",
  "header": "앱 만들기",
  "options": [
    {"label": "진행", "value": "execute", "description": "backend app + GitHub repo + 첫 deploy 를 자동으로 진행해요"},
    {"label": "취소", "value": "취소", "description": "지금은 만들지 않아요"}
  ]
}
```

Subprocess/no TTY safe default is `취소`.

## Execute And Watch

Before execute, write resume state with the same idempotency key that execute will use:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --json
axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
```

Run the tool with a timeout longer than 9 minutes, for example 570000ms. Narrate about every 30s with short Korean progress lines like "앱 만들고 있어요", "GitHub repo 만들고 있어요", "첫 배포 중이에요. 거의 다 왔어요".

If the watch times out with a resume hint, fetch bootstrap id with the same idempotency key and then watch status:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
BOOTSTRAP_ID=$(axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --execute --idempotency-key "$IDEMPOTENCY_KEY" --field-expr '.data.bootstrap_id // empty' 2>/dev/null || true)
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --json
axhub apps bootstrap-status "$BOOTSTRAP_ID" --tenant "$AXHUB_TENANT" --watch --watch-timeout 9m --json
```

## Device-Code Event

If stdout contains:

```json
{"event":"device_code_issued","data":{"verification_uri":"https://github.com/login/device","verification_uri_complete":null,"user_code":"XXXX-XXXX","expires_in":899}}
```

write pending state:

```bash
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --pending-device-flow true --json
```

Immediately show:

```text
GitHub 연결이 필요해요. 다음 단계로 진행해 주세요:
1. 브라우저에서 열기: <verification_uri_complete 우선, 없으면 verification_uri>
2. 코드 입력: <user_code>
3. axhub GitHub App 설치 승인

브라우저에서 승인한 다음 "승인했어" 라고 알려주세요. 제가 이어서 마무리할게요.
```

In interactive TTY, CLI may keep polling; narrate until the next stage event. In agent/non-TTY, CLI fast-exits after emitting the challenge. On user approval signal, resume the cached flow:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --tenant "$AXHUB_TENANT" --execute --resume-last --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
```

While an outstanding code exists, never run fresh `bootstrap --execute` without `--resume-last`; it can issue a new code and invalidate the user's approved one. If response remains `device_code_pending`, ask the user to finish approval and resume again. If code expired, start Step 7 execute again to issue a fresh challenge.

If resume says `no pending github device flow`, follow `resume-and-tenant.md` recovery: re-check `axhub github accounts list --json`, confirm owner installation, then run one same-idempotency recovery execute without `--resume-last`.

## Clone Current Directory

After saga reaches terminal success, read repo from status and fill current directory. Do not create a subdirectory:

```bash
REPO=$(axhub apps bootstrap-status "$BOOTSTRAP_ID" --tenant "$AXHUB_TENANT" --field-expr '.data.status.repo_full_name // empty' 2>/dev/null || true)
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
axhub manifest --json
axhub manifest validate --file axhub.yaml --json
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --repo-full-name "$REPO" --clone-done true --json
axhub plugin-support init-resume clear --json
```

If manifest slug differs, edit only the slug field to `$APP_SLUG`, then validate. This protects later deploy resolve from targeting the template app.

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
