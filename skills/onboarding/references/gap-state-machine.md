# Onboarding Gap State Machine

Load this after `axhub plugin-support onboarding-detect --json` when `first_gap` is not `no_gap`, or when you need the exact completion rule for a gap. The detector owns order. This reference only maps the detected first gap to the next safe action.

## Loop

```text
DETECT_ALL(read-only)  <- axhub plugin-support onboarding-detect --json
  cli_missing          -> installer approval -> DETECT_ALL
  cli_path_missing     -> repair-path -> user reload or DETECT_ALL
  cli_old              -> update check/apply approval -> DETECT_ALL
  auth_missing         -> auth status/refresh/login -> DETECT_ALL
  git_missing          -> git install approval -> DETECT_ALL
  node_missing         -> node install approval -> DETECT_ALL
  node_mismatch        -> node version correction approval -> DETECT_ALL
  github_app_missing   -> install_url gate -> DETECT_ALL
  existing_repo_gap    -> apps git status/connect -> DETECT_ALL
  no_manifest_empty    -> advisory only -> Ready card
  deps_missing         -> lockfile install approval -> DETECT_ALL
  deploy_unverified    -> deploy verify on known id -> DETECT_ALL
  doctor_gap           -> preflight recovery phrase -> DETECT_ALL
  no_gap               -> Ready card
```

Do not process the second item in `gaps` from the same JSON. Handle one `first_gap`, then re-run detect.

## Gap Completion Rules

| gap id | Detection cue | Completion rule |
| --- | --- | --- |
| `cli_missing` | `cli_present=false` | User installs official CLI, then detect reports `cli_present=true`. |
| `cli_path_missing` | `cli_state=on_disk_not_on_path` or `cli_on_path=false` | `repair-path` reports repaired/already present, or user opens a new terminal and re-runs onboarding. |
| `cli_old` | `cli_too_old=true` or `has_update=true` | update apply succeeds, or user stops with `READY_WITH_USER_ACTION`. |
| `auth_missing` | `auth_ok=false` | refresh/login succeeds and re-detect is green. |
| `git_missing` | `git_present=false` | git becomes present after user-approved install. |
| `node_missing` | `node_present=false` | node becomes present after user-approved install. |
| `node_mismatch` | `node_mismatch=true` | required version is active, or continue is degraded. |
| `github_app_missing` | `github.state=uninstalled` or `empty` | re-detect shows `installed`/`mixed`, or user explicitly defers with `READY_WITH_USER_ACTION`. |
| `existing_repo_gap` | repo has git+commit but no manifest | app/repo connection succeeds, or onboarding stops with user action needed. |
| `no_manifest_empty` | empty dir and no manifest | advisory only; no re-detect loop for this gap. |
| `deps_missing` | lockfile+manifest present and deps missing | lockfile install exits 0; native build issues downgrade to user action. |
| `deploy_unverified` | deployment known but not verified | `axhub deploy verify "$DEPLOYMENT_ID"` exits 0 for that id. |
| `doctor_gap` | final core check is not green | preflight gives recovery phrase; no destructive action. |

## Existing Repo Gap

Existing repo onboarding is not init. If the directory has a commit and no axhub manifest, avoid clone/scaffold collision and use the app/repo Git surface.

1. If `$APP_ID` is unknown, stop with `READY_WITH_USER_ACTION`: tell the user to say `첫 앱 만들어줘` first. Onboarding must not auto-create the app.
2. Run status for the known app:

   ```bash
   axhub apps git status --app "$APP_ID" --json
   ```

3. Show `install_url`, `repo_full_name`, `branch`, and installed login names only. Do not print `installation_id`.
4. Ask before connecting. The dry run is:

   ```bash
   axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json
   ```

5. After the preview is acceptable and the user approves, execute:

   ```bash
   axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json
   ```

OAuth or GitHub installation approval is a user-action gate. In headless mode, show the command/phrase and stop.

## Empty Directory Gap

For `no_manifest_empty`, do not ask an init question and do not call init. Say: "이 폴더는 비어 있어요. 첫 앱을 만들려면 `첫 앱 만들어줘` 라고 말해 주세요." Then continue to the Ready card as `READY_WITH_USER_ACTION` or a Ready card with that next phrase. This avoids an infinite detect loop on an intentionally empty directory.

## Doctor And Deploy Evidence

Final doctor-style check is read-only:

```bash
axhub plugin-support preflight --json
```

Use it to produce recovery phrases such as `다시 로그인해줘` or `새 터미널을 열고 온보딩 계속`. Do not mutate from `doctor_gap`.

For deployment evidence, verify only the deployment id already returned by a deploy flow:

```bash
axhub deploy verify "$DEPLOYMENT_ID"
```

Never search for latest deployment during onboarding. If the id cannot be proven live, report `READY_WITH_USER_ACTION` instead of green success.
