# Init Templates And GitHub Gate Reference

Load this reference when template registry display, template/app-name choice, GitHub App installation/account gating, or multi-owner behavior needs detail.

## Template Registry

Read templates from backend only:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub apps templates list --tenant "$AXHUB_TENANT" --json
```

The response envelope contains `data.items[]` with fields like `id`, `folder_name`, `name`, and `resource_tier`. `schema_version` and raw IDs are internal primitives; do not dump them to chat. The selected `--template` may be a returned `id` or a built-in alias (`react`, `nextjs`, `astro`) that corresponds to a returned item.

Exit routing:

- exit 4/auth: say `다시 로그인해줘`.
- exit 8/tenant unresolved: use `axhub profile current --json` guidance and ask for login/profile fix.
- other abnormal exit: say `설치 상태 진단해줘` can inspect.

## Template Descriptions

This is not a second registry. Attach descriptions only to items returned by backend:

| alias / folder | Use when |
|---|---|
| `nextjs` / `nextjs-axhub` | 쇼핑몰, 예약, 결제, 로그인, 관리자 화면처럼 화면과 기능이 함께 있는 웹서비스 |
| `astro` / `astro-axhub` | 회사 소개, 랜딩 페이지, 블로그, 문서처럼 글과 이미지 중심이고 자주 바뀌지 않는 사이트 |
| `react` / `react-axhub` | 로그인 뒤 쓰는 설정 화면, 입력 폼, 관리 화면처럼 버튼을 눌러 내용이 자주 바뀌는 화면 |

Unknown backend templates are not hidden. Show backend `name` and `folder_name`, then give neutral guidance like "이름을 보고 고르면 돼요. 잘 모르겠으면 먼저 Next.js 추천을 봐요."

Structured AskUserQuestion can show at most 3 choices, and every choice must map to a real backend template. Do not add generic `Other`, `직접 고르기`, or `취소` choices to the template picker. If there are more than 3 templates, show the full text list and make the buttons the 3 best actual recommendations; free-text must match exact alias/folder/name before starting saga.

Example shape, only when those templates exist in backend output:

```json
{
  "question": "어떤 템플릿으로 시작할까요?",
  "header": "템플릿",
  "options": [
    {"label": "Next.js 추천", "description": "쇼핑몰·예약·결제·로그인·관리자 화면"},
    {"label": "Vite + React", "description": "로그인 뒤 쓰는 설정·입력·관리 화면"},
    {"label": "Astro", "description": "회사 소개·랜딩 페이지·블로그·문서"}
  ]
}
```

If the user's utterance already contains an exact alias/folder/name, use it without asking. In subprocess/no TTY, do not auto-pick a template; safe default is `abort`.

## App Name

`--name` is required. If the utterance implies a name, use it, for example "결제 앱 만들어줘" -> "결제 앱". Otherwise ask once:

```json
{
  "question": "앱 이름 뭘로 할래요?",
  "header": "앱 이름",
  "options": [
    {"label": "지금 발화 기준 자동", "value": "auto_from_utterance", "description": "발화에서 유추한 이름을 그대로 써요"},
    {"label": "직접 입력", "value": "manual_name", "description": "원하는 이름을 한 번만 말해요"},
    {"label": "취소", "value": "abort", "description": "프로젝트를 만들지 않아요"}
  ]
}
```

Derive `--slug` by lowercasing, replacing spaces with hyphens, and removing special characters. If backend reports slug policy/collision, ask once for a new name/slug and retry the same flow.

## GitHub App Gate

After templates are readable, check GitHub App installation/account state:

```bash
axhub github accounts list --json
```

Rules:

- If output is empty or not parseable, state is unavailable; do not block.
- If auth envelope says auth expired, say `다시 로그인해줘`, then re-run this gate after login.
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

If normal response confirms zero installed accounts, block before template choice/dry-run/execute. Show install_url if available, otherwise point to the dashboard GitHub connection menu:

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
