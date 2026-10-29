# Init Templates And Repository Provider Gate Reference

Load this reference only for template registry detail or after the top-level bootstrap backend gate selected GitHub. A confirmed selfhosted app must not load the provider-specific sections below.

## Template Registry

Read templates from backend only:

```bash
axhub apps templates list --tenant test --json
```

The command above is a Desktop-visible shape: replace `test` with the selected tenant literal. Do not use `export`, `$AXHUB_TENANT`, command substitution, or multi-command shell glue in Codex-visible tool calls.

The response envelope contains `data.items[]` with fields like `id`, `folder_name`, `name`, and `resource_tier`. `schema_version` and raw IDs are internal primitives; do not dump them to chat. Bootstrap only checks that `nextjs-axhub` is returned. Inactive templates are never returned, so an alias for a retired template (`react`, `astro`, `chatbot`) does not resolve.

Exit routing:

- exit 4/auth: if the envelope subcode is `github_relogin_required`, the backend GitHub link is missing or expired and axhub re-login does not fix it — run the device-flow fast path (`AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link`, then `axhub github accounts list --json`) and re-run this gate. That fast path is the fallback that restores the link, not the normal path. 코드를 놓쳤거나 만료됐으면 같은 명령을 반복하지 말고 `axhub github link --fresh` 로 새 코드를 받아요. Otherwise say `다시 로그인해줘`.
- exit 8/tenant unresolved: use `axhub profile current --json` guidance and ask for login/profile fix.
- other abnormal exit: say `설치 상태 진단해줘` can inspect.

## Template

There is one template: `nextjs-axhub`. Bootstrap never asks a template question and never shows a template card — it passes `--template nextjs-axhub` to dry-run and execute. Ignore any other item the registry returns. Requests for a chatbot, AI chat, or an in-house assistant also start from `nextjs-axhub`; that feature is added after creation through development. If `nextjs-axhub` is missing from the registry output, stop before dry-run with `새 앱 템플릿을 찾지 못했어요. 잠시 뒤 다시 시도해 주세요.`

## App Name

`--name` is required. If the utterance implies a name, propose it as the first option, for example "결제 앱 만들어줘" -> "결제 앱". Do not finalize the name before one user-facing confirmation in Codex. Recommendation wording like "알아서 이름 지어줘" or "use the recommended name" is approval only after the app-name prompt is visible; before that, propose the recommendation and ask. Use the exact question text `앱 이름을 무엇으로 할까요?`; never write `앵 이름` or a shortened variant. Ask with a native Question/명시 텍스트 승인 card first. Fall back to normal chat text only when the card does not render or the answer UI is unavailable. If the utterance does not imply a name, ask once:

Pre-bootstrap answers to concept or slug questions are not confirmation. They may seed the first recommendation, but bootstrap must still show `앱 이름 확인` and ask `앱 이름을 무엇으로 할까요?` before setting `--name`, `--slug`, `--repo-name`, or `--subdomain`. Never display typoed labels or malformed choice descriptions. Keep choice descriptions short and proofread: prefer natural Korean like `기존 앱들과 겹치지 않는 새 콘셉트`, `예약 폼과 시간 선택에 적합`, or `정적 페이지 중심이면 가까운 구조`.

```text
앱 이름 확인

앱 이름을 무엇으로 할까요?

1. <발화에서 유추한 이름>
2. 직접 입력
3. 취소

번호나 원하는 앱 이름으로 답해 주세요.
```

Derive `--slug` by lowercasing, replacing spaces with hyphens, and removing special characters. If backend reports slug policy/collision, ask once for a new name/slug and retry the same flow.

## Backend Gate

The top-level skill owns backend resolution. Read top-level `git_backend.backend` and `git_backend.source` from `axhub apps get <app> --json` for resume/existing apps or `axhub apps git-backend --tenant <tenant> --json` for fresh apps. The latter is read-only and returns source `tenant|platform_default`; never pre-create an app row or call C1/Gitea directly.

When `git_backend.backend=selfhosted`, stop reading this file here. Do not run the account check, do not start a device flow, do not show an install URL, and do not render any owner or installation question. Continue to the backend-neutral availability and bootstrap preview with no `--github-owner`.

When `git_backend.backend=github` or `git_backend.source=legacy_github`, preserve every gate and question below.

## GitHub App Gate

Ask for the app name before the GitHub App gate. This gate exists to confirm the repository owner for dry-run/execute, so do not run it before the user has seen the app-name confirmation.

After templates are readable and the app name is confirmed, check GitHub App installation/account state:

```bash
axhub github accounts list --json
```

Rules:

- A normal (non-error) response means the backend already holds a linked GitHub account, so bootstrap continues with **no authentication step at all**. The CLI resolves owner and installation from this response, so do not pre-start a device flow and do not tell the user that authentication is required.
- If output is empty or not parseable, state is unavailable; do not block.
- If the auth envelope subcode is `github_relogin_required`, the link is missing or expired and re-login does not fix it: run the device-flow fast path (`AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link`, then `axhub github accounts list --json`) and re-run this gate. Device flow is the fallback for this case only. 코드를 놓쳤거나 만료됐으면 같은 명령을 반복하지 말고 `axhub github link --fresh` 로 새 코드를 받아요. For other auth-expired envelopes, say `다시 로그인해줘`, then re-run this gate after login.
- If `install_url` exists, always show it once as "GitHub App 설치·계정 추가 링크: `<install_url>`", regardless of installed status.
- If onboarding already showed the same install link in this conversation, repeated display can be skipped, but account check, owner pick, and zero-install gate still run.

## Installed Accounts

If one or more accounts have `installed:true`, proceed and choose owner:

- `AXHUB_GITHUB_OWNER` env wins without question.
- Exactly one installed account: use that `login`.
- Two or more installed accounts: ask once, using only installed logins and at most 3 options.

```json
{
  "questions": [{
    "question": "어느 GitHub 계정에 저장소를 만들까요?",
    "header": "GitHub 계정",
    "multiSelect": false,
    "options": [
      {"label": "<login-1>", "description": "이 계정/org 에 비공개 repo 를 만들어요"},
      {"label": "<login-2>", "description": "이 계정/org 에 비공개 repo 를 만들어요"}
    ]
  }]
}
```

In subprocess/no TTY, use `AXHUB_GITHUB_OWNER` if present; otherwise safe default is `취소`, so do not start bootstrap.

## Zero Installed Accounts

If normal response confirms zero installed accounts, block before dry-run/execute. Show install_url if available, otherwise point to the dashboard GitHub connection menu:

```text
GitHub App 이 아직 어떤 GitHub 계정에도 설치 안 됐어요. repo 를 만들려면 먼저 설치가 필요해요.
1. 위 링크를 브라우저에서 열어요.
2. repo 를 만들 계정/org 을 고르고 저장소 접근을 승인해요.
3. 끝나면 "설치했어" 라고 알려줘요.
```

Then ask:

```json
{
  "questions": [{
    "question": "GitHub App 설치를 끝냈을까요?",
    "header": "GitHub App",
    "multiSelect": false,
    "options": [
      {"label": "설치 완료", "description": "설치·연결을 끝냈으면 다시 확인하고 이어서 만들어요"},
      {"label": "취소", "description": "지금은 앱 만들기를 멈춰요"}
    ]
  }]
}
```

On `설치 완료`, run `axhub github accounts list --json` again. Continue only when an installed account is confirmed. On `취소`, stop with "GitHub App 을 설치하면 '다시 만들어줘' 라고 말해 주세요. 이어서 만들게요."

In subprocess/no TTY, safe default is `취소`; leave install_url and resume phrase, and do not bootstrap.
