# GitHub App 설치 surface 상시 노출 — 설계

- 날짜: 2026-06-08
- 범위: `skills/init/SKILL.md` (onboarding 은 #171 이 이미 충족 — 아래 status 참조)
- 접근: A — 각 skill 인라인 (helper 추출은 3번째 사이트 등장 시점으로 보류)

## Status (2026-06-08 갱신)

- **onboarding(setup)**: #171 (`make onboarding sufficient for first-run`, v0.9.34) 이 `setup → onboarding` rename 과 함께 GitHub App surface 를 이미 구현했어요. onboarding Step 2 DETECT_ALL 이 `axhub github accounts list --json` 를 파싱해 `github_app_missing` gap 을 emit 하고, Step 6 "GitHub App frontload" 가 install_url 을 전진배치해요. 그래서 이 설계의 onboarding 부분은 **이미 main 에 들어가 있어요** — 중복 구현하지 않아요.
- **init**: 직접 `init` 호출(온보딩을 거치지 않는 "앱 만들어줘") 흐름에는 proactive GitHub App surface 가 아직 없어요. 이 브랜치가 그 부분만 채워요.

## 문제

init 에서 GitHub 연결은 reactive 로만 일어나요 — `axhub apps bootstrap` saga 가 실행 도중 App 미설치/installation 만료/scope 부족을 감지하면 그때서야 `device_code_issued` / `install_url` event 를 emit 해요. 직접 init 을 호출한 사용자는 그 전까지 GitHub App 설치 진입점을 못 봐요. 설치 여부와 무관하게 init 시작부에서 install_url 을 한 번 보여줘요.

## 결정 (사용자 확정)

| 항목 | 결정 |
|---|---|
| 시나리오 | 신규(0 설치) + 일부 설치(새 org 추가) 둘 다 |
| 강도 | 항상 노출, skip 가능 (비차단). 차단 gate 아님. 새 AskUserQuestion 없음 |
| 적용 skill | init (onboarding 은 #171 이 충족) |
| 구현 접근 | A — 인라인 |

## 핵심 사실 (ax-hub-cli 0.17.4 소스 확정)

`axhub github accounts list --json` 출력:

```json
{
  "schema_version": "1",
  "status": "ok",
  "data": {
    "accounts": [
      {"login": "realitsyourman", "type": "User", "installed": true,
       "installation_id": 137870131, "install_url": "https://github.com/apps/ax-hub-deploy/installations/new"}
    ]
  }
}
```

소스(`crates/axhub-api/src/github.rs`)에서 확정한 계약:

1. **`AccountDto.installed: bool` 은 per-account required 필드예요.** backend 가 계정별 설치 여부를 채워요. DTO 가 `installed: false` 를 표현하도록 설계됐으니 카드는 `installed: true` / `false` 를 per-entry 로 갈라서 렌더해요.
2. **`install_url: String` 은 per-account required 필드이고 app 단위 상수예요.** skill 은 JSON 만 보므로 entry 의 `install_url` 을 source 로 써요.
3. **`AccountsResponse.accounts` 는 `#[serde(default)]`** — 0 설치면 `accounts: []` 가능. entry 가 없으면 URL 을 못 읽어요 (빈-목록 degrade).
4. **init bootstrap saga 는 미설치 시 reactive 로 `install_url` event 를 이미 emit해요.** proactive 카드가 URL 을 못 구해도 from-scratch 설치는 reactive 경로(Step 7a)가 책임져요. 두 surface 는 경쟁이 아니라 역할 분담이에요 (인지/추가 vs 실제 설치).

## 설계 — init install surface 카드 (해요체, 비차단)

Step 2(templates list, auth 확인됨) 직후 sub-step `2.5.` 로 read-only `axhub github accounts list --json` + 카드. AskUserQuestion 없음. 출력 후 Step 3 으로 진행.

- sub-step `2.5.` 는 top-level step-numbering regex `^\d+\. \*\*` 에 안 걸려 충돌이 없어요 (`tests/init-skill-step-numbering.test.ts` green 유지).
- 카드 분기: 설치된 계정 ≥1 / 미설치·혼재 / 빈 목록(첫 배포 때 자동 안내로 degrade).
- `installation_id` 등 internal 값 echo 금지, `login` + `install_url` 만 노출. 링크 자동 열기 금지.
- Vibe Coder Visibility Rules 표에 "Step 2.5 GitHub App 설치 안내" 행 추가, 기존 Step 1~8 행 불변.
- bootstrap saga 내부 reactive device flow(Step 7a)는 그대로 유지.

## 비-목표

- onboarding 변경 안 해요 (#171 이 충족).
- 차단 gate / 새 AskUserQuestion / CLI 변경 / helper 추출 안 해요.

## 제약 — Phase 17/18 skill 게이트 유지

- init `needs-preflight: false`, in-body 구조 유지.
- 신규 한글 텍스트 전부 해요체 (`lint:tone --strict` 0 err).
- frontmatter `description:` 불변 (`lint:keywords --check` no diff).
- step-numbering collision(FU-3) 회피 — sub-step `2.5.` 사용.

## 검증

- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 err
- `bun run lint:keywords --check` no diff
- `bun test` regression 0 fail (init-skill-step-numbering / init-skill-visibility-rules / skill-noninteractive-guard)
- `bunx tsc --noEmit` clean
