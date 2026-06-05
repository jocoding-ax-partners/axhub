# Onboarding (setup 스킬 진화) — CEO Review Plan

> `/plan-ceo-review` 산출물. Mode: **SELECTIVE EXPANSION** (setup 현 스코프 = baseline, 추가분 cherry-pick).
> 결정일: 2026-06-05 · Branch: main · Reviewer: Claude (Opus 4.8)

## TL;DR

Node.js 바이브코더가 "설치 → 로그인 → 개발 시작" 을 한 흐름으로 끝내도록 **기존 `setup` 스킬을 진화**해요.
새 `onboarding` 스킬을 신설하지 **않아요** — `setup` 이 이미 온보딩 오케스트레이터고
`온보딩`·`getting started`·`setup` 트리거 어구를 소유하고 있어서, 병렬 스킬은 keyword baseline 을
깨고 첫사용 스킬 2개가 트리거를 두고 경쟁해요 (DRY 위반). 사용자 의도도 "리팩토링" 이에요.

**제품 계약:** 사용자는 `온보딩`, `처음인데 뭐부터`, `getting started` 한 마디만 하면 돼요. 내부 slug 는
`setup` 으로 유지하지만, 사용자 경험상 이 스킬이 **온보딩 단일 진입점**이에요. 온보딩 중 사용자가 직접
알아야 하는 sibling skill 이름(`install-cli`/`auth`/`github`/`init`/`deploy`/`doctor`)은 없어야 해요.
setup 이 감지→위임→재감지→다음 gap 을 끝까지 닫고, 마지막에는 **VIBE_READY 카드**를 보여줘요.

**100% 의 의미:** 안전한 자동화로 닫을 수 있는 모든 gap 을 온보딩 스킬 안에서 닫아요. 단, 원격 mutation,
브라우저 승인, 시스템 설치, 의존성 설치처럼 사용자 consent 가 필요한 단계는 카드 안에서 한 번씩 묻고,
승인 뒤에는 에이전트가 계속 진행해요. 사용자가 "무슨 명령을 쳐야 하지?" 상태로 남으면 실패예요.

## North Star: VIBE_READY Contract

온보딩의 종료 조건은 "설치 끝" 이 아니라 **바로 바이브코딩 가능한 상태**예요. `setup` 은 아래 조건 중
자동/consent 가능한 것은 모두 끝내고, 불가능한 것은 같은 스킬 안에서 복구/재개 문구까지 제공해요.

| 영역 | VIBE_READY 조건 | 실패 시 계약 |
|------|------------------|--------------|
| CLI | `axhub --version` 성공 + 최소 지원 버전 이상 | `install-cli` 또는 `update` 위임 후 재감지 |
| Plugin | 현재 플러그인 버전과 CLI 호환 range 를 surface | 중간 업데이트 금지, 끝단 advisory + 새 세션 안내 |
| Auth | `axhub-helpers preflight --json` 의 `auth_ok=true` | `auth` 위임, device flow 코드는 즉시 surface, 승인 후 재감지 |
| Git | `git --version` 성공 | consent 설치 또는 수동 설치 링크, init/github 진입 전 차단 |
| Node | node 존재 + `.nvmrc`/`engines.node` 권장 range 와 일치 또는 교정됨 | consent 교정, 실패 시 정확한 수동 경로 |
| GitHub App | 계정레벨 GitHub App 설치 완료 | `install_url` surface, 설치 후 `accounts list` 재검증 |
| Repo/App | 새 사용자: init saga 로 app+repo+clone+첫 deploy 완료. 기존 repo: github skill 로 app↔repo 연결 | clone 충돌 방지, 기존 repo 는 init 금지 |
| Dependencies | repo on disk 뒤 lockfile 기반 install 완료, 모든 install 은 `--ignore-scripts` | 실패해도 원인+다음 말할 phrase 제공, postinstall 자동 실행 금지 |
| Deploy evidence | init saga URL 또는 기존repo deploy 결과를 status/watch 로 확인 | 재배포 금지, status/logs 기반 복구 phrase |
| Doctor | 최종 `doctor` 통합 점검에서 PATH/helper/auth/profile 핵심 green | PATH reload / 재실행 phrase 제공 |

**VIBE_READY 카드 예시(최종 UX):**

```text
axhub 온보딩 완료예요.
  ✓ CLI vX.Y.Z
  ✓ 로그인 <masked-email>
  ✓ git vA.B.C
  ✓ node vN.N.N (pm: bun|pnpm|npm|yarn)
  ✓ GitHub App 설치됨
  ✓ 앱 <app-slug> 연결됨
  ✓ 첫 배포 live: <deployment-url>
  ✓ doctor 점검 통과

이제 바로 코딩하면 돼요.
다음에 말할 수 있는 것: "배포해", "로그 봐줘", "환경변수 추가해줘", "테이블 추천해줘"
```

**Ready 상태 등급:**

- `VIBE_READY` — 위 조건이 모두 green. 온보딩 성공.
- `READY_WITH_USER_ACTION` — 브라우저 승인, OS installer GUI, PATH reload 처럼 에이전트가 대신 완료할 수
  없는 외부 행동만 남음. 반드시 "승인했어/다시 셋업해줘" 같은 자연어 재개 문구를 제공해요.
- `SAFE_STOP_NONINTERACTIVE` — `claude -p`/CI/headless 에서 mutation 이 필요한 경우. 자동 설치/생성 없이
  상태 요약만 출력하고 멈춰요.
- `BLOCKED_UNSUPPORTED` — OS/패키지매니저/권한 제약으로 안전 경로가 없을 때. 수동 링크와 재개 문구를
  제공해요.

## 0. Premise Challenge (왜 이 문제, 왜 이 방식)

- **Q: 새 스킬이 맞나?** → 아니요. `setup` 이 이미 detect-first 오케스트레이터예요. 신규 가치는
  *버전체크 레이어* + *GitHub 연결 전진배치* 두 개뿐. 나머지는 setup 이 이미 함.
- **Q: setup 이 god-skill 되지 않나?** → 핵심 규율: setup 은 **오케스트레이션만** 해요. 모든 로직은
  기존 스킬에 위임 (`install-cli`/`auth`/`update`/`upgrade`/`github`/`init`/`doctor`/`deploy`).
  setup 이 직접 소유하는 건 (a) phase 순서, (b) node 런타임, (c) 신규 의존성 설치뿐.
- **Q: 아무것도 안 하면?** → 바이브코더가 git 연결(GitHub App 설치 + OAuth 승인)에서 컨텍스트 전환을
  여러 번 하다 이탈. 이게 사용자가 지목한 #1 pain.

## 1. What Already Exists (위임 맵 — 재사용)

```
온보딩 오케스트레이터 (= 진화된 setup)
 ├ CLI 설치            → install-cli   (공식 installer 단일채널)
 ├ CLI 버전체크/적용    → update        [신규 연결] cosign 검증, 자동 apply ✓
 ├ 플러그인 버전체크    → upgrade       [신규 연결] 끝단 advisory-only (자동 ✗, 새 세션 필요)
 ├ 로그인              → auth          (device flow, headless token)
 ├ git 런타임          → setup 직접     [신규] consent 설치 (clone/push 전제조건)
 ├ node 런타임         → setup 직접     (consent 설치) + [신규] 버전 자동교정
 ├ GitHub 연결 전진배치 → github(재사용)  [신규 순서] App 설치(install_url)만 auth 뒤 1회
 ├ 의존성 설치         → setup 직접     [신규] repo 생긴 뒤 pm 감지 + consent install --ignore-scripts
 ├ 새 앱(+repo+배포)    → init          (bootstrap saga — repo+첫배포 자동, deploy 재호출 X)
 ├ 환경 진단           → doctor        [신규 연결] 마무리 종합점검
 └ 배포 마무리         → deploy        [신규] init 경로=saga URL surface / 기존repo 경로만 deploy
```

**버전체크 핵심 사실:** 요구사항 "cli/플러그인 버전 확인 + 안 맞으면 업데이트" 는 타겟이 둘로 갈려요.
- `update` = CLI 바이너리 — cosign 검증 후 **자동 apply 가능**.
- `upgrade` = 플러그인 self — Claude Code 가 `/plugin update` 를 처리, 스킬은 **자동 실행 불가** (안내만).
- node = `.nvmrc`/`engines` advisory — 현재 경고만, 신규로 자동교정 추가.

## 2. Scope

### Accepted (이번 PR)
0. **온보딩 단일 진입점 계약** — `setup` 을 사용자-facing onboarding skill 로 강화. 사용자는 sibling skill
   이름이나 slash command 를 몰라도 돼요. 모든 gap 은 setup 이 detect-first 로 찾아 위임하고, 위임 후
   반드시 재감지해 다음 gap 으로 이어가요. 최종 출력은 `VIBE_READY`/`READY_WITH_USER_ACTION`/
   `SAFE_STOP_NONINTERACTIVE`/`BLOCKED_UNSUPPORTED` 중 하나여야 해요.
1. **버전체크 phase 신설** — CLI(`update`)는 Phase A(앞단), 플러그인(`upgrade`)은 advisory-only(끝단).
   CLI mismatch 는 consent 후 cosign self-replace. **플러그인 mismatch 는 온보딩 중 적용 안 함** —
   `/plugin update` 는 새 세션을 요구해 흐름을 리셋하므로 맨 끝 ready 카드에서 한 줄 안내만 (eng
   review #1). "버전 안 맞으면" 은 두 뜻: (a) 최신 가용 → 업데이트, (b) **플러그인↔CLI 호환 skew**.
   단 `upgrade` 는 `CLI 호환 vMIN~vMAX` range 를 **surface(advisory)만 하고 block 안 해요** (codex #8) —
   호환 안 되는 조합도 막지 않으니, 온보딩은 skew 를 advisory 로 안내하고 recovery-flows "version-skew §3b"
   로 라우팅. (incompatible 조합 hard-block 은 의도적으로 안 함 — 첫사용 차단 과함.)
2. **GitHub App 설치 전진배치 (install_url 만)** — auth 확정 후 `axhub github accounts list --json` 로
   App 설치 감지. 미설치면 **`install_url` 로 계정레벨 App 설치만** init 전 완료. ⚠️ **OAuth device-flow
   인가는 전진배치 불가** (codex #1) — `apps git connect`(APP_ID 필요)에 묶여 connect 단계에 남아요.
   app↔repo connect 도 init saga(새앱)/github(기존repo)에 그대로. 효과 = init saga 의 connect 가
   install_url 단계에서 안 멈춤 → **non-stall 검증 필수** (T3). OAuth device 승인은 connect 시 1회 불가피.
3. **git/node 런타임 보장** — detect-first 에 `git --version` 추가. git 미설치면 consent 설치
   (macOS Xcode CLT / Windows winget·scoop). git 은 clone/remote/push 전제조건이라 안 깔리면
   GitHub 연결·init saga 가 cryptic 하게 깨져요. **node 버전 mismatch** 는 `.nvmrc`/`engines` 와
   런타임 불일치 시 consent 후 nvm 교정 (현재 setup 은 경고만). git 은 node 에 이은 **2번째
   third-party install 예외** — setup NEVER 의 "node = 유일 예외" 문구도 갱신 필요.
4. **doctor 통합** — 온보딩 끝에 doctor 1회로 PATH/헬퍼/인증 종합점검 (PATH 갱신 갭 조기 포착).
5. **배포 마무리 (재배포 안 함)** — ⚠️ init 의 bootstrap saga 가 **이미 첫 배포 포함** (codex #2,
   init SKILL "deploy 또 부르지 마"). 그래서 init 경로면 saga 가 낸 **배포 URL 을 surface 만** (재배포 X).
   단 URL 만 보여주고 끝내지 말고, saga 가 deployment id/status 를 surface 하면 `status/watch` 로 live
   evidence 를 확인해 VIBE_READY 카드에 넣어요. `deploy` 위임은 init 안 쓴 경로(기존repo connect)에서만.
6. **의존성 설치 (repo 생긴 뒤, --ignore-scripts 필수)** — ⚠️ **타이밍**: fresh 온보딩은 init 이 repo
   clone 한 *뒤*에야 package.json 존재 (codex #3) → dep-install 은 Phase C(repo on disk) 에 배치. pm
   (bun/pnpm/npm/yarn) lockfile 감지 후 consent install. **`--ignore-scripts` 필수** (codex #5 — consent
   로 transitive postinstall 못 막음; native module 빌드 깨지면 사용자에게 명시 후 수동). ⚠️ **하드 제약
   변경**: `allows-dependency-execution: false → true` + skill-doctor 허용목록 등록 + CI 갱신 (§5 R1·T7).
7. **기존 repo 라우팅** — Phase C 에서 'git repo 존재 + 커밋 있음 + manifest 없음' 감지 시 init(새 repo
   saga + clone)은 현재 dir 과 충돌하므로 **차단하고 github 스킬로 라우팅**('앱 만들고 이 repo 연결').
   이미 코딩 시작한 바이브코더의 흔한 케이스 (eng review #3).

### NOT in scope
- **새 `onboarding` 스킬 신설** — 트리거 충돌 + baseline 깨짐으로 기각. setup 진화로 대체.
- **기존 로컬 repo 6단계 ladder 자체 재설계** — github 스킬 그대로 위임. 전진배치는 **계정레벨 App 설치
  (install_url) 레이어만** 최적화 (OAuth device-flow 는 connect 에 불가피하게 남음 — codex #1).
- **streamlined inline git 미니플로우** — DRY 위반(github 스킬과 2벌 관리)으로 기각.
- **무동의 100% 자동화** — "100%" 는 무조건 silent mutation 이 아니에요. 브라우저 승인, OS 설치,
  원격 repo/app 생성, dependency install 은 consent 가 필요해요. 대신 consent 뒤 흐름을 setup 이 이어가요.

## 3. Architecture (phase 순서 + 데이터 흐름)

```
[detect-first: read-only, 위임 전]
   axhub --version / node --version / git --version / (CLI 있으면) helper preflight
        │
        ▼
 ┌─ Phase A: 도구 준비 ──────────────────────────────┐
 │  1. CLI 없음    → install-cli                       │
 │  2. CLI 버전 old → update  (consent → cosign apply)  │
 │  3. 미로그인    → auth                               │
 └────────────────────────────────────────────────────┘
        │  (auth 확정 — 이제 GitHub 쿼리 가능)
        ▼
 ┌─ Phase B: 런타임 + GitHub App ───────────────────┐
 │  4. git 없음   → consent 설치 (Xcode CLT/winget·scoop)│
 │  5. node 없음/mismatch → consent 설치/교정           │
 │  6. GitHub App 미설치 → install_url (계정레벨 App     │  ◀ 전진배치
 │       설치만; OAuth device-flow 는 connect 단계)      │
 └────────────────────────────────────────────────────┘
        │  (App 설치됨 — connect 시 install 단계 안 멈춤)
        ▼
 ┌─ Phase C: 첫 성공경험 (repo 생긴 뒤 의존성) ──────┐
 │  7. 기존 git repo+커밋 & manifest 없음               │
 │       → github 라우팅 (앱 생성+이 repo 연결, init X)  │
 │  8. 빈 dir & manifest 없음 → init (saga=앱+repo+     │
 │       첫배포+clone)                                  │
 │  9. (repo on disk) lockfile+manifest → consent       │
 │       install --ignore-scripts                       │
 │ 10. doctor 종합점검 (PATH 갱신 갭 포착)              │
 │ 11. 배포 마무리: init 경로=saga URL surface(재배포X)  │
 │       / 기존repo 경로=deploy 유도                     │
 │ 12. 플러그인 새 버전 → advisory (/plugin update +    │
 │       새 세션, 끝나고)                                │
 └────────────────────────────────────────────────────┘
```

**왜 GitHub 가 Phase B (맨 앞 아님):** GitHub installation 쿼리는 auth 필수. "전진배치" 는
init/deploy 전에 1회 완료한다는 뜻 — auth 직후. 그래서 setup 은 `needs-preflight: false` 유지
(detect-first; CLI 부재 가능). GitHub 감지는 auth gap 을 채운 *뒤* 조건부 실행.

**전진배치 정밀화 (eng #2 + codex #1):** Phase B 에서 가능한 건 **계정레벨 GitHub App 설치(install_url)만**
이에요 (`axhub github accounts list` = read-only 발견 + install_url emit). **OAuth device-flow 인가는
`apps git connect`(APP_ID 필요, 앱은 Phase C 에서 생김)에 묶여 있어서 전진배치 불가** — connect 단계에
그대로 남아요. 전진배치의 효과는 "init saga 의 connect 가 install_url 단계에서 안 멈춤 (App 이 이미
설치돼 있어서)" — **이 재사용/non-stall 을 T3 에서 검증**해요. App 설치가 friction 의 컨텍스트 전환
주범이라, 이것만 앞당겨도 효과 있어요 (OAuth device 승인은 connect 시 1회 불가피).

### 3.1 Gap State Machine (SKILL 에 그대로 반영)

```text
START
  ↓
DETECT_ALL(read-only)
  ├─ cli_missing         → install-cli → DETECT_ALL
  ├─ cli_old             → update(consent+cosign) → DETECT_ALL
  ├─ auth_missing        → auth(device/token) → DETECT_ALL
  ├─ git_missing         → install_git(consent) → DETECT_ALL
  ├─ node_missing        → install_node(consent) → DETECT_ALL
  ├─ node_mismatch       → fix_node(consent+nvm) → DETECT_ALL
  ├─ github_app_missing  → install_url → DETECT_ALL
  ├─ existing_repo_gap   → github guided setup/connect → DETECT_ALL
  ├─ no_manifest_empty   → init saga(app+repo+deploy+clone) → DETECT_ALL
  ├─ deps_missing        → install_deps(consent+ignore-scripts) → DETECT_ALL
  ├─ deploy_unverified   → status/watch or deploy(existingrepo only) → DETECT_ALL
  ├─ doctor_gap          → doctor → DETECT_ALL
  └─ no_gap              → VIBE_READY_CARD
```

**상태 테이블(테스트가 assert 할 문구 source):**

| gap id | 감지 명령/조건 | 처리 owner | 자동 재개 조건 |
|--------|----------------|------------|----------------|
| `cli_missing` | `axhub --version` 실패 | `install-cli` | `axhub --version` 성공 |
| `cli_old` | `MIN_AXHUB_CLI_VERSION=0.17.3` 미만, preflight `cli_too_old=true`, 또는 `axhub update check --json` 의 `has_update=true` | `update` | cosign apply 후 version 재확인 |
| `auth_missing` | preflight `auth_ok=false` | `auth` | device approval/token import 후 preflight green |
| `git_missing` | `git --version` 실패 | setup | 설치 후 `git --version` 성공 |
| `node_missing` | `node --version` 실패 | setup | 설치 후 `node --version` 성공 |
| `node_mismatch` | `NODE_ACTIVE` 가 `.nvmrc` 또는 `package.json engines.node` 의 `NODE_REQUIRED` 를 만족하지 않음 | setup | target version active |
| `github_app_missing` | `axhub github accounts list --json` 의 `accounts/installations` 에 `installed=true` 또는 `installation_id` 가 없고 `install_url` 이 있음 | setup→github data | install_url 완료 후 accounts 재조회 |
| `existing_repo_gap` | `.git` 있음 + commit 있음 + manifest 없음 | `github` | app↔repo connect 완료 |
| `no_manifest_empty` | manifest 없음 + 빈 dir/신규 흐름 | `init` | manifest+repo+deployment evidence 존재 |
| `deps_missing` | lockfile+manifest 있음, install marker/node_modules 없음 | setup | lockfile install exit 0 |
| `deploy_unverified` | `.axhub/bootstrap.state.json` 의 `app_id`+`last_deploy_id` 는 있으나 `axhub deploy status ... --watch --json` 이 `succeeded/live/running/deployed` 를 못 확인 | setup/status/deploy | live/running/deployed 확인 |
| `doctor_gap` | doctor 핵심 체크 fail | `doctor` | doctor 핵심 green 또는 PATH reload 안내 |

**중요:** state 는 별도 DB 없이 매번 read-only 감지로 재구성해요. device flow, OS installer, PATH reload 로
대화가 끊겨도 사용자가 "승인했어", "다시 셋업해줘", "온보딩 계속" 이라고 말하면 같은 감지 루프가
다음 gap 부터 이어가야 해요.

**구체 predicate lock:** 구현 테스트는 단어 존재만 보지 않고 `S1`~`S14` 시나리오를 첫 gap → 처리 owner →
재감지 → ready 등급으로 시뮬레이션해요. `cli_old` 는 `MIN_AXHUB_CLI_VERSION=0.17.3`, helper
`cli_too_old`, update-check `has_update` 로 묶고, `node_mismatch` 는 `NODE_ACTIVE/NODE_REQUIRED` 를 실제
predicate fixture 로 실행해 exact version 과 `>=20 <23` 같은 upper-bound range 의 false-green 을 막아요.
`github_app_missing` 도 실제 predicate fixture 로 실행해 invalid/empty JSON 은 `github_app_unknown`,
`install_url` 이 있는 미설치 상태만 `github_app_missing` 으로 나누어요. `deploy_unverified` 는
`.axhub/bootstrap.state.json` 의 `app_id`+`last_deploy_id` 와 `status/watch` 결과로 묶어요.

## 4. Workflow Failure Modes (스킬 고유 — 반드시 처리)

markdown 스킬이라 DB/N+1/threat-model 섹션은 N/A. 대신 온보딩이 실제로 깨지는 지점:

| # | 실패 모드 | 처리 (계약) | 사용자가 보는 것 |
|---|-----------|-------------|------------------|
| F1 | subprocess/CI/headless 에서 AUQ 호출 | D1 guard (`! -t 1 \|\| $CI \|\| $CLAUDE_NON_INTERACTIVE`) → registry safe default | (자동설치/연결 안 함, 안내만) |
| F2 | hook 실패 | fail-open, exit 0 보장 (`AXHUB_DISABLE_HOOKS` kill switch) | main 흐름 안 막힘 |
| F3 | Claude Desktop 에 TodoWrite 없음 | TodoWrite 호출 skip, fallback 메시지도 안 만듦 | progress UI 언급 없이 진행 |
| F4 | 설치 직후 `--version` 실패 (PATH 미갱신) | ✗ 로 끝내지 말고 "새 터미널 / 셸 재로드 후 다시" 안내 | 복구 가능한 안내 |
| F5 | headless 에서 브라우저 OAuth | 환경 감지 → token_file 흐름, 브라우저 옵션 숨김 | 토큰 붙여넣기 경로 |
| F6 | device_code 발급 후 에이전트 fast-exit | `verification_uri`+`user_code` 즉시 surface → "승인했어" 받으면 `--resume-last` 로 에이전트가 마무리 | 2단계 카드, 명령 떠넘김 없음 |
| F7 | 플러그인 버전 mismatch | 자동 불가 + 새 세션 필요 → **온보딩 끝단 advisory** 1줄 (`/plugin update`). 중간 적용 금지 (흐름 리셋) | 끝에 "끝나고 업데이트" 안내 |
| F8 | node 자동교정 실패 (pm/nvm 없음) | nodejs.org LTS 안내로 graceful degrade | 수동 다운로드 링크 |
| F9 | 의존성 install 실패 | 빌드 안 막고 경고, pm 명령 직접 실행 안내 | "직접 install 후 알려줘" |
| F10 | git 미설치 (clone/remote/push 직전 cryptic 실패) | detect-first `git --version` 감지 → consent 설치 (macOS Xcode CLT / Windows winget·scoop). node 에 이은 2nd 3rd-party 예외 | "git 설치할게요" 안내 후 진행 |
| F11 | 기존 repo 사용자가 init 진입 (clone 충돌) | Phase C 에서 'repo+커밋+manifest없음' 감지 → init 차단, github 스킬 라우팅 | "기존 repo 감지 — 앱 연결로 안내" |
| F12 | 온보딩 중 sibling skill 이 제어를 안 돌려줌 | ready/user-action 카드에 자연어 재개 phrase 를 항상 포함 | "승인했어" / "온보딩 계속" |
| F13 | lockfile 없는 repo 에서 dep-install 요구 | install skip, package manager 선택을 묻지 않음 | "lockfile 만들고 다시 말해줘" |
| F14 | `--ignore-scripts` 로 native module 미빌드 | VIBE_READY 대신 READY_WITH_USER_ACTION, 이유와 수동 build 안내 | "native build 가 필요해요" |

## 5. Risks / Tradeoffs

- **R1 (의존성 게이트 flip, 中~高):** `allows-dependency-execution: true` 는 임의 코드 실행 벡터를
  첫사용 흐름에 들여요. ⚠️ **정직한 경고**: `npm install`/`bun install` 은 모든 의존성의 `postinstall`
  lifecycle 스크립트를 실행해요 — 이게 npm supply-chain 공격의 *실제* 벡터예요. "canonical 명령만"
  으로는 **안 막혀요**. 완화 (codex #5 — (d) 를 optional→**필수 acceptance criterion** 승격):
  (a) lockfile 있을 때만, (b) consent 필수, (c) D1 guard 로 subprocess 차단, (d) **`--ignore-scripts`
  필수** — postinstall 하드 차단. native module 빌드가 깨지면 사용자에게 명시 후 수동 안내 (consent
  만으로는 transitive postinstall 못 막으니 이게 핵심 가드). **(d) 빼면 feature 자체를 cut.** CI 게이트
  + 테스트 동반 갱신 필요.
- **R2 (god-skill, 中):** phase 12개로 setup 비대. 완화: (a) 전부 위임 — setup 은 순서 + git/node +
  의존성만 소유, (b) **SKILL 에 ASCII 상태머신 + phase/gap state table 명시** (detect→위임→재감지 루프
  가독성, eng review #4), (c) SKILL.md 길이 예산 <800줄.
- **R3 (step-numbering collision, 低):** phase 추가 시 `skill:doctor` FU-3 중복 헤더 검사 통과 필요.
- **R4 (git system-install 정책 부재, 中, codex #6):** git 설치(Xcode CLT/winget/scoop)는 **system-level
  mutation** 이라 JS 의존성 게이트(skill-doctor `allows_dependency_execution`, `scripts/skill-doctor.ts`)와
  다른 클래스예요 — dep-exec 게이트가 안 덮어요. 완화: system-install 도 consent 필수 + 버전 핀 + **별도
  guard/test 정책 정의** (T2·T7). "node = 유일 예외" → "git+node system-install 2개 예외" 갱신과 함께.
- **R5 (100% ready 과장, 中):** 브라우저 OAuth, GitHub App 설치, OS installer GUI 는 agent 가 대신 클릭할
  수 없는 외부 행동이에요. 완화: "100%" 를 **single-skill orchestration completeness** 로 정의해요.
  즉, 외부 행동이 필요해도 setup 이 정확한 카드와 재개 phrase 를 주고, 사용자가 돌아오면 같은 루프가
  끝까지 닫아요. silent 자동화로 포장하지 않아요.
- **R6 (ready 카드가 거짓 green, 高):** URL surface 만 하고 live 확인을 안 하면 바이브코더는 바로 코딩하다
  배포 실패를 늦게 발견해요. 완화: deployment evidence 가 있으면 status/watch 를 확인하고, 없으면
  `READY_WITH_USER_ACTION` 으로 낮춰요.

## 6. Implementation File Map

| 파일 | 변경 책임 |
|------|-----------|
| `skills/setup/SKILL.md` | 온보딩 단일 진입점, state machine, phase/gap table, git/node/GitHub/deps/doctor/final ready card |
| `tests/fixtures/ask-defaults/registry.json` | setup 신규 AUQ safe default + rationale |
| `scripts/skill-doctor-allowlist.json` | `setup` dep-exec allowlist rationale 추가 |
| `scripts/skill-doctor.ts` | 필요 시 system-install guard/문구 검사 표면 추가 |
| `tests/skill-doctor-dep-execution.test.ts` | setup dep-exec allowlist + `--ignore-scripts` 회귀 |
| `tests/skill-noninteractive-guard.test.ts` | 신규 AUQ D1 guard 문구 회귀 |
| `tests/ux-todowrite.test.ts` | expanded checklist stale 방지 회귀 |
| `tests/manifest.test.ts` | setup first-run/onboarding contract, github 라우팅, ready card 문구 회귀 |
| `tests/manifest.test.ts` 또는 신규 `tests/setup-onboarding-evolution.test.ts` | state machine/gap id/ready status matrix 회귀 |
| `tests/vibe-bootstrap-measurement.test.ts` | "한 마디→ready" 경로 walltime/step count 측정 업데이트 |
| `tests/corpus.jsonl` / baseline fixtures | `온보딩`, `getting started`, `처음인데 뭐부터` expected skill 유지 |
| `docs/HOOKS.md` / 관련 docs | phase 변경, consent boundary, ready-state 설명 |

## 7. Implementation Tasks

> setup 진화이므로 `skill:new` scaffold 아님 — `skills/setup/SKILL.md` 직접 편집하되 Phase 17/18 패턴
> (D1 guard / TodoWrite Step 0 / AUQ registry / 해요체 / keyword baseline) 전부 유지.

- [ ] **T0 (P0, 제품 계약 잠금)** — `skills/setup/SKILL.md` 에 **VIBE_READY contract** 추가:
  사용자-facing onboarding single entrypoint, ready 등급 4개, 최종 ready 카드, 자연어 재개 phrase.
  Slash command/sibling skill 이름을 사용자에게 떠넘기지 않는다고 명시. Verify:
  `tests/manifest.test.ts` 또는 신규 `tests/setup-onboarding-evolution.test.ts` 가 `VIBE_READY`,
  `READY_WITH_USER_ACTION`, `SAFE_STOP_NONINTERACTIVE`, `BLOCKED_UNSUPPORTED`, `"온보딩 계속"` 문구 assert.
- [ ] **T1 (P1)** — setup/SKILL.md: 버전체크 phase — CLI(`update`) Phase A apply, 플러그인(`upgrade`)
  **끝단 advisory-only** (중간 적용 금지 — 새 세션 함정) + 호환 skew(`CLI 호환 range`) 라우팅. SKILL 에
  **ASCII 상태머신 + phase/gap state table** 포함 (eng review #1,#4). Verify: `bun run skill:doctor --strict` exit 0.
- [ ] **T2 (P1)** — detect-first 에 `git --version` 추가 + git 미설치 consent 설치 분기
  (macOS Xcode CLT / Windows winget·scoop). setup NEVER 의 "node = 유일 예외" → "git+node 2개 예외".
  Verify: git 없는 환경 시뮬 — 안내+설치 분기, fail-open.
- [ ] **T3 (P1)** — GitHub App 설치 전진배치 phase (auth 후, init 전) — **install_url 로 계정레벨 App
  설치만** (`axhub github accounts list`). OAuth device-flow 인가·app↔repo connect 는 connect 단계
  (init saga/github)에 남김 (codex #1). Verify: App 설치 후 init saga connect 가 install_url 단계에서
  **non-stall** 인지 검증 (eng #2 + codex #1).
- [ ] **T4 (P1)** — 기존 repo 라우팅: Phase C 에서 'git repo+커밋+manifest없음' 감지 → init 차단 →
  github 스킬 라우팅 (clone 충돌 방지). Verify: `manifest.test.ts` 에 기존repo 감지 안내 문자열 assert (eng review #3).
- [ ] **T5 (P1)** — AUQ registry: 새 question 마다 `tests/fixtures/ask-defaults/registry.json` 에
  `safe_default`+`rationale` 등록 (버전적용/git설치/github인가/node교정/의존성설치/배포유도/기존repo라우팅).
  Verify: `bun test tests/ux-ask-fallback-registry.test.ts`.
- [ ] **T6 (P2)** — node 버전 mismatch 자동교정 (nvm, consent, 버전태그 핀) + doctor 통합 phase +
  배포 마무리 phase: **init 경로는 saga 배포 URL surface 만 (재배포 X, codex #2)**, 기존repo 경로만
  `deploy` 위임. Verify: init 후 deploy 재호출 안 함 확인, 경고→교정 분기.
- [ ] **T7 (P1, 게이트변경+보안)** — 의존성 설치 gate flip **전체 표면** (Phase C, repo on disk 뒤 — codex #3):
  (a) `allows-dependency-execution: false→true` + **skill-doctor `allows_dependency_execution` 허용목록에
  setup 추가** + `tests/skill-doctor-dep-execution.test.ts` 갱신 (미등록 시 `skill:doctor` exit 1 = CI 즉사),
  (b) **subprocess/CI 의존성설치 → skip 안전 테스트 신규** (supply-chain D1),
  (c) R1 완화 — **`--ignore-scripts` 필수**(acceptance criterion, codex #5) / lockfile-only / consent / D1,
  (d) git system-install 별도 guard/test (R4, codex #6 — JS dep-exec 게이트와 분리).
  Verify: `bun test` + `bun run skill:doctor --strict` + dep-install 이 init/기존repo 감지 *뒤* 실행 확인.
- [ ] **T8 (P1)** — 검증 게이트: `bun run lint:tone --strict` 0err / `bun run lint:keywords --check`
  no-diff / `bunx tsc --noEmit` clean / `bun test` ≥ baseline pass.
- [ ] **T9 (P3)** — `docs/HOOKS.md`/관련 docs 에 phase 변경 반영, setup description 트리거 어구 유지
  (변경 금지 — baseline lock).
- [ ] **T10 (P1, 시나리오 회귀)** — "온보딩 한 마디" 경로별 scenario matrix 추가. 최소 케이스:
  CLI missing, CLI old, auth missing, git missing, node missing, node mismatch, GitHub App missing,
  empty dir fresh init, existing repo without manifest, repo with manifest, lockfile install success, lockfile install failure,
  CI/headless safe stop, PATH reload gap. Verify: matrix test 가 각 케이스의 ready 등급과 다음 phrase 를 assert.
- [ ] **T11 (P2, 측정)** — `tests/vibe-bootstrap-measurement.test.ts` 또는 baseline measurement 에
  cold first-run path 를 추가해 "사용자 발화 1개 + 필요한 consent 답변만" 으로 VIBE_READY 까지 가는지 측정.
  Verify: step count/major context-switch count 가 기존 setup 대비 감소.

## 8. Acceptance Criteria (100% onboarding 기준)

- [ ] `온보딩`, `처음인데 뭐부터`, `getting started` 가 모두 `setup` 으로 라우팅돼요.
- [ ] 사용자는 sibling skill 이름이나 slash command 를 몰라도 온보딩을 끝낼 수 있어요.
- [ ] setup 은 항상 detect-first → 첫 gap 처리 → 재감지 루프를 타요. 한 번에 여러 mutate gap 을 추측해
  실행하지 않아요.
- [ ] 모든 AskUserQuestion 은 D1 guard 와 registry safe default 를 가져요.
- [ ] CLI 업데이트는 `update` 에 위임하고 cosign 검증 경로를 유지해요.
- [ ] 플러그인 업데이트는 온보딩 중 실행하지 않고 끝단 advisory 로만 surface 해요.
- [ ] GitHub App 전진배치는 install_url 계정설치까지만 하며, OAuth/app↔repo connect 는 init/github
  단계에 남겨요.
- [ ] init 경로에서는 deploy 를 재호출하지 않아요. saga 의 deployment evidence 를 status/watch 로 확인해요.
- [ ] 기존 repo+커밋+manifest 없음은 init 으로 가지 않고 github guided setup/connect 로 가요.
- [ ] 의존성 install 은 repo on disk 뒤 lockfile 있을 때만, consent 후, `--ignore-scripts` 로만 실행돼요.
- [ ] headless/CI 에서는 install/update/auth/init/deploy/deps mutation 과 git/node system install/version switch 이 자동 실행되지 않아요.
- [ ] 최종 출력은 `VIBE_READY` 또는 명확한 degraded ready 등급과 자연어 재개 phrase 를 포함해요.
- [ ] `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`,
  `bunx tsc --noEmit`, `bun test` 가 green 이에요.

## Self-Check (착수 전)
- [ ] keyword baseline: setup `description:` 트리거 어구 **변경 안 함** (추가만 신중)
- [ ] 모든 신규 AUQ → registry 등록
- [ ] D1 guard 모든 신규 AUQ 진입부
- [ ] 해요체 통일 / skill:doctor / tsc / bun test green
- [ ] 의존성 게이트 flip 은 R1 완화책 4개 전부 구현됐을 때만

## 9. Scenario Matrix (구현 전 테스트 설계)

| ID | 시작 상태 | 기대 ready 등급 | 핵심 assert |
|----|-----------|------------------|-------------|
| S1 | CLI 없음 | `READY_WITH_USER_ACTION` 또는 `VIBE_READY` | install-cli 위임, 재감지 |
| S2 | CLI old | `VIBE_READY` | update 위임, cosign apply 안내 |
| S3 | auth 없음 | `READY_WITH_USER_ACTION` | device code surface + 승인 후 재개 phrase |
| S4 | git 없음 | `READY_WITH_USER_ACTION` | OS별 git 설치 consent, init 전 차단 |
| S5 | node 없음 | `READY_WITH_USER_ACTION` | node 설치 consent, 실패 시 nodejs.org LTS |
| S6 | node mismatch | `VIBE_READY` 또는 `READY_WITH_USER_ACTION` | nvm 교정 consent, target version 재확인 |
| S7 | GitHub App 미설치 | `READY_WITH_USER_ACTION` | install_url surface, accounts 재조회 |
| S8 | 빈 dir | `VIBE_READY` | init saga 후 manifest+repo+deploy evidence |
| S9 | 기존 repo+커밋+manifest 없음 | `VIBE_READY` 또는 `READY_WITH_USER_ACTION` | init 금지, github guided setup/connect |
| S10 | manifest+lockfile | `VIBE_READY` | install command 에 `--ignore-scripts` 포함 |
| S11 | manifest+lockfile+native postinstall 필요 | `READY_WITH_USER_ACTION` | false green 금지, native build 안내 |
| S12 | CI/headless | `SAFE_STOP_NONINTERACTIVE` | 모든 mutate action skip, safe default |
| S13 | PATH reload 필요 | `READY_WITH_USER_ACTION` | 새 터미널/셸 reload 안내, `✗` 종결 금지 |
| S14 | doctor fail | `READY_WITH_USER_ACTION` | doctor issue + 자연어 복구 phrase |

## 10. Worktree Parallelization

대부분 task 가 `setup/SKILL.md` 단일 파일을 만져 **순차 구현이 기본**이에요. 분리 가능한 lane:

| Lane | Tasks | 모듈 | 비고 |
|------|-------|------|------|
| A (주) | T0→T1→T2→T3→T4→T6 | `skills/setup/SKILL.md` | 같은 파일 — 순차 |
| B | T5 | `tests/fixtures/ask-defaults/registry.json` | A 의 AUQ 문구 확정 후 |
| C | T7 | skill-doctor src + dep-exec 테스트 | ⚠️ **A 의존** (아래) |
| D | T10→T11 | scenario/measurement tests | A 의 ready-state 문구 확정 후 |

⚠️ **거짓 병렬 정정 (codex #7):** T7 의 허용목록 rationale + 안전 테스트는 A 가 setup/SKILL.md 에 박는
**정확한 install 명령·`--ignore-scripts`·안전 문구**를 assert 해요. A 먼저 확정 안 하면 stale 테스트 /
rubber-stamp 허용목록 위험. 그래서 **C 는 A 뒤 순차** (파일은 disjoint 지만 *의미* 의존). 실행: A 주경로
→ B(A 의 AUQ 문구 후) + C(A 의 dep-install 문구 후) + D(A 의 ready-state 문구 후) → T8(검증)·T9(docs) 마지막.

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 1 | CLEAR | SELECTIVE EXPANSION, 7 accepted, 3 advisor fixes (git/R1/skew) |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR | 5 issues resolved, 1 CI-blocker caught (dep-exec allowlist) |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | N/A (CLI/skill, no UI) |
| Codex Review | `/codex review` | Independent 2nd opinion | 1 | issues→fixed | 8 findings: 5 사실 반영 + 1 전략 하드닝 |

- **ENG FINDINGS:** #1 plugin advisory-only(끝단) · #2 GitHub 전진배치=App설치(install_url)만+non-stall검증 ·
  #3 기존repo 라우팅 · #4 ASCII 상태머신 · #5 dep-exec 테스트표면 3-gap(허용목록 CI-blocker 포함).
- **CODEX (outside-voice):** 5 사실오류 반영 — deploy 이중호출 제거(#2) · dep-install post-repo 타이밍(#3) ·
  OAuth 전진배치 불가→install_url 만(#1) · T7 거짓병렬→A의존(#7) · skew=advisory 명시(#8). + 전략 하드닝:
  `--ignore-scripts` 필수 승격(#5) · git system-install 별도 guard=R4(#6).
- **CROSS-MODEL:** codex 가 eng #2 를 더 정밀화(OAuth/App 분리), deploy 이중호출·dep 타이밍 신규 캐치.
  합의 = plan 강화 (충돌 0).
- **UNRESOLVED:** 0 — eng 5 + codex 5 사실 + 1 전략 전부 반영.
- **VERDICT:** CEO + ENG + CODEX CLEARED — 구현 준비 완료. UI scope 없음 (design review N/A).
