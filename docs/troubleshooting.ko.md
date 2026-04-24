# 문제 해결 가이드 (한국어)

> **대상**: 빠른 시작을 따라 했는데 빨간 글씨가 떴거나, 평소 쓰다가 막힌 바이브코더.
> **읽는 시간**: 5분 (찾고자 하는 에러 코드만 읽으면 1분).

---

## 무서워하지 마세요

먼저 깊게 한 번 숨 쉬세요. 다음을 기억하세요.

1. **당신이 잘못한 것이 아닐 가능성이 큽니다.** axhub 의 흔한 에러 90% 는 토큰 만료, 동시 배포 충돌, 이름 오타 같은 환경적인 이슈입니다.
2. **당신 앱은 안전합니다.** 새 배포가 실패해도 이전에 성공한 버전이 그대로 prod 에서 돌아갑니다. 사용자가 보는 화면은 멈추지 않습니다.
3. **돌이킬 수 없는 일은 일어나지 않습니다.** axhub 플러그인은 모든 destructive 동작 (배포, 토큰 갱신 등) 전에 승인 카드로 한 번 더 묻습니다. 카드 없이 prod 가 바뀌는 일은 절대 없습니다.
4. **모르면 묻기가 정답입니다.** 이 문서로 안 풀리면 [도움 받기](#도움-받기) 의 채널 중 하나로 바로 연락하세요. 시니어를 깨우는 것은 부끄러운 일이 아닙니다.

이제 차분하게 에러 코드를 찾아봅시다.

---

## 에러 코드별 가이드

각 항목은 **(1) 무슨 감정 (2) 왜 발생했는지 (3) 어떻게 고치는지 (4) 어디 클릭** 4 부분 형태로 적었습니다.

### exit 65 — 토큰 만료 (가장 흔함)

**무슨 감정** — 갑자기 모든 axhub 명령이 빨간 글씨로 멈췄다. "내가 권한이 없어진 건가?"

**왜 발생** — axhub 토큰의 유효 기간 (보통 14 일) 이 지났습니다. 아무 잘못 안 했어도 시간이 지나면 자연스럽게 만료됩니다. 회사 보안 정책상 정상입니다.

**어떻게 고치기** — Claude Code 에 그대로 입력:

```text
axhub 로그인해줘
```

플러그인이 자동으로 OAuth Device Flow 를 다시 시작하고, 브라우저로 SSO 로그인 화면을 엽니다. 30 초면 끝납니다.

**클릭** — 승인 카드의 `[로그인 다시 하기]` 버튼이 자동으로 뜹니다. 또는 그냥 위 문장을 입력하세요.

---

### exit 64 + `deployment_in_progress` — 이미 다른 배포 중

**무슨 감정** — "내가 방금 누른 게 잘못된 건가? 두 번 누른 건가?"

**왜 발생** — 같은 앱에 대한 다른 배포가 아직 진행 중입니다. 본인이 1 분 전에 누른 배포일 수도 있고, 같은 팀 다른 동료가 동시에 배포한 것일 수도 있습니다. axhub 는 안전상 동시 배포를 금지합니다.

**어떻게 고치기** — **재시도하지 마세요.** 재시도해도 똑같이 막힙니다. 대신 진행 중인 배포를 먼저 봅니다:

```text
지금 진행 중인 배포 어떻게 됐어
```

플러그인이 자동으로 `axhub deploy status --watch` 를 띄우고, 한국어로 "1분 경과, 빌드 중이에요 (정상)" 같은 진행 상황을 보여줍니다. 그게 끝나면 (보통 3-5 분) 그 다음에 본인 배포를 다시 시도하세요.

**클릭** — 에러 카드의 `[진행 중인 배포 보기]` 버튼이 자동으로 뜹니다.

---

### exit 64 + `app_ambiguous` — 앱 이름이 모호함

**무슨 감정** — "분명히 내 앱 이름인데 왜 모르는 거지?"

**왜 발생** — 회사 axhub 안에 같은 prefix 또는 같은 slug 의 앱이 둘 이상입니다. 예: `paydrop`, `paydrop-test`, `paydrop-v2` — 그냥 "paydrop" 만 말하면 어느 것인지 알 수 없습니다.

**어떻게 고치기** — 플러그인이 자동으로 후보 목록을 보여줍니다:

```
"paydrop" 으로 시작하는 앱이 3개 있어요. 어느 것?

  1. paydrop          (id=42, last deploy 2일 전)
  2. paydrop-test     (id=87, last deploy 1주 전)
  3. paydrop-v2       (id=104, last deploy 어제)

번호 또는 정확한 이름으로 다시 말씀해 주세요.
```

번호로 답하거나 정확한 이름을 말하면 됩니다. 예: "1번", "paydrop-v2".

**클릭** — 후보 카드에서 해당 번호 버튼을 누르세요.

---

### exit 67 — 앱을 찾지 못함 (did-you-mean 패턴)

**무슨 감정** — "이름 똑바로 쳤는데?"

**왜 발생** — 입력한 이름이 회사 axhub 안에 등록된 어떤 앱 이름과도 정확히 일치하지 않습니다. 흔한 원인: 오타, 대소문자 불일치, 다른 사람 앱을 기억함.

**어떻게 고치기** — 플러그인이 가장 비슷한 이름 1-3 개를 자동으로 추천합니다:

```
"paydrap" 라는 앱은 못 찾았어요. 혹시 이거 말씀이신가요?

  1. paydrop      (id=42)
  2. pay-drop-v2  (id=88)

또는 "내 앱 목록" 이라고 말씀해 주시면 전체 보여드릴게요.
```

후보가 맞으면 그 이름으로 다시 말하세요. 없으면 `내 앱 목록` 으로 전체를 봅니다.

**클릭** — 추천 카드의 해당 이름 버튼.

---

### exit 68 — 요청 너무 많음 (rate limit)

**무슨 감정** — "내가 뭐 빨리 한 거 있나?"

**왜 발생** — 짧은 시간 안에 너무 많은 axhub 명령을 보냈습니다. 보통 동일 명령을 빠르게 여러 번 시도했을 때 발생합니다. 회사 axhub 가 보호 차원에서 잠시 거절하는 정상 동작입니다.

**어떻게 고치기** — **즉시 재시도 금지.** 에러 메시지에 `Retry-After: 30s` 같은 숫자가 있습니다. 그 시간만큼 (보통 10초 ~ 2분) 기다린 뒤 다시 시도합니다. 플러그인은 자동으로 카운트다운을 보여줍니다:

```
⏳ 30초 후에 자동으로 재시도할게요.
   (지금 다른 일 하셔도 됩니다)
```

**클릭** — 카운트다운이 끝나면 자동 진행. `[취소]` 버튼으로 중단 가능.

---

### exit 66 + `cosign_verification_failed` — 보안 검증 실패 ⚠️

**무슨 감정** — "이거 진짜 무서운 메시지인 것 같은데?"

**왜 발생** — `axhub update apply` 로 새 CLI 바이너리를 받았는데, 그 바이너리의 cosign 서명이 검증 안 되었습니다. **이것은 진짜 보안 신호입니다.** 다음 중 하나입니다:

1. 네트워크 중간에서 누군가 바이너리를 바꿔치기했을 가능성 (드물지만 가능).
2. CI/CD 사고로 axhub 측 서명이 깨진 상태 (회사 axhub 운영팀 이슈).
3. 이전에 설치한 비공식 빌드와 충돌.

**어떻게 고치기** — **재시도 / 강제 우회 절대 금지.** 플러그인은 이 상황에서 hard stop 합니다. 대신:

1. **회사 axhub 관리자에게 즉시 보고** (Slack #axhub 채널 또는 사내 보안팀).
2. 메시지 그대로 캡처해서 첨부.
3. 관리자가 axhub 운영팀과 확인하고 정상 서명을 다시 받을 때까지 `axhub update apply` 시도하지 마세요.

이 동안 기존 axhub CLI 는 계속 정상 동작합니다 (배포, 상태 확인 모두 가능). 업데이트만 보류된 상태입니다.

**클릭** — 카드의 `[관리자에게 알리기]` 또는 위 절차대로.

> 회사 관리자: 이 시나리오 처리는 [org-admin-rollout.ko.md — 사고 대응 runbook](./org-admin-rollout.ko.md#사고-대응-runbook) 참고.

---

## 자주 묻는 질문 (FAQ)

### 토큰이 어디 저장되나요?

OS 의 표준 keychain 에 저장됩니다.

- **macOS**: Keychain Access (`security` 도구가 접근)
- **Windows**: Credential Manager
- **Linux**: libsecret (gnome-keyring / KWallet)

토큰은 **머신마다 따로 (per-machine)** 저장됩니다. 회사 노트북에서 로그인했다고 해서 집 데스크탑에서 자동으로 인증되지 않습니다. 각 머신에서 한 번씩 `axhub 로그인해줘` 해야 합니다. 이것이 정상이고, 다중 머신 환경에서 보안상 더 안전합니다.

keychain 이 사용 불가한 환경 (특정 Linux 컨테이너 등) 에서는 `--token-file` 옵션으로 임시 폴백되며, 이 경우 파일은 `0600` 권한으로 `${XDG_RUNTIME_DIR}` (tmpfs, 사용자 전용) 안에만 만들어집니다.

---

### 실수로 잘못 배포했어요. 되돌릴 수 있나요?

**axhub v0.1.0 은 자동 rollback 기능을 지원하지 않습니다.** 대신 권장하는 방법은 **forward-fix** (앞으로 고쳐서 새 배포로 덮어쓰기) 입니다.

1. 로컬 코드를 이전 버전 / 정상 버전으로 되돌리기 (git revert, 또는 마지막으로 잘 돌던 commit 으로 checkout).
2. commit & push.
3. `paydrop 다시 배포해` — 새 배포가 prod 를 정상 버전으로 덮어씁니다.
4. 약 3-5 분 후 정상 복구.

또는 자연어로:

```text
방금 거 되돌려줘
```

라고 말하면 플러그인의 `recover` skill 이 자동으로 위 절차를 안내합니다 (앞으로 ship 예정 — M1+ 에서 작동).

> 회사 정책상 진짜 즉각적인 rollback 이 필요한 경우 (장애 대응 등) → 회사 axhub 관리자에게 백엔드 차원의 traffic switch 를 요청하세요. [org-admin-rollout.ko.md — 사고 대응 runbook](./org-admin-rollout.ko.md#사고-대응-runbook) 참고.

---

### `axhub` 명령이 안 보여요

원인 후보 3 가지:

**(1) axhub CLI 가 설치 안 됨.**

```bash
# macOS / Linux (Homebrew)
brew install jocoding-ax-partners/tap/axhub

# Windows (Scoop)
scoop bucket add jocoding-ax-partners https://github.com/jocoding-ax-partners/scoop-bucket
scoop install axhub
```

**(2) 설치는 됐는데 PATH 에 없음.**

```bash
# 설치 위치 찾기
which axhub          # macOS / Linux
where axhub          # Windows

# 결과가 없으면 PATH 환경변수 확인
echo $PATH           # macOS / Linux
echo %PATH%          # Windows
```

Homebrew 경로 (보통 `/opt/homebrew/bin` 또는 `/usr/local/bin`) 가 PATH 에 있는지 확인. 없으면 `~/.zshrc` (macOS) 또는 `~/.bashrc` 에 추가하고 새 터미널을 여세요.

**(3) 회사 IT 가 차단함.** 사내 보안 정책상 임의 바이너리 실행이 막혀 있을 수 있습니다. 회사 IT 또는 axhub 관리자에게 "axhub CLI 화이트리스트 등록 부탁드립니다" 요청.

빠른 진단은 `axhub 설치돼 있어?` 라고 말하면 플러그인의 `doctor` skill 이 자동으로 위 항목을 다 점검해 줍니다.

---

### Codespaces 에서 로그인이 안 돼요

Codespaces / SSH 원격 / Docker 컨테이너 / CI 같은 **headless** (브라우저가 없는) 환경에서는 OAuth Device Flow 의 자동 브라우저 열기가 동작하지 않습니다. 다음 절차로 해결합니다.

1. Claude Code 에 입력: `axhub 로그인해줘`.
2. 플러그인이 headless 환경을 감지하고 다음과 같이 응답:

   ```
   브라우저가 없는 환경 같아요. 다음 절차로 진행하세요:

   1. 아래 URL 을 본인의 PC 또는 폰의 브라우저에 복사해서 여세요:
      https://hub-api.jocodingax.ai/device

   2. 화면에 표시된 코드를 입력: ABCD-1234
   3. SSO 로그인 → "Approve" 클릭.
   4. 그러면 자동으로 여기 인증이 완료됩니다.
   ```

3. 또는 본인 PC 에서 토큰을 받은 뒤 (`axhub auth token export`) 그 결과를 token-file 로 paste:

   ```bash
   # PC 에서 (한 번만)
   axhub auth token export > /tmp/axhub-token.json

   # Codespaces 에서
   export AXHUB_TOKEN_FILE=/tmp/axhub-token.json   # 안전한 위치로 옮긴 후
   ```

토큰 파일은 반드시 `${XDG_RUNTIME_DIR}` (tmpfs) 안에만 두고, 세션이 끝나면 삭제됩니다.

---

### 회사 다른 사람의 앱이 보여요

`axhub apis list` 또는 `내 API 카탈로그 보여줘` 결과에 다른 팀 / 다른 사람의 API 가 포함되어 보일 수 있습니다. 이것은 토큰의 scope 가 본인 팀을 넘어 cross-team 까지 허용되어 있기 때문입니다.

기본 동작:
- 플러그인은 기본적으로 `--team-id $CURRENT_TEAM` 으로 필터링하여 본인 팀만 보여줍니다.
- cross-team 까지 보고 싶으면 명시적으로 "회사 전체 API 보여줘" 같은 발화가 필요하며, 이때 한 번 더 승인 카드가 뜹니다 + 감사 로그에 남습니다.
- 다른 팀 API 의 `service_base_url` 같은 민감 필드는 redact (●●●● 표시) 됩니다.

여전히 다른 사람 앱이 너무 많이 보인다면 토큰 scope 가 너무 넓을 가능성이 큽니다. 회사 axhub 관리자에게:

```
제 axhub 토큰의 scope 를 우리 팀 ($TEAM_NAME) 에만 제한해 주세요.
```

라고 요청하세요. 자세한 정책은 [org-admin-rollout.ko.md — Data classification 컨트롤](./org-admin-rollout.ko.md#data-classification-컨트롤) 참고.

---

## 도움 받기

위 내용으로 풀리지 않으면:

| 문제 종류 | 어디로 |
|---|---|
| **에러 메시지 의미 / 권한 관련** | 회사 axhub 관리자 (Slack `#axhub` 또는 사내 안내 채널) |
| **회사 정책 / SSO / 보안** | 회사 IT 보안팀, 또는 [org-admin-rollout.ko.md](./org-admin-rollout.ko.md) 의 관리자에게 전달 |
| **플러그인 자체 버그 / 기능 요청** | https://github.com/jocoding-ax-partners/axhub-plugin-cc/issues |
| **axhub CLI 자체 이슈** | https://github.com/jocoding-ax-partners/ax-hub-cli/issues |
| **사내에서 자기 동료에게 물어보기** | 빠른 시작을 함께 따라 해 달라고 부탁. 첫 배포는 동료와 함께 하면 5분 안에 끝납니다. |

> 마지막으로 다시 한 번: **에러는 정상입니다. 당신 앱은 안전합니다.** 차분하게, 한 단계씩.
