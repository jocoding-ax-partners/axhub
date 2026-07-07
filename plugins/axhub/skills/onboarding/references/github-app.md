# GitHub App Onboarding

Load this whenever detect includes `github.install_url`, when `first_gap=github_app_missing`, or when an existing repo needs `axhub apps git`.

## Install URL Visibility

Immediately after detect, if `github.install_url` is not null, show it once regardless of `first_gap`:

```text
GitHub App 설치·계정 추가 링크: <github.install_url>
이미 설치돼 있어도 다른 org/계정을 더 붙일 수 있어요.
```

If `github.installed_logins` is non-empty, add `이미 연결된 계정: <login...>`. Show login names and the URL only. Do not show `installation_id` or internal API details. Do not automatically open the link unless the user chooses an install action.

If `github.install_url` is null because `github.state=auth_error`, tell the user to login again. If `unavailable`, leave it as best-effort unavailable and continue only when the current gap does not require GitHub installation.

## Already Installed Or Mixed

For `github.state=installed` or `mixed`, ask once whether the user wants to add another org/account. This is non-blocking; the default is continue.

```json
{
  "questions": [{
    "question": "다른 org/계정에도 GitHub App 을 설치할래요?",
    "header": "GitHub App",
    "multiSelect": false,
    "options": [
      {"label": "아니요, 계속", "description": "설치를 더 하지 않고 다음 gap 처리로 이어가요"},
      {"label": "설치할래요", "description": "install_url 을 보여주고 브라우저를 열어요. 설치 후 `온보딩 계속`"}
    ]
  }]
}
```

In subprocess/headless mode, skip the question and choose `아니요, 계속`. Never open a browser automatically there.

## Missing Install Gate

For `github.state=uninstalled` or `empty`, installation is a gate. Do not advance to repo/app connection while this remains the `first_gap`.

```json
{
  "questions": [{
    "question": "GitHub App 을 먼저 설치할까요?",
    "header": "GitHub App",
    "multiSelect": false,
    "options": [
      {"label": "설치", "description": "install_url 을 열어 계정레벨 GitHub App 설치를 끝내요"},
      {"label": "나중에", "description": "다음 단계로 넘어가지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
    ]
  }]
}
```

If the user chooses install, show/open `github.install_url`. After they say `승인했어` or `온보딩 계속`, re-run detect exactly once and follow the new `first_gap`.

If the user chooses later, leave the install URL, the phrases `승인했어` / `온보딩 계속`, and `READY_WITH_USER_ACTION`. Do not call `axhub apps git connect`.

## Existing Repo Connection Notes

GitHub App installation is account-level. OAuth device-flow authorization belongs to the later app/repo connect step, not to the install URL display. Do not describe the install URL step as OAuth completion.

When `axhub apps git status` returns installed logins and repo metadata, use that output for ambiguity handling. If multiple installed accounts could own the repo, ask the user which owner to use before dry-run connect.
