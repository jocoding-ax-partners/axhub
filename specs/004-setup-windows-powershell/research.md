# Phase 0 Research: setup 스킬 Windows PowerShell 지원 + CLI v0.17.2 정합

**Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

spec 의 Deferred 항목과 Technical Context 미확정을 해소해요. 모든 결정은 `ax-hub-cli` v0.17.2 + `crates/axhub-helpers` 소스 실증 기반이에요.

---

## Decision 1 — helper `preflight --json` 필드 정합 (FR-008)

**Decision**: setup Step 1 이 읽는 `auth_ok` / `user_email` 필드는 현행 helper 에 실재하므로 **변경 불필요**. 정합 확인됨.

**Rationale**: `crates/axhub-helpers/src` 실증 —
- `auth_ok: bool` — `audit.rs:35`, `statusline.rs:26`, `deploy_prep.rs:330` 에 존재
- `user_email: Option<String>` — `session_bundle.rs:25`, `deploy_prep.rs:335` 에 존재

setup 이 `"$HELPER" preflight --json` 으로 두 필드를 읽는 계약이 helper 출력과 일치해요. helper 는 axhub repo 소속(ax-hub-cli 아님)이라 CLI 버전 bump 와 무관하게 plugin 과 lockstep 이에요.

**Alternatives considered**: 필드명 변경 / 폴백 추가 — 불필요 (정합). FR-008 은 "정합 확인" 으로 종료.

---

## Decision 2 — Windows node 설치 폴백 정책

**Decision**: Windows node 설치는 **winget → scoop → nodejs.org 수동 안내** 순서. unix 의 `nvm curl|bash` 자동 2순위에 대응하는 Windows 자동 스크립트 폴백은 **두지 않아요**.

**Rationale**:
- winget 은 Windows 10/11 에 기본 탑재(App Installer)라 1순위로 대부분 커버. scoop 은 보조.
- unix 의 nvm 은 `curl|bash` 로 pipe 설치되지만, Windows 대응(nvm-windows)은 `.exe` GUI installer 라 비대화형 pipe 에 부적합해요. fnm 은 cross-platform 이나 결국 winget/scoop 으로 설치하므로 "pm 없음" 상황에선 자기모순이에요.
- 따라서 pm(winget/scoop)도 없는 Windows 환경은 **nodejs.org LTS 수동 안내**가 supply-chain 안전 + 현실적이에요. consent-gate 는 pm 자동 실행에만 적용해요.

이 비대칭(unix=자동 스크립트 2순위 있음 / Windows=없음)은 플랫폼 기술 제약이지 누락이 아니에요.

**Alternatives considered**:
- nvm-windows 자동 설치 — `.exe` installer 라 비대화형 부적합, 거부.
- fnm 강제 — pm 의존이라 "pm 없음" 케이스 미해결, 거부.
- winget 부재 시 scoop 부트스트랩 자동 — scoop 설치 자체가 `irm|iex` supply-chain deviation, 거부 (수동 안내가 안전).

---

## Decision 3 — Windows PowerShell 명령 패턴 (기존 스킬 차용)

**Decision**: 새 패턴을 발명하지 않고 검증된 sibling 스킬 패턴을 차용해요.

**Rationale**: 다음이 이미 프로덕션 검증된 cross-platform 컨벤션이에요 —
- **OS 감지**: `install-cli` 의 `$env:OS`(=`Windows_NT`) 3분기 (Darwin/Linux/Windows_NT)
- **helper `.exe` 탐색**: `doctor` 의 `& "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe"` + cache-scan 폴백 `$env:USERPROFILE\.claude\plugins\cache\axhub\axhub\*\bin\axhub-helpers.exe`
- **경로 규칙**: `recovery-flows` 의 `$HOME`↔`$env:USERPROFILE`, `/`↔`\`
- **블록 라벨**: `Unix / Git Bash:` / `Windows PowerShell:` (doctor/upgrade PR #160 일치)
- **manifest 읽기**: PowerShell `(Get-Content ... -Raw | ConvertFrom-Json)` (upgrade PR #160 에서 jq 대체로 검증)

**Alternatives considered**: 새 OS 감지/경로 헬퍼 작성 — 컨벤션 분산, 거부. helper-pick 의 awk/sort cache-scan 을 PowerShell 로 포팅 — doctor 가 PS 블록에선 단순 `.exe` 직접 호출만 하므로 동일 단순도 유지(awk/sort 등가물 금지).

---

## Decision 4 — manifest 감지 정합 (apphub.yaml canonical)

**Decision**: Step 6 의 앱 존재 감지는 `apphub.yaml`(canonical) 우선, `axhub.yaml`(legacy) 보조.

**Rationale**: `ax-hub-cli/crates/axhub-manifest/src/lib.rs` 실증 —
- `manifest_filename()` → `"apphub.yaml"` (canonical)
- `LEGACY_FILENAME` → `"axhub.yaml"` + stale 경고 `"mv axhub.yaml apphub.yaml"`
- `axhub init` help: "Scaffold a new **apphub.yaml**"

둘 다 감지하되 순서·강조를 canonical-first 로 정정해요. legacy 발견 시 mv 안내를 곁들일 수 있어요(plan 후 tasks 에서 문구 확정).

**Alternatives considered**: legacy 완전 제거 — 기존 `axhub.yaml` 사용자 호환 깨짐, 거부.

---

## Resolved Unknowns 요약

| spec 항목 | 상태 |
|---|---|
| FR-008 helper preflight 필드 | **Resolved** — auth_ok/user_email 실재, 정합 |
| Windows node 2순위 도구 (Deferred) | **Resolved** — winget→scoop→수동, 자동 스크립트 2순위 미적용 |
| Windows PowerShell 패턴 | **Resolved** — install-cli/doctor/recovery/upgrade 차용 |
| manifest legacy-first (minor) | **Resolved** — apphub.yaml canonical 우선 |

NEEDS CLARIFICATION 잔여: 0.
