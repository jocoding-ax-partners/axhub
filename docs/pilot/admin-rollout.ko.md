# IT/Admin 가이드 — axhub Plugin 회사 도입

회사 IT 담당자 또는 보안팀이 vibe coder를 위해 axhub 플러그인을 안전하게 deploy 할 때 따라야 할 정책 + troubleshooting first-line.

---

## 1. 환경 변수 정책

각 vibe coder의 노트북 환경변수에 회사 정책에 맞게 설정:

| 환경변수 | 권장값 | 설명 |
|---|---|---|
| `AXHUB_REQUIRE_COSIGN` | `1` | helper 바이너리가 cosign 서명 안 됐으면 session-start에 경고. 회사 보안 강화. |
| `AXHUB_TELEMETRY` | `0` 또는 unset | 사용량 telemetry. **default OFF**. ON 으로 바꿀 경우 vibe coder에게 명시적 동의 + 데이터 수집 항목 안내 필요. |
| `AXHUB_PROFILE` | `production` 또는 `staging` | default profile. 실수 방지를 위해 노트북마다 staging 으로 두는 것 권장. |
| `AXHUB_ALLOW_UNSIGNED` | **절대 unset** | IT-only 비상 escape hatch. vibe coder 노트북에 절대 두지 마세요 (PLAN row 59). |

회사 표준 dotfile / Ansible playbook 에 위 4개 변수를 박아두면 vibe coder가 따로 신경쓸 필요 없습니다.

---

## 2. HMAC consent 키 분리 정책

각 vibe coder의 컴퓨터에 HMAC consent 키 (32 bytes, mode 0600) 가 자동 생성됩니다:

```
${XDG_STATE_HOME:-$HOME/.local/state}/axhub/hmac-key
```

**중요**:
- **per-user 격리**: 이 파일은 절대 cross-user 또는 cross-machine 으로 복사하지 마세요. 토큰 binding이 깨져서 모든 destructive op이 deny 됩니다.
- **백업 X**: IT 표준 백업 솔루션 (Time Machine, dotfile sync 등) 에서 이 디렉토리는 **제외**해주세요. 키가 다른 사람 노트북에 복원되면 그 사람이 본인의 destructive op을 우회 가능.
- **rotation**: 키 손상 또는 의심 사고 시 단순히 파일 삭제 → 다음 helper 호출 시 자동 재생성.

`~/.local/state/axhub/` 디렉토리를 .gitignore에 포함시키도록 모든 vibe coder의 dotfiles repo 검토.

---

## 3. 사고 대응 (incident response)

### Severity 1: cosign verify failed (exit 66 + `update.cosign_verification_failed`)

vibe coder가 이 에러를 보면:

1. **즉시 본인 IT/보안팀 호출**
2. **절대 `AXHUB_ALLOW_UNSIGNED=1` 으로 강제 진행 금지**
3. axhub 현재 버전으로 계속 사용 가능 (기존 helper 바이너리는 신뢰)
4. 사고 보고서 양식:
   - 시점: ____
   - 다운로드 시도한 새 버전: ____
   - 네트워크 환경 (회사 망/외부 망/VPN): ____
   - cosign 명령 출력 그대로 첨부

회사 측 추가 액션:
- vendor/distribution channel 검증 (jocoding-ax-partners/axhub Github 진위 확인)
- 같은 사고가 다른 vibe coder에게도 보였는지 cross-check
- 우리에게 (jocoding-ax-partners) 즉시 통보 — 가능한 supply chain attack

### Severity 2: token leak suspected (axhub_pat_* 가 transcript/log에 노출)

1. **즉시 vibe coder 본인 토큰 revoke**: `axhub auth logout` (해당 노트북에서)
2. admin 측: `axhub token revoke --token-id <ID>` (또는 axhub admin UI)
3. 새 token 발급 + secure 채널 (Slack DM, secure mail) 로 재전달
4. 노출된 transcript 폐기 (Claude Code session log 삭제, 공유된 파일 회수)

helper의 redact 필터 (`axhub-helpers redact`) 가 1차 방어선이지만 100% 보장 X. 의심 시 반드시 token rotation.

### Severity 3: cross-team API 누출 의심 (apis list 결과가 다른 팀 endpoint 포함)

1. `~/.cache/axhub-plugin/cross-team-list.ndjson` 파일에서 해당 호출 timestamp + utterance hash 확인
2. 의도된 cross-team 조회였는지 (AskUserQuestion에서 "네, 전체 보기" 답변) vs 우회 시도였는지 검증
3. 우회로 판명되면: helper preauth-check 로그 (telemetry 활성화 시 usage.jsonl) 검토 + 우리에게 보고

---

## 4. Troubleshooting first-line (vibe coder가 admin에게 묻기 전 본인이 시도)

| 증상 | 첫 번째 시도 | 두 번째 시도 |
|---|---|---|
| `/axhub:doctor` 결과에 "axhub binary not found" | `which axhub` 확인 | `bash ${CLAUDE_PLUGIN_ROOT}/bin/install.sh` 재실행 |
| 매번 "다시 로그인해주세요" 가 떠요 | `axhub auth status --json` 단독 실행해서 expires_at 확인 | token rotate |
| preview card 가 안 떠요 | `/axhub:doctor` 로 plugin enabled 확인 | `/plugin reload axhub` |
| Korean 메시지가 깨져요 (□□□ 같은) | 터미널 font 한국어 지원 확인 | locale env 설정 (`LANG=ko_KR.UTF-8`) |
| `bun run smoke:full` 실행 시 "Broken: 9" 등 | docs-link-audit 결과 → broken refs는 우리에게 보고 (skill 업데이트 필요) | 우회 X — 필수 |

위 시나리오 외 모든 막힘은 admin 또는 우리(jocoding-ax-partners)에게 즉시 컨택. SLA 4시간.

---

## 5. Cross-machine 작업 정책

vibe coder가 같은 회사 내 여러 노트북 (사무실 + 집) 사용 시:

- **각 노트북별 독립 token 발급**: 절대 token 복사 X (HMAC 키도 각각 생성됨)
- **deploy 권한은 단일 노트북에 한정 권장**: 여러 노트북에서 동시에 deploy 시도 → race condition + 동시-배포 차단 (validation.deployment_in_progress)
- **headless 환경 (Codespaces, SSH, Windows)**: token-paste flow (skills/auth/SKILL.md step 4 / `skills/deploy/references/headless-flow.md`) 사용. 옵션 A — 헤드리스 환경에서 직접 `export AXHUB_TOKEN=axhub_pat_...` (PowerShell: `$env:AXHUB_TOKEN='axhub_pat_...'`). 옵션 B — 브라우저 노트북에서 `axhub auth login` 후 keychain 에서 토큰 추출 (`security find-generic-password -s axhub -w` / `secret-tool lookup service axhub`) → secure 채널 (Slack DM 등) 로 전달. Windows 는 PowerShell + Credential Manager 통합으로 token-init 자동 처리됨.
- **Windows 자동 설치 (v0.1.7+)**: Git Bash / WSL 불필요. `bin/install.ps1` + `hooks/session-start.ps1` 가 PowerShell 5.1+ stock 환경에서 자동 다운로드 + token-init 실행. **Claude Code >= 2.1.84 필수** (`"shell": "powershell"` hook 필드 도입). 더 오래된 client 는 silent fail.

---

## 6. Pilot 종료 후 (회사가 정식 도입 결정 시)

- vibe coder 토큰 expires 갱신 (default 7d → 30d 또는 90d, 회사 보안 정책에 따름)
- HMAC 키 백업 정책 재확인 (default: 백업 안 함, per-user 격리 유지)
- telemetry 정책 재결정 (사용 패턴 데이터 필요시 opt-in 활성화)
- admin training (이 문서를 회사 IT 표준 onboarding 자료로 흡수)

---

PLAN reference: §16.16 (multi-tenant credential isolation), §16.10 (cosign default-on supply chain), §16.17 (apis cross-team privacy filter), Phase 6 row 47/59 (cosign + audit policies).
