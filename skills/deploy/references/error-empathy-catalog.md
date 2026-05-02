# Error Empathy Catalog (4-Part Korean Templates)

This catalog implements the DX-2 fix from PLAN Phase 3.5: every axhub exit code maps to a Korean message with **emotion + cause + action + button**, not a clinical exit-code dictionary.

The **vibe coder is anxious** (P3 persona, 11pm demo scenario in PLAN §1000). Clinical messages like "토큰이 만료됐습니다. /axhub:login을 실행하세요." trigger the give-up cascade. Empathy + plain Korean + a next phrase the user can literally say keeps them in the loop.

---

## Template Structure (4 parts, MANDATORY order)

Every entry uses these four parts, in this order:

1. **감정 (Emotion)** — 1 sentence reassurance. Pick from:
   - "이건 흔한 일이에요." (this happens often)
   - "당신 앱은 안전합니다." (your app is safe)
   - "잠깐만요." (one moment)
   - "축하해요!" (success only)
   - "전혀 문제없어요." (no problem at all)

2. **원인 (Cause)** — what + why in plain Korean. **No CLI jargon. No slash references on first-time users.** Avoid words like exit code, JSON, NDJSON, payload, transport.

3. **해결 (Action)** — 1–2 lines, written as **the next natural-language phrase the user can literally say**. Not "/axhub:login" but "'다시 로그인해줘' 라고 말씀해주세요."

4. **버튼 (Buttons)** — AskUserQuestion options array, max 3 short Korean labels.

---

## exit 0 — success (celebration template)

**감정:** 축하해요! 배포 성공입니다.

**원인:** `<APP_SLUG>` (id=`<APP_ID>`) 가 `<PROFILE>` 환경에 정상 반영됐어요. 빌드 `<DEPLOY_ID>` 가 `<ELAPSED>` 만에 끝났습니다.

**해결:** 라이브 URL을 한 번 확인해보시겠어요? 다음에 또 배포하실 때는 "방금 거 상태" 또는 "방금 거 로그" 라고 말씀하시면 바로 보여드려요.

**버튼:** ["라이브 확인", "로그 보기", "닫기"]

---

## exit 1 — transport / unclassified

**감정:** 잠깐만요. 일시적인 통신 문제예요. 당신 앱은 안전합니다.

**원인:** axhub 서버까지 연결이 잠깐 끊겼어요. 네트워크가 느리거나 서버가 잠시 응답을 못한 경우예요. 배포 자체는 시작도 안 됐으니 걱정하지 마세요.

**해결:** 한 번 더 시도해보시겠어요? "다시 시도해줘" 라고 말씀하시면 한 번만 자동 재시도해요. 배포 명령은 자동 재시도하지 않아요 (중복 배포 방지).

**버튼:** ["다시 시도", "잠시 후 다시", "도와주세요"]

---

## exit 2 — deploy status in-progress

**감정:** 정상이에요. 배포가 아직 진행 중일 뿐입니다.

**원인:** `<DEPLOY_ID>` 가 현재 `<STATUS_PHASE>` 단계예요. 평균 `<ETA>` 정도 걸리는데, 지금까지 `<ELAPSED>` 경과했어요.

**해결:** "계속 지켜봐줘" 라고 말씀하시면 끝날 때까지 자동으로 알려드려요. 다른 일 하시다가 끝나면 알림 드릴게요.

**버튼:** ["계속 지켜보기", "지금 그만 보기", "로그도 같이 보기"]

---

## exit 64 (base) — validation / usage error

**감정:** 잠깐만요. 배포는 시작 안 됐어요. 당신 앱은 안전합니다.

**원인:** 입력값에 문제가 있어서 배포 요청이 막혔어요. axhub가 받기 전에 검증에서 멈췄다는 뜻이에요.

**해결:** 무엇을 배포하려 하셨는지 다시 한 번 풀어서 말씀해주세요. 예: "paydrop 메인 브랜치 최신 커밋 배포해" 처럼 구체적으로요.

**버튼:** ["다시 풀어 말하기", "도와주세요", "취소"]

---

### exit 64 + `validation.deployment_in_progress`

**감정:** 당신 앱은 안전합니다. 다른 배포가 먼저 진행 중이에요.

**원인:** `<APP_SLUG>` 의 다른 배포 (`<IN_FLIGHT_DEPLOY_ID>`) 가 아직 끝나지 않았어요. 같은 앱은 한 번에 한 배포만 진행됩니다 (서로 덮어쓰지 못하게 막아주는 안전장치예요).

**해결:** 새로 배포하지 마시고 진행 중인 그 배포를 함께 지켜볼까요? "그거 끝날 때까지 지켜봐줘" 라고 말씀해주시면 됩니다. **절대 다시 시도하지 않습니다 — 끝나면 자연스럽게 다음 배포가 가능해요.**

**버튼:** ["진행 중인 거 지켜보기", "5분 후 다시 알려줘", "지금 취소"]

---

### exit 64 + `validation.app_ambiguous`

**감정:** 잠깐만요. 같은 이름이 두 개라서 헷갈렸어요.

**원인:** `<INPUT_SLUG>` 라는 이름의 앱이 여러 개 있어요. 어떤 거 말씀하신 건지 골라주세요.

**해결:** 아래 후보 중 하나를 골라주세요. 다음부터는 정확한 ID로 기억해둘게요.

**버튼:** [동적 — 후보 앱 슬러그 + ID 최대 3개, 예: "paydrop (id=42)", "paydrop-staging (id=43)", "더 많은 후보 보기"]

---

### exit 64 + `validation.app_list_truncated`

**감정:** 잠깐만요. 회사에 앱이 너무 많아서 다 못 가져왔어요.

**원인:** 앱이 100개를 넘어서 목록이 잘렸어요. 이름만으로는 정확히 어떤 앱인지 못 찾아요.

**해결:** 앱의 ID 숫자를 직접 알려주실 수 있나요? 예: "id 42 배포해" 또는 "app-3 배포해" 처럼요. ID는 `apps list` 결과에 표시돼요.

**버튼:** ["앱 검색하기", "앱 ID 직접 입력", "도와주세요"]

---

## exit 65 — auth required / token expired

**감정:** 잠깐만요. 로그인이 만료됐을 뿐이에요. 당신 앱은 그대로예요.

**원인:** axhub 로그인 토큰이 만료됐어요. 보안을 위해 일정 시간이 지나면 다시 로그인해야 해요. 평소 회사 메일·은행 사이트랑 똑같아요.

**해결:** "다시 로그인해줘" 라고 말씀하시면 브라우저로 안내드릴게요. (브라우저가 안 열리는 환경 — 예: GitHub Codespaces — 이시면 별도 안내드려요.)

**버튼:** ["다시 로그인", "토큰 파일로 로그인 (헤드리스)", "도와주세요"]

---

## exit 66 (base) — scope insufficient

**감정:** 잠깐만요. 권한 문제예요. 당신 앱은 안전합니다.

**원인:** 지금 토큰의 권한 범위로는 이 작업을 할 수 없어요. 회사 정책상 사람 (보통 토큰 발급해주신 분 — IT 담당자나 PM) 이 권한을 더 부여해야 해요.

**해결:** 토큰을 발급해준 분께 이 메시지 그대로 보내주세요: "axhub 토큰에 `<REQUIRED_SCOPE>` scope 추가 필요합니다." 그 분이 처리해주시면 다시 로그인하시면 됩니다.

**버튼:** ["담당자에게 메시지 복사", "현재 권한 확인", "도와주세요"]

---

### exit 66 + `scope.downgrade_blocked`

**감정:** 잠깐만요. 안전장치가 작동했어요.

**원인:** 더 낮은 환경으로의 다운그레이드 시도가 감지됐어요. 예를 들어 production에 있는 앱을 staging 빌드로 덮으려 했을 때 안전을 위해 막아드려요.

**해결:** 정말로 다운그레이드가 필요하시면 명시적으로 "강제로 다운그레이드해" 라고 말씀해주세요. 그게 아니라면 의도하신 환경 (보통 production) 의 빌드를 다시 확인해주세요.

**버튼:** ["환경 다시 확인", "강제 다운그레이드 (위험)", "취소"]

---

### exit 66 + `update.cosign_verification_failed`

**감정:** 잠깐만요. 보안 검증에 실패했어요. 절대 진행하지 않아요.

**원인:** 다운로드받은 axhub 업데이트 파일이 정품인지 검증하는 cosign 절차에서 실패했어요. 파일이 변조됐거나 네트워크 중간에 누군가 끼어든 가능성이 있어요. 보안상 업데이트를 차단했습니다.

**해결:** 절대 강제로 진행하지 마세요. 회사 IT 보안 담당자에게 즉시 알려주세요. 그동안 axhub는 현재 버전으로 계속 사용하실 수 있어요.

**버튼:** ["IT 보안팀에 알리기", "업데이트 취소", "현재 버전 유지"]

---

## exit 67 — resource not found (with did-you-mean)

**감정:** 잠깐만요. 그런 이름은 못 찾았어요.

**원인:** `<INPUT_NAME>` 이라는 이름의 `<RESOURCE_TYPE>` (앱/배포/API) 이 회사 axhub에 등록되어 있지 않아요. 오타이거나, 다른 회사 계정의 앱일 수도 있어요.

**해결:** 혹시 이 중 하나를 말씀하셨나요? 가장 비슷한 후보를 보여드릴게요. (Levenshtein 거리 ≤2 또는 prefix match)

```
혹시 이걸 말씀하셨나요?
  ① paydrop (가장 유사)
  ② paydrop-v2
  ③ paydrop-staging
  ④ 위에 없어요 — 앱 목록 보기
```

**버튼:** ["가장 유사한 거로", "앱 목록 보기", "다시 입력"]

---

## exit 68 — rate limit (with auto-backoff)

**감정:** 잠깐만요. 너무 많이 요청해서 서버가 잠시 쉬자고 해요. 당신 앱은 안전합니다.

**원인:** 짧은 시간 안에 axhub 호출이 많이 누적돼서 잠깐 멈춰야 해요. 보통 다른 사람이랑 같은 토큰을 공유하거나, 자동화 스크립트가 너무 빨리 돌 때 생겨요. 서버에서 `Retry-After: <SECONDS>` 초 후 다시 시도하라고 알려줬어요.

**해결:** `<SECONDS>` 초 (보통 30초~2분) 만 기다려주세요. 자동으로 다시 시도할게요. 그동안 커피 한 잔 어떠세요?

**버튼:** ["자동으로 기다리기", "지금 취소", "도와주세요"]

---

# Deploy-Preview Card Template

This is the AskUserQuestion card rendered by `skills/deploy/SKILL.md` step 3 BEFORE any destructive `axhub deploy create` call. It echoes the five identity fields verbatim in Korean (E4 fix from PLAN — never trust cached app_id for mutations).

## Card body (Korean, NFKC-normalized)

```
다음을 실행할게요:
  ① 앱:    <APP_SLUG> (id=<APP_ID>)
  ② 환경:  <PROFILE> (<ENDPOINT>)
  ③ 브랜치: <BRANCH>
  ④ 커밋:  <COMMIT_SHA_SHORT> — "<COMMIT_MESSAGE_FIRST_LINE>"
           (<RELATIVE_TIME> 푸시, <COMMIT_AUTHOR>)
  ⑤ 예상:  약 <ETA_MIN>분 소요

진행할까요?
```

## AskUserQuestion options (mandatory three)

```json
{
  "question": "위 내용으로 배포 진행할까요?",
  "options": [
    {
      "label": "네, 진행",
      "value": "confirm",
      "description": "위 5가지 내용 그대로 axhub deploy create 실행"
    },
    {
      "label": "아니요, 취소",
      "value": "reject",
      "description": "배포를 시작하지 않습니다. 안전합니다."
    },
    {
      "label": "미리보기만 (--dry-run)",
      "value": "dry_run",
      "description": "실제 배포 없이 어떻게 진행될지만 시뮬레이션"
    }
  ]
}
```

## Rendering rules

- **NFKC normalize** every displayed string before showing. If NFKC altered the slug (Cyrillic lookalike attack, ZWJ injection), surface a warning row above the card: `⚠️ 앱 이름에 비정상 문자가 감지됐어요. 확인해주세요: 원본=<RAW_SLUG>, 정규화=<NFKC_SLUG>`. (Reference: PLAN §16.11 Unicode + F14 Korean Unicode 공격.)
- **Verbatim echo** — never substitute `app_id` from local cache. The five fields MUST come from the latest live `axhub auth status --json` + `axhub apps list --json --slug-prefix <slug>` resolution (E4 fix).
- **Profile mismatch** — if `--profile` arg differs from `$AXHUB_PROFILE`, prepend a yellow warning row: `⚠️ 현재 환경(<ENV_PROFILE>) 과 다른 환경(<ARG_PROFILE>) 으로 배포하려 합니다. 의도하신 게 맞나요?` See `recovery-flows.md` ("profile-mismatch").
- **Slash invocation does NOT skip this card.** `/axhub:deploy paydrop` still renders the card. Slash is consent for invoking the skill, not for the destructive op (E2 fix).
- **Consent token binding** — on user "confirm", the helper mints a token bound to `{action=deploy_create, app_id, profile, branch, commit_sha}`. PreToolUse hook verifies the token before letting `axhub deploy create` run. Mismatch = deny.

## Special: ETA calculation

If `eta_sec` from helper resolution is null (first deploy on this app), render: `⑤ 예상:  처음 배포라서 시간 예측 어려워요 (보통 2~5분)`. Do NOT fabricate a number.

## Special: dry-run preview output

When user picks "미리보기만 (--dry-run)", run `axhub deploy create --app <ID> --branch <BRANCH> --commit <SHA> --dry-run --json` and render the response as:

```
미리보기 결과 (실제로는 아무것도 안 올렸어요):
  · 새 컨테이너 이미지 빌드: 예상 ~2분
  · DB 마이그레이션: <N>개 변경 감지
  · 환경변수 변경: <N>개 (이전 배포 대비)
  · 헬스체크 endpoint: <URL>
  · 라이브 전환 방식: <STRATEGY>

이대로 진짜 배포하시려면 "이대로 진행해" 라고 말씀해주세요.
```

---

### exit 64 + `env.prod_force_required`

**감정:** 잠깐만요. production 환경변수는 더 조심해야 해요.

**원인:** production 값은 즉시 라이브 앱에 영향을 줄 수 있어서 일반 env 변경보다 위험해요.

**해결:** key 와 app slug 를 다시 확인하고 exact confirm 후 진행해요. 값은 화면에 노출하지 않아요.

**버튼:** ["key 다시 확인", "취소", "도와주세요"]

---

### exit 64 + `env.prod_confirm_mismatch`

**감정:** 잠깐만요. 확인 문구가 일치하지 않아요.

**원인:** 입력한 key 또는 app 확인값이 실제 변경 대상과 달라요.

**해결:** 대상 key 와 app slug 를 다시 보고 정확히 일치할 때만 다시 시도해요.

**버튼:** ["다시 확인", "취소", "도와주세요"]

---

### exit 67 + `github.install_not_found`

**감정:** GitHub App 설치를 찾지 못했어요.

**원인:** 선택한 account 에 axhub GitHub App 이 설치되어 있지 않거나 repo 권한이 없어요.

**해결:** CLI 가 제공한 install URL 로 설치를 마친 뒤 다시 연결해요. 권한 부여는 자동으로 진행하지 않아요.

**버튼:** ["설치 URL 보기", "repo 다시 고르기", "닫기"]

---

### exit 64 + `github.git_connection_already_exists`

**감정:** 이미 GitHub repo 가 연결되어 있어요.

**원인:** 이 앱에는 기존 GitHub 연결이 있어서 중복 연결을 만들 수 없어요.

**해결:** 현재 연결을 확인한 뒤 바꾸려면 disconnect preview 와 exact confirm 을 먼저 진행해요.

**버튼:** ["현재 연결 보기", "연결 해제 검토", "닫기"]

---

### exit 64 + `github.confirm_slug_mismatch`

**감정:** 확인한 앱 slug 가 달라요.

**원인:** GitHub 연결 해제 대상과 확인 문구가 일치하지 않아 안전하게 중단했어요.

**해결:** 앱 slug 를 다시 확인하고 정말 해제할 때만 정확히 입력해요.

**버튼:** ["slug 다시 확인", "취소", "도와주세요"]

---

### exit 67 + `open.no_app_manifest`

**감정:** 열 수 있는 axhub 앱 정보를 찾지 못했어요.

**원인:** 현재 디렉토리에 apphub.yaml 또는 axhub.yaml 이 없고 최근 배포 cache 도 비어 있어요.

**해결:** 먼저 init 으로 앱 파일을 만들거나 apps 목록에서 열 앱을 골라요.

**버튼:** ["init 시작", "앱 목록 보기", "닫기"]

---

### exit 66 + `profile.endpoint_not_in_allowlist`

**감정:** endpoint 가 허용 목록 밖이에요.

**원인:** 사내 endpoint 나 임시 endpoint 일 수 있지만, 토큰과 요청이 다른 서버로 갈 수 있어서 조심해야 해요.

**해결:** 회사에서 승인한 endpoint 인지 확인하고 exact confirm 후에만 profile 에 추가해요.

**버튼:** ["endpoint 확인", "취소", "도와주세요"]

---

### exit 65 + `apis.call_consent_required`

**감정:** API 호출에는 사전 승인이 필요해요.

**원인:** 이 endpoint 호출은 서버 상태를 바꿀 수 있어요. read-only 조회처럼 자동 실행할 수 없어요.

**해결:** method, endpoint, body source 를 preview 로 확인한 뒤 동의 token 을 mint 해서 다시 실행해요.

**버튼:** ["preview 보기", "취소", "도와주세요"]
