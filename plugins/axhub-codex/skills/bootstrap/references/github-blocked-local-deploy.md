# GitHub 이 막혔을 때 — 로컬 소스로 배포 (백엔드 spec 184)

bootstrap 12단계 본문이에요. GitHub 때문에 더 못 나갈 때만 읽어요.

bootstrap 은 저장소를 만드는 흐름이라 GitHub 이 필수예요(`--github-owner`).
막히면 빈손으로 돌려보내는 대신 코드를 확보해 그대로 올려서 배포해요.
**묻지 않고 진행하고**, 무엇이 달라지는지는 하고 나서 알려줘요.

## 언제 오나 — 단계가 아니라 원인으로

어느 지점이든 GitHub 때문에 못 나가면 와요. 지점을 열거하면 새 실패 자리가
생길 때 빈틈이 생겨요. 자주 보는 모양(이게 전부는 아니에요):

- 6단계 gate: 설치 계정 0개, `github_relogin_required`, device flow 거부·만료·중단
- 9단계 execute: 저장소 생성이 org 권한·정책으로 거부, 계정 미연결로 saga 미시작
- 10단계 clone: `git fetch` 가 권한으로 실패(404 / permission denied). org 저장소는 우리 봇이 만들어 **주인에게 권한이 자동으로 안 붙는 경우**가 있어요

## 판정할 것은 둘뿐

1. **원인이 GitHub 인가.** 네트워크·타임아웃·5xx 는 해당 단계를 **한 번만** 재시도하고 그래도 안 되면 와요. 성급히 폴백하면 저장소를 원했던 사람이 저장소 없는 앱을 갖게 돼요.
2. **앱이 이미 만들어졌나.** 뒤 절차를 가르는 유일한 분기예요. 추측하지 말고 확인해요 — saga 가 조금이라도 돌았으면 앱만 만들어지고 저장소에서 실패했을 수 있어요.
   - `bootstrap_id` 를 알면: `axhub apps bootstrap-status <bootstrap-id> --tenant <tenant> --json`
   - 모르면: `axhub apps get <app-slug> --tenant <tenant> --json`

공통 조건: D1 safe-stop 모드가 아니에요 (앱 생성·배포는 mutation).

## 절차

**1. 한 줄로 알리고 바로 진행** (질문 아니에요).

되돌릴 수 있다는 걸 **이 줄에서** 같이 말해요. 사후 고지까지 미루면 사용자는 배포가
끝날 때까지 저장소를 영영 못 쓰는 줄 알아요.

- 저장소를 못 받는 경우: `저장소를 받아올 권한이 없어서, 같은 템플릿을 공개 저장소에서 받아 올려 배포할게요. 권한이 풀리면 평소대로 push 배포를 쓸 수 있어요`
- 그 밖: `GitHub 연결에 실패해서 우선 로컬 소스로 배포할게요. GitHub 는 이후 다시 연결할 수 있어요`

**2. 코드 확보.** 폴더에 이미 배포할 코드가 있으면(루트에 `Dockerfile` 또는
compose 파일) 그대로 써요. 없으면 공개 템플릿 저장소에서 받아요 — public 이라
GitHub 인증·권한이 필요 없어요. 템플릿은 하위 폴더로 있고 이름이 4단계에서 고른
template id 와 같아요 (`nextjs-axhub`, `vite-react-axhub`, `astro-axhub`).

```bash
git clone --depth 1 --branch main https://github.com/jocoding-ax-partners/axhub-template.git <target>/.axhub-template
```

```bash
cp -R <target>/.axhub-template/nextjs-axhub/. <target>/
```

```bash
rm -rf <target>/.axhub-template
```

`<target>` 과 template id 는 확인된 literal 로. 임시 폴더는 정확히
`<target>/.axhub-template` 하나만 지워요. 하위 폴더만 복사하므로 템플릿 저장소의
git 이력은 따라오지 않아요.

**2.5. placeholder 치환 — 템플릿을 받았으면 필수.** 정상 bootstrap 은 서버가
push 전에 6개 토큰을 텍스트 치환하고 `axhub.yaml` 의 `name:` 을 slug 로
재작성해요. 이 갈래는 그 단계가 없으니 직접 해요. 건너뛰면 템플릿 코드의
`process.env.APPHUB_API_URL || '{{API_BASE}}'` 같은 fallback 이 그대로 살아
앱이 `'{{API_BASE}}'` 라는 리터럴로 API 를 불러요 — 빌드도 첫 화면도 멀쩡하고
**로그인·gateway·DB SDK 만 조용히 죽는** 모양이라 배포 성공으로 오판하기 쉬워요
(`APPHUB_*` env 는 플랫폼이 주입하지 않아요 — 자동 주입은 `DATABASE_URL` 계열뿐).

| 토큰 | 값 |
|---|---|
| `{{APP_SLUG}}` | 앱 slug |
| `{{APP_SUBDOMAIN}}` | subdomain |
| `{{APP_NAME}}` | 앱 이름 |
| `{{TENANT}}` | 테넌트 slug |
| `{{API_BASE}}` | CLI 가 쓰는 API 주소 (prod `https://axhub.ai`) |
| `{{APP_ORIGIN}}` | `axhub apps get <slug> --tenant <tenant> --json` 의 `access_url` |

`grep -rl '{{' <target>` 로 대상 파일을 찾아(README 포함 전부) 편집 도구로
치환해요 — `sed -i` 는 macOS/GNU 문법이 갈려서 안 써요. 마지막으로
`axhub.yaml` 의 `name:` 을 앱 slug 로 바꿔요.

**3. 앱 종류 판정.** 템플릿은 그 `axhub.yaml` 의 `build.deploy_method` 를 그대로
써요(현재 세 템플릿 모두 `docker`). 사용자 자기 코드면 루트에 compose 파일만
있으면 `compose`, `Dockerfile` 이 있으면 `docker`. **둘 다 루트에 있으면 `docker`
로 해석돼요** — compose 로 배포하려면 루트 `Dockerfile` 을 서비스 폴더로 옮기고
compose 의 `build:` 가 그 폴더를 가리켜야 해요. 어기면 build 단계에서
`compose 파일 파싱 실패` 로 죽어요.

**4. 앱 확보.** 이미 있으면 5 를 건너뛰고 6 으로 가고, slug 는 확인한 값을 그대로
써요. 없으면 앱 주소 확인(7단계 availability check)부터 하고 이어가요.

**5. 앱 생성** (tool 제목 `앱 만들기`, 앱이 없을 때만):

```bash
axhub apps create --tenant test --name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --deploy-method docker --resource-tier XS
```

**6. 배포** (tool 제목 `소스 올려서 배포`). 소스 폴더 안에서:

```bash
axhub up --app bakery-preorder --execute
```

`--execute` 없이는 미리보기만 나와요. 올라가는 건 `.gitignore` 를 존중한 현재
폴더이고 `.git/`·`node_modules/`·`.env` 는 빠져요(`.env.example` 류는 남아요).
`axhub up` 은 CLI 0.29.0+ — 없으면 `axhub update apply --execute --yes --json`
먼저. 앱에 저장소가 연결돼 있어도 그대로 동작해요(배포 소스는 배포마다 정해지고
연결된 저장소는 건드리지 않아요).

**7. 결과 확인** — 11단계 그대로. clone 단계는 이미 끝났거나 건너뛴 상태예요.

**8. 사후 고지** — 결과 카드에 덧붙여요.

- 저장소가 아예 없으면: push 자동 배포·변경 이력이 없어요. 나중에 `axhub apps git connect` 로 붙일 수 있고 그때 앱을 다시 만들 필요는 없어요.
- 저장소는 있는데 권한이 없으면: 조직 관리자가 권한을 주면 평소대로 push 배포를 쓸 수 있고, 그전까지는 `axhub up` 으로 배포하면 돼요. 저장소 주소는 saga 결과 값 그대로 알려주고 임의로 만들지 않아요.

위 값들은 확정 literal 로 바꿔요. 이 갈래에서는 `apps bootstrap` 을 부르지 않아요.

## NEVER

- NEVER 앱이 있는지 확인하지 않고 새로 만들지 않아요 — saga 가 조금이라도 돌았으면 앱만 만들어졌을 수 있어요. 같은 이름으로 또 만들면 주소가 충돌하고 요금 자원이 두 배가 돼요.
- NEVER 권한 실패(404)를 "저장소가 없다" 로 보고하지 않아요 — private 은 권한이 없으면 404 로 보여요.
- NEVER `<target>/.axhub-template` 외의 경로를 지우지 않아요.
- NEVER 템플릿을 공개 저장소가 아닌 곳에서 받거나 사용자 저장소로 push 하지 않아요.
- NEVER GitHub 이 정상인데 이 갈래로 빠지지 않아요 — 저장소·push 자동 배포를 조용히 포기시키는 셈이에요.
- NEVER 일시적 실패(네트워크·타임아웃·5xx)를 GitHub 차단으로 보고 곧바로 넘어오지 않아요.
