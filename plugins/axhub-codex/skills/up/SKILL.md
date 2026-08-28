---
name: up
description: 'GitHub 저장소 없이 지금 폴더의 소스를 그대로 올려서 배포해요. 커밋을 만들지 않고 작업 트리를 그대로 업로드하는 lane 이라, 커밋하지 않은 변경이 있어도 그대로 진행해요. 트리거: "GitHub 없이 배포해", "저장소 없이 배포해줘", "이 폴더 그대로 올려줘", "소스 올려서 배포", "레포 안 만들고 배포", "GitHub push 가 막혀서 배포가 안 돼", "레포 권한이 없어서 배포 못 해", "deploy without github", "upload this folder and deploy". 저장소가 연결돼 있고 push 배포가 정상인 평시 "배포해" 는 deploy 로, 빈 폴더 새 앱은 bootstrap 으로, 기존 앱 첫 연결은 import 로, 정적 파일 앱(deploy_method=static)은 deploy 의 static lane 으로 양보해요. 이 트리거는 axhub 맥락(대화의 axhub 언급·현재 폴더의 axhub 연결·직전 axhub 작업)이 있을 때만 유효해요.'
allows-dependency-execution: false
model: sonnet
---
> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.


# Deploy local source via axhub up

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 `bin_path` 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

이 스킬은 이미 axhub 에 연결된 앱의 **로컬 소스 배포**를 맡아요. 소스는 GitHub 커밋이 아니라 지금 폴더를 tar.gz 로 묶은 아카이브이고, 그 뒤 단계(빌드·스캔·기동·verify)는 저장소 배포와 완전히 같은 파이프라인이에요.

**커밋 상태를 게이트로 쓰지 않아요.** 커밋하지 않은 변경이 있는 작업 트리는 이 lane 의 정상 입력이에요 — 커밋을 만들거나 push 하지 않고 지금 파일을 그대로 올려요.

## 승인 게이트 계약 (요약)

절단이 있는 host 는 이 본문을 앞에서부터 일부만 읽어요. 아래 게이트는 뒤쪽 절차 설명이 잘려도 그대로 지켜요.

1. 실행 승인 — preview 카드를 보여준 뒤 `axhub 로 지금 이 폴더를 올려서 배포할까요?` 로 한 번만 묻고, 이 승인 하나가 axhub 진입 확인을 겸해요. 승인 전에는 `--execute` 를 실행하지 않아요.
2. 승인 방식 — 네이티브 선택 UI 가 없으면 같은 확인을 명시 텍스트 승인 1회로 받아요. preview 를 본 뒤 사용자가 새로 입력한 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요.
3. 빈 답변 = 미승인 — 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.
4. 승인 채널이 없는 headless 에서는 dry-run 에서 멈춰요 — 승인을 조용히 건너뛰지 않아요.

**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.

**장기 대기.** codex 는 긴 명령을 최대 30초에 yield 하고 백그라운드 터미널로 넘겨요 — yield 는 실패도 완료도 아니에요. `deploy verify --wait` 가 yield 되면 같은 명령을 다시 실행하지 말고 같은 터미널을 빈 입력으로 폴링해 완주를 기다려요. 성공 선언 규칙은 그대로예요.

## 승인 게이트 계약 (요약)

codex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 승인 방식은 명시 텍스트 승인 1회예요 — preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

## First Visible Sentence

사용자가 로컬 소스 배포를 말하면(`GitHub 없이 배포해`, `이 폴더 그대로 올려줘`, `소스 올려서 배포`) 첫 visible chat 문장은 정확히 이거예요:

`올릴 소스를 확인할게요.`

`deploy` 의 첫 문장(`배포 준비를 확인할게요.`)과 다르게 둬서, 사용자가 어느 lane 에 있는지 첫 줄에서 알 수 있게 해요.

그다음 한국어 제목 `소스 배포 준비 확인` 으로 tool call 하나를 실행해요. 이 첫 명령 전에는 설치·플러그인·앱·git·curl probe 를 하지 않아요.

`deploy`·`bootstrap` 에서 넘어온 경우에는 이 첫 문장과 probe 0 규칙을 적용하지 않아요 — 이미 진행 중인 흐름이라 넘겨받은 `APP_ID` 로 2단계(미리보기)부터 시작하고 1단계는 건너뛰어요.

## Tool Authority

이 스킬은 **CLI 전용**이에요. preflight·preview·실행·verify·진단 라우팅은 전부 아래 `axhub` 명령으로 해요. 세션에 MCP 배포 도구(`deployment_trigger`·`deployment_status` 등)가 보여도 호출하지 않아요.

`axhub plugin-support deploy-preview-summary` 와 `deploy-approved-run` 은 **쓰지 않아요.** 두 명령은 작업 트리가 dirty 하면 exit 64 로 끊는데, 그 상태가 이 lane 의 정상 입력이라 여기서는 잘못된 게이트예요.

## Headless Contract

Headless 는 `codex exec`, CI, TTY 없음, 또는 사용자 확인 채널이 아예 없는 상태예요.

- Headless = 명시 텍스트 승인 0회. 선택지를 렌더링하고 멈추지도 않아요.
- Headless 안전 기본값은 dry-run 이에요. `--execute` 를 실행하지 않아요.
- Headless 에서도 non-mutating CLI·auth·dry-run 확인은 실행해 자동 QA 가 실제 동작을 보게 해요.

## User-Facing Language

- tool 제목은 한국어 명사구만 써요 — `소스 배포 준비 확인`, `올릴 내용 확인`, `소스 배포 실행`, `배포 결과 확인`.
- raw deployment id·exit 번호·영어 진행어를 사용자에게 노출하지 않아요. 기술 실패는 한국어로 번역해서 보여줘요.
- URL 은 평문 절대 URL 로만 보여줘요.
- 마지막 메시지는 한국어 한 줄 요약 + 다음 행동이에요.

## 진입 가드 (AP-11)

이 스킬의 트리거는 **axhub 맥락**(대화의 axhub 언급·현재 폴더의 axhub 연결 manifest·직전 axhub 작업)이 있을 때만 유효해요. 맥락이 없는 일반 발화("이 폴더 올려줘")는 실행·안내로 밀어붙이지 않고 "이 폴더는 axhub 에 연결돼 있지 않아요. axhub 로 배포하려는 거예요?" 를 한 번만 묻고, 아니라는 답이면 종료해요. 다른 axhub 스킬로 넘기지 않아요. headless 는 묻지 않고 조용히 멈춰요.

## 양보 (AP-7)

자기 담당 밖의 요청은 담당 스킬로 넘겨요(양보). 요청이 섞여 있으면 자기 몫만 끝내고 나머지를 넘겨요.

| 상황 | 넘길 곳 |
|---|---|
| 빈 폴더에서 새 앱 만들기 | `bootstrap` |
| 앱이 아직 없거나 첫 연결·첫 배포 | `import` |
| 저장소가 연결돼 있고 push 배포가 정상인 평시 배포 | `deploy` |
| `deploy_method=static` 정적 파일 앱 | `deploy` 의 static lane |
| CLI·플러그인 버전 업데이트 | `update` |
| 배포 실패 원인 진단 | `diagnosis` |
| 로그·환경변수·롤백·GitHub 재연결 | `clarity` |

사용자가 로컬 소스 배포를 **명시**하면 `github_connected` 가 true 여도 이 스킬이 이겨요. `deploy` 로 되돌리는 역방향 양보는 소스 업로드를 명시하지 않은 평시 배포 발화에만 적용해요.

## 1단계 — preflight

```bash
axhub plugin-support deploy-prep --intent deploy --json
```

이 명령은 `deploy-preview-summary` 와 같은 envelope 를 커밋 게이트 없이 줘요. 결과에서 읽는 값은 `preflight.auth_ok`, `preflight.cli_too_old`, `preflight.capabilities`, `resolve.app_id`, `github_connected`, `in_flight_deploy`, `bootstrap_plan` 이에요.

분기:

| 관찰 | 행동 |
|---|---|
| `auth_ok` false | `axhub 로그인이 필요해요.` 한 줄 뒤 auth 복구로 안내하고 멈춰요. |
| `cli_too_old` true | `update` 로 보내고 멈춰요. |
| `bootstrap_plan` 존재, 또는 `resolve.app_id` 없음 | 앱이 아직 없다는 뜻이에요. 비어 있지 않은 폴더는 `import`, 빈 폴더는 `bootstrap` 으로 양보해요. |
| `in_flight_deploy` 존재 | `이미 진행 중인 배포가 있어요. 그 배포를 계속 확인할게요.` 한 줄 뒤 `in_flight_deploy.id` 를 `DEPLOY_ID` 로 바인딩하고 5단계로 바로 가요. 새 배포를 시작하지 않아요 — `deploy-prep` 은 이 상태를 exit code 로 알리지 않으니 여기서 직접 막아요. |
| exit 64 (`validation.quality_gate_failed`) | 품질 확인에서 막힌 항목이 있다는 뜻이에요. 실행하지 않고 사유를 한국어로 알린 뒤 멈춰요. |
| 현재 폴더에 `axhub.yaml`·`apphub.yaml` 이 둘 다 없음 | `이 폴더에 axhub 매니페스트가 없어서 어떤 앱의 소스인지 확인할 수 없어요.` 로 멈추고 `import` 로 양보해요. `axhub up` 은 패킹 전에 매니페스트를 보지 않으니 이 확인은 여기서 해요. |

`APP_ID` 는 `resolve.app_id` 로 바인딩해요. tenant 와 profile 은 플래그로 넘기지 않아요 — CLI 가 `AXHUB_TENANT`·`AXHUB_PROFILE` 환경변수를 직접 읽어요. 셸 변수로만 잡아 둔 값은 넘어가지 않으니, 명시 스코프가 필요하면 그 환경변수를 설정해요.

### static 앱 확인

`APP_ID` 를 잡은 직후 배포 방식을 확인해요:

```bash
axhub apps get "$APP_ID" --no-input --field-expr '.deploy_method // empty'
```

`static` 이면 이 스킬을 멈추고 `deploy` 의 static lane 으로 양보해요 — 정적 파일 앱은 release 기반이라 `axhub deploy verify` 가 404 를 주고, 성공 선언 경로 자체가 달라요. `up` 은 deployment-record 를 만드는 lane 만 소유해요.

## 2단계 — 올릴 내용 미리보기

```bash
axhub up --app "$APP_ID" --path . --dry-run --json
```

dry-run 은 앱 resolve·인증·네트워크 이전에 패킹만 하고 끝나요. 반환된 `target` 문자열에서 파일 수·압축 크기·source 버전을 읽어 preview 카드를 만들어요. 이 값은 **승인 시점의 스냅샷**이에요 — `--execute` 는 폴더를 다시 패킹하므로, 승인 뒤 파일을 고치면 카드와 다른 소스가 올라가요.

올라가는 것은 `.gitignore` 를 존중한 현재 폴더예요. `.git/`·`node_modules/`·`.venv/`·`__pycache__/`·`.env` 계열은 항상 빠지고 `.env.example` 류는 남아요. 커밋은 보내지 않고, 연결된 저장소가 있어도 건드리거나 끊지 않아요.

**dry-run 이 실패하면 preview 카드를 만들지 않아요.** 업로드 상한 초과, 단일 파일 상한 초과, 제외 규칙으로 파일 0개, `--path` 폴더 없음 — 어느 경우든 CLI 가 준 한국어 사유를 그대로 전한 뒤 멈춰요. 재시도하거나 `--execute` 로 넘어가지 않아요.

CLI 0.29.0 미만이면 이 명령이 unknown command 로 끝나요. 그때는 `update` 로 보내고 멈춰요 — `axhub deploy create` 로 대체하지 않아요. 로컬 커밋은 AxHub 가 저장소에서 찾지 못해 commit-not-found 로 실패해요.

## 3단계 — 승인 (AP-12)

interactive 에서는 preview 카드 하나가 **axhub 진입 확인**을 겸해요. 카드에 앱, 환경, 파일 수, 압축 크기, source 버전, 예상 소요를 보여주고 `axhub 로 지금 이 폴더를 올려서 배포할까요?` 를 한 번만 물어요. 환경은 `운영` 으로 표시하고 `prod`·`production` 같은 raw 값을 쓰지 않아요.

명시 텍스트 승인 1회만 받아요. 승인 채널 없는 headless 에서는 execute하지 않아요 — 승인을 조용히 건너뛰지 않아요. headless 는 dry-run preview 까지만 하고 `--execute` 로 넘어가지 않아요. slash 호출도 이 카드를 건너뛰지 못해요. 이 preview-confirm 은 `deploy` 와 같은 mutation 에 대한 같은 게이트이고, 소스만 달라요.

`취소` 면 종료해요.

안내 문구는 상황에 따라 갈라요:

- 저장소 없음: `이 앱은 저장소 없이 소스를 올려서 배포해요.`
- GitHub 차단: `GitHub 쪽이 막혀서, 지금 폴더의 소스를 그대로 올려서 배포할게요. GitHub 는 이후 다시 연결할 수 있어요.`

## 4단계 — 실행

승인 뒤 인증 상태를 한 번 확인해요. 제목은 `인증 상태 확인` 으로 보여주고 token-gate 라는 말을 쓰지 않아요.

```bash
AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate
```

exit 0 이면 계속하고, exit 65 면 auth 복구로 가요. `AXHUB_AUTH_BG_REFRESH=0` 이면 이 게이트를 건너뛰어요.

승인과 실행 사이에 폴더가 바뀌었을 수 있으니 2단계 dry-run 을 한 번 더 돌려 source 버전을 카드 값과 대조해요. 다르면 실행하지 않고 `승인하신 뒤 폴더 내용이 바뀌었어요. 지금 올라갈 소스가 보여드린 것과 달라서 다시 확인할게요.` 로 2단계 preview 부터 다시 해요.

그다음 실행해요:

```bash
axhub up --app "$APP_ID" --path . --execute --field-expr '.id // .deployment_id // empty'
```

**이 명령은 실패로 보여도 다시 실행하지 않아요.** 폴더가 크면 업로드가 도구 타임아웃보다 오래 걸려, 배포는 이미 시작됐는데 출력만 잘려 돌아올 수 있어요. 그때 재실행하면 같은 소스로 프로덕션 배포가 하나 더 생겨요. 출력이 잘리거나 타임아웃이면 `deploy list --app "$APP_ID"` 로 방금 만들어진 배포가 있는지 먼저 확인하고, 있으면 그 id 를 `DEPLOY_ID` 로 써요. exit 64 에 `deployment_in_progress` 가 있으면 실패가 아니라 진행 중이라는 뜻이라 5단계로 가요.

결과 모양이 `deploy create` 와 같아서 이후 단계는 저장소 배포와 동일해요. `DEPLOY_ID` 는 이 출력 또는 1단계의 `in_flight_deploy.id` 에서만 바인딩해요. id 를 못 받으면 성공을 선언하지 않고 `배포 시작은 확인했지만 결과 확인 id 를 못 받았어요. 자동으로 지켜볼 id 가 없어 여기서 멈출게요.` 라고 말한 뒤 멈춰요.

## 5단계 — 성공 확인

성공 선언은 바인딩된 id 로 `axhub deploy verify` 를 실행해서만 해요. 시작 명령의 화면 출력, 중간 상태 조회, "가장 최근 배포" 재탐색 같은 간접 근거로는 성공이라고 말하지 않아요.

preflight 의 `capabilities.import.verify_wait` 가 true 면 **권한 카드 한 번으로 끝나는** 단일 대기 호출을 정확히 한 번만 실행해요:

```bash
axhub deploy verify "$DEPLOY_ID" --app "$APP_ID" --wait --wait-interval 20s --wait-timeout 10m --json
```

호출 직전에 `아직 빌드 중이에요. 같은 배포를 계속 확인할게요.` 한 줄만 남겨요. 이 한 호출이 성공·실패·예산 제한까지 책임지므로 같은 verify 를 반복 호출하거나 `apps get`·`deploy list`·`deploy status` 같은 사후 확인을 덧붙이지 않아요.

`verify_wait` capability 가 없는 구 CLI 에서만 `--wait` 없는 개별 호출을 반복해요. 이때도 스킬 레벨 **폴링 예산**이 적용돼요 — **최대 30회 또는 10분** 중 먼저 닿는 쪽까지만이에요. `sleep`·shell loop·pipe·command substitution 으로 묶지 않고 독립 tool call 로 실행해요. 짧은 독립 호출 사이에는 host 가 지원하면 실제 대기 수단(예약 재확인)을 써요. 대기 수단이 없으면 30회를 연달아 쓰지 말고 확인 가능한 횟수까지만 하고 재개 요약으로 끝내요.

예산에 닿으면 실패로 선언하지 않아요. `아직 진행 중이에요. 여기서 실패로 보지 않고, 제가 확인 가능한 범위까지는 같은 배포를 지켜봤어요.` 로 재개 요약을 남기고 `DEPLOY_ID` 를 보존해요. 사용자에게 다시 상태 확인을 요청하지 않아요.

verify exit:

| exit | 뜻 | 행동 |
|---|---|---|
| 0 | 최종 성공 | 한국어로 요약하고 접근 가능한 URL 을 평문으로 보여줘요. `url_checked=false` 면 `apps get` 으로 `access_url` 을 읽고 제한된 횟수의 HTTPS HEAD 재확인을 한 뒤에 열린다고 말해요. |
| 4 | 인증 만료 | auth 복구 문구를 써요. |
| 5 | 알 수 없는 배포 id | 멈춰요. 최신 배포를 재탐색하지 않아요. |
| 6 | 아직 진행 중 | 위 예산 규칙대로 처리해요. 새 승인 카드를 띄우지 않아요. |
| 7 | 최종 실패 | `배포가 실패했어요. 지금부터 원인 진단만 읽기 전용으로 확인할게요. 재배포나 롤백은 하지 않아요.` 뒤 `diagnosis` 로 인계해요. |

`diagnosis` 인계는 같은 앱 식별자와 실패 근거를 유지하고, 재배포·롤백·새 배포 생성을 실행하지 않아요.

## 결과 안내

결과가 나온 **뒤에** 상황에 맞는 것만 알려줘요.

- 저장소 없음: 이 앱은 폴더를 올려서 배포하니 push 자동 배포가 없고, 나중에 `axhub apps git connect` 로 저장소를 붙여도 앱을 다시 만들 필요는 없어요.
- GitHub 차단: 이번 배포만 저장소에서 오지 않았으니 push 자동 배포와 버전 이력이 이 건을 덮지 않고, GitHub 이 풀리면 다음 배포는 저장소 경로로 돌아가요.

## 금지

- `axhub plugin-support deploy-preview-summary` / `deploy-approved-run` 호출 금지 — 커밋 게이트가 이 lane 을 잘못 막아요.
- 커밋 생성·`git add`·`git push`·`.gitignore` 수정 금지. 사용자의 저장소를 대신 정리하지 않아요.
- `axhub deploy create` 로 대체 금지. 로컬 커밋은 저장소에서 찾을 수 없어요.
- `axhub up --execute` 재실행 금지 — 같은 소스라도 새 프로덕션 배포가 하나 더 생겨요.
- `axhub deploy watch` / `deploy status --watch` 호출 금지 — Desktop·non-TTY 에서 저하돼요.
- MCP 배포 mutation 도구 호출 금지.
- verify 전 성공 선언 금지.

분기별 상세는 [references/workflow-details.md](references/workflow-details.md) 를 읽어요.
