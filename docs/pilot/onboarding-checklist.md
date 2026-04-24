# Onboarding Checklist — axhub Plugin Pilot

체크박스 형태로 admin + 각 vibe coder가 사용. 모든 항목 ✅ 후 day-1 시작.

---

## Part 1: Admin (T-3 ~ T-1)

### Account + scope setup

- [ ] 회사 axhub team 생성 (`axhub team create --name <company> --owner-email <admin-email>`)
- [ ] 5명 vibe coder 추가 (`axhub team member add --email <vibe-coder-email>` × 5)
- [ ] 각 vibe coder에게 발급할 token scope 결정:
  - 권장 default: `apps:read`, `deploy:write`, `logs:read` (3개)
  - 절대 부여 금지: `team:admin`, `*:write` (와일드카드 scope)
- [ ] token 발급 (`axhub token issue --user <email> --scopes <scopes> --expires 7d`)
- [ ] token을 vibe coder에게 안전하게 전달 (Slack DM 또는 secure email — 평문 채팅 X)

### Plugin install instructions

각 vibe coder에게 발송할 메일 템플릿:

```
안녕하세요, axhub 플러그인 파일럿에 참여해주셔서 감사합니다.

설치 (한 번만):
  1. Claude Code 최신 버전 확인
  2. /plugin marketplace add jocoding-ax-partners/axhub-plugin-cc
  3. /plugin install axhub@axhub
  4. bash ${CLAUDE_PLUGIN_ROOT}/bin/install.sh   ← 환경에 맞는 helper 자동 선택
  5. /axhub:login → 받으신 token을 붙여넣어주세요

문제가 생기면 즉시 답장해주세요. 30분 안에 도와드릴 수 있어요.
```

### Pilot policies (회사 보안)

- [ ] `AXHUB_TELEMETRY` 정책 결정 (회사 보안팀 + 본인): 기본 OFF (default), 또는 ON
  - ON: vibe coder에게 명시적으로 안내 + opt-in 동의 받음
  - OFF: 별다른 액션 필요 없음 (default)
- [ ] `AXHUB_REQUIRE_COSIGN=1` 강제 enable 결정
  - 권장: 회사 노트북 표준 환경변수에 포함 (signed binary가 아닐 시 session-start에서 경고)
- [ ] HMAC consent 키 보관 정책: 각 vibe coder의 `~/.local/state/axhub/hmac-key` 는 IT 백업 X (per-user 격리)

### Day -1 dry-run

- [ ] Admin 본인이 1번 끝까지 해보기 (deploy → status → logs → recover)
- [ ] error-empathy-catalog.md 한 번 훑어서 흔한 에러 4-5개 친숙해지기
- [ ] response time SLA 계획 (4시간 권장) + 24/7 oncall person 결정

---

## Part 2: Each vibe coder (Day 1, 30분 walkthrough)

### Setup verification (5분)

- [ ] Claude Code 열림
- [ ] `/axhub:help` → command 메뉴 떠 (Korean)
- [ ] `/axhub:doctor` → axhub CLI installed + auth status OK
- [ ] `axhub auth status --json` 단독 실행 → user_email + scopes 확인

### 첫 배포 (15분, 라이브 지원 가능)

- [ ] 본인 앱 (회사가 미리 만들어둔 sandbox 앱) 의 git 레포에서 작업 시작
- [ ] 코드 한 줄 변경 (예: `console.log("hello")` 추가) + commit + push
- [ ] Claude Code에서 자연어 입력: `"방금 푸시한 거 배포해"` 또는 `/axhub:deploy`
- [ ] AskUserQuestion preview card 확인: 5 fields (앱/환경/브랜치/커밋/예상시간) 정확
- [ ] "네, 진행" → 배포 시작
- [ ] status watch 자동으로 작동 → 1-3분 안에 "배포 성공" 메시지
- [ ] 라이브 URL 클릭 → 변경사항 보임

**측정**: Day 1 walkthrough 시작 시점부터 "배포 성공" 메시지까지의 시간 (target ≤30분)

### 에러 의도적 발생 (10분)

배포 후 다음 중 하나를 의도적으로 시도:

- [ ] 다른 사람의 앱 이름으로 deploy 시도 → exit 67 (resource not found) + did-you-mean
- [ ] 의도적으로 비정상 commit sha 입력 → exit 64 (validation) + 4-part Korean message
- [ ] `/axhub:logs` 명령 → 빌드 로그 stream 확인

**측정**: 에러 메시지가 4-part Korean (감정 + 원인 + 해결 + 버튼) 형식 + 사용자가 다음 액션을 자력으로 알 수 있는지 (1-5 점수 → feedback-template.md)

### Wrap-up (5분)

- [ ] feedback-template.md 첫 제출 (5분 안에 끝나는 quick form)
- [ ] 다음 5일간 매일 자유롭게 사용해주세요 안내
- [ ] Slack/이메일 채널 안내 (issue 발생 시 즉시 컨택)

---

## Part 3: Daily use (Day 2-5)

각 vibe coder가 자기 페이스대로:

- 자기 앱 작업 → 평소 deploy 명령으로 자연스럽게
- 막힌 경우 즉시 Slack/이메일 → 우리 (4시간 SLA)
- 주 1회 feedback-template.md 제출 (또는 issue 발생 시마다)

---

## Part 4: Weekly retrospective (Day 5)

- [ ] 60분 group call (5 vibe coders + 우리)
- [ ] feedback-template.md 답변 통합 review
- [ ] Top 3 pain points 합의
- [ ] Top 3 delight moments 합의
- [ ] exit-criteria.md 의 5개 metric 잠정 평가
