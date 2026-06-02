# Contract: setup 스킬 OS 명령 블록 매트릭스

**Date**: 2026-06-02 | **Spec**: [../spec.md](../spec.md)

Claude Code 스킬의 "인터페이스 계약" = SKILL 본문이 제시하는 셸 명령 블록이에요. 이 매트릭스가 `skills/setup/SKILL.md` 의 각 Step 별로 **어떤 블록이 `Unix / Git Bash` + `Windows PowerShell` 쌍을 가져야 하는지** 정의해요 (FR-005 커버리지 = SC-002 측정 대상).

## 명령 블록 계약 (현행 → 목표)

| Step | 현행 (bash 단독) | 목표 PowerShell 등가 | FR |
|---|---|---|---|
| **Step 1 — CLI/node 감지** | `axhub --version`, `node --version`, `ls *.lock*`, `cat .nvmrc`, `node -p engines` | `axhub --version`(동일), `node --version`(동일), `Get-ChildItem` lockfile, `Get-Content .nvmrc`, engines 읽기 | FR-001 |
| **Step 1 — helper preflight** | helper-pick `awk/sort` cache-scan + `"$HELPER" preflight --json` | `& "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" preflight --json` + cache-scan 폴백 (`$env:USERPROFILE\...\*.exe`) — doctor 단순도 (awk/sort 등가물 금지) | FR-002 |
| **Step 4 — node 설치** | brew / apt·dnf·pacman / nvm `curl\|bash` | winget → scoop → nodejs.org 수동 (nvm 자동 2순위 미적용, research Decision 2) | FR-003 |
| **Step 6 — manifest 감지** | 산문만 (명시 셸 명령 없음) — `axhub.yaml`/`apphub.yaml` 부재를 자연어로 언급 | (신규 명령) bash `test -f apphub.yaml`·`ls`, PS `Test-Path apphub.yaml` — canonical 우선 + legacy 보조 | FR-004 |

## 라벨 규칙 (FR-005)

각 cross-platform 블록은 다음 형태로 분리 — doctor/upgrade(PR #160) 일치:

```
   Unix / Git Bash:

   ```bash
   <기존 명령 유지>
   ```

   Windows PowerShell:

   ```powershell
   <등가 명령>
   ```
```

## 보존 계약 (FR-006 — 회귀 0)

| 보존 대상 | 검증 |
|---|---|
| 기존 bash 블록 동작 | macOS/Linux 에서 그대로 실행 가능 (Git Bash on Windows 포함) |
| 위임 모델 (`Skill()` 호출) | install-cli/auth/init 위임 로직 재구현 안 함 |
| D1 guard | `if ! [ -t 1 ] ...` 서술 그대로 (install-cli 컨벤션, PS 미러 불필요) |
| `allows-dependency-execution: false` | frontmatter 불변, npm/bun install 미추가 |
| `description:` frontmatter | nl-lexicon byte-lock, **불변** (lint:keywords) |

## 검증 계약 (SC-004)

변경 후 통과해야 할 게이트:
- `bun run skill:doctor --strict` → exit 0
- `bun run lint:tone --strict` → 0 err (SKILL 본문 해요체)
- `bun run lint:keywords --check` → no diff (description 불변 확인)
- `bun test` → 회귀 0 (기존 pass 유지)
- `bunx tsc --noEmit` → clean (TS 미변경)

## 비계약 (Out of Scope)

- helper preflight 필드 자체는 변경 안 함 (FR-008 정합 확인됨, research Decision 1).
- 위임 대상 스킬 본문(install-cli/auth/init)은 이 contract 밖.
