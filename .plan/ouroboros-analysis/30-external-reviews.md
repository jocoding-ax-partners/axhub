# 30. External Reviews — Claude vs Codex Hermes Integration Audit

> US-004 deep-dive. `Code-Review-Claude.md` (17 KB) + `Code-Review-Codex.md` (13 KB) 두 외부 리뷰 정독 + 차이점 비교. 둘 다 Hermes runtime 통합 (Phase 21~) PR 에 대한 audit.

## 30.1 공통 컨텍스트

두 리뷰 모두 **같은 changeset** 검토:
- 새 `HermesCliRuntime` 클래스 + Hermes artifact installer
- 새 `ouroboros.skills` 패키지 (runtime-agnostic 시작)
- Codex/OpenCode setup 이 Claude MCP 자동 등록 **제거** (intentional decoupling)
- pyproject.toml `force-include` 변경 — 휠에서 skills 위치 이동

## 30.2 발견된 동일 Critical 결함

### C1 — `src/ouroboros/skills/__init__.py` shadowing

> **두 리뷰 모두 같은 critical 발견.**

```
src/ouroboros/codex/artifacts.py:_packaged_codex_skills_dir()
가 부모 디렉토리 walk 하다가 첫 발견된 skills/ = src/ouroboros/skills/ (방금 추가됨, __init__.py 만 있음)
→ 실 skill bundle 없는 디렉토리에서 멈춤 → 모든 skill resolution 깨짐 (editable install)
```

#### 영향
- `pytest tests/unit/test_codex_artifacts.py tests/integration/test_codex_skill_smoke.py tests/integration/test_codex_cli_passthrough_smoke.py tests/integration/test_codex_skill_fallback.py -q` → **8 failed, 17 passed**
- editable (개발) install 모두 깨짐
- 휠 install 은 OK (force-include 가 동시에 `__init__.py` + skill bundles 포함)

#### 정확한 8 실패 테스트 (Claude review C1)
- `test_codex_artifacts.py::test_resolves_repo_packaged_skill_path_by_default`
- `test_codex_artifacts.py::test_installs_repo_packaged_skills_by_default`
- `test_codex_artifacts.py::test_resolves_repo_skills_and_packaged_rules_by_default`
- `test_codex_skill_smoke.py::test_packaged_ooo_prefixes_dispatch_from_skill_frontmatter[run]`
- `test_codex_skill_smoke.py::test_packaged_ooo_prefixes_dispatch_from_skill_frontmatter[interview]`
- `test_codex_cli_passthrough_smoke.py` (2 failures)
- `test_codex_skill_fallback.py::test_codex_mcp_timeout_falls_back_to_pass_through_cli_flow`

#### Fix 옵션 (Claude C1)
1. `_contains_skill_bundles()` check 추가 — 부모 walk 가 빈 dir 스킵
2. resolver 를 `ouroboros.codex` 에서 `ouroboros.skills.resolver` 로 이동 + `importlib.resources.files("ouroboros.skills")` 1차 경로
3. `src/ouroboros/skills/__init__.py` 삭제 + implicit namespace package

## 30.3 동일 High 결함 4 건

### H1 — Hermes subprocess timeout 부재

```python
# hermes_runtime.py:252
await process.communicate()   # 무한 블록
```

Codex/OpenCode 는 60s startup + 300s idle timeout 둘 다 보호. Hermes 는 둘 다 없음.

### H2 — Hermes recursion depth + env isolation 부재

- Codex/OpenCode: `_OUROBOROS_DEPTH` env 카운터 + 5 cap + 자식에 Ouroboros env 변수 strip
- Hermes: 둘 다 안 함 → MCP → Hermes → MCP → Hermes 무한 재귀 가능

### H3 — `--runtime hermes` CLI 가 거절 (Codex 리뷰 only)

```bash
$ ouroboros run workflow examples/dummy_seed.yaml --runtime hermes --no-orchestrator
# ↑ "invalid value" 에러
```

`run.py:57, 418` 의 enum 이 hermes 누락. Setup/config 에선 valid 하지만 CLI 에선 reject.

### H4 — recoverable-dispatcher tuple 처리 누락 (Codex 리뷰 only)

```
Codex: dispatch 에서 recoverable error tuple → log + return None (CLI 로 fall through)
Hermes: 같은 tuple 받으면 yield + return → exact-prefix skill hard-fail
```

→ MCP/dispatch 일시 오류가 hard failure 로 변환됨 (graceful degradation 누락).

### H5 — Codex/OpenCode 가 Claude MCP 자동 등록 제거 (Claude 리뷰 only — 'H4' 라벨)

```python
# Before:
if (Path.home() / ".claude").is_dir():
    _ensure_claude_mcp_entry()
# After:
# 제거됨 — multi-runtime 사용자 silent 회귀
```

→ User-facing behavioral change. 릴리스 노트 + 마이그레이션 hint 권장.

## 30.4 두 리뷰 사이 차이

| 측면 | Code-Review-Claude.md | Code-Review-Codex.md |
|---|---|---|
| **구조** | 명시적 severity 카테고리 (CRITICAL/HIGH/MEDIUM/LOW) + 헤딩 | 단일 "Findings" 리스트, severity 인라인 prefix |
| **출처 인용** | 각 finding 마다 `file:line` 링크 (URL 형식) | `file:line` 인라인 + 직접 repro 명령 풍부 |
| **테스트 결과 구체성** | 8 실패 테스트 이름 명시 + 75% 커버리지 | 8 failed 카운트만 + 4529 collected |
| **Verification section** | 별도 "Verification Steps to Execute" — 12 step 절차 | "Verification" 단락 — 어떤 게 PASS/FAIL 인지 압축 |
| **Architectural recommendations** | 5 R 형식 (R1–R5) + 비-블로커 명시 | "Recommendations" 단락에 5 항목 (불릿) |
| **Open Questions / Residual Risks** | 없음 | 명시적 두 섹션 — 미커버 영역 인정 |
| **Smoke test 자세함** | 12 step bash 블록으로 친절히 제공 | "temp-home Hermes setup smoke" 한 줄 요약 |
| **Severity rubric 적용** | CRITICAL/HIGH/MEDIUM/LOW 명확 분류 | 같은 라벨 사용하나 더 inline |
| **태도** | "Block merge" / "Must fix" / "Should fix" 결정형 | 더 분석적, "I would expect", "minimal repro reasoning" |
| **누락된 finding** | H3 (`--runtime hermes` CLI reject), H4 (recoverable dispatcher) 못 봄 | 이 둘 잡음 |
| **추가된 finding** | M1 (install_hermes_skills 가 `__init__.py` 까지 복사), M5 (session ID 정규식 lowercase only), M6 (skill_path stale ref) 더 구체적 | M1 (Hermes copies entire source tree) — Claude 도 발견 |
| **포괄성** | 더 친절한 설명 (왜 + Fix options 다중) | 더 forensic — 직접 repro 결과 강조 |
| **분량** | 17.4 KB | 13.6 KB |
| **권장 사항 5개** | shared skill resolver / subprocess base / declarative registry / unified installer / smoke harness | 같은 5개 — 표현만 다름 |

## 30.5 핵심 차이점 분석

### Claude 리뷰의 강점
- **Block merge / Must fix / Should fix** 분류로 우선순위 명확
- **Verified Test Results** 마지막 블록 — 종합 dashboard
- **Smoke test bash 풀 절차** — reviewer 가 direct execute 가능
- **Severity table** 마지막 — 의사결정에 직결

### Codex 리뷰의 강점
- **Open Questions / Residual Risks** 섹션 — 자신이 못 본 영역 인정 (epistemic humility)
- **Direct repro** 인라인 (`env HOME=... uv run pytest...`) — 검증 즉시 가능
- **인용된 원문 결과** ("returns 8 failed, 17 passed") — false positive 가능성 적음
- **`--runtime hermes` CLI breakage 발견** — Claude 리뷰가 놓침
- **recoverable dispatcher fallthrough 발견** — Claude 리뷰가 놓침
- **frontmatter unsafe parsing** Medium 발견 — Claude 도 발견했으나 다르게 표현

### Claude 리뷰의 누락
- `--runtime hermes` 가 CLI 에서 reject 되는 사실
- recoverable dispatcher tuple 이 Hermes 에서 hard-fail 로 변환됨
- 정확한 4 ruff format 실패 파일 목록

### Codex 리뷰의 누락
- M5 (session ID 정규식 lowercase) — 미래 문제 가능
- M6 (skill_path stale ref after context manager) — latent bug
- 12 step verification harness — Claude 가 더 친절

## 30.6 합집합 — 모든 발견 사항

### Critical (1)
- C1: `src/ouroboros/skills/__init__.py` shadowing → editable install 8 테스트 실패

### High (4)
- H1: subprocess timeout 부재
- H2: recursion depth + env isolation 부재
- H3: `--runtime hermes` CLI reject
- H4: recoverable dispatcher fallthrough 누락 (Hermes ↔ Codex 격차)
- H5: Claude MCP 자동 등록 제거 (behavioral change)

### Medium (6)
- M1: `install_hermes_skills` 가 `__init__.py` 까지 복사 (전체 source tree)
- M2: Hermes 가 Codex-named resolver 의존 (coupling)
- M3: `_parse_quiet_output` 가 `session_id` mid-output 시 truncation
- M4: `prune` 파라미터 무시
- M5: session ID 정규식 lowercase hex only
- M6: `skill_path` stale ref after context manager exit
- M7: docs 가 Hermes "all execution phases" 라 과장 (실제 runtime-only)
- M8: Hermes frontmatter parsing unsafe (non-mapping crash, unterminated → `{}`)

### Low (5+)
- L1: `_setup_hermes` config I/O 가 `encoding="utf-8"` 누락
- L2: `stdout_data.decode()` system default encoding
- L3: Hermes docs 과장
- L4: `.codex` empty untracked file
- L5: `__pycache__/` 디렉토리 untracked
- L6: ruff format check 실패 (4 파일)
- L7: `prune=...` 파라미터 무시 (M4 와 중복 — Codex 가 Low 로 분류)

## 30.7 두 리뷰의 동일 5 architectural recommendations

| # | Claude 표현 | Codex 표현 | 핵심 |
|---|---|---|---|
| R1 | Backend-agnostic shared skill resolver | Move toward a backend-agnostic shared skill resolver | `ouroboros.skills.resolver` 로 추상화 |
| R2 | Common subprocess runtime base class | Add a common subprocess runtime base class | `SubprocessAgentRuntime` base — ~300 LOC 중복 제거 |
| R3 | Declarative runtime registry | Introduce a declarative runtime registry | `if/elif` 분기 대신 self-register |
| R4 | Unified installer/config-writer | Unify installer and config-writer behavior | safe-write + 일관된 encoding |
| R5 | Packaging/setup smoke-test harness | Add a dedicated packaging/setup smoke harness | 휠 빌드 + temp HOME setup 자동 검증 |

→ 두 리뷰가 독립적으로 같은 5 추천 도출 = 강한 신호.

## 30.8 검증 결과 비교

| Quality Gate | Claude 리뷰 | Codex 리뷰 |
|---|---|---|
| `ruff check` | PASS | PASS |
| `ruff format --check` | FAIL — 4 files | FAIL — 4 files (`hermes_runtime.py`, `test_setup.py`, `test_artifacts.py`, `test_hermes_runtime.py`) |
| `mypy` | PASS — 244 source files | PASS |
| `pytest` 전체 | 8 failed, 4519 passed, 2 skipped, 75% coverage | 4529 collected — pytest 종료 trustworthy 안 됨 |
| 휠 빌드 | PASS | PASS |
| 휠 importlib | WORKS via fallback | WORKS via fallback |
| editable importlib | FAILS (C1) | 8 failed reproduced |
| Hermes setup smoke (temp HOME) | PASS — 모두 정상 | PASS |
| Codex Claude decoupling smoke | PASS | (별도 검증 안 함) |

## 30.9 시사점

### 두 리뷰가 일치하는 강한 신호
- C1 / H1 / H2 / R1–R5 — 무조건 fix 필요
- Codex/OpenCode → Claude decoupling 은 docs 추가 필수 (Claude 가 더 강조)

### 한 리뷰만 잡은 약한 신호
- `--runtime hermes` CLI reject (Codex only) — 명백한 user-visible 결함
- frontmatter non-mapping crash (Codex 가 더 자세) — corrupted skill bundle 위험
- skill_path stale reference (Claude only) — latent bug, 현재 영향 없음

### 외부 리뷰 메타 학습
1. **다중 reviewer 가 critical 합의** = 진짜 critical
2. **diverging finding** = 한 reviewer 의 blind spot 노출 → 두 리뷰 합쳐야 완전
3. **Codex 가 더 evidence-driven** (직접 repro), **Claude 가 더 actionable** (PR-blocking decision)
4. **두 리뷰 모두 architectural recommendations 일치** = 진짜 design smell

## 30.10 Hermes 통합 PR 의 머지 결정 (양 리뷰 합의)

> Block merge 까지 fix 필요:
> 1. C1 — skills shadowing 해결 (3 옵션 중 하나)
> 2. H1 — subprocess timeout 추가
> 3. H2 — depth guard + env isolation 추가
> 4. ruff format check 4 파일
>
> Document 필요:
> - H5 — Codex/OpenCode 가 Claude MCP 자동 등록 안 함 (마이그레이션 hint)
>
> Should fix (별도 PR):
> - H3 — `--runtime hermes` CLI accept (Codex 발견)
> - H4 — recoverable dispatcher fallthrough parity (Codex 발견)
> - M1–M8 — 누적 maintainability debt
