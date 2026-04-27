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

## 별첨: release verification baseline (v0.1.9)

**Pilot 권장 release: v0.1.9** (Phase 11 work shipped as v0.1.8 → UTF-8 BOM hotfix v0.1.9 — windows-smoke CI caught Korean mojibake before pilot exposure).

Pilot 시작 전 plugin v0.1.9 release cosign signature + sha256 무결성 검증 결과:

- 검증 명령: `bash scripts/release/verify-release.sh v0.1.9`
- 검증 시점: 2026-04-27
- 검증자: jocoding-ax-partners (Phase 11 ralph)
- 검증 결과: **✓ All release assets verified for jocoding-ax-partners/axhub@v0.1.9**
- Live binary self-report (darwin-arm64): `axhub-helpers 0.1.9 (plugin v0.1.9, schema v0)` ✓
- Windows GHA CI smoke: ✓ install.ps1 + session-start.ps1 + Add-Type advapi32!CredReadW PInvoke 모두 green
- Linux Docker smoke: ✓ secret-tool → axhub-helpers token-init exit=0 → token written mode=600

### v0.1.9 evolution from v0.1.8 (BOM hotfix story)

1. v0.1.8 shipped Phase 11 (codegen install.ps1 + format-parity + Windows GHA CI workflow)
2. windows-smoke CI ran for FIRST time on v0.1.8 tag
3. PowerShell 7 on Windows mojibake'd Korean strings (UTF-8 .ps1 read as Windows-1252 without BOM)
4. install.ps1 crashed at parse time before AXHUB_SKIP_AUTODOWNLOAD env check
5. v0.1.9 hotfix prepended UTF-8 BOM to all 3 .ps1 files
6. v0.1.9 windows-smoke CI: GREEN
7. **Detection win:** without Phase 11 windows-smoke CI workflow, this would have shipped to vibe coder Windows pilots

## 별첨 (예전): release verification baseline (v0.1.7)

**Pilot 권장 release: v0.1.7** (Phase 10 — Windows PS1 hooks 추가. Git Bash/WSL 없이도 stock Windows 10/11 에서 plugin end-to-end 작동. Claude Code >= 2.1.84 필수).

Pilot 시작 전 plugin v0.1.7 release cosign signature + sha256 무결성 검증 결과:

- 검증 명령: `bash scripts/release/verify-release.sh v0.1.7`
- 결과 캡처: `docs/pilot/v0.1.7-verify-result.txt`
- 검증 시점: `2026-04-27 (Phase 10 ralph)`
- 검증자: `jocoding-ax-partners (Phase 10 architect ralph)`
- 검증 결과: **✓ All release assets verified for jocoding-ax-partners/axhub@v0.1.7** — manifest.json signature OK + 5 binary signatures OK + sha256 manifest match.
- Live binary self-report (darwin-arm64): `axhub-helpers 0.1.7 (plugin v0.1.7, schema v0)` ✓
- Windows binary smoke: 109.6M PE32+ x86-64 ✓ (실제 Windows VM 실행 검증은 다음 pilot 세션).
- Release-time cross-check: `bin/install.{sh,ps1}` 둘 다 v0.1.7 literal 포함 (D2 통과).

### v0.1.7 known tradeoffs (Phase 11 deferred)

1. `.ps1` NOT Authenticode-signed — EDR / V3 / AhnLab / CrowdStrike 차단 가능. ERR_EDR systemMessage 가 AXHUB_TOKEN env var fallback 안내.
2. macOS 에서 wrong-OS `"shell":"powershell"` spawn 동작 silent 가정 (직접 verify 안 함). Hotfix v0.1.7.1 hotfix-ready if first pilot reports visible-error.
3. `install.sh:80` operator precedence bug NOT replicated in install.ps1 — sh-side fix tracked for future v0.1.x.
4. codegen-install-version 가 아직 install.ps1 자동 sync 안 함 — 매 release 마다 수동 bump 필요. v0.1.8 에서 codegen 확장.
5. Claude Code minVersion 2.1.84 floor 은 CHANGELOG + admin-rollout 문자열에만 명시. manifest schema 에 `claude_code.minVersion` 필드 없음 → 더 오래된 client silent fail.

### v0.1.5 known tradeoff (EDR 차단 가능성)

windows-amd64.exe 는 **Authenticode 서명 안 됨** (v0.1.6 deferred). 회사 EDR / AMSI / V3 / AhnLab / CrowdStrike 가 inline PInvoke (advapi32!CredReadW) 를 Mimikatz 패턴으로 분류해 차단할 가능성 있음. Korean 에러 메시지 (`keychain-windows.ts:ERR_EDR`) 가 이를 honest 하게 owning — `AXHUB_TOKEN` 환경변수가 v0.1.6 코드사이닝 전까지 정식 회피 경로.

### v0.1.0 ~ v0.1.6 known limitations (참고용, history preservation)

- **v0.1.0**: release pipeline 의 `--output-certificate` flag 누락 → cosign keyless verify 실패. v0.1.3 fix.
- **v0.1.1**: codegen 이 install.sh 만 sync, in-binary `PLUGIN_VERSION` constants 미동기화 → self-report 거짓. v0.1.3 fix.
- **v0.1.2**: codegen 정상화 baseline.
- **v0.1.3**: SessionStart 자동 token-init trigger 추가했으나 `axhub auth login --print-token` 가정으로 token-init 자체가 broken (CLI 미지원 flag). v0.1.4 fix — helper 가 OS keychain 직접 read.
- **v0.1.4**: macOS + Linux keychain bridge 만 — Windows 는 deferred error 반환. v0.1.5 fix — Windows Credential Manager 통합 추가.
- **v0.1.5**: Windows 통합 첫 ship 했으나 ERR_NOT_FOUND 메시지가 cmdkey /list:axhub 로 자격증명 확인하라고 안내 (cmdkey 는 present/absent 둘 다 exit 0 → useless probe). v0.1.6 fix — single-line patch 으로 AXHUB_TOKEN env var path 안내로 교체.
- **v0.1.6**: Windows keychain 통합 작동했지만 install.sh + session-start.sh 가 bash-only — Git Bash/WSL 없는 stock Windows 사용자는 plugin 자동 설치/SessionStart 모두 broken. v0.1.7 fix — bin/install.ps1 + hooks/session-start.ps1 + hooks.json sibling powershell entry 추가.

신규 install + pilot 은 **v0.1.7** 사용.
