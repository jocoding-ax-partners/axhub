# Feature Specification: setup 스킬 Windows PowerShell 지원 + CLI v0.17.2 정합

**Feature Branch**: `004-setup-windows-powershell`

**Created**: 2026-06-02

**Status**: Draft

**Input**: User description: "`skills/setup` 스킬을 바뀐 CLI(`ax-hub-cli`)와 비교해 맞지 않는 부분을 파악하고 리팩토링 계획을 세워줘. 추가로 Windows PowerShell 도 지원해야 해."

## 배경 및 재검토 결과 *(컨텍스트)*

`skills/setup/SKILL.md` 를 `ax-hub-cli` **v0.17.2** 와 1:1 로 독립 재검토했어요 (기존 `specs/002-skills-cli-alignment` 결론에 의존하지 않고 실증). setup 은 **위임 모델**이라 CLI 를 직접 호출하는 곳이 `axhub --version` 한 곳뿐이고, 나머지는 helper preflight 와 sibling skill(`install-cli`/`auth`/`init`) 위임이에요.

재검토로 확정한 사실:

- **CLI 명령 정합**: setup 이 의존하는 `axhub --version`·위임 대상 `axhub init`(→ `apphub.yaml` scaffold)·`axhub auth login` 모두 v0.17.2 에 실재해요. 위임 스킬 3개도 전부 존재해요. → **CLI 명령 수준 mismatch 없음**.
- **확정 mismatch ①(major)**: setup 의 모든 셸 명령이 **bash 단독**이에요. Windows PowerShell 사용자는 상태 감지(Step 1)·helper 탐색·node 설치(Step 4)에서 명령이 동작하지 않아요.
- **확정 mismatch ②(minor)**: manifest 감지(Step 6)가 `axhub.yaml`/`apphub.yaml` 을 나열하는데, v0.17.2 에서 `apphub.yaml` 이 canonical 이고 `axhub.yaml` 은 legacy("`mv axhub.yaml apphub.yaml`" 경고 대상)예요. 둘 다 체크해서 기능은 동작하지만 legacy-first 순서·강조가 stale 이에요.

이 spec 의 실질 작업은 **Windows PowerShell 지원**이고, manifest 정합은 부수 정정이에요.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Windows PowerShell 첫 사용자 온보딩 (Priority: P1)

Windows 에서 Claude Code 로 axhub 를 처음 쓰는 사용자가 "셋업해줘" 라고 말하면, setup 이 CLI 설치·로그인·node 환경을 순서대로 점검하고 안내해요. 현재는 점검 단계의 셸 명령이 bash 전용이라, PowerShell 세션에서는 명령이 실패하거나 빈 결과를 내서 온보딩이 첫 단계에서 막혀요. 이 스토리는 Windows PowerShell 환경에서도 macOS/Linux 와 동일하게 온보딩 전 과정을 완주하게 만들어요.

**Why this priority**: setup 의 핵심 가치(첫 사용자가 막히지 않는 온보딩)가 Windows 사용자에겐 현재 전혀 전달되지 않아요. 플랫폼 전체가 깨진 상태라 가장 시급해요.

**Independent Test**: Windows PowerShell 세션에서 setup 워크플로의 각 단계(상태 감지 → 체크리스트 카드 → gap 위임 → 준비 카드)를 따라갔을 때, bash 없이도 명령이 정상 실행되고 카드가 올바른 상태를 표시하는지로 단독 검증돼요.

**Acceptance Scenarios**:

1. **Given** Windows PowerShell 세션·axhub CLI 미설치 상태, **When** 사용자가 setup 을 시작, **Then** CLI 부재가 정확히 감지되고 `install-cli` 로 위임돼요 (bash 전용 명령 실패 없이).
2. **Given** Windows·CLI 설치됨·미로그인, **When** setup 상태 감지, **Then** helper 를 PowerShell 경로 규칙으로 찾아 로그인 필요를 표시하고 `auth` 로 안내해요.
3. **Given** Windows·node 미설치, **When** node 설치 consent 후, **Then** Windows 에서 동작하는 설치 경로로 진행하고 (unix 전용 `curl|bash` 대신), `node --version` 으로 재확인해요.

---

### User Story 2 - manifest 파일 정합 (Priority: P2)

마지막 "첫 앱 만들기" 단계에서 setup 은 현재 디렉토리에 앱 manifest 가 있는지 확인해요. v0.17.2 의 canonical manifest 는 `apphub.yaml` 이에요. setup 이 canonical 을 우선 인식하고 legacy `axhub.yaml` 은 보조로만 다루게 정정해요.

**Why this priority**: 기능상 둘 다 체크하므로 당장 깨지진 않지만, legacy-first 순서는 신규 사용자에게 잘못된 멘탈모델을 심고 CLI 의 stale-manifest 경고와 어긋나요. Windows 작업과 함께 정리하면 효율적이에요.

**Independent Test**: `apphub.yaml` 만 있는 디렉토리에서 setup 이 "앱 있음" 으로 인식하고 첫 앱 생성을 권하지 않는지, `axhub.yaml`(legacy) 만 있을 때도 인식하되 canonical 안내를 곁들이는지로 검증돼요.

**Acceptance Scenarios**:

1. **Given** `apphub.yaml` 이 있는 디렉토리, **When** setup 최종 단계, **Then** 앱이 있다고 인식하고 배포 안내로 마무리해요.
2. **Given** 빈 디렉토리, **When** setup 최종 단계, **Then** 첫 앱 만들기를 제안하고 `init`(→ `apphub.yaml` 생성)으로 위임해요.

---

### User Story 3 - CLI v0.17.2 정합 유지 (회귀 방지) (Priority: P3)

리팩토링 과정에서 setup 이 참조하는 CLI 명령·위임 대상·helper 계약이 v0.17.2 와 계속 일치하도록 보증해요. 재검토 결과 현재는 정합이므로, 이 스토리는 신규 정합이 아니라 변경 중 회귀를 막는 안전망이에요.

**Why this priority**: 이미 정합이라 추가 작업이 적어요. 다만 Windows/manifest 변경이 기존 bash 동작·위임 모델·D1 guard 를 깨지 않게 보증하는 가치는 있어요.

**Independent Test**: 변경 후 setup 이 여전히 `axhub --version` 으로 CLI 를 감지하고, `install-cli`/`auth`/`init` 로 위임하며, helper preflight 로 인증을 확인하는지로 검증돼요.

**Acceptance Scenarios**:

1. **Given** macOS/Linux 기존 사용자, **When** 변경 후 setup 실행, **Then** 기존 bash 경로 동작이 그대로 유지돼요 (회귀 0).

---

### Edge Cases

- **helper 탐색 폴백 (Windows)**: `CLAUDE_PLUGIN_ROOT` 가 비어있을 때, setup 이 PowerShell 에서도 알려진 cache 경로(`$env:USERPROFILE\.claude\plugins\cache\axhub\axhub\*\bin\axhub-helpers.exe`)를 스캔해 최신 버전을 찾을 수 있어야 해요.
- **node 설치 방법 부재 (Windows)**: 패키지 매니저(winget/scoop)도 없는 Windows 환경에서, unix 전용 nvm `curl|bash` 는 동작하지 않아요. Windows 용 대체 경로 또는 명시적 안내가 필요해요.
- **PATH 갱신 갭 (Windows)**: 방금 설치한 CLI/node 가 현재 PowerShell 세션 PATH 에 아직 없을 때, 실패로 끝내지 않고 "새 터미널" 안내로 이어가야 해요.
- **non-interactive(D1) 일관성**: PowerShell 기반 subprocess(`claude -p`/CI)에서도 D1 guard 가 동일하게 적용돼 자동 설치/위임 없이 안전 기본값으로 진행해야 해요.
- **혼합 셸 (Git Bash on Windows)**: Windows 의 Git Bash 사용자는 bash 경로가 여전히 유효해야 해요 — PowerShell 추가가 Git Bash 동작을 빼앗지 않아야 해요.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: setup 의 상태 감지 단계(CLI·node 존재, lockfile/`.nvmrc`/engines advisory)는 Windows PowerShell 에서 실행 가능한 동등 명령을 제공해야 해요(MUST).
- **FR-002**: setup 의 helper 탐색(plugin-root → PATH → cache 스캔)은 Windows PowerShell 경로 규칙·`.exe` 확장자·`$env:` 변수로 동작하는 동등 절차를 제공해야 해요(MUST).
- **FR-003**: node 런타임 설치(Step 4)는 Windows 에서 동작하는 설치 경로를 제공해야 하며(MUST), unix 전용 `curl|bash` nvm 폴백을 Windows 에 그대로 노출하지 않아야 해요. consent-gate 와 공식/핀고정 채널 제약은 유지해요.
- **FR-004**: manifest 감지는 `apphub.yaml` 을 canonical 로 우선 인식하고 `axhub.yaml` 은 legacy 보조로 다뤄야 해요(MUST).
- **FR-005**: FR-001~004 가 추가·수정하는 모든 명령 블록은 `Unix / Git Bash:` / `Windows PowerShell:` 라벨 규칙을 **일관되게** 따라야 해요(MUST, cross-cutting). 개별 블록의 PowerShell 등가 추가는 FR-001~004 가 책임지고, FR-005 는 그 라벨 규칙의 **전역 일관성(누락·형식 불일치 0)** 만 보증해요 — `install-cli`/`doctor` 컨벤션 일치.
- **FR-006**: 변경은 기존 bash 경로 동작·위임 모델(`Skill()` 호출)·D1 guard·`allows-dependency-execution: false` 계약을 보존해야 해요(MUST) — 회귀 0.
- **FR-007**: setup 이 참조하는 CLI 표면(`axhub --version`, 위임 대상 `init`/`auth`)이 v0.17.2 와 정합함을 유지해야 하며(MUST), 변경이 이 정합을 깨지 않아야 해요.
- **FR-008**: helper `preflight --json` 이 setup 이 읽는 필드(`auth_ok`, `user_email`)를 현행 helper 에서 동일하게 제공하는지 확인하고, 불일치가 있으면 문서화해야 해요(SHOULD — helper 는 axhub repo 소속이라 CLI 비교 범위 경계, 검증 항목으로 표기).

### 명시적 비범위 (Out of Scope)

- **위임 스킬 본문 수정**: `install-cli`/`auth`/`init` 의 내부 로직·CLI 정합은 이 spec 범위 밖이에요 (각 스킬 책임, 필요 시 별도 spec).
- **deploy `--branch` consent 3-way 모순** (`specs/002` 부록): setup 무관·cross-repo 보안 결정이라 제외.
- **다른 스킬의 Windows 지원**: setup 단독. 다른 스킬은 별도로 다뤄요.

### Key Entities

해당 없음 — 이 feature 는 단일 문서(`skills/setup/SKILL.md`) 리팩토링이라 데이터 엔티티가 없어요.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Windows PowerShell 세션에서 setup 온보딩의 모든 단계(상태 감지 → 카드 → 위임 → 준비/handoff)가 bash 없이 완주돼요.
- **SC-002**: setup 의 OS 의존 명령 블록 100% 가 `Unix / Git Bash` + `Windows PowerShell` 두 형태를 모두 보유해요 (커버리지 갭 0).
- **SC-003**: `apphub.yaml` 만 있는 프로젝트를 setup 이 "앱 있음" 으로 정확히 인식하고, 빈 디렉토리에서만 첫 앱 생성을 제안해요.
- **SC-004**: 변경 후 검증 게이트(`skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`, `bun test`, `tsc --noEmit`)가 모두 통과하고, 기존 macOS/Linux 동작 회귀가 0 이에요.
- **SC-005**: setup 이 참조하는 CLI 명령·위임 대상이 v0.17.2 에서 전부 유효함이 재확인돼요 (이미 정합 — 회귀만 방지).

## Assumptions

- setup 은 **위임 모델을 유지**해요 — `install-cli`/`auth`/`init` 의 설치·로그인 로직을 재구현하지 않고 `Skill()` 위임만 해요.
- Windows PowerShell 명령은 기존 `install-cli`(OS 감지 `$env:OS`)·`doctor`(helper `.exe` 탐색)·`recovery-flows`(`$env:USERPROFILE` 경로) 스킬의 검증된 cross-platform 패턴을 차용해요 — 새 패턴을 발명하지 않아요.
- Windows node 설치 폴백의 구체 도구 선택(winget/scoop 우선, nvm-windows/fnm 등)은 plan 단계 결정사항이에요.
- helper `preflight --json` 필드 계약은 axhub repo 의 helper 와 정합한다고 가정하되, FR-008 로 실증 검증해요.
- `description:` frontmatter 의 nl-lexicon trigger 어구는 `lint:keywords` byte-lock 이라 **변경하지 않아요** — Windows 지원은 본문 명령 블록에만 추가해요.
- spec/plan 분리를 지켜요 — 구체 PowerShell 명령 구문·블록 배치는 `/speckit-plan` 단계에서 정해요.
