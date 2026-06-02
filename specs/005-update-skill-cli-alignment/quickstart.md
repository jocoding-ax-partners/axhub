# Quickstart: update 스킬 정렬 구현 + 검증

> 대상: `skills/update/SKILL.md` 를 [contract](./contracts/update-cli-contract.md) 에 맞춰 rewrite. 신규 스킬 아니라 **scaffold 불필요** (기존 파일 편집).

## 구현 절차

### 0. (착수 전) catalog 키 blast radius 확인 (research D10 — 체인은 이미 확인됨)
catalog 정정은 **codegen 체인**: `catalog.json`(source) → `generated.md`(regen) → hand `.md`. 변경 전 키 사용처를 grep:
```bash
rg -n "cosign_verification_failed" crates/ skills/ scripts/ tests/   # 예상 5 site + 추가 참조 확인
rg -n "cosign_verification_failed|cosign_enforce_failed" crates/axhub-helpers   # helper lookup/매핑
```
helper 가 CLI `error.subcode` 로 catalog 를 lookup 하면 키 = 실제 subcode(`update.cosign_enforce_failed`)여야 정합. blast radius 가 과하면 tasks 에서 범위 재조정.

### 1. frontmatter 보존 (수정 금지 영역)
- `description:` (트리거 lexicon) **byte 그대로**. `model: sonnet`, `multi-step: true`, `needs-preflight: false`, examples 유지.
- TodoWrite Step 0 / D1 비대화형 guard 패턴 유지.

### 2. 본문 rewrite (contract 기준)
- Step 1 check: `axhub update check --json` 유지. **단** "exit 2 = autoupdate disabled" 시나리오 + `AXHUB_DISABLE_AUTOUPDATE` 언급 삭제. `{current,latest,has_update}` 파싱 유지.
- Step 4 apply:
  - `axhub update apply --dry-run --json` preview 유지 — `is_downgrade`/`feed_base`/`next_step` 활용.
  - execute 를 **`axhub update apply --execute --yes --json`** 로 (env 접두 `AXHUB_REQUIRE_COSIGN=1` **삭제**).
  - 성공 → `applied:true` + `binary` 안내, "새 터미널/`axhub --version`".
- exit 처리 (data-model 상태전이대로):
  - 14 → 변조 하드 스톱.
  - 15 → 재시도 금지 + `.bak` 롤백 안내.
  - 66 + `update.downgrade_blocked` → `--force`(cosign 안전) 안내.
  - 66 + `update.cosign_enforce_failed` → cosign 하드 스톱(우회 없음).
  - 1/4/10/64 → error-empathy-catalog 라우팅.
- NEVER 절 갱신: `AXHUB_ALLOW_UNSIGNED` 언급 삭제 → "`--force` 는 cosign 우회 아님" + "14/cosign-66 우회 금지" 로 교체.
- brew/scoop 분기(Step 6 등) **통째 삭제**.

### 3. 연계 참조 정정 (codegen 체인 — 순서 중요, step 0 blast radius OK 후)
1. `crates/axhub-helpers/data/catalog.json`: 키 `update.cosign_verification_failed` → `update.cosign_enforce_failed`. 필요 시 exit 14/15 항목 추가.
2. `bun run codegen:catalog` → `error-empathy-catalog.generated.md` 재생성 (**직접 편집 금지**).
3. `skills/deploy/references/error-empathy-catalog.md`(hand, line 160): 헤더·문구 동일 정정.
4. `bun test tests/codegen.test.ts` → catalog.json ↔ generated 일치 확인.

### 4. (AskUserQuestion 변경 시에만) registry 동기화
AskUserQuestion 추가/변경하면 `tests/fixtures/ask-defaults/registry.json` 에 `safe_default`+`rationale` 등록. (기존 `update.apply_consent` 유지면 불요.)

## 검증 (DoD — 순서대로)

```bash
# 1. 가공 명령/env 0 건
rg -n "AXHUB_REQUIRE_COSIGN|AXHUB_ALLOW_UNSIGNED|AXHUB_DISABLE_AUTOUPDATE" skills/update/SKILL.md   # expect 0
rg -n "package_manager|brew|scoop" skills/update/SKILL.md                                            # expect 0
rg -n "cosign_verification_failed" skills/update/SKILL.md skills/deploy/references/ crates/axhub-helpers/data/   # expect 0 (체인 전체 정정 후)

# 2. exit 2 가 정책으로 안 쓰임 (clap 예약만)
rg -n "exit 2" skills/update/SKILL.md   # autoupdate 정책 해석 없어야

# 3. live CLI 대조 (바이너리 있을 때)
~/.axhub/bin/axhub update --help
~/.axhub/bin/axhub update apply --help     # --dry-run/--execute/--yes/--force 확인
# (선택) preview 실행: ~/.axhub/bin/axhub update apply --dry-run --json

# 4. repo 게이트 (CLAUDE.md 순서)
bun run skill:doctor --strict
bun run lint:tone --strict
bun run lint:keywords --check     # description byte-lock — diff 0
bun test                          # ux-todowrite / ux-ask-fallback-registry 등 회귀
bunx tsc --noEmit
```

## 성공 기준 매핑 (spec SC ↔ 검증)
- SC-001 (명령 수용) → 검증 3 (live --help/dry-run)
- SC-002 (가공 env 0) → 검증 1
- SC-003 (exit 매핑) → contract §3 ↔ SKILL 본문 수동 대조
- SC-004 (게이트) → 검증 4
- SC-005 (성공 경로) → dry-run preview + (가능 시) 실제 upgrade 또는 안전 정지

## 주의
- `lint:keywords --check` 가 빨개지면 description 을 건드린 것 — 되돌려요 (트리거 byte-lock).
- ax-hub-cli(Rust)는 **읽기 전용 권위**. 이 repo 에서 빌드/수정하지 않아요.
- drift-guard 는 범위 밖 — 추가하지 않아요.
