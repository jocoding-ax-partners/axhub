# Phase 1 Data Model: setup 스킬 Windows PowerShell 지원

**Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

## 개요

이 feature 는 단일 문서(`skills/setup/SKILL.md`) 리팩토링이라 **persistent data entity 가 없어요**. 데이터베이스·스키마·영속 상태 변경이 없어요.

단, setup 이 런타임에 감지하는 **온보딩 상태(ephemeral, 세션 한정)**를 개념 모델로 정리해요 — 명령 블록 cross-platform 화의 대상이라서요.

## 감지 상태 (ephemeral — 영속 저장 안 함)

| 상태 항목 | 감지 방법 (Unix) | 감지 방법 (Windows PowerShell) | 값 |
|---|---|---|---|
| CLI 설치 여부 | `axhub --version` | `axhub --version` (동일) | version string \| `cli-missing` |
| node 설치 여부 | `node --version` | `node --version` (동일) | version string \| `node-missing` |
| 패키지 매니저 선호 | `ls *.lock*` (bun/pnpm/npm/yarn) | `Get-ChildItem` lockfile | pm 이름 \| none |
| 권장 node 버전 | `cat .nvmrc` / engines | `Get-Content .nvmrc` / engines | semver \| none |
| helper 경로 | plugin-root → PATH → cache-scan (awk/sort) | `.exe` plugin-root → `Get-Command` → cache-scan | 절대경로 \| missing |
| 인증 상태 | `axhub-helpers preflight --json` → `auth_ok`/`user_email` | 동일 (`.exe`) | bool / email |
| 앱 manifest | `ls apphub.yaml axhub.yaml` | `Test-Path apphub.yaml`,`axhub.yaml` | canonical \| legacy \| none |

**상태 전이**: 없음 (각 감지는 독립 read-only 스냅샷). setup 은 감지 → 첫 gap 위임 → 복귀 후 재감지 루프이며, 영속 전이가 아니라 매번 fresh read 예요.

**Validation rules**: 감지는 read-only 라 입력 검증 대상이 아니에요. node 설치(Step 4)만 consent-gate(사용자 명시 동의) 후 실행해요.

## 결론

데이터 모델 변경 없음. 이 문서는 명령 블록 cross-platform 매트릭스([contracts/setup-command-matrix.md](./contracts/setup-command-matrix.md))의 입력 역할만 해요.
