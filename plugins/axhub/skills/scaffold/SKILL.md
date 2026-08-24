---
name: scaffold
description: '템플릿으로 시작하되 저장소는 사용자 소유로 만들어요. Use when the user wants the repository under THEIR account/org — even if the sentence also says "새 앱 만들어줘/시작해줘", repository ownership words win over bootstrap: "내 계정에 레포 만들어서 시작", "내 계정에 레포 만들어서 새 앱", "회사 조직에 저장소 파고 새 앱", "템플릿 받아서 내 깃허브에 올려줘", "레포는 우리 org 소유로", "start from template in my org". Start directly with a Korean progress sentence; no preamble; no route/skill label. 저장소 소유 언급이 전혀 없으면 bootstrap 으로, 이미 코드가 있는 폴더는 import 로 양보해요. 흐름: 템플릿 내려받기 → placeholder 치환 → 사용자 계정/조직에 저장소 생성+push(axhub github repo create) → origin 등록 → 여기서 멈추고 확인 → 동의하면 import 인계. push 뒤 앱 생성·배포로 자동으로 이어가지 않아요.'
examples:
  - utterance: "내 계정에 레포 만들어서 새 앱 시작해줘"
    intent: "scaffold a template app with a user-owned repository"
  - utterance: "회사 org 에 저장소 파고 템플릿으로 시작"
    intent: "scaffold a template app with an org-owned repository"
  - utterance: "템플릿 받아서 내 깃허브에 올리고 배포까지"
    intent: "scaffold a template app with a user-owned repository"
  - utterance: "start from template in my org"
    intent: "scaffold a template app with an org-owned repository"
allows-dependency-execution: false
model: sonnet
---

# Scaffold — 템플릿으로 시작, 저장소는 내 소유

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

bootstrap 과의 차이 하나뿐이에요: bootstrap 은 axhub 이 저장소를 만들어 주고(봇 소유 생성 — org 에서 주인 권한이 자동으로 안 붙을 수 있어요), 이 스킬은 **사용자 연동 계정으로 사용자의 계정/조직에** 저장소를 만들어요. 생성자가 곧 주인이라 그 권한 문제가 구조적으로 없어요. 저장소 생성만 CLI(`axhub github repo create`, v0.30.0+)가 하고 clone·커밋은 git 이에요. 앱 생성·연결·배포는 `import` 가 해요 — 이 스킬은 GitHub 쪽 준비까지만 소유하고, push 가 끝나면 이어갈지 한 번 물어본 뒤에만 넘겨요.

## 순서

### 1. CLI·로그인 확인

`axhub --version` 으로 CLI 존재 확인(AP-17). `axhub github repo --help` 가 unknown command 면 v0.30.0 미만이에요 — `update` 스킬로 보내고 멈춰요(다른 명령으로 대체하지 않아요). 로그인은 `axhub auth status --json` — 미로그인(exit 4)이면 `axhub auth login` 안내 후 이어가요.

### 2. GitHub 계정 연동 확인

Tool 제목 `저장소 계정 확인`:

```bash
axhub github accounts list --json
```

정상 응답이면 연동돼 있는 거예요. `github_relogin_required` 면 `axhub github link` 로 연동해요(코드 두 줄 형식·재개·코드 유실 시 `--fresh` 재발급은 bootstrap 9단계 계약 그대로). 응답의 계정 목록이 3단계 owner 후보예요 — **GitHub App 이 설치된 계정/조직만** 후보로 보여줘요. 원하는 조직이 목록에 없으면 그 조직에 App 설치가 먼저예요(설치 링크 안내 후 재확인).

### 3. 질문 한 번에: 템플릿·이름·소유자

AskUserQuestion 하나로 물어요 — 템플릿(`nextjs-axhub`·`vite-react-axhub`·`astro-axhub`), 앱 이름(slug·subdomain 은 이름에서 kebab-case 파생, 다르게 원하면 조정), 저장소 소유자(2단계 목록에서). 테넌트가 여럿이면 함께 물어요. 여기서 정한 slug·subdomain·tenant 가 5단계 치환 값이자 8단계 import 가 만들 앱의 값이에요 — 중간에 바꾸면 치환된 코드와 앱이 어긋나요.

### 4. 템플릿 내려받기 (인증 불필요)

빈 폴더(또는 새 폴더) `<target>` 에서 진행해요. 먼저 `<target>` 을 canonical absolute path로 고정하고, 경로 자체가 symlink이거나 비어 있지 않으면 중단해요. clone 뒤에도 template 안 symlink가 하나라도 있으면 복사·치환하지 않고 중단하며, placeholder 편집은 canonical target 내부의 regular file만 대상으로 해요.

```bash
git clone --depth 1 --branch main -- https://github.com/jocoding-ax-partners/axhub-template.git "<target>/.axhub-template"
```

```bash
cp -R "<target>/.axhub-template/<template-id>/." "<target>/"
```

```bash
rm -rf -- "<target>/.axhub-template"
```

지우는 경로는 canonical target 바로 아래의 정확한 임시 폴더 하나뿐이에요.

### 5. placeholder 치환 (필수)

서버 bootstrap 은 push 전에 치환하지만 이 흐름엔 서버가 없어요. 건너뛰면 앱이 `'{{API_BASE}}'` 라는 글자 그대로 API 를 불러 **로그인·데이터 연동만 조용히 죽어요**(화면은 떠요). `grep -rl -- '{{' "<target>"` 로 찾은 canonical target 내부 regular file 전부(README 포함)에서 6개 토큰을 편집 도구로 바꿔요 — symlink는 따라가지 않고, `sed -i` 는 macOS/GNU 문법이 갈려서 안 써요.

| 토큰 | 값 |
|---|---|
| `{{APP_SLUG}}` / `{{APP_SUBDOMAIN}}` / `{{APP_NAME}}` / `{{TENANT}}` | 3단계에서 정한 값 |
| `{{API_BASE}}` | CLI 가 쓰는 API 주소 (prod `https://axhub.ai`) |
| `{{APP_ORIGIN}}` | `https://<subdomain>.<tenant>.axhub.ai` (서버와 같은 조립식) |

`axhub.yaml` 의 `name:` 도 slug 로 바꿔요.

### 6. 커밋

```bash
git -C "<target>" init -q -b main && git -C "<target>" add -A && git -C "<target>" commit -q -m "Initial commit from axhub template"
```

### 7. 저장소 생성 + push (tool 제목 `저장소 만들기`)

미리보기 먼저(기본이 dry-run), 사용자 승인 후 `--execute`. 승인 문구는 다른 스킬과 같은 `진행`/`취소` 로 고정해요 — 미리보기 끝에 `지금 저장소를 만들고 코드를 올릴까요? 진행 또는 취소 로 답해 주세요.` 를 그대로 쓰고 매번 새 문구를 지어내지 않아요. 승인 문구를 굵게 강조하지 않아요 — `** 진행 **` 처럼 별표 안쪽에 공백이 들어가면 별표가 그대로 노출돼요.

```bash
axhub github repo create --owner "<owner>" --name "<slug>" --push "<target>" --execute
```

기본 private 이에요(공개를 원하면 `--public`). 토큰은 CLI 가 env 로만 다뤄서 채팅·히스토리에 안 남아요. device flow pending 으로 끝나면 URL·코드 두 줄을 보여주고 `--resume-last` 로 이어가요. `name already exists` 는 다른 이름을 물어요 — 기존 저장소에 덮어 push 하지 않아요.

### 7.5. origin 등록 (push 성공 직후 필수)

CLI 는 토큰이 `.git/config` 에 박히지 않게 원격 URL 을 인자로 직접 넘겨 push 해요 — 그래서 push 는 되지만 로컬에 named remote 가 남지 않아요. 이 등록을 빼먹으면 사용자의 **다음 `git push` 가 곧바로 `'origin' does not appear to be a git repository` 로 실패**하고, 저장소 연결 뒤 살아나는 push 자동 배포도 쓸 수 없어요.

```bash
git -C "<target>" remote add origin "https://github.com/<owner>/<slug>.git"
```

이 폴더는 6단계에서 방금 `git init` 한 상태라 `origin` 이 이미 있을 수 없어요 — 존재 확인 분기를 만들지 않아요. upstream 은 여기서 붙이지 않고, 사용자의 첫 push 때 `git push -u origin main` 으로 붙이면 돼요. 8단계 안내에 그 문장을 포함해요.

### 8. 여기서 한 번 멈춰요 (import 인계 확인)

7단계까지가 이 스킬의 기본 완료선이에요. push 가 끝나면 **묻지 않고 이어가지 않아요** — 저장소 주소를 평문 절대 URL 로 보여주고 AskUserQuestion 한 번으로 axhub 진입 확인을 해요. 사용자가 처음에 "배포까지 해줘" 라고 말했더라도 여기서 한 번은 물어요. 저장소 생성과 앱 배포는 되돌리는 비용이 달라서, 한 번의 승인으로 묶지 않아요. 네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.

```json
{
  "questions": [{
    "question": "저장소는 준비됐어요. 이어서 axhub 앱을 만들고 첫 배포까지 할까요?",
    "header": "다음 단계",
    "multiSelect": false,
    "options": [
      {"label": "여기서 멈추기", "description": "저장소만 만들어두고 끝내요. 나중에 '이 폴더 axhub에 올려' 라고 말하면 이어서 진행해요"},
      {"label": "앱 만들고 배포까지", "description": "앱 생성·GitHub 연결·첫 배포를 이어서 진행해요 (진행 전에 미리보기를 한 번 더 확인해요)"}
    ]
  }]
}
```

멈추기를 고르면 저장소 URL·로컬 폴더 경로·재개 문장(`이 폴더 axhub에 올려`)만 남기고 끝내요. 이때 다음 코드 변경을 올릴 때 쓸 `git push -u origin main` 한 줄도 같이 알려줘요 (7.5 에서 등록한 origin 에 upstream 을 붙이는 첫 push 예요). headless 에서는 묻지 않고 같은 자리에서 멈춰요.

이어가기를 고른 경우에만 `import` 스킬을 호출해 앱 생성·GitHub 연결·첫 배포를 맡겨요. 3단계에서 정한 slug·subdomain·tenant 를 그대로 넘겨요. 연결이 끝나면 push 자동 배포도 그대로 살아나요 — 이 흐름은 저장소가 있으므로 `axhub up` 을 쓰지 않아요.

## NEVER

- NEVER `apps bootstrap` 을 부르지 않아요 — 그건 axhub 소유 생성 경로예요. 여긴 `github repo create` + `import` 만 써요.
- NEVER 치환(5단계)을 건너뛰지 않아요 — 배포는 성공하고 로그인만 조용히 죽어요.
- NEVER 이미 코드가 있는 폴더에 템플릿을 덮지 않아요 — 기존 코드는 import 소관이에요.
- NEVER 토큰·인증 URL 을 제외한 device flow 진행 상황을 사용자에게 승인 완료 보고로 요구하지 않아요 — `--resume-last` 재개 계약을 따라요.
- NEVER `--execute` 를 미리보기·승인 없이 붙이지 않아요.
- NEVER push 직후 확인 없이 `import` 로 넘어가지 않아요 — 첫 발화에 "배포까지" 가 있어도 8단계 질문은 생략하지 않아요.
- NEVER push 성공 뒤 `origin` 등록(7.5)을 건너뛰지 않아요 — CLI 가 URL 직접 push 라 remote 를 안 남겨서, 빼먹으면 사용자의 다음 `git push` 가 실패해요.
- NEVER 연동 계정 목록에 없는 owner 로 진행하지 않아요 — App 미설치 조직은 생성이 되더라도 이후 연결(`VerifyInstallation`)이 404 로 죽어요.
