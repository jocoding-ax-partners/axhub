# Pilot Exit Criteria — GO/KILL Decision

Pilot Day 7 (월요일) 까지 5개 metric 평가. 5개 모두 통과 → GO (Phase 4 marketplace publish + 5-10 회사 expansion). 1개라도 fail → KILL (root cause + Phase 4 plan 재작성).

---

## Metric 1: 5/5 vibe coder가 첫 배포 성공

**기준**: pilot Day 1-5 안에 5명 모두 1번 이상 `axhub deploy create` exit 0 + 라이브 URL 확인까지 완주.

**측정**:
- 각 vibe coder의 onboarding-checklist.md "첫 배포" 섹션 ✅ 카운트
- telemetry usage.jsonl (활성화 시) 의 `event: classify_exit, exit_code: 0, command_class: "axhub deploy create"` 행 카운트 per session_id

**Pass**: 5/5
**Fail**: 4/5 이하 → 막힌 vibe coder 1:1 인터뷰로 root cause + KILL

---

## Metric 2: 첫 배포까지 ≤30분 (median)

**기준**: 5명의 onboarding-checklist Day 1 walkthrough 시작 시각 → 첫 deploy 성공 시각 차이의 median.

**측정**:
- per-vibe-coder timestamp 기록 (Day 1 walkthrough 시작 + 첫 deploy 성공)
- 5명 차이의 median 계산

**Pass**: median ≤30분
**Fail**: median >30분 → onboarding flow 의 어떤 단계에서 시간 잡아먹는지 분석 (login? install.sh? preview card? auth flow?)

---

## Metric 3: Critical incident 0건

**기준**: 다음 중 어떤 것도 발생하지 않음:
- token leak: vibe coder의 axhub_pat_* 가 transcript / 공유 파일 / 외부 시스템에 노출
- cross-team list bypass: AskUserQuestion 거치지 않고 다른 팀의 endpoint 가 노출
- cosign verification override: `AXHUB_ALLOW_UNSIGNED=1` 으로 검증 우회 (vibe coder 노트북에서 발견 시)
- destructive op 가 consent gate 우회: PreToolUse hook deny 없이 deploy_create / update_apply / auth_login 실행 (telemetry usage.jsonl 의 `preauth_check_deny` 없이 destructive command 가 fired)

**측정**:
- audit log 검토 (~/.cache/axhub-plugin/cross-team-list.ndjson, telemetry usage.jsonl)
- vibe coder 5명의 Slack 채널 keyword 검색 (axhub_pat, "강제로", "우회")
- weekly retro 그룹 콜에서 직접 질문

**Pass**: 0건 (모든 4가지 카테고리)
**Fail**: 1건이라도 → 즉시 KILL + sev1 incident response (위 admin-rollout.ko.md §3 참고)

---

## Metric 4: 평균 에러 메시지 이해도 ≥4.0/5.0

**기준**: feedback-template.md 항목 #2 (에러 메시지 이해도) 의 5명 × N 답변 평균.

**측정**:
- 각 vibe coder의 모든 feedback 제출 항목 #2 점수 합산 → 평균
- N 은 가변 (vibe coder별 1주일에 0회~여러 회). 최소 N=10 (5명 × 평균 2회) 이상이어야 통계적 유의

**Pass**: 평균 ≥4.0
**Fail**: 평균 <4.0 → 어떤 에러 메시지가 점수 끌어내렸는지 frequency × score 곱으로 ranking, top 3 catalog.ts 항목 다시 작성

---

## Metric 5: Korean copy quality ≥4.0/5.0

**기준**: feedback-template.md 항목 #3 (한국어 톤) 의 5명 × N 답변 평균.

**측정**: Metric 4 와 동일 방식.

**Pass**: 평균 ≥4.0
**Fail**: 평균 <4.0 → 인용된 어색한 구절 list → 회사 한국어 카피라이터 + 우리 cross-review → SKILL.md 업데이트

---

## Decision matrix

| Metric 1 | Metric 2 | Metric 3 | Metric 4 | Metric 5 | 결정 |
|---|---|---|---|---|---|
| ✅ | ✅ | ✅ | ✅ | ✅ | **GO** — Phase 4 진행 |
| ✅ | ✅ | ✅ | ✅ | ❌ | **PARTIAL** — Korean copy 만 fix 후 mini-pilot 1주 추가 |
| ✅ | ✅ | ✅ | ❌ | ✅ | **PARTIAL** — top 3 에러 catalog 수정 후 mini-pilot |
| ✅ | ❌ | ✅ | ✅ | ✅ | **PARTIAL** — onboarding flow simplify 후 mini-pilot |
| ❌ | * | * | * | * | **KILL** — 5/5 100% 안 되면 무조건 root cause 우선 |
| * | * | ❌ | * | * | **KILL** — critical incident 는 무조건 KILL |

`*` = 결과 무관

PARTIAL → 최대 1회 mini-pilot 후 재평가. 2회 연속 PARTIAL → 사실상 KILL.

---

## After GO decision (Phase 4 immediate next steps)

1. Marketplace asset 준비 (cosign-signed manifest + landing page copy)
2. 도입 paying customer 5-10 회사 영업 시작 (제품 + 영업팀)
3. Pilot 5명 vibe coder를 advocate 로 전환 (case study 인터뷰 동의 받음)
4. Phase 5 telemetry analytics dashboard 시작 (opt-in 데이터 → 사용 패턴 분석 → 다음 hardening 우선순위)

## After KILL decision (rollback)

1. Pilot 5명에게 솔직한 결과 공유 + plugin uninstall 안내
2. 사고 (있다면) 사후 분석 보고서 vibe coder 본인 + 회사 admin에게 공유
3. Root cause + Phase 4 plan 재작성 (Tier 1 ~ Phase 3 의 어떤 가정이 깨졌는지 명시)
4. 우리 입장 재평가 — pilot이 실패한 가설은 다음 ralph 사이클의 PRD 첫 항목

---

GO / KILL 결정은 우리(jocoding-ax-partners) + 회사 admin 합의로 발표. vibe coder 5명에게는 결정 후 24시간 안에 결과 + 다음 단계 안내.
