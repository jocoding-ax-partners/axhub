# migrate workflow 배선 갭 3건 마감 플랜

> 2026-06-10 배선 감사에서 식별한 미연결 3건을 닫는 플랜이에요. 셋 다 "PR 스택 미머지" 시절의
> 의도적 연기였고, #198/#202/#203 머지 + v0.10.0 릴리즈로 전제가 충족돼 지금 활성화할 수 있어요.
> 작업 장소: main worktree (`axhub-remove-consent-logic`), 브랜치 `fix/migrate-wiring-gaps`, 1 PR.

## 갭 요약

| # | 갭 | 현재 상태 | 근거 위치 |
|---|---|---|---|
| 1 | `scan-sites` ↔ migrate SKILL 미연결 | 도구는 빌드됨(`Commands::ScanSites`), SKILL 은 0회 참조 — 변환 대상 탐지가 expert 수동 분석 의존 | `crates/axhub-helpers/src/cli/args/mod.rs:323` / `skills/migrate/SKILL.md` |
| 2 | migrate SKILL 에 `mcp-install` 없음 | init SKILL step 9 에만 존재 — migrate 사용자는 `.mcp.json` 미설치라 sdk_search 필수-조회 정책이 비활성 | `skills/init/SKILL.md:545` / `cli/args/mod.rs:335` |
| 3 | advisory → `migrate-data-verify` 위임 미활성 | `rule_message` 가 "권장" 문구만 방출(미머지 fallback 잠금 테스트가 위임 문구 금지) | `crates/axhub-helpers/src/ast_validate.rs:342`, `:1123` / `rules/PROVENANCE.md:50-57` |

---

## Task 1 — scan-sites 를 migrate SKILL 변환 단계에 연결

**무엇**: `skills/migrate/SKILL.md` §2.6 (SDK 변환 실행, expert dispatch 직전)에 sub-bullet 추가.

```bash
"$HELPER" scan-sites "${AXHUB_MIGRATE_DIR:-.}" --json
```

- 결과(JSON site 목록: data/auth call site 후보)를 expert dispatch prompt 에 전달해요 — expert 가
  "어떤 파일의 어떤 호출을 변환할지" deterministic 목록으로 시작하고, 수동 탐색은 보완용으로 내려가요.
- §2.7 (data 변환)도 같은 결과를 재사용해요 — 두 번 스캔하지 않아요.
- **advisory 성격 유지**: scan-sites 가 비었거나 실패해도 변환 흐름은 막지 않아요(fail-open).
  목록이 비면 "site 후보를 못 찾았어요 — expert 가 직접 탐색해요"로 안내만 해요.

**제약**:
- top-level `^N. **` step 신설 금지 — sub-bullet(2.6 내부)로만 추가해요 (skill-doctor FU-3 step-collision).
- frontmatter `description:` 불변 (nl-lexicon baseline 잠금).
- 본문 해요체 (`lint:tone --strict`).

**검증**: `bun run skill:doctor --strict` · `bun run lint:tone --strict` · `bun run lint:keywords --check` · `bun test`

## Task 2 — migrate SKILL 에 mcp-install 추가

**무엇**: §2.6 도입부(expert dispatch 전, Task 1 의 scan-sites 보다 앞)에 sub-bullet 추가. init step 9 문구를 미러해요.

```bash
"$HELPER" mcp-install --dir "${AXHUB_MIGRATE_DIR:-.}"
```

- `.mcp.json` 에 axhub local(stdio `mcp-serve`) + remote(ax-mcp) 항목을 idempotent 머지해요
  (기존 사용자 항목 보존, 원자쓰기 — helper 가 이미 보장).
- 설치 후 expert 가 **sdk_search 1회 필수 조회** 정책(packs DATA_RELIABILITY 절)을 실제로 수행할 수
  있게 돼요 — migrate 가 이 정책의 1순위 수혜 flow 라는 점을 본문에 명시해요.
- 실패해도 **비차단**이에요(init step 9 와 동일) — MCP 없이도 packs 만으로 변환은 진행돼요.

**제약**: Task 1 과 동일 (sub-bullet only · description 불변 · 해요체). 새 AskUserQuestion 없음 →
`tests/fixtures/ask-defaults/registry.json` 변경 불필요.

**검증**: Task 1 과 동일 게이트.

## Task 3 — advisory 위임 활성화 (Rust)

**무엇**: `crates/axhub-helpers/src/ast_validate.rs` 의 advisory 메시지에 위임 문구를 켜요.

1. `rule_message` (:342) advisory 분기 2개 메시지 끝에 위임 문구 추가:
   - `where_required`: "… 런타임 스키마(owner_column) 확인을 권장해요." → 뒤에
     "`axhub-helpers migrate-data-verify` 로 검증을 위임해요." 추가.
   - fallback (`_` arm): 동일 패턴으로 추가.
2. doc-comment (:339-341) 의 "**migrate-data-verify 위임 문구는 절대 넣지 않아요**(PR 스택 미머지
   fallback)" 를 활성화 후 상태로 갱신해요.
3. **기계 분기(`--help` exit 0 체크)는 생략** — PROVENANCE follow-up 의 분기 설계는 위임 대상이
   별도 PR 스택에 있던 시절의 안전장치예요. 지금은 `migrate-data-verify` 가 **같은 바이너리**의
   dispatch (main.rs legacy)라 항상 존재해요. 무조건 활성화가 더 단순하고 정확해요. 이 논거를
   PROVENANCE 에 기록해요.
4. 잠금 테스트 반전: `advisory_messages_recommend_without_delegation` (:1123) →
   `advisory_messages_delegate_to_migrate_data_verify` 로 rename 하고 assert 를 뒤집어요:
   - `msg.contains("migrate-data-verify")` **필수** (기존: 금지)
   - "권장" 문구 유지 assert 는 그대로 둬요 (advisory 톤 보존).
5. `rules/PROVENANCE.md:50-57` follow-up 3항목을 완료 처리로 갱신해요 (위임 문구 ✓ / 기계 분기 →
   동일-바이너리 논거로 불필요 판정 ✓ / 잠금 테스트 분기-aware 갱신 ✓).

**선행 절차** (프로젝트 룰): 편집 전 `gitnexus_impact({target: "rule_message", direction: "upstream"})`
실행, HIGH/CRITICAL 시 보고. 예상 blast: `ast_validate` 내부 호출 2곳 (:426, 테스트) — LOW 예상.

**검증**: `cargo test -p axhub-helpers` (특히 `ast_validate` 모듈 + `rule_messages_are_rule_id_specific`)
· `cargo build` · 기존 good fixture 6언어 advisory 메시지 스냅샷 확인.

---

## 실행 순서와 의존성

- Task 1·2 는 같은 파일(`skills/migrate/SKILL.md`)이라 **순차 편집** (2 → 1 순서: mcp-install 이
  scan-sites 보다 앞 sub-bullet 이라 위에서부터 작성).
- Task 3 은 독립 — 병렬 가능해요.
- 셋 다 한 브랜치 `fix/migrate-wiring-gaps` 에 커밋하고 **1 PR** 로 올려요.
  커밋 분리: `fix: migrate SKILL 에 mcp-install·scan-sites 배선 추가` / `fix: advisory 메시지에
  migrate-data-verify 위임 활성화`. AI attribution 문구 금지.

## 최종 게이트 (Self-Check)

- [ ] `bun run skill:doctor --strict` exit 0
- [ ] `bun run lint:tone --strict` 0 err
- [ ] `bun run lint:keywords --check` no diff
- [ ] `bun test` ≥ 기존 baseline pass / 0 fail
- [ ] `bunx tsc --noEmit` clean
- [ ] `cargo test -p axhub-helpers` green
- [ ] `gitnexus_detect_changes()` — 변경 범위가 SKILL 1 + ast_validate 1 + PROVENANCE 1 로 한정 확인
- [ ] PR 본문에 갭 3건 ↔ 커밋 매핑 표 포함

## 리스크 / 비고

- **scan-sites 출력 schema**: SKILL 문구가 실제 JSON shape 를 참조해야 해요 — 구현 시
  `scan-sites <fixture> --json` 실출력으로 필드명 확인 후 작성해요 (추측 금지).
- **size ceiling**: 코드 변경이 메시지 문자열 수준이라 바이너리 크기 영향 미미 — `release:check`
  ceiling(실측+15%) 여유 충분해요.
- **후속(이 플랜 범위 밖)**: `migrate-data-verify` legacy dispatch → typed Commands 이관 (specs/001
  백로그) / release postbump 에 PLAN.md plugin-schema 스니펫 자동 동기화 추가 (0.9.43 화석 재발 방지).
