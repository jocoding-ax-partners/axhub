# GitHub App 설치 surface 상시 노출 — 설계

- 날짜: 2026-06-08
- 범위: `skills/setup/SKILL.md` (onboarding), `skills/init/SKILL.md`
- 접근: A — 각 skill 인라인 (helper 추출은 3번째 사이트 등장 시점으로 보류)

## 문제

onboarding(setup) 과 init 에서 **GitHub App 이 설치되지 않은 계정은 사용자에게 전혀 보이지 않아요.** GitHub 연결은 오직 *reactive* 로만 일어나요 — init 의 `axhub apps bootstrap` saga 가 실행 도중 App 미설치/installation 만료/scope 부족을 감지하면 그때서야 `device_code_issued` / `install_url` event 를 emit 해요. 그 전까지는 GitHub App 설치 진입점이 흐름 어디에도 없어요.

그래서 사용자는 (a) 처음 쓰는데 어떤 계정에도 App 을 안 깐 상태이거나, (b) 개인 계정엔 깔았지만 새 org 를 붙이려 할 때, "GitHub App 을 어디서 까는지" 알 방법이 흐름상 없어요.

**원하는 것:** 설치 여부와 무관하게, onboarding 과 init **양쪽 모두에서 GitHub App 설치 진입점(install_url)을 상시 노출**해요. 단 **비차단(skip 가능)** — 카드만 보여주고 흐름은 계속 진행해요. 이미 깔았거나 나중에 할 사람은 그냥 넘어가요.

## 결정 (사용자 확정)

| 항목 | 결정 |
|---|---|
| 시나리오 | 신규(0 설치) **+** 일부 설치(새 org 추가) 둘 다 |
| 강도 | **항상 노출, skip 가능** (비차단). 차단 gate 아님. 확인 프롬프트(AskUserQuestion) 아님 |
| 적용 skill | setup(onboarding) + init |
| 구현 접근 | A — 각 skill 인라인 |

## 핵심 사실 (ax-hub-cli 0.17.4 소스 확정)

`axhub github accounts list --json` 출력 (실측):

```json
{
  "schema_version": "1",
  "status": "ok",
  "data": {
    "accounts": [
      {
        "login": "realitsyourman",
        "type": "User",
        "installed": true,
        "installation_id": 137870131,
        "avatar_url": "https://avatars.githubusercontent.com/u/139864668?v=4",
        "install_url": "https://github.com/apps/ax-hub-deploy/installations/new"
      }
    ]
  }
}
```

소스(`crates/axhub-api/src/github.rs`)에서 확정한 계약:

1. **`AccountDto.installed: bool` 은 per-account required 필드예요.** backend `/api/v1/github/accounts` 가 계정별 설치 여부를 채워요. DTO 가 `installed: false` 를 표현하도록 명시 설계됐고, CLI help 도 "accounts visible to `AxHub`" 라고 해요 — 즉 backend 는 사용자에게 보이는 계정을 설치 안 된 것까지 포함해 반환할 수 있어요. → **카드는 `installed: true` / `false` 를 per-entry 로 갈라서 렌더해야 해요.** "전부 설치됨" 가정 금지.
2. **`install_url: String` 은 per-account required 필드이고 app 단위 상수예요** (`https://github.com/apps/ax-hub-deploy/installations/new`, 모든 entry 동일). CLI 에는 별도 `provider::install_url()` 상수(env override `AXHUB_GITHUB_INSTALL_BASE_URL`)도 있지만 skill 은 JSON 만 보므로 entry 의 `install_url` 을 source 로 써요.
3. **`AccountsResponse.accounts` 는 `#[serde(default)]`** — backend 가 아무 계정도 안 주면 `accounts: []` 가능. 이 경우 entry 가 없어 skill 이 install_url 을 못 읽어요 (아래 빈-목록 처리 참조).
4. **init bootstrap saga 는 미설치 시 reactive 로 `install_url` event 를 이미 emit해요** (device-flow surface 설계, 2026-05-25 문서). 즉 proactive 카드가 URL 을 못 구해도 from-scratch 설치는 reactive 경로가 끝까지 책임져요. proactive 카드는 *인지(awareness) + 다른 계정 추가* 용도예요 — 두 surface 는 경쟁이 아니라 역할 분담이에요.

## 설계

### 공통 — install surface 카드 (해요체, 비차단)

read-only 명령 하나 + 파싱 + 한국어 카드 1장. AskUserQuestion 없음. 출력 후 흐름 계속.

```bash
axhub github accounts list --json
```

파싱:
- `data.accounts[]` 를 `installed == true` 와 `false` 로 분리.
- `install_url` = 첫 entry 의 `install_url` (전부 동일).

카드 렌더 분기:

- **설치된 계정 ≥1:**
  ```
  GitHub App 이 설치된 계정: <login1>, <login2>
  다른 org/계정을 추가하려면: <install_url>
  ```
- **미설치 계정(installed:false) 만 또는 혼재:** 미설치 계정을 이름으로 짚어줘요.
  ```
  GitHub App 설치가 안 된 계정: <login>
  설치 링크: <install_url>
  ```
- **`accounts: []` (빈 목록):** entry 가 없어 install_url 을 못 읽어요. URL 을 임의 생성하지 말고, 첫 배포 때 자동으로 GitHub 연결이 뜬다고 안내만 해요 (reactive saga 가 책임).
  ```
  아직 GitHub App 이 설치된 계정이 없어요. 첫 배포를 시작하면 GitHub 연결 안내가 자동으로 떠요.
  ```

never-force 원칙: install_url 을 자동으로 열지 않아요. 링크만 보여줘요 (github skill NEVER 룰 일치).

### setup(onboarding) 배치

현 흐름: detect → 온보딩 체크리스트 카드 → 첫 gap 위임(CLI/auth/node) → 준비 카드 → 첫 앱(init).

GitHub App read 는 **auth ✓ 일 때만** 가능해요 (`accounts list` 가 로그인 필요). 따라서:

- **Step 1 감지**에 auth ✓ 조건부로 `axhub github accounts list --json` read 추가.
- **Step 2 온보딩 상태 카드** + **Step 5 준비 카드**에 GitHub App 줄 한 줄 추가 (설치된 계정 수 또는 ✗), 그리고 위 install surface 카드를 함께 출력.
- auth ✗ 면 "로그인 후 확인" 으로 표시하고 read 는 건너뛰어요.
- 비차단 — 출력 후 그대로 첫 앱(init) 핸드오프로 진행.

### init 배치

현 흐름: preflight → (Step 1) current app → 템플릿 → 이름 → dry-run → execute → … → bootstrap saga(여기서 reactive device flow).

- **preflight/auth 확인 직후, 템플릿 선택 전(가장 앞)** 에 install surface 카드 step 을 1개 추가해요 ("맨 처음" 요구 충족). read-only `accounts list` + 카드 출력 후 곧장 다음 step(템플릿)으로 진행.
- bootstrap saga 내부의 reactive `device_code_issued` / `install_url` 처리(Step 6)는 **그대로 유지** — 실제 from-scratch 설치 mechanism 이에요. proactive 카드는 그 앞에서 인지 + 계정 추가 진입점만 제공.

## 비-목표 (out of scope)

- **github skill 변경 안 해요.** 사용자가 지목한 건 onboarding + init. github skill 은 이미 연결 status/connect 를 담당하고 install_url 도 조건부 노출해요. surgical scope 유지.
- **차단 gate 아니에요.** 미설치여도 흐름 막지 않아요.
- **새 AskUserQuestion 없어요.** "항상 노출 skip 가능" 은 카드 출력만으로 충족 → registry 등록 불필요.
- **CLI/backend 변경 없어요.** skill-only. (빈-목록 install_url 을 helper 로 surface 하는 건 3번째 사이트 등장 시 재검토.)
- **helper 추출 안 해요.** 사이트 2개뿐 — device-flow spec(2026-05-25) line 108 의 "3번째 사본 시점에 추출" YAGNI 룰 적용.

## 제약 — Phase 17/18 skill 게이트 유지

기존 skill 편집(scaffold 우회 아님). 보존 필수:

- setup: `multi-step: true`, `needs-preflight: false` 유지. init: `needs-preflight: true` + in-body preflight 블록(`CANONICAL_PREFLIGHT_BLOCK`) 그대로.
- D1 non-interactive guard: 새 카드는 AskUserQuestion 이 없어 D1 무관. 비대화형에서도 카드 출력은 무해.
- TodoWrite Step 0 + step status sync 유지. setup 체크리스트에 GitHub 항목 추가 시 todos 배열도 갱신.
- 신규 한글 텍스트 전부 **해요체** (`bun run lint:tone --strict` 0 err). 금지 token(합니다/입니다/당신 등) 회피.
- frontmatter `description:` 의 nl-lexicon trigger 어구 **불변** (`bun run lint:keywords --check` baseline lock). body 에 새 trigger 어구 도입 금지 — "GitHub App 설치 링크" 같은 중립 표현만.
- **step-numbering collision(FU-3) 회피** — top-level `^N. **` 헤더 중복 금지. init 앞에 step 삽입 시 번호 재정렬 또는 sub-step(`1.5.`) 사용.

## 검증

- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 err
- `bun run lint:keywords --check` no diff
- `bun test` 회귀 0 fail (Phase 18 baseline ≥498 pass)
- `bunx tsc --noEmit` clean
- (이상적) authed 계정으로 `axhub github accounts list --json` 실측 → 카드 분기 렌더 확인. 0-설치/미설치 계정은 live 재현이 어려워 정적 분기로 커버.

## 미확인 / 리스크

1. **backend 가 실제로 `installed: false` entry 를 반환하는지 live 미확인.** 실측 3계정은 전부 `installed: true`. DTO/contract 는 false 를 지원하므로 카드는 양쪽을 처리하게 설계 — false 가 안 와도 무해, 오면 정상 동작.
2. **`accounts: []` 빈 목록의 install_url source 없음.** entry 가 없으면 URL 을 못 읽어요. 임의 URL 생성 대신 "첫 배포 때 자동 안내" 로 degrade — init reactive saga 가 안전망. (정 필요하면 후속에서 helper 가 `provider::install_url()` 을 surface 하는 방안 검토, 단 env override 주의.)
3. **두 surface 중복 노출.** 0-설치 init 사용자는 proactive 카드 → 이후 saga 의 reactive install_url 을 둘 다 봐요. 역할(인지 vs 실제 설치)을 카드 문구로 구분해 혼선을 줄여요.
