# Quickstart: setup 스킬 Windows PowerShell 검증

**Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

리팩토링 후 setup 스킬이 Windows PowerShell 에서 동작하는지 검증하는 방법이에요. (macOS 개발 환경엔 `pwsh` 가 없을 수 있어 구문 정합 = 검증 — PR #160 와 동일 한계.)

## 1. 정적 검증 (개발 머신, macOS/Linux)

```bash
# 패턴/톤/baseline 게이트 (SC-004)
bun run skill:doctor --strict        # exit 0
bun run lint:tone --strict           # 0 err
bun run lint:keywords --check        # no diff (description 불변)
bun test                             # 회귀 0
bunx tsc --noEmit                    # clean
```

블록 커버리지(SC-002) 수동 확인:

```bash
# OS 분기 블록마다 Unix + Windows 쌍 존재 확인
grep -c "Unix / Git Bash:" skills/setup/SKILL.md
grep -c "Windows PowerShell:" skills/setup/SKILL.md   # 두 수가 일치해야 함
# 단 Step 4 의 nvm curl|bash 는 Windows 자동경로 부재(research Decision 2)라 라벨-쌍 비대상 = 문서화된 의도적 예외 1건
```

## 2. PowerShell 구문 검증 (선택 — pwsh 있을 때)

```bash
# macOS 에 pwsh 설치돼 있으면 각 PS 블록 구문 파싱 확인
pwsh -NoProfile -Command '$null = { <블록 내용> }'
```

`pwsh` 부재 시 → 기존 스킬(install-cli/doctor/recovery-flows) 컨벤션 일치로 대체 검증 (research Decision 3).

## 3. 행위 검증 (Windows 실기 — QA)

Windows PowerShell 세션에서 시나리오별 확인:

| 시나리오 | 기대 (spec AC) |
|---|---|
| CLI 미설치 | `axhub --version` 실패 감지 → `install-cli` 위임 (US1-AC1) |
| CLI 설치·미로그인 | helper `.exe` PS 경로로 발견 → `auth` 안내 (US1-AC2) |
| node 미설치 | consent 후 winget/scoop, 둘 다 없으면 nodejs.org 수동 안내 (US1-AC3) |
| `apphub.yaml` 존재 | "앱 있음" 인식, 배포 안내 (US2-AC1) |
| 빈 디렉토리 | 첫 앱 제안 → `init` 위임 (US2-AC2) |
| Git Bash on Windows | 기존 bash 경로 동작 유지 (US3 / 회귀) |

## 4. 회귀 검증 (macOS/Linux)

기존 bash 경로가 그대로 동작하는지 — setup 을 macOS 에서 실행해 감지·위임·카드가 변경 전과 동일한지 확인 (FR-006, SC-004 회귀 0).

## Done 기준

- [ ] 정적 게이트 5종 통과
- [ ] `Unix / Git Bash:` ↔ `Windows PowerShell:` 블록 수 일치 (커버리지 갭 0)
- [ ] Windows 실기 시나리오 6종 (가능 시)
- [ ] macOS/Linux 회귀 0
