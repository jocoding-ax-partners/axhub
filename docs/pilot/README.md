# axhub Plugin First-Customer Pilot

> 1 회사 × 5 vibe coders × 1 week — 첫 실사용자 검증.

## Pilot scope

axhub Claude Code 플러그인이 실제 비-개발 환경에서 작동하는지 검증합니다. 6 phases of audit + 241+ test passing은 코드 품질을 증명하지만, **vibe coder가 실제로 막힘 없이 첫 배포까지 도달하는지**는 사람이 직접 써봐야만 알 수 있습니다.

## Success metrics (필수 통과 조건)

| Metric | Target | 측정 방법 |
|---|---|---|
| 첫 배포까지 소요 시간 | ≤30분 (median) | onboarding 시작 → 첫 `axhub deploy create` exit 0 까지의 timestamp 차 |
| 5/5 vibe coder가 첫 배포 성공 | 100% (5명 모두) | per-vibe-coder onboarding 체크리스트의 "deploy success" 칸 |
| Critical incident (token leak, cross-team list bypass, cosign verification override) | 0건 | telemetry usage.jsonl + admin audit log |
| 평균 에러 메시지 이해도 점수 | ≥4.0/5.0 | 매 에러 발생 직후 1-5 점수 (feedback-template.md) |
| Korean copy quality 점수 | ≥4.0/5.0 | feedback-template.md 항목 #3 |

5개 metric 모두 통과해야 GO. 1개라도 fail이면 KILL → root cause + fix → re-pilot.

## Timeline (1 week sprint)

| Day | 활동 | 담당 |
|---|---|---|
| -3 | Admin onboarding (회사 axhub team 생성, 5 vibe coder 토큰 발급) | IT/admin |
| -1 | Vibe coder accounts ready, plugin install 안내 메일 발송 | IT/admin + 우리 |
| 1 (월) | Day-1 walkthrough (5 vibe coder × 30분 each) | 우리 (라이브 지원 가능) |
| 1-5 | Daily use, async feedback (feedback-template.md submission) | 5 vibe coder |
| 5 (금) | Weekly retrospective (60분 group call) | 5 vibe coder + 우리 |
| 6-7 | Exit criteria 평가, GO/KILL 판단 | 우리 + IT/admin |

## Feedback collection mechanism

- **인시던트 발생 시**: 즉시 Slack/이메일 → 우리 (response SLA 4시간)
- **Daily**: feedback-template.md submission (async, 30분 cadence)
- **Weekly**: group retrospective call (Day 5 오후)

## Pilot artifacts (this directory)

- `README.md` — 이 문서 (pilot 전체 개요)
- `onboarding-checklist.md` — admin + vibe coder day-1 체크리스트
- `feedback-template.md` — 표준 피드백 폼 (Korean)
- `admin-rollout.ko.md` — IT/admin 정책 가이드
- `exit-criteria.md` — GO/KILL 결정 기준 + 평가 프로세스

## Out of scope

- Customer recruitment (이건 우리 영업/제품 팀의 일, 이 PRD는 prep kit만 ship)
- Real ax-hub-cli staging credentials (US-206 separately scaffolded)
- Marketing assets (landing page copy, demo video) — Phase 4

## Phase 4 follow-up (after pilot)

Pilot success → Marketplace publish → 5-10 회사 expansion. Pilot fail → root cause document + Phase 4 plan revision.
