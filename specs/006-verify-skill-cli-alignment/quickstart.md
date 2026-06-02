# Quickstart: verify 스킬 정렬 구현 + 검증

> 대상: `skills/verify/SKILL.md` 를 [contract](./contracts/verify-cli-contract.md) 에 맞춰 rewrite. 기존 스킬 편집 (scaffold 불필요). helper/CLI 코드 변경 없음 (audit 에서 CLI 버그 없음 확인).

## 구현 절차

### 1. frontmatter 보존 (수정 금지)
- `description:`(트리거) byte 그대로. `multi-step: true`, `needs-preflight: true`, examples 유지.
- in-body CANONICAL_PREFLIGHT_BLOCK (HELPER pick + `PREFLIGHT_JSON=$("$HELPER" preflight --json`) 보존.
- TodoWrite Step 0 / D1 비대화형 가드 / health_endpoint AskUserQuestion 보존.

### 2. 본문 rewrite (contract 기준)
- **CI 예시 (line ~119-120)**: `{"verdict":"passed"}` → `VerifyResult` 실제 모양 — `{"verdict":"live","state":"active","last_deploy_id":"...","last_deploy_age_secs":120,"errors":[],"reasons":["..."]}`. `axhub-helpers verify --json --app-id <app>` (primary `--app-id`, alias `--app`).
- **verdict 매핑**: helper `verdict` ∈ {live,suspect,not_live} → ✅/⚠️/❌. `reasons` 배열을 verdict 아래 verbatim 출력.
- **Step 1 (식별)**: helper `list-deployments --app-id <app> --limit 1` (primary 인자명 정정).
- **Step 2 (status)**: `axhub deploy status <id> --app <app> --json` 유지. **status 를 닫힌 enum 으로 서술하지 말고** LIVE_STATES(`live/running/deployed/active/ok/succeeded`, `ok` 포함) 미러 + 그 외=미라이브 휴리스틱으로 재서술. `pending/building/...` 는 "예시 휴리스틱"으로 명시.
- **Step 3 (logs)**: `axhub deploy logs --app <app> --json` (**app-level**) 로 변경. `<DEPLOY_ID>` 스코핑 + `--source pod` 제거 (deployment_id legacy, source 고정 enum 없음). client-side 마지막 ~50줄 trim → ERROR/FATAL grep 유지.
- **`--app-id`/`--app` 설명**: "primary `--app-id`, alias `--app`" 로 정정.
- error_code 분기: `../recover/SKILL.md` 표 cross-link 유지 (정정 불요).
- TodoWrite Step 0 의 "axhub deploy logs 확인" 등 라벨 — app-level 반영해 자연스럽게.

### 3. (변경 시에만) registry
health_endpoint AskUserQuestion 그대로면 registry 불요. 새 AskUserQuestion 추가 시 `tests/fixtures/ask-defaults/registry.json` 등록.

## 검증 (DoD)

```bash
# 1. verdict 정합
rg -n "verdict.*passed" skills/verify/SKILL.md          # expect 0
rg -n "live|suspect|not_live" skills/verify/SKILL.md     # verdict 3값 매핑 존재

# 2. logs app-level / source 가정 제거
rg -n "deploy logs.*--source pod|deploy logs <DEPLOY" skills/verify/SKILL.md   # expect 0 (app-level 로 교체)

# 3. --app-id/--app 설명 정확
rg -n "app-id|--app\b" skills/verify/SKILL.md            # primary/alias 정확

# 4. live CLI 대조 (바이너리 있을 때)
~/.axhub/bin/axhub deploy status --help
~/.axhub/bin/axhub deploy logs --help                    # --app/--source/--follow, no --tail
~/.axhub/bin/axhub deploy list --help                    # no --limit
# helper verify surface
"$HELPER" verify --help 2>/dev/null || true              # --app-id primary / --app alias
# (선택) app-level 로그 단발 spot-check
# ~/.axhub/bin/axhub deploy logs --app <app> --json | head

# 5. repo 게이트
bun run skill:doctor --strict
bun run lint:tone --strict
bun run lint:keywords --check     # description byte-lock — diff 0
bun test                          # ux-todowrite / ux-ask-fallback-registry 등
bunx tsc --noEmit
cargo test -p axhub-helpers       # verify_helper 회귀 (무변경이라 green 유지 확인)
```

## SC 매핑
- SC-001 (verdict 일치) → 검증 1 + verify_helper.rs 대조
- SC-002 (명령 수용) → 검증 4 (live --help)
- SC-003 (status/LIVE_STATES) → contract §1-2 ↔ 본문 대조
- SC-004 (verdict passed 0 / app-id 정확) → 검증 1,3
- SC-005 (게이트) → 검증 5

## 주의
- in-body preflight 블록 훼손 금지 — skill-doctor 의 needs-preflight 계약 fail.
- `description:` 손대면 lint:keywords 빨개짐 — 되돌려요.
- helper/ax-hub-cli 코드는 read-only — 안 건드림. (audit 에서 CLI 버그 없음.)
- recover error_code 표는 정본 — verify 는 cross-link 만.
- 재-drift guard 는 이 feature 범위 밖 (별도 공용 feature).
