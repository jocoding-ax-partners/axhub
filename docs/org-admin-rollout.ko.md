# 조직 관리자용 axhub 플러그인 도입 가이드 (한국어)

> **대상**: axhub 플러그인을 회사 / 부서 단위로 도입하려는 IT, 보안, DevOps, 플랫폼팀 관리자.
> **목표**: 30 분 안에 도입 가부 판단 + 안전한 rollout 계획 수립.

---

## 한 줄 요약

axhub 플러그인은 **사내 바이브코더 (LLM 으로 앱 만드는 비기술 / 주니어 직원) 가 자연어로 자기 앱을 prod 에 안전하게 배포할 수 있게 하는 Claude Code 플러그인** 입니다. 시니어 개발자에게 매번 묻지 않아도 되어 시간이 절약되고, 동시에 모든 destructive 동작이 HMAC consent gate 와 cosign 서명 검증을 거쳐 무권한 배포 / 공급망 공격을 차단합니다.

**관리되는 위험**: 무단 prod 배포 (HMAC consent token), 공급망 공격 (cosign default-on), cross-team 데이터 유출 (team-scope filter + audit log), 토큰 leak (per-machine OS keychain + headless flow), 잘못된 환경 배포 (live profile resolve + preview card 의무 표시).

**남는 위험 (회사가 정책으로 관리해야 함)**: 적절한 토큰 scope 발급 정책, 공유 머신에서의 logout 의무화, SSO 통합, audit log 수집/보관 정책.

---

## Pre-rollout 체크리스트 (10 항목)

도입 전에 다음을 확인하고, 미해결 항목이 있으면 그 항목을 먼저 처리하세요. 모두 ✅ 가 되어야 안전한 rollout 이 가능합니다.

| # | 항목 | 확인 방법 / 결정 사항 | 권장값 |
|---|---|---|---|
| 1 | **SSO 통합 준비됨?** | axhub 백엔드 측 SSO (SAML / OIDC) 가 회사 IdP 와 연결되어 있는지 axhub 영업/지원팀에 확인. | SSO 강제 (개인 이메일 가입 차단) |
| 2 | **토큰 정책 결정됨?** | 토큰 TTL (default 14일), 토큰 발급 권한자 (관리자만 vs 자가 발급), scope 의 최소화 원칙 | TTL 14일, scope 는 항상 본인 팀 한정으로 시작 |
| 3 | **cosign 강제 정책** | `AXHUB_REQUIRE_COSIGN=1` 을 회사 머신 기본 환경변수로 설정 | **권장: 강제 ON.** override 는 명시 옵트인 (`AXHUB_ALLOW_UNSIGNED=1`) 만 허용. |
| 4 | **플러그인 자동 업데이트 정책** | `AXHUB_DISABLE_PLUGIN_AUTOUPDATE=1` 으로 자동 업데이트 막을지, 옵트인으로 둘지 | **B2B 권장: OFF (자동 업데이트 차단).** 관리자 주도 업데이트 사이클로 변경 검증 후 sweep. |
| 5 | **공유 머신 정책** | 인턴 노트북, hot-desk PC 처럼 여러 사람이 한 머신을 쓰는 경우 정책 | 세션 종료 시 `axhub auth logout` 의무화 + 사내 가이드에 명시 |
| 6 | **Audit log endpoint 준비됨?** | 사내 SIEM / log 수집 인프라 (Splunk, Datadog, Loki 등) 에 axhub 로그를 보낼 endpoint 결정. | `AXHUB_AUDIT_URL=https://siem.yourcompany.com/axhub` 같이 설정 |
| 7 | **TLS proxy 정책** | 사내 corporate proxy / TLS 검사 (MITM) 가 있는 환경에서 OAuth / API 호출이 통과하는지 | 필요 시 `AXHUB_ALLOW_PROXY=1` + 회사 root CA 를 axhub 신뢰 store 에 추가 |
| 8 | **비상 연락 채널** | 바이브코더가 prod 를 망가뜨렸을 때 누가 받는지 명확히 | Slack `#axhub-incident` 채널 + on-call rotation |
| 9 | **Data classification 컨트롤** | `apis list` 에서 cross-team 데이터 leak 방지 정책 | 기본 `--team-id $CURRENT_TEAM` scope 강제, cross-team 은 audit-logged AskUserQuestion 후만 |
| 10 | **GDPR / data residency** | 회사가 EU / KR 등 데이터 거주 규제 대상인 경우 axhub backend 의 region 옵션 확인 | axhub 영업에 region pinning 가능 여부 사전 확인 |

> 미해결 항목이 3 개 이상이면 rollout 보류를 권장합니다. 기술적으로는 동작하나, **trust thesis** (사용자가 안심하고 쓸 수 있다는 전제) 가 깨집니다.

---

## 배포 계획 (3-step rollout)

### Step 1. Marketplace URL 게시 + 사내 안내 (1주차)

플러그인을 사내 마켓플레이스 또는 GitHub org 로 등록하고, 바이브코더 대상 그룹에 안내합니다.

**사내 Slack 안내 템플릿 (한국어, 그대로 복사 가능)**:

```
🚀 axhub 플러그인 (Claude Code) 사내 도입 안내

안녕하세요, [부서 / 팀 이름] 님.

오늘부터 회사 axhub 플랫폼에 자기 앱을 배포할 때 Claude Code 한 곳에서
자연어로 처리할 수 있게 되었습니다 ("내 paydrop 배포해" 같이).

📌 5 분 안에 첫 배포까지 따라 할 수 있는 가이드:
   👉 https://github.com/jocoding-ax-partners/axhub-plugin-cc/blob/main/docs/vibe-coder-quickstart.ko.md

📌 막혔을 때 보는 troubleshooting:
   👉 https://github.com/jocoding-ax-partners/axhub-plugin-cc/blob/main/docs/troubleshooting.ko.md

📌 onboarding 30 분 세션:
   - [날짜 / 시간 / Zoom 링크]
   - 라이브로 함께 첫 배포 해 봅니다 (재미있어요).

📌 문제 / 질문 / 피드백:
   - Slack #axhub 채널 (담당자: @[관리자 ID])
   - 비상 시: #axhub-incident 채널 (24/7)

처음이라 무서울 수 있지만 plugin 이 모든 destructive 동작 전에 한 번 더 묻습니다.
"엔터" 한 번에 prod 가 망가지는 일은 일어나지 않습니다.

질문 환영합니다 🙌
```

### Step 2. 30 분 onboarding 세션 (1주차 후반)

라이브로 진행하면 가입 → 첫 배포까지 함께 할 수 있어 abandonment 율이 크게 줄어듭니다.

**슬라이드 outline (10 슬라이드, 한국어)**:

1. **이게 왜 필요한가** (3 분) — 시니어에게 매번 부탁하는 시간 절약, 동시에 안전 보장
2. **준비물 점검** (2 분) — Claude Code, axhub CLI, 사내 axhub 계정
3. **라이브: 플러그인 설치** (2 분) — `/plugin marketplace add ...` 함께 입력
4. **라이브: 첫 로그인** (3 분) — "axhub 로그인해줘" → SSO 화면 → Approve
5. **라이브: 내 앱 확인** (2 분) — "내 앱 목록 보여줘"
6. **라이브: 첫 배포 (dry-run)** (5 분) — "한번 해보기만" 으로 시뮬레이션
7. **라이브: 진짜 배포** (5 분) — preview card 5 줄 같이 읽기, 승인, watch
8. **에러가 떴을 때** (3 분) — exit 65, 64, 67 시연 + 한국어 안내 메시지 보기
9. **흔한 질문** (3 분) — rollback, 다른 환경, 토큰 만료
10. **도움 받는 곳** (2 분) — Slack 채널, troubleshooting.ko.md, 비상 시

### Step 3. 첫 1 주 모니터링 (2 주차)

도입 첫 주에 다음을 매일 점검하세요. 문제가 보이면 즉시 대응합니다.

| 모니터링 항목 | 어디서 | 임계값 |
|---|---|---|
| 첫 배포 성공률 | audit log + Slack 자가 보고 | < 80% 면 onboarding 보강 필요 |
| Unsafe-trigger 발생 (잘못된 앱/환경 배포) | audit log 의 `app_mismatch` / `profile_mismatch` 이벤트 | **0 건이 목표**. 1 건이라도 발견 시 즉시 분석 + 사내 alert |
| `cosign_verification_failed` | `axhub update apply` 의 exit 66 카운트 | 0 건. 발생 시 [사고 대응 runbook](#사고-대응-runbook) 즉시 |
| 토큰 만료로 인한 재로그인 | `exit 65` 카운트 | 사용자당 주 1-2회 정상. 그 이상이면 TTL 정책 재검토 |
| Cross-team API 조회 시도 | audit log 의 `apis_list_cross_team` | 정당한 사용 외에는 0. 의외의 시도 발견 시 토큰 scope 좁히기 |
| 신규 사용자의 abandonment (설치 후 7일 내 미사용) | 로그인 이벤트 | > 30% 면 onboarding 1:1 follow-up |

---

## 정책 레버 (환경변수 표)

회사 정책으로 다음 환경변수를 사내 머신의 기본 환경 (`/etc/profile.d/axhub.sh` 또는 EDR 의 환경변수 push) 에 설정할 수 있습니다.

| 환경변수 | 효과 | 권장값 (B2B 회사) | 주의 |
|---|---|---|---|
| `AXHUB_AGENT=1` | `--json --no-input + ANSI strip` 자동 적용. agent 친화. | 1 | Claude Code 환경에서는 플러그인이 자동 설정 |
| `AXHUB_REQUIRE_COSIGN=1` | `update apply` 시 cosign 서명 검증 강제. 검증 실패 = hard stop. | **1 (강제)** | override 는 `AXHUB_ALLOW_UNSIGNED=1` 만 허용 |
| `AXHUB_ALLOW_UNSIGNED=1` | cosign 검증 우회 (긴급 시만). | **0 (금지)** | 사용 시 audit log 에 명시 + 관리자 승인 필수 |
| `AXHUB_DISABLE_PLUGIN_AUTOUPDATE=1` | 플러그인 자동 업데이트 차단. | **1 (차단 권장)** | 관리자 주도 sweep 으로 전환 |
| `AXHUB_DISABLE_AUTOUPDATE=1` | CLI 자동 업데이트 차단. | 1 (CI / airgap) / 0 (개발자 머신) | 사내 정책에 따라 |
| `AXHUB_PROFILE=<name>` | 기본 profile 명시. 다중 환경 시 필수. | 회사 production profile 명 | profile mismatch 시 plugin 이 명시 confirm 요구 |
| `AXHUB_ENDPOINT=<url>` | API endpoint override. | 기본값 사용 (회사 dedicated 인 경우만 변경) | 잘못 설정 시 무한 401 |
| `AXHUB_AUDIT_URL=<url>` | audit log 를 사내 SIEM 으로 전송. | `https://siem.yourcompany.com/axhub` | TLS + auth header 필요 |
| `AXHUB_ALLOW_PROXY=1` | corporate MITM proxy 의 root CA 신뢰. | 회사 proxy 환경이면 1 | proxy 가 토큰을 볼 수 있음 → 신뢰 가능한 환경에만 |
| `AXHUB_TOKEN_FILE=<path>` | keychain 폴백. headless / Codespaces 용. | 미설정 (keychain 우선) | 경로는 `${XDG_RUNTIME_DIR}` 안만 허용 |
| `AXHUB_WATCH_INTERVAL=5s` | `deploy status --watch` 폴링 간격. | 기본값 | 1s..30s clamp |
| `AXHUB_WATCH_TIMEOUT=2m` | watch 최대 대기 시간. | 기본값 | 빌드가 긴 앱이 많으면 5m 권장 |
| `AXHUB_TIMEOUT=30s` | API 호출 timeout. | 기본값 | 사내 망 느리면 60s |
| `AXHUB_NO_INPUT=1` | non-TTY 환경에서 prompt 없이 실패. | CI 에서 1 | 개발자 머신에서는 0 |

> 환경변수 push 는 회사 EDR / MDM (Jamf, Intune 등) 또는 dotfiles 자동 배포 도구로 일관되게 적용하세요. 사용자가 임의로 끄지 못하도록 read-only 로 설정하는 것을 권장합니다.

---

## 사고 대응 runbook

바이브코더가 prod 를 망가뜨렸거나, 의심스러운 보안 이벤트가 발생했을 때의 단계별 절차입니다.

### Severity 1 — Prod 가 다운 / 사용자 트래픽 영향

```
T+0    Slack #axhub-incident 에 사용자 또는 모니터링이 알림.
T+2분  on-call 관리자가 받음 → axhub 콘솔에서 해당 앱의 마지막 정상
       deployment id 확인.
T+5분  axhub 백엔드 차원의 traffic switch 로 prod 트래픽을 마지막 정상
       deployment 로 즉시 회귀 (axhub 운영팀 또는 관리자 권한 필요).
       -- 바이브코더의 forward-fix 보다 빠르게 user impact 차단.
T+10분 사용자 트래픽 정상화 확인 + 사내 status page 업데이트.
T+30분 root cause 분석 시작 (실패한 deployment 의 빌드/런타임 로그).
T+1일  postmortem 문서 + 재발 방지 항목 (테스트 추가, preview card
       추가 안내 문구 등).
```

### Severity 2 — 의심스러운 행동 / 무권한 시도

다음 audit 이벤트가 보이면 Severity 2 로 다룹니다:

- `unsafe_trigger` (deploy intent 가 잘못된 앱 / 환경에 라우팅됨)
- `consent_bypass_attempt` (HMAC consent token 없이 destructive 시도)
- `app_ambiguous` 후 사용자가 명백히 다른 사람 앱을 선택
- `apis_list_cross_team` 가 정당한 사유 없이 빈번히 발생

```
T+0     Audit log 알람.
T+15분  관리자가 사용자에게 직접 contact (Slack DM) → 의도 확인.
T+30분  실수면 onboarding 보강. 의도적이면 토큰 scope 즉시 좁히기 +
       사내 보안팀에 escalate.
T+1일  사내 보안 인시던트 트래커에 등록 + 정책 review.
```

### Severity 3 — `cosign_verification_failed`

공급망 공격 가능성. 관리자가 직접 처리.

```
T+0     사용자가 #axhub-incident 에 신고 또는 audit log 알람.
T+5분   관리자가 axhub 운영팀에 confirm: "지금 우리 회사 사용자가
       cosign 검증 실패를 보고 있습니다. 정상 서명 이미지인가요?"
T+15분  axhub 운영팀 답변에 따라:
       - 운영 사고면 (서명 깨짐): 모든 사용자에게 update apply 보류 안내,
         정상 서명 재배포 대기.
       - 운영 사고 아니면 (실제 서명 위조 의심): 보안팀 에스컬레이션,
         네트워크 트래픽 분석 (proxy 로그, DNS 로그 점검).
T+1일  postmortem.
```

### Severity 4 — 토큰 leak / 분실

```
T+0     사용자가 토큰이 leaked 됐다고 신고 (예: 실수로 commit, 노트북 분실).
T+5분   관리자가 axhub 콘솔에서 해당 사용자 토큰 즉시 revoke.
T+10분  사용자에게 새 토큰 발급 + 사내 가이드 따라 재로그인 안내.
T+30분  audit log 점검: revoke 이전에 그 토큰으로 destructive 동작이 있었는지.
T+1일  사내 secret-scanning (TruffleHog, Gitleaks) 정책 보강 검토.
```

---

## Data classification 컨트롤

axhub 의 가장 큰 운영 risk 는 cross-team 데이터 leak 입니다. 다음 패턴을 강제하세요.

### 1. apis list 기본 scope 차단

플러그인은 기본적으로 `--team-id $CURRENT_TEAM` 스코프로 호출하도록 설계되어 있습니다 (PLAN §16.17). 회사 정책으로 다음을 보장하세요:

- 사용자 토큰의 scope 발급 시 cross-team 권한은 명시 요청자에게만 (시니어 / 플랫폼팀).
- 플러그인의 `apis list` 실행 결과는 다른 팀 API 의 `service_base_url` 을 자동 redact 합니다 (●●●●).
- cross-team list 시도는 audit log 의 `apis_list_cross_team` 이벤트로 기록됩니다.

### 2. 감사 로그 활성화

```bash
# 회사 표준 머신 환경변수
export AXHUB_AUDIT_URL="https://siem.yourcompany.com/axhub"
export AXHUB_AUDIT_AUTH_HEADER="Bearer ${AXHUB_AUDIT_TOKEN}"
```

수집 권장 이벤트:
- `auth_login`, `auth_logout`, `auth_token_refresh`
- `deploy_create_consent_minted`, `deploy_create_succeeded`, `deploy_create_failed`
- `consent_bypass_attempt`, `unsafe_trigger`
- `apis_list_cross_team`, `apis_list_redacted`
- `cosign_verification_failed`, `update_apply_succeeded`
- `profile_mismatch_warning`, `app_ambiguous_resolved`

### 3. 신규 사용자 등록 절차

신규 바이브코더 합류 시:

1. 회사 SSO 에 사용자 추가 (HR 자동 프로비저닝 권장).
2. axhub 콘솔에서 본인 팀 only scope 토큰 발급.
3. 첫 머신에서 `axhub 로그인해줘` → 가이드 함께 진행.
4. [vibe-coder-quickstart.ko.md](./vibe-coder-quickstart.ko.md) URL 을 안내.
5. 30분 onboarding 세션 (필수) — 위 [Step 2](#step-2-30-분-onboarding-세션-1주차-후반) 참고.
6. 첫 배포는 시니어 또는 관리자가 옆에서 함께 (psychological safety + 정책 학습).

---

## 권장 sample policy (회사가 카피해서 쓰는 1-page 정책 템플릿)

> 아래 템플릿을 회사 정책 문서에 그대로 복사해서 사용 (필요 부분 수정).

```markdown
# [회사명] axhub Claude Code 플러그인 사용 정책 v1.0

## 적용 범위
- 본 정책은 [회사명] 의 axhub 플랫폼에 앱을 배포 / 관리하는 모든 직원에게 적용된다.
- axhub 플러그인을 Claude Code 에 설치한 모든 머신에 적용된다.

## 1. 인증
- 회사 SSO 통한 로그인만 허용된다. 개인 이메일 가입 금지.
- 토큰 TTL 은 14 일이다. 만료 시 SSO 재로그인.
- 토큰은 OS keychain 에 보관된다. 토큰 파일 export 금지.
- 회사 노트북 분실 / 도난 시 즉시 #axhub-incident 채널에 신고.

## 2. 환경변수 (회사 표준)
- `AXHUB_REQUIRE_COSIGN=1` (필수, 변경 금지)
- `AXHUB_DISABLE_PLUGIN_AUTOUPDATE=1` (필수, 변경 금지)
- `AXHUB_AUDIT_URL=https://siem.[회사명].com/axhub` (필수, 변경 금지)
- `AXHUB_PROFILE=production` (기본)

## 3. 배포 행동 규범
- preview card 의 5 줄 (앱 / 환경 / 브랜치 / 커밋 / 예상 시간) 을 반드시
  읽은 후 승인한다.
- 다른 사람의 앱이 표시되면 즉시 `[아니요]` 누르고 [관리자 이름] 에게 알린다.
- prod 첫 배포는 시니어 또는 관리자가 옆에서 함께 한다.
- `--dry-run` 또는 "한번 해보기만" 발화로 사전 시뮬레이션을 권장한다.

## 4. 공유 머신
- 인턴 노트북, hot-desk, 회의실 PC 등 공유 머신 사용 시 작업 종료 직전
  반드시 `axhub auth logout` 실행.
- 자기 토큰을 다른 사람 머신에 절대 복사 / 동기화 금지.

## 5. 사고 대응
- prod 다운 / 사용자 영향 의심: #axhub-incident 즉시 (24/7).
- `cosign_verification_failed` 발생: 즉시 관리자 contact, 절대 강제 우회 금지.
- 본인 토큰이 leak 됐을 가능성 (예: 실수 commit): 즉시 #axhub-incident.

## 6. 위반 시
- 1 회 위반: 안내 + 추가 onboarding.
- 2 회 위반: 토큰 scope 축소 (read-only 강등).
- 3 회 위반 또는 의도적 정책 우회 시도: 토큰 revoke + 사내 보안팀 escalate.

문의: [관리자 이름], [이메일], Slack #axhub
```

---

## 참고

- 신규 바이브코더 등록 / 첫 배포 절차의 사용자 측 가이드: [vibe-coder-quickstart.ko.md](./vibe-coder-quickstart.ko.md)
- 사용자가 마주치는 에러 카탈로그 + FAQ: [troubleshooting.ko.md](./troubleshooting.ko.md)
- 플러그인 설계 근거 (6 phases of review, 65 audit-tracked decisions): [PLAN.md](../PLAN.md)
- ax-hub-cli (CLI 자체) v0.1.0 contract: https://github.com/jocoding-ax-partners/ax-hub-cli

질문 / 정책 customization 요청은 axhub 영업 또는 [관리자 contact] 로 보내 주세요.
