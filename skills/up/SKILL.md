---
name: up
description: 'GitHub 저장소 없이 지금 폴더의 소스를 그대로 올려서 배포해요. 커밋을 만들지 않고 작업 트리를 그대로 업로드하는 lane 이라, 커밋하지 않은 변경이 있어도 그대로 진행해요. 트리거: "GitHub 없이 배포해", "저장소 없이 배포해줘", "이 폴더 그대로 올려줘", "소스 올려서 배포", "레포 안 만들고 배포", "deploy without github", "upload this folder and deploy". 저장소가 연결돼 있고 push 배포가 정상인 평시 "배포해" 는 deploy 로, 빈 폴더 새 앱은 bootstrap 으로, 기존 앱 첫 연결은 import 로, 정적 파일 앱(deploy_method=static)은 deploy 의 static lane 으로 양보해요. 이 트리거는 axhub 맥락(대화의 axhub 언급·현재 폴더의 axhub 연결·직전 axhub 작업)이 있을 때만 유효해요.'
examples:
  - utterance: "GitHub 없이 이 폴더 그대로 올려줘"
    intent: "deploy the local working tree without a git repository"
  - utterance: "저장소 없이 배포해줘"
    intent: "deploy the local working tree without a git repository"
  - utterance: "소스 올려서 배포해줘"
    intent: "deploy the local working tree without a git repository"
  - utterance: "deploy this folder without github"
    intent: "deploy the local working tree without a git repository"
allows-dependency-execution: false
model: sonnet
---

# Deploy local source via axhub up

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 `bin_path` 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

이 스킬은 이미 axhub 에 연결된 앱의 **로컬 소스 배포**를 맡아요. 소스는 GitHub 커밋이 아니라 지금 폴더를 tar.gz 로 묶은 아카이브이고, 그 뒤 단계(빌드·스캔·기동·verify)는 저장소 배포와 완전히 같은 파이프라인이에요.

**커밋 상태를 게이트로 쓰지 않아요.** 커밋하지 않은 변경이 있는 작업 트리는 이 lane 의 정상 입력이에요 — 커밋을 만들거나 push 하지 않고 지금 파일을 그대로 올려요.

## First Visible Sentence

사용자가 로컬 소스 배포를 말하면(`GitHub 없이 배포해`, `이 폴더 그대로 올려줘`, `소스 올려서 배포`) 첫 visible chat 문장은 정확히 이거예요:

`올릴 소스를 확인할게요.`

`deploy` 의 첫 문장(`배포 준비를 확인할게요.`)과 다르게 둬서, 사용자가 어느 lane 에 있는지 첫 줄에서 알 수 있게 해요.

그다음 한국어 제목 `소스 배포 준비 확인` 으로 tool call 하나를 실행해요. 이 첫 명령 전에는 설치·플러그인·앱·git·curl probe 를 하지 않아요.

## Tool Authority

이 스킬은 **CLI 전용**이에요. preflight·preview·실행·verify·진단 라우팅은 전부 아래 `axhub` 명령으로 해요. 세션에 MCP 배포 도구(`deployment_trigger`·`deployment_status` 등)가 보여도 호출하지 않아요.

`axhub plugin-support deploy-preview-summary` 와 `deploy-approved-run` 은 **쓰지 않아요.** 두 명령은 작업 트리가 dirty 하면 exit 64 로 끊는데, 그 상태가 이 lane 의 정상 입력이라 여기서는 잘못된 게이트예요.

## Headless Contract

Headless 는 `claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, TTY 없음, 또는 사용자 확인 채널이 아예 없는 상태예요.

- Headless = AskUserQuestion 0회. 선택지를 렌더링하고 멈추지도 않아요.
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
| `in_flight_deploy` 존재 | `이미 진행 중인 배포가 있어요.` 를 알리고, 그 배포를 계속 볼지 새 배포를 시작할지 **한 번 묻고 멈춰요.** `deploy-prep` 은 이 상태를 exit code 로 알리지 않으니 여기서 직접 막아요. headless 는 묻지 않고 진행 중 배포를 지켜보는 쪽으로 멈춰요. |
| exit 64 (`validation.quality_gate_failed`) | 품질 확인에서 막힌 항목이 있다는 뜻이에요. 실행하지 않고 사유를 한국어로 알린 뒤 멈춰요. |

`APP_ID` 는 `resolve.app_id` 로 바인딩해요.

### static 앱 확인

`APP_ID` 를 잡은 직후 배포 방식을 확인해요:

```bash
axhub apps get "$APP_ID" --no-input --field-expr '.deploy_method // empty'
```

`static` 이면 이 스킬을 멈추고 `deploy` 의 static lane 으로 양보해요 — 정적 파일 앱은 release 기반이라 `axhub deploy verify` 가 404 를 주고, 성공 선언 경로 자체가 달라요. `up` 은 deployment-record 를 만드는 lane 만 소유해요.

## 2단계 — 올릴 내용 미리보기

```bash
axhub up --app "$APP_ID" --path . --tenant "$AXHUB_TENANT" --dry-run --json
```

dry-run 은 앱 resolve·인증·네트워크 이전에 패킹만 하고 끝나요. 반환된 `target` 문자열에서 파일 수·압축 크기·source 버전을 읽어 preview 카드를 만들어요. 이 값은 **승인 시점의 스냅샷**이에요 — `--execute` 는 폴더를 다시 패킹하므로, 승인 뒤 파일을 고치면 카드와 다른 소스가 올라가요.

올라가는 것은 `.gitignore` 를 존중한 현재 폴더예요. `.git/`·`node_modules/`·`.venv/`·`__pycache__/`·`.env` 계열은 항상 빠지고 `.env.example` 류는 남아요. 커밋은 보내지 않고, 연결된 저장소가 있어도 건드리거나 끊지 않아요.

**dry-run 이 실패하면 preview 카드를 만들지 않아요.** 업로드 상한 초과, 단일 파일 상한 초과, 제외 규칙으로 파일 0개, `--path` 폴더 없음 — 어느 경우든 CLI 가 준 한국어 사유를 그대로 전한 뒤 멈춰요. 재시도하거나 `--execute` 로 넘어가지 않아요.

CLI 0.29.0 미만이면 이 명령이 unknown command 로 끝나요. 그때는 `update` 로 보내고 멈춰요 — `axhub deploy create` 로 대체하지 않아요. 로컬 커밋은 AxHub 가 저장소에서 찾지 못해 commit-not-found 로 실패해요.

## 3단계 — 승인 (AP-12)

interactive 에서는 preview 카드 하나가 **axhub 진입 확인**을 겸해요. 카드에 앱, 환경, 파일 수, 압축 크기, source 버전, 예상 소요를 보여주고 `axhub 로 지금 이 폴더를 올려서 배포할까요?` 를 한 번만 물어요. 환경은 `운영` 으로 표시하고 `prod`·`production` 같은 raw 값을 쓰지 않아요.

네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. headless 는 dry-run preview 까지만 하고 `--execute` 로 넘어가지 않아요. slash 호출도 이 카드를 건너뛰지 못해요. 이 preview-confirm 은 `deploy` 와 같은 mutation 에 대한 같은 게이트이고, 소스만 달라요.

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

그다음 실행해요:

```bash
axhub up --app "$APP_ID" --path . --tenant "$AXHUB_TENANT" --execute --field-expr '.id // .deployment_id // empty'
```

결과 모양이 `deploy create` 와 같아서 이후 단계는 저장소 배포와 동일해요. `DEPLOY_ID` 는 이 출력에서만 바인딩해요. id 를 못 받으면 성공을 선언하지 않고 `배포 시작은 확인했지만 결과 확인 id 를 못 받았어요. 자동으로 지켜볼 id 가 없어 여기서 멈출게요.` 라고 말한 뒤 멈춰요.

## 5단계 — 성공 확인

성공 선언은 바인딩된 id 로 `axhub deploy verify` 를 실행해서만 해요. 시작 명령의 화면 출력, 중간 상태 조회, "가장 최근 배포" 재탐색 같은 간접 근거로는 성공이라고 말하지 않아요.

preflight 의 `capabilities.import.verify_wait` 가 true 면 **권한 카드 한 번으로 끝나는** 단일 대기 호출을 정확히 한 번만 실행해요:

```bash
axhub deploy verify "$DEPLOY_ID" --app "$APP_ID" --wait --wait-interval 20s --wait-timeout 10m --json
```

호출 직전에 `아직 빌드 중이에요. 같은 배포를 계속 확인할게요.` 한 줄만 남겨요. 이 한 호출이 성공·실패·예산 제한까지 책임지므로 같은 verify 를 반복 호출하거나 `apps get`·`deploy list`·`deploy status` 같은 사후 확인을 덧붙이지 않아요.

`verify_wait` capability 가 없는 구 CLI 에서만 `--wait` 없는 개별 호출을 반복해요. 이때도 스킬 레벨 **폴링 예산**이 적용돼요 — **최대 30회 또는 10분** 중 먼저 닿는 쪽까지만이에요. `sleep`·shell loop·pipe·command substitution 으로 묶지 않고 독립 tool call 로 실행해요.

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
- `axhub deploy watch` / `deploy status --watch` 호출 금지 — Desktop·non-TTY 에서 저하돼요.
- MCP 배포 mutation 도구 호출 금지.
- verify 전 성공 선언 금지.

분기별 상세는 [references/workflow-details.md](references/workflow-details.md) 를 읽어요.
