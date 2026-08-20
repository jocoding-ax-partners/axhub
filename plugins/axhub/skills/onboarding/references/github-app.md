# GitHub App Onboarding

Load this whenever detect includes `github.install_url`, when the gap is `github_link_missing` or `github_app_missing`, or when an existing repo needs `axhub apps git`.

## GitHub 계정 연동 (설치보다 먼저)

GitHub 표면은 두 단계이고 순서가 고정돼 있어요 — **계정 연동**(내 axhub 계정에 GitHub 계정을 붙이는 인증)이 먼저, **App 설치**(그 계정의 저장소 접근 승인)가 나중이에요. 연동이 없으면 계정·설치 상태를 읽는 조회가 exit 4 + subcode `github_relogin_required` 로 끝나서 설치 상태 자체를 알 수 없어요. 그래서 연동을 못 끝낸 채로는 설치 gate 를 판단할 수 없어요.

연동 상태 확인은 읽기 전용이라 언제든 다시 돌려도 안전해요:

```bash
axhub github accounts list --json
```

정상 응답이면 연동이 살아 있는 상태라 이 단계는 건너뛰고 설치 확인으로 이어가요. exit 4 + `github_relogin_required` 면 즉시 끝나는 fast path 로 연동을 시작해요:

```bash
AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link
```

받은 값은 본문에 평문 두 줄로만 써요. URL 은 Markdown 링크나 inline code 로 감싸지 않아요:

```text
인증 URL: https://github.com/login/device
입력 코드: <USER_CODE>
```

코드를 놓쳤거나 만료돼서 승인을 못 했으면 같은 명령을 반복하지 말고 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh` 로 새 코드를 받아요 — 저장된 pending link 는 죽은 코드를 그대로 돌려줘요. 브라우저 승인 반영은 위 `axhub github accounts list --json` 를 한 번 다시 실행해서 확인해요 — 사용자에게 승인 완료를 채팅으로 알려 달라고 쓰지 않아요. 정상 응답이 오면 그때 설치 단계(install_url)로 넘어가요.

연동을 한 번 해두면 앱 만들기·저장소 연결이 그 연동을 그대로 써서 인증 단계 없이 진행돼요. 다만 연동은 시간이 지나면 만료돼서 같은 안내가 다시 필요할 수 있어요 — 재연동은 고장이 아니라 정상 흐름이에요. `다시 묻지 않아요` 처럼 영구적이라고 단정하지 말아요.

subprocess/headless 에서는 브라우저를 열거나 연동을 시작하지 않고, 위 명령과 `READY_WITH_USER_ACTION` 으로 멈춰요.

## Install URL Visibility

Immediately after detect, if `github.install_url` is not null, show it once regardless of `first_gap`:

```text
GitHub App 설치·계정 추가 링크: <github.install_url>
이미 설치돼 있어도 다른 org/계정을 더 붙일 수 있어요.
```

If `github.installed_logins` is non-empty, add `이미 연결된 계정: <login...>`. Show login names and the URL only. Do not show `installation_id` or internal API details. Do not automatically open the link unless the user chooses an install action.

If `github.install_url` is null because `github.state=auth_error`, error subcode 를 따라요 — `github_relogin_required` 계열이면 axhub 재로그인으로는 풀리지 않으니 위 계정 연동 단계(`axhub github link`)로 돌아가고, 그 외에는 `다시 로그인해줘` 로 안내해요. If `unavailable`, leave it as best-effort unavailable and continue only when the current gap does not require GitHub installation.

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

If the user chooses install, show/open `github.install_url`. After they say `설치 끝났어` or `온보딩 계속`, re-run detect exactly once and follow the new `first_gap`. 브라우저의 App 설치 완료는 CLI 가 폴링할 수 없어 사용자 신호가 필요해요 — device flow 의 "승인 문구를 기다리지 않는다" 규칙과는 다른 표면이에요.

If the user chooses later, leave the install URL, the phrases `설치 끝났어` / `온보딩 계속`, and `READY_WITH_USER_ACTION`. Do not call `axhub apps git connect`.

## Existing Repo Connection Notes

GitHub App installation is account-level. 계정 연동의 device flow 는 그 앞의 별개 단계이고, install URL 표시와 절대 한 단계로 합치지 않아요. Do not describe the install URL step as OAuth completion.

계정 연동이 끝나 있으면 `axhub apps git connect` 는 추가 인증 없이 그 연동을 그대로 써요. 연동이 만료돼 `github_relogin_required` 가 다시 나오면 계정 연동 단계로 돌아갔다가 이어가요.

When `axhub apps git status` returns installed logins and repo metadata, use that output for ambiguity handling. If multiple installed accounts could own the repo, ask the user which owner to use before dry-run connect.
