# Pilot Launch Readiness Checklist

Day -1 admin 점검 양식. **5/5 항목 모두 ✓ 일 때만 day 1 시작**. ✗ 1개라도 → pilot 24시간 연기 + root cause 처리.

---

## 1. Account setup (5명 vibe coder 토큰 발급 status)

- [ ] axhub team 생성 — team name: `_____________________`, owner: `_____________________`
- [ ] vibe coder #1 — email: `_____________________`, token issued: ✓ ✗, scopes: `apps:read, deploy:write, logs:read`
- [ ] vibe coder #2 — email: `_____________________`, token issued: ✓ ✗
- [ ] vibe coder #3 — email: `_____________________`, token issued: ✓ ✗
- [ ] vibe coder #4 — email: `_____________________`, token issued: ✓ ✗
- [ ] vibe coder #5 — email: `_____________________`, token issued: ✓ ✗
- [ ] 각 token expires 7d 이상 (pilot 기간 cover)
- [ ] 회사 sandbox 앱 1개 이상 등록 (vibe coder 가 first deploy 할 target)

verified by: `_____________________`
date: `____________`

## 2. Plugin install 안내 메일 발송 status

- [ ] 5명 vibe coder 모두에게 안내 메일 발송됨 (template: `docs/pilot/onboarding-checklist.md` 의 메일 본문)
- [ ] 메일에 token 포함 (secure 채널: Slack DM 또는 secure email — 평문 채팅 X)
- [ ] 메일에 day-1 walkthrough 일정 안내 포함
- [ ] 메일 reception 확인 (vibe coder 5/5 답장 또는 confirm 받음)

verified by: `_____________________`
date: `____________`

## 3. SLA on-call 배정 (4시간 response)

- [ ] Primary on-call person: `_____________________`, slack handle: `_____________________`
- [ ] Backup on-call: `_____________________` (대체 인원, primary 부재 시)
- [ ] On-call 본인 이번 주 axhub 플러그인 docs 한 번 훑음 (`README.md`, `docs/RELEASE.md`, `docs/pilot/admin-rollout.ko.md`)
- [ ] On-call 본인 직접 plugin 설치 + first deploy 1번 해봄 (vibe coder 가 만날 모든 화면 친숙)
- [ ] Slack 또는 이메일 어떤 채널에서 SLA 4시간 측정 시작 시점 정의 (예: vibe coder 메시지 발송 시점)

verified by: `_____________________`
date: `____________`

## 4. Feedback 수집 채널 확립

- [ ] 메인 채널: ___ Slack DM / ___ secure email / ___ 기타 `_____________________`
- [ ] 5명 vibe coder 모두 채널 access 확인 (테스트 메시지 1개 발송 + 5/5 ack)
- [ ] feedback-template.md 양식 안내됨 (link 또는 PDF/doc 첨부)
- [ ] 일일 feedback 제출 + 주 1회 retro 시점 모두 캘린더에 등록

verified by: `_____________________`
date: `____________`

## 5. Day-1 walkthrough 일정 + emergency rollback procedure

- [ ] 5명 × 30분 walkthrough 일정 confirmed (vibe coder + on-call 양쪽 캘린더 invite)
- [ ] Walkthrough 동안 화면 공유 도구 합의 (Zoom/Slack huddle/Google Meet 등)
- [ ] Emergency rollback procedure 위치 link: `_____________________` (보통 `docs/pilot/admin-rollout.ko.md` §3 sev1/2/3 incident response)
- [ ] Token revoke 절차 admin 본인이 한 번 dry-run 해봄: `axhub token revoke --token-id <id>` 또는 axhub admin UI 접근 확인
- [ ] Plugin disable 절차 안내 가능: `/plugin disable axhub` (vibe coder side)

verified by: `_____________________`
date: `____________`

---

## GO / NO-GO 결정

| Section | Status |
|---|---|
| 1. Account setup | ___ ✓ / ___ ✗ |
| 2. Install 안내 메일 | ___ ✓ / ___ ✗ |
| 3. SLA on-call | ___ ✓ / ___ ✗ |
| 4. Feedback 채널 | ___ ✓ / ___ ✗ |
| 5. Walkthrough + rollback | ___ ✓ / ___ ✗ |

**5/5 ✓ 일 때만 GO**. 4/5 이하 → 24시간 연기, 부족 항목 처리.

GO date: `____________`
GO decision by: `_____________________` (admin) + `_____________________` (jocoding-ax-partners 측)

---

## 별첨: release verification baseline (v0.1.1)

**Pilot 권장 release: v0.1.1** (v0.1.0 은 cosign cert 누락으로 keyless verify 부분 실패 — Phase 6 release pipeline fix 후 v0.1.1 출시).

Pilot 시작 전 plugin v0.1.1 release cosign signature + sha256 무결성 검증 결과:

- 검증 명령: `bash scripts/release/verify-release.sh v0.1.1`
- 결과 캡처: `docs/pilot/v0.1.1-verify-result.txt`
- 검증 시점: `2026-04-24 (Phase 6 ralph)`
- 검증자: `jocoding-ax-partners (Phase 6 architect ralph)`
- 검증 결과: **✓ All release assets verified for jocoding-ax-partners/axhub@v0.1.1** — manifest.json signature OK + 5 binary signatures OK + sha256 manifest match.

이 baseline 은 pilot 진행 중 supply chain 의심 시점에 재검증 비교 기준이 됩니다.

### v0.1.0 known limitation (참고용)

v0.1.0 은 release pipeline 의 `--output-certificate` flag 누락으로 .pem 파일 미생성 → cosign keyless verify 실패 (signature 만 있고 cert 부재). v0.1.0 release 자체는 그대로 두되 (history preservation), 신규 install + pilot 은 v0.1.1 사용 권장.

이는 supply chain 공격 신호 X — 단순 release pipeline 의 step 누락. v0.1.1 에서 fix 되어 모든 14 asset (5 bin + 5 sig + 5 cert + manifest + checksums + 4 sidecars = 17 actual) 정상 검증.
