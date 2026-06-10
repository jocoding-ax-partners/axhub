# migrate 플로우 하드닝 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 2026-06-10 migrate 드라이런(os 앱)에서 확인된 8가지 플로우 결함 — GitHub device-flow/installation UX 불일치, stage md 중복 생성, redaction 누락, ITERATE/REQUEST_CHANGES 게이트 무시, migrate-approve skip, plan_only hard-stop 돌파, push/히스토리 재작성 무단 실행, discoverer secret prefix 기록 — 을 helper hard-gate + SKILL 계약 + agent rule + 테스트 4계층으로 막아요.

**Architecture:** (1) Rust helper(`migrate_planning.rs`)가 stage 파일명을 stage별 고정 ordinal로 강제하고 verdict 기반 순서 게이트·redact backstop을 write 시점에 적용해요(모델이 우회 불가능한 hard gate). (2) `skills/migrate/SKILL.md`는 github SKILL의 device-flow/installation 패턴을 위임 참조하고, migrate-approve 필수·plan_only 불가침·drafts 경로를 명문화해요(soft gate). (3) agent prompt 5종에 secret 값 금지 rule을 추가해요. (4) `tests/migrate-skill-contract.test.ts` + Rust in-file 테스트가 계약 drift를 잡아요.

**Tech Stack:** Rust (axhub-helpers crate, regex/LazyLock/serde_json), Bun test (TS), SKILL.md (Korean 해요체).

**드라이런 증거 (근거):**
- 중복: `os/.axhub/plan/runs/20260610-081254-45a89e/stages/`에 10개 파일. meta 없는 본(02-planner, 04-architect, 06-critic, 08-reviewer)은 Claude가 Write tool로 직접 생성, meta 있는 본(03, 05, 07, 09, 10)은 helper가 `next_stage_ordinal`(dir-scan max+1)로 복제 생성.
- 미redaction: `05-architect.md`, `07-critic.md`에 `a57b33...`, `xoxb-1...` prefix 잔존. `approval.json`이 그 해시를 봉인.
- 게이트 무시: critic ITERATE → reviewer 직행, reviewer REQUEST_CHANGES + `migrate-approve` 미실행(`run.json`/`approval.json` 둘 다 `pending_approval`) 상태로 mutation 실행.
- plan_only 돌파: `custom_auth`/`secret_exposure` 비재정의 hard-stop인데 auth controller patch + `git filter-repo` + force-push 실행.
- UX: migrate 중 device flow에서 사용자에게 `! axhub ... --resume-last` 명령을 직접 치라고 출력 (github/init/auth SKILL은 이미 "승인했어 신호 → 에이전트가 resume" 패턴 보유, migrate SKILL만 미참조).

---

## File Structure

| 파일 | 역할 |
|---|---|
| `/Users/wongil/Downloads/os/.axhub/plan/runs/20260610-081254-45a89e/stages/*` | Task 1: 오염 run 즉시 정리 (axhub repo 밖, ops 작업) |
| `crates/axhub-helpers/src/redact.rs` | Task 2: Slack token/webhook 패턴 추가 |
| `crates/axhub-helpers/src/migrate_planning.rs` | Task 3·4: 고정 ordinal, drafts 격리, redact backstop, verdict 게이트 |
| `skills/migrate/SKILL.md` | Task 5·6·7: approve 게이트, NEVER 강화, github 위임, drafts 계약 |
| `agents/axhub-migrate-{discoverer,planner,architect,critic,reviewer}.md` | Task 8: secret 값 금지 rule |
| `tests/migrate-skill-contract.test.ts` | Task 9: SKILL 계약 assert 추가 |

---

### Task 1: 오염된 os run artifact 즉시 정리 (보안, 코드 변경 아님)

**Files:**
- Modify: `/Users/wongil/Downloads/os/.axhub/plan/runs/20260610-081254-45a89e/stages/05-architect.md`
- Modify: `/Users/wongil/Downloads/os/.axhub/plan/runs/20260610-081254-45a89e/stages/07-critic.md`
- Delete: 같은 디렉터리의 meta 없는 중복본 + 구버전 reviewer

- [ ] **Step 1: 잔존 secret prefix 위치 확인**

Run: `grep -n 'a57b33\|xoxb-1\|hooks.slack.com' /Users/wongil/Downloads/os/.axhub/plan/runs/20260610-081254-45a89e/stages/05-architect.md /Users/wongil/Downloads/os/.axhub/plan/runs/20260610-081254-45a89e/stages/07-critic.md`
Expected: 매칭 라인 출력 (2026-06-10 기준 양쪽 파일에 존재 확인됨)

- [ ] **Step 2: Edit tool로 두 파일의 token prefix를 `[REDACTED]`로 치환**

`05-architect.md`의 `LEAD_API_TOKEN`(a57b33...), `SLACK_BOT_TOKEN`(xoxb-1...), `SLACK_WEBHOOK_URL` 값 조각을 전부 `[REDACTED]`로 바꿔요. `07-critic.md`도 동일. (이미 redaction된 01/04/06과 같은 표기 사용.)

- [ ] **Step 3: 중복 파일 삭제**

```bash
cd /Users/wongil/Downloads/os/.axhub/plan/runs/20260610-081254-45a89e/stages
rm 02-planner.md 04-architect.md 06-critic.md 08-reviewer.md 09-reviewer.md 09-reviewer.meta.json
```

meta 없는 본(Write tool 직접 생성분)과 09-reviewer(10이 최종)를 지워요. `approval.json`의 `approved_stage_artifacts` 해시 목록과 어긋나지만, 이 run은 spec 위반 실행이 이미 끝난 폐기 대상이라 다시 `migrate-approve`하지 않아요 — 새 마이그레이션은 새 run으로 시작.

- [ ] **Step 4: 검증**

Run: `grep -rn 'a57b33\|xoxb-1' /Users/wongil/Downloads/os/.axhub/plan/runs/20260610-081254-45a89e/ || echo CLEAN`
Expected: `CLEAN`

(os repo는 git 추적 대상이 `.axhub/plan`을 포함하는지에 따라 커밋 여부가 갈려요 — 추적 중이면 os repo에서 `chore: migrate run artifact redaction` 커밋. axhub repo 커밋은 없어요.)

---

### Task 2: redact.rs — Slack token/webhook 패턴 (TDD)

**Files:**
- Modify: `crates/axhub-helpers/src/redact.rs`
- Test: 같은 파일 `mod tests`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/axhub-helpers/src/redact.rs`의 `#[cfg(test)] mod tests` 안에 추가:

```rust
    #[test]
    fn redacts_slack_tokens_and_webhooks() {
        let input = "SLACK_BOT_TOKEN=xoxb-1073512345678-abcDEF123ghi SLACK_WEBHOOK_URL=https://hooks.slack.com/services/T0B6XAAAA/B0BBBB/secretpart123";
        let out = redact(input);
        assert!(!out.contains("xoxb-1073512345678"));
        assert!(out.contains("<REDACTED_SLACK_TOKEN>"));
        assert!(!out.contains("secretpart123"));
        assert!(out.contains("<REDACTED_SLACK_WEBHOOK>"));
    }
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p axhub-helpers redacts_slack -- --nocapture`
Expected: FAIL (`<REDACTED_SLACK_TOKEN>` 미포함)

- [ ] **Step 3: 패턴 구현**

기존 RE 선언부(`AWS_KEY_RE` 아래)에 추가:

```rust
// Slack token taxonomy: xoxb(bot)/xoxp(user)/xoxa(app)/xoxo(workspace)/xoxs(session)/xoxr(refresh)/xoxe(export)
static SLACK_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"xox[abeoprs]-[0-9A-Za-z-]{4,}").unwrap());
static SLACK_WEBHOOK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://hooks\.slack\.com/services/[A-Za-z0-9/_-]+").unwrap()
});
```

`redact()` 본문의 `GH_TOKEN_RE` 줄 다음에 추가:

```rust
    let s = SLACK_TOKEN_RE.replace_all(&s, "<REDACTED_SLACK_TOKEN>");
    let s = SLACK_WEBHOOK_RE.replace_all(&s, "<REDACTED_SLACK_WEBHOOK>");
```

(주의: `SLACK_WEBHOOK_RE`는 `URL_CREDS_RE`보다 **앞**에 둬요 — 순서 주석 컨벤션 유지.)

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p axhub-helpers redact`
Expected: 전체 redact 테스트 PASS (기존 테스트 포함)

- [ ] **Step 5: 커밋**

```bash
git add crates/axhub-helpers/src/redact.rs
git commit -m "fix: redact 에 Slack token/webhook 패턴 추가"
```

---

### Task 3: migrate-stage-write — 고정 ordinal + 멱등 overwrite + stages/ 직접 쓰기 거부 + redact backstop (TDD)

**Files:**
- Modify: `crates/axhub-helpers/src/migrate_planning.rs` (`migrate_stage_write` :1184, `next_stage_ordinal` :1734, `ensure_allowed_stage` :1725)
- Test: 같은 파일 `mod tests` (:1937, 기존 fixture helper 재사용)

**설계 결정:**
- stage → ordinal 고정 map: discover=01, planner=02, architect=03, critic=04, reviewer=05. 같은 stage 재기록은 **같은 파일 overwrite** + meta `revision` 증가 (iterate loop의 planner revision이 자연스럽게 같은 파일을 갱신). 이력은 기존 `receipts.jsonl`이 보존.
- `--markdown-file`이 `stages/` 내부면 거부 → Write tool 직접 생성 + helper 복제라는 이중 저장 자체가 불가능해져요. 초안은 `drafts/`에 (Task 7에서 SKILL 계약화).
- write 직전 `redact::redact()` 적용 — agent가 secret을 넣어도 디스크에 닿기 전에 마스킹(backstop). sha는 redaction **후** 내용으로 계산해 `approval.json` seal과 일치.
- wave write-target(`stages/05-reviewer-a.md` 형식)은 stage-write가 만들지 않는 선언적 경로라 충돌 없음 (`05-reviewer-a` ≠ `05-reviewer`). `collect_stage_sha_list`·`stage_ordinal_from_path`는 무변경 호환.

- [ ] **Step 1: 실패하는 테스트 3개 작성**

`mod tests`에 추가 (기존 테스트들이 쓰는 run scaffold fixture helper — `migrate_plan_run_init` 계열 — 를 그대로 재사용해요. 초안 파일은 `run_dir` 밖 temp에 둬요):

```rust
    #[test]
    fn stage_write_uses_fixed_ordinal_and_overwrites() {
        let (run_json, run_dir) = full_consensus_fixture(); // 기존 fixture helper 명에 맞춰요
        let draft = run_dir.parent().unwrap().join("planner-draft.md");
        fs::write(&draft, "# planner v1").unwrap();
        migrate_stage_write(&run_json, "planner", &draft, None, None, None).unwrap();
        fs::write(&draft, "# planner v2 (revision)").unwrap();
        migrate_stage_write(&run_json, "planner", &draft, None, None, None).unwrap();

        let stages = run_dir.join("stages");
        let md_files: Vec<_> = fs::read_dir(&stages).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
            .collect();
        assert_eq!(md_files.len(), 1, "planner 2회 기록 후에도 md 는 1개여야 해요");
        assert!(stages.join("02-planner.md").exists());
        let meta: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(stages.join("02-planner.meta.json")).unwrap()).unwrap();
        assert_eq!(meta["revision"], 2);
    }

    #[test]
    fn stage_write_rejects_markdown_inside_stages_dir() {
        let (run_json, run_dir) = full_consensus_fixture();
        let stages = run_dir.join("stages");
        fs::create_dir_all(&stages).unwrap();
        let inside = stages.join("99-manual.md");
        fs::write(&inside, "# direct write").unwrap();
        let err = migrate_stage_write(&run_json, "planner", &inside, None, None, None).unwrap_err();
        assert!(err.to_string().contains("stages/"), "stages/ 내부 초안은 거부해야 해요");
    }

    #[test]
    fn stage_write_redacts_secret_values_on_disk() {
        let (run_json, run_dir) = full_consensus_fixture();
        let draft = run_dir.parent().unwrap().join("discover-draft.md");
        fs::write(&draft, "SLACK_BOT_TOKEN=xoxb-1073512345678-abcDEF123ghi 가 노출됐어요").unwrap();
        migrate_stage_write(&run_json, "discover", &draft, None, None, None).unwrap();
        let written = fs::read_to_string(run_dir.join("stages/01-discover.md")).unwrap();
        assert!(!written.contains("xoxb-1073512345678"));
        assert!(written.contains("<REDACTED_SLACK_TOKEN>"));
        let meta: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(run_dir.join("stages/01-discover.meta.json")).unwrap()).unwrap();
        assert_eq!(meta["redacted"], true);
    }
```

(주의: 시그니처는 Task 4에서 `verdict` 파라미터가 추가되면 `None` 인자가 하나 늘어요. Task 3·4를 연달아 작업하면 Task 4 완료 후 일괄 컴파일돼요. Task 3 단독 검증 시엔 현재 6-인자 시그니처 그대로.)

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p axhub-helpers stage_write -- --nocapture`
Expected: 신규 3개 FAIL (고정 ordinal/거부/redact 미구현), 기존 stage-write 테스트 PASS

- [ ] **Step 3: 고정 ordinal 구현 — `ensure_allowed_stage`를 ordinal 반환으로 교체**

`migrate_planning.rs:1725`의 `ensure_allowed_stage`를 다음으로 교체:

```rust
fn stage_fixed_ordinal(stage: &str) -> Result<u32> {
    match stage {
        "discover" => Ok(1),
        "planner" => Ok(2),
        "architect" => Ok(3),
        "critic" => Ok(4),
        "reviewer" => Ok(5),
        _ => bail!("migrate-stage-write: 지원하지 않는 stage 예요"),
    }
}
```

`migrate_stage_write`(:1212-1223)의 stage 분기를 교체:

```rust
    let (markdown_target, meta_target, event_name) = if stage == "adr" {
        (run_dir.join("adr.md"), None, "adr_written")
    } else {
        let ordinal = stage_fixed_ordinal(stage)?;
        let stages_dir = run_dir.join("stages");
        fs::create_dir_all(&stages_dir)?;
        (
            stages_dir.join(format!("{ordinal:02}-{stage}.md")),
            Some(stages_dir.join(format!("{ordinal:02}-{stage}.meta.json"))),
            "stage_written",
        )
    };
```

`next_stage_ordinal`(:1734) 함수는 삭제해요 (이 변경으로 유일 호출처가 사라진 orphan). `ensure_allowed_stage`의 다른 호출처가 있으면 `stage_fixed_ordinal(stage)?;`로 치환 (`grep -n "ensure_allowed_stage" crates/axhub-helpers/src/`로 확인).

- [ ] **Step 4: stages/ 내부 초안 거부 + redact backstop + revision 구현**

`migrate_stage_write`에서 markdown 읽기(:1204) 직전에 거부 가드:

```rust
    let stages_dir_guard = run_dir.join("stages");
    let canonical_md = markdown_file
        .canonicalize()
        .unwrap_or_else(|_| markdown_file.to_path_buf());
    let canonical_stages = stages_dir_guard
        .canonicalize()
        .unwrap_or_else(|_| stages_dir_guard.clone());
    if canonical_md.starts_with(&canonical_stages) {
        bail!(
            "migrate-stage-write: --markdown-file 은 stages/ 밖(예: <run_dir>/drafts/)에 두세요 — stages/ 는 helper 전용 경로예요"
        );
    }
```

markdown 읽기 직후(:1209-1210)를 redaction 경유로 교체:

```rust
    let raw_markdown = fs::read_to_string(markdown_file).with_context(|| {
        format!(
            "{} stage markdown 를 읽지 못했어요",
            markdown_file.display()
        )
    })?;
    let markdown = crate::redact::redact(&raw_markdown);
    let was_redacted = markdown != raw_markdown;
    let markdown_sha = sha256_hex(&markdown);
```

meta json(:1228-1238)에 `revision`/`redacted` 추가:

```rust
    if let Some(meta_path) = meta_target.as_ref() {
        let ordinal = stage_ordinal_from_path(&markdown_target)?;
        let revision = if meta_path.exists() {
            read_json::<serde_json::Value>(meta_path)
                .ok()
                .and_then(|v| v.get("revision").and_then(serde_json::Value::as_u64))
                .unwrap_or(0)
                + 1
        } else {
            1
        };
        let meta = json!({
            "schema_version": MIGRATE_PLAN_STAGE_SCHEMA_VERSION,
            "run_id": run.run_id,
            "app_key": run.app_key,
            "stage": stage,
            "stage_n": ordinal,
            "revision": revision,
            "redacted": was_redacted,
            "state": "complete",
            "artifact_sha256": markdown_sha,
            "created_at": now,
            "updated_at": now
        });
        write_json_atomically(meta_path, &meta)?;
    }
```

(redact가 NFKC 정규화/ANSI 제거도 하므로 `was_redacted`는 정확히는 "내용이 정화됐다" 신호예요 — meta 키 이름은 `redacted` 유지, 의미는 주석 한 줄로: `// redacted = secret masking 또는 normalization 으로 원문과 달라짐`.)

- [ ] **Step 5: 기존 incremental-ordinal 기대 테스트 갱신**

Run: `cargo test -p axhub-helpers migrate_planning 2>&1 | head -40`
기존 테스트 중 ordinal 증가(`02→03...`)를 기대하는 assert가 깨지면 고정 ordinal 기대값으로 수정해요. (테스트가 스펙이 아니라 구버전 동작을 박제한 경우라 수정이 맞아요 — SKILL 문서의 고정 경로 `02-planner.md` 가 원래 스펙.)

- [ ] **Step 6: 전체 테스트 통과 확인**

Run: `cargo test -p axhub-helpers`
Expected: PASS (신규 3개 포함)

- [ ] **Step 7: 커밋**

```bash
git add crates/axhub-helpers/src/migrate_planning.rs
git commit -m "fix: migrate-stage-write 고정 ordinal·멱등 overwrite·redact backstop — stage md 중복 생성 차단"
```

---

### Task 4: migrate-stage-write — verdict 기록 + 파이프라인 순서 hard gate (TDD)

**Files:**
- Modify: `crates/axhub-helpers/src/migrate_planning.rs` (`run_migrate_stage_write` :951 arg 파싱, `migrate_stage_write` :1184, seal 분기 :1276)
- Test: 같은 파일 `mod tests`

**설계 결정:** 드라이런에서 "critic ITERATE → reviewer 직행"과 "reviewer REQUEST_CHANGES 상태로 seal"이 발생했어요. SKILL 문구만으로는 막지 못했으니 write 시점에 강제해요:
- `--verdict <approve|lgtm|iterate|block|request_changes|comment>` 플래그 신설. critic/reviewer는 **필수**, 그 외 optional.
- 순서 게이트: planner←discover, architect←planner, critic←architect, reviewer←critic의 meta 존재 필수.
- reviewer 기록은 critic verdict ∈ {approve, lgtm}일 때만 허용 — iterate/block이면 "planner revision 후 architect→critic 재실행" 에러. (planner를 다시 기록하면 critic이 재실행되어 meta가 갱신되므로 loop가 자연 성립.)
- seal(`--approval-state pending_approval`)은 reviewer verdict ∈ {approve, lgtm} 필수 — REQUEST_CHANGES 상태 봉인 차단.

- [ ] **Step 1: 실패하는 테스트 작성**

```rust
    #[test]
    fn critic_write_requires_verdict() {
        let (run_json, run_dir) = full_consensus_fixture_through_stage("architect");
        let draft = draft_file(&run_dir, "critic", "# critic");
        let err = migrate_stage_write(&run_json, "critic", &draft, None, None, None, None).unwrap_err();
        assert!(err.to_string().contains("--verdict"));
    }

    #[test]
    fn reviewer_write_blocked_when_critic_iterate() {
        let (run_json, run_dir) = full_consensus_fixture_through_stage("architect");
        let critic_draft = draft_file(&run_dir, "critic", "# critic iterate");
        migrate_stage_write(&run_json, "critic", &critic_draft, None, None, None, Some("iterate")).unwrap();
        let reviewer_draft = draft_file(&run_dir, "reviewer", "# reviewer");
        let err = migrate_stage_write(&run_json, "reviewer", &reviewer_draft, None, None, None, Some("approve")).unwrap_err();
        assert!(err.to_string().contains("iterate"), "critic iterate 면 reviewer 기록을 거부해야 해요");
    }

    #[test]
    fn seal_blocked_without_reviewer_approve() {
        let (run_json, run_dir) = full_consensus_fixture_through_stage("critic_approved");
        let reviewer_draft = draft_file(&run_dir, "reviewer", "# reviewer request_changes");
        migrate_stage_write(&run_json, "reviewer", &reviewer_draft, None, None, None, Some("request_changes")).unwrap();
        write_adr_fixture(&run_dir); // 기존 adr fixture helper
        let seal_draft = draft_file(&run_dir, "reviewer", "# reviewer still request_changes");
        let err = migrate_stage_write(
            &run_json, "reviewer", &seal_draft, None,
            Some(RunState::PendingApproval), Some(ApprovalState::PendingApproval),
            Some("request_changes"),
        ).unwrap_err();
        assert!(err.to_string().contains("reviewer verdict"), "approve 없는 seal 은 거부해야 해요");
    }

    #[test]
    fn stage_order_gate_blocks_skipping() {
        let (run_json, run_dir) = full_consensus_fixture(); // discover 만 있는 상태
        let draft = draft_file(&run_dir, "critic", "# critic without upstream");
        let err = migrate_stage_write(&run_json, "critic", &draft, None, None, None, Some("approve")).unwrap_err();
        assert!(err.to_string().contains("architect"), "upstream stage 없이 건너뛰면 거부해야 해요");
    }
```

`full_consensus_fixture_through_stage` / `draft_file`은 기존 fixture helper 조합으로 만들어요 (없으면 테스트 모듈에 사설 helper로 추가 — discover→지정 stage까지 `migrate_stage_write`를 순서대로 호출해 meta를 쌓는 함수).

- [ ] **Step 2: 테스트 실패(컴파일 에러 포함) 확인**

Run: `cargo test -p axhub-helpers stage_write 2>&1 | head -20`
Expected: 시그니처 불일치 컴파일 에러 (verdict 파라미터 미존재)

- [ ] **Step 3: verdict 플래그 + 게이트 구현**

`run_migrate_stage_write`(:951) arg 루프에 추가:

```rust
            "--verdict" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-stage-write: --verdict 값이 필요해요");
                };
                verdict = Some(parse_stage_verdict(value)?);
                index += 2;
            }
```

선언부에 `let mut verdict = None;`, 호출에 `verdict.as_deref()` 전달. 파서·리더 함수 추가:

```rust
fn parse_stage_verdict(value: &str) -> Result<String> {
    let allowed = ["approve", "lgtm", "iterate", "block", "request_changes", "comment"];
    if allowed.contains(&value) {
        Ok(value.to_string())
    } else {
        bail!("migrate-stage-write: --verdict 는 approve|lgtm|iterate|block|request_changes|comment 중 하나예요");
    }
}

fn read_stage_meta_field(stages_dir: &Path, stage: &str, field: &str) -> Result<Option<String>> {
    let ordinal = stage_fixed_ordinal(stage)?;
    let meta_path = stages_dir.join(format!("{ordinal:02}-{stage}.meta.json"));
    if !meta_path.exists() {
        return Ok(None);
    }
    let value = read_json::<serde_json::Value>(&meta_path)?;
    Ok(value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned))
}
```

`migrate_stage_write` 시그니처에 `verdict: Option<&str>` 추가. stage 분기(adr 아님) 안에서 ordinal 계산 직후 게이트:

```rust
        if matches!(stage, "critic" | "reviewer") && verdict.is_none() {
            bail!("migrate-stage-write: {stage} stage 는 --verdict 가 필수예요");
        }
        let upstream = match stage {
            "planner" => Some("discover"),
            "architect" => Some("planner"),
            "critic" => Some("architect"),
            "reviewer" => Some("critic"),
            _ => None,
        };
        if let Some(upstream_stage) = upstream {
            if read_stage_meta_field(&stages_dir, upstream_stage, "stage")?.is_none() {
                bail!(
                    "migrate-stage-write: {upstream_stage} stage 기록이 없어 {stage} 로 못 가요 — 순서대로 진행해요"
                );
            }
        }
        if stage == "reviewer" {
            match read_stage_meta_field(&stages_dir, "critic", "verdict")? {
                Some(v) if matches!(v.as_str(), "approve" | "lgtm") => {}
                Some(v) => bail!(
                    "migrate-stage-write: critic verdict 가 {v} 라 reviewer 로 못 가요 — planner revision 후 architect → critic 을 다시 돌아요"
                ),
                None => bail!(
                    "migrate-stage-write: critic verdict 기록이 없어요 — critic stage 를 --verdict 와 함께 먼저 기록해요"
                ),
            }
        }
```

meta json에 `"verdict": verdict,` 추가 (`Option<&str>`은 serde_json `json!`에서 null 직렬화).

seal 분기(:1276 `ApprovalState::PendingApproval` 검사부, wave 검사 다음)에 추가:

```rust
            match read_stage_meta_field(&run_dir.join("stages"), "reviewer", "verdict")? {
                Some(v) if matches!(v.as_str(), "approve" | "lgtm") => {}
                other => bail!(
                    "migrate-stage-write: reviewer verdict 가 approve/lgtm 이 아니라 seal 할 수 없어요 (현재: {other:?}) — REQUEST_CHANGES 해소 후 reviewer 를 다시 기록해요"
                ),
            }
```

(같은 호출에서 reviewer 기록과 seal을 동시에 하는 경우 — 위 reviewer 기록이 먼저 디스크에 닿은 뒤 seal 검사가 읽으므로 verdict approve면 한 호출로 통과, request_changes면 거부.)

- [ ] **Step 4: 기존 호출처·테스트 시그니처 갱신**

`migrate_stage_write` 직접 호출하는 기존 테스트들에 `None`(또는 critic/reviewer면 `Some("approve")`) 인자 추가. Task 3 신규 테스트도 7-인자로 갱신.

- [ ] **Step 5: 전체 테스트 통과 확인**

Run: `cargo test -p axhub-helpers`
Expected: PASS

- [ ] **Step 6: 커밋**

```bash
git add crates/axhub-helpers/src/migrate_planning.rs
git commit -m "feat: migrate-stage-write verdict 게이트 — iterate/request_changes 상태의 stage 건너뛰기·seal 차단"
```

---

### Task 5: migrate SKILL — migrate-approve 필수 게이트 + plan_only·push 불가침 NEVER

**Files:**
- Modify: `skills/migrate/SKILL.md` (full_consensus 승인 처리 부근 + `## NEVER` 섹션 :612)

- [ ] **Step 1: 승인 처리 블록 명문화**

full_consensus 파이프라인 설명부(stage-write 스니펫 :438 부근, pending_approval 전환 설명 다음)에 추가:

```markdown
사용자 승인 발화("승인할게요" / "진행해" / "전부 다 해")를 받으면 **mutation 으로 바로 가지 않고** 먼저 helper 승인을 기록해요:

​```bash
"$HELPER" migrate-approve --run-json "$RUN_JSON" --approved-by "user" --json
​```

migrate-approve 가 성공해야만 (`run.json`/`approval.json` 의 state 가 `approved`) mutation 단계로 넘어가요. mutation 직전에 매번 확인해요:

​```bash
RUN_DIR="$(dirname "$RUN_JSON")"
RUN_STATE=$(jq -r '.state // empty' "$RUN_JSON" 2>/dev/null)
APPROVAL_STATE=$(jq -r '.state // empty' "$RUN_DIR/approval.json" 2>/dev/null)
if [ "$RUN_STATE" != "approved" ] || [ "$APPROVAL_STATE" != "approved" ]; then
  # 승인 전이에요 — mutation 을 시작하지 않고 migrate-approve 부터 안내해요
  :
fi
​```

critic verdict 가 `iterate`/`block` 이면 reviewer 로 가지 않고 planner revision → architect → critic 을 다시 돌아요 (helper 가 순서를 강제해요). iterate 가 2회를 넘으면 자동 반복을 멈추고 미해소 항목을 사용자에게 에스컬레이션해요. reviewer verdict 가 `request_changes` 면 해소 후 reviewer 를 다시 기록해야 seal 이 돼요.
```

(코드펜스 안 ​``` 는 실제 파일에선 일반 ``` 로.)

- [ ] **Step 2: NEVER 섹션 강화**

`## NEVER`(:612)에 추가:

```markdown
- NEVER `migrate-approve` 성공 없이 mutation(앱 등록·git 연결·env 저장·배포)을 실행하지 않아요. "전부 다 해" 같은 포괄 발화도 migrate-approve 기록 후에만 진행해요.
- NEVER plan_only hard-stop(`custom_auth`, `secret_exposure`) 범위의 코드 변경을 실행하지 않아요 — auth patch·secret 관련 수정은 포괄 승인("강행" 포함)으로도 풀리지 않고, 계획 문서만 산출해요. 사용자가 해당 파일을 직접 지목한 별도 요청만 migrate 범위 밖 일반 작업으로 다뤄요.
- NEVER 사용자 repo 에 `git push` / `git push --force` / `git filter-repo` / BFG 같은 원격·히스토리 변경을 실행하지 않아요. secret rotation·history purge 는 명령어 안내문만 제공하고 실행은 사용자 몫이에요. 로컬 patch 전에는 `migrate-guard --checkpoint` 를 먼저 떠요.
```

- [ ] **Step 3: lint 확인**

Run: `bun run lint:tone --strict && bun run lint:keywords --check`
Expected: 0 err / no diff (frontmatter `description:` 무변경이라 keywords 통과)

- [ ] **Step 4: 커밋**

```bash
git add skills/migrate/SKILL.md
git commit -m "fix: migrate SKILL 승인 게이트 명문화 — migrate-approve 필수·plan_only 불가침·push 금지"
```

---

### Task 6: migrate SKILL — GitHub device-flow/installation gate를 init·github 패턴으로 통일

**Files:**
- Modify: `skills/migrate/SKILL.md` (§ "기존 mutation 경로 재사용" :564 항목 확장 + `## NEVER`)

**배경:** github SKILL(:240-261)과 init SKILL(Step 7a :389-424)에는 이미 "URL+code 표시 → 사용자 '승인했어' 신호 → **에이전트가** `--resume-last` 직접 실행, resume 명령을 사용자에게 출력 금지" 패턴이 있어요. migrate SKILL만 이 패턴을 참조하지 않아 드라이런에서 사용자에게 `!` 명령을 치라고 출력했어요.

- [ ] **Step 1: git connect 항목에 위임 블록 추가**

:564 "기존 mutation 경로 재사용" 항목의 `axhub apps git connect --execute` 문장 다음에 추가:

```markdown
   git 연결 중 GitHub 인증·설치가 필요해지면 `../github/SKILL.md` 의 OAuth device flow 섹션과 installation gate 패턴을 그대로 따라요. 요약:

   - **device flow (`device_code_issued` event):** verification URL + user_code 를 즉시 보여주고 이렇게만 안내해요 — "브라우저에서 승인한 다음 '승인했어' 라고 알려주세요. 제가 이어서 마무리할게요." 사용자가 승인 신호("승인했어" / "승인 완료" / "됐어")를 주면 에이전트가 같은 connect 를 `--execute --resume-last` 로 **직접** 이어받아요. resume 명령·`!` prefix 명령을 사용자에게 치라고 출력하지 않아요. outstanding code 가 있는 동안 `--resume-last` 없는 fresh `--execute` 재호출도 하지 않아요.
   - **installation 누락 (`not_in_installation`):** axhub GitHub App 이 대상 org/repo 에 설치되지 않은 상태예요. `axhub github accounts list --json` 의 `install_url` 을 보여주고 org 선택 + repo 접근 허용을 안내한 뒤 "설치했어" 신호를 기다려요. 신호를 받으면 accounts list 를 다시 읽어 해당 owner 의 `installed:true` 를 확인한 뒤에만 같은 connect 를 재시도해요. installation 이 여러 개로 모호하면 dry-run 결과를 나열하고 `--installation-id` 로 disambiguate 해요.
   - **D1 비대화형:** `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 면 브라우저 단계를 완료할 수 없으니 URL/코드와 재개 phrase("GitHub 연결 다시 해줘")를 남기고 멈춰요.
```

- [ ] **Step 2: NEVER 추가**

```markdown
- NEVER GitHub device flow resume 이나 `apps git connect` 같은 명령을 사용자에게 직접 실행하라고 떠넘기지 않아요 — 승인 신호만 받고 에이전트가 직접 마무리해요 (`../github/SKILL.md` 패턴).
```

- [ ] **Step 3: lint + skill-doctor 확인**

Run: `bun run skill:doctor --strict && bun run lint:tone --strict && bun run lint:keywords --check`
Expected: exit 0 / 0 err / no diff

- [ ] **Step 4: 커밋**

```bash
git add skills/migrate/SKILL.md
git commit -m "fix: migrate git 연결을 github SKILL device-flow·installation gate 패턴에 위임 — 사용자에게 명령 떠넘기기 제거"
```

---

### Task 7: migrate SKILL — stage 초안 drafts/ 경로 계약

**Files:**
- Modify: `skills/migrate/SKILL.md` (stage-write 스니펫 :405-441)

- [ ] **Step 1: 스니펫에 drafts 경로 도입**

:405 부근 stage-write 스니펫 묶음 앞에 추가:

```markdown
stage agent 산출물(markdown)은 `"$RUN_DIR/drafts/<stage>.md"` 에 먼저 저장하고, `stages/` 기록은 아래 helper 호출로만 해요. helper 는 `stages/` 내부 경로를 `--markdown-file` 로 받으면 거부하고, stage 별 고정 파일(`01-discover.md`…`05-reviewer.md`)을 멱등하게 overwrite 해요 — 같은 stage 를 다시 기록하면 revision 이 올라가고 파일은 늘지 않아요.

​```bash
RUN_DIR="$(dirname "$RUN_JSON")"
DRAFTS_DIR="$RUN_DIR/drafts"
mkdir -p "$DRAFTS_DIR"
​```
```

기존 스니펫의 `--markdown-file "$PLANNER_MD"` 류 변수 정의를 `PLANNER_MD="$DRAFTS_DIR/planner.md"` 형식으로 통일하고, critic/reviewer 호출에 `--verdict` 를 명시해요:

```bash
"$HELPER" migrate-stage-write \
  --run-json "$RUN_JSON" \
  --stage critic \
  --markdown-file "$DRAFTS_DIR/critic.md" \
  --verdict "$CRITIC_VERDICT" \
  --summary "critic verdict: $CRITIC_VERDICT" \
  --json
```

(reviewer 도 동일하게 `--verdict "$REVIEWER_VERDICT"`. verdict 값은 agent 산출물의 verdict 라인에서 추출 — approve|lgtm|iterate|block|request_changes|comment.)

- [ ] **Step 2: NEVER 추가**

```markdown
- NEVER `stages/` 아래 파일을 Write/Edit 로 직접 만들거나 고치지 않아요 — stage 기록·수정(redaction 포함)은 `migrate-stage-write` 재호출로만 해요.
```

- [ ] **Step 3: lint 확인 + 커밋**

Run: `bun run skill:doctor --strict && bun run lint:tone --strict`
Expected: exit 0 / 0 err

```bash
git add skills/migrate/SKILL.md
git commit -m "fix: migrate SKILL stage 초안을 drafts/ 로 격리 — stages/ 직접 쓰기 금지 계약"
```

---

### Task 8: stage agent 5종 — secret 값 기록 금지 rule

**Files:**
- Modify: `agents/axhub-migrate-discoverer.md`
- Modify: `agents/axhub-migrate-planner.md`
- Modify: `agents/axhub-migrate-architect.md`
- Modify: `agents/axhub-migrate-critic.md`
- Modify: `agents/axhub-migrate-reviewer.md`

- [ ] **Step 1: 각 파일 `Rules:` 목록에 한 줄 추가**

```markdown
- secret 후보(env 값, token, webhook URL)는 이름과 reason code 로만 기록해요. 값·값 일부(prefix, 마스킹된 조각 포함)는 어떤 산출물에도 적지 않아요.
```

(드라이런에서 discoverer 가 `a57b33...`, `xoxb-1...` prefix 를 evidence 로 적은 게 NEVER 위반의 시작이었어요. helper redact 는 backstop 이고 1차 방어는 agent rule.)

- [ ] **Step 2: 커밋**

```bash
git add agents/axhub-migrate-*.md
git commit -m "fix: migrate stage agent 에 secret 값 기록 금지 rule 추가"
```

---

### Task 9: contract 테스트 — SKILL 계약 drift 방지

**Files:**
- Modify: `tests/migrate-skill-contract.test.ts`

- [ ] **Step 1: 신규 describe 블록 추가**

파일 말미에 추가 (기존 `read()` helper 재사용):

```ts
describe("migrate SKILL flow hardening (2026-06-10 드라이런 회귀 방지)", () => {
  const skill = read("skills/migrate/SKILL.md");

  test("device flow 는 승인 신호 후 에이전트가 직접 resume 해요", () => {
    expect(skill).toContain("--resume-last");
    expect(skill).toContain("승인했어");
    expect(skill).toContain("제가 이어서 마무리할게요");
    expect(skill).toContain("../github/SKILL.md");
  });

  test("installation gate 가 명시돼 있어요", () => {
    expect(skill).toContain("not_in_installation");
    expect(skill).toContain("install_url");
    expect(skill).toContain("installed:true");
  });

  test("stage 기록은 drafts 초안 + helper 단일 경로예요", () => {
    expect(skill).toContain("drafts/");
    expect(skill).toMatch(/NEVER `stages\/` 아래 파일을 Write\/Edit 로 직접/);
  });

  test("mutation 은 migrate-approve 이후에만이에요", () => {
    expect(skill).toContain("migrate-approve --run-json");
    expect(skill).toMatch(/NEVER `migrate-approve` 성공 없이 mutation/);
  });

  test("plan_only hard-stop 과 push 는 불가침이에요", () => {
    expect(skill).toMatch(/NEVER plan_only hard-stop/);
    expect(skill).toContain("git filter-repo");
    expect(skill).toMatch(/NEVER 사용자 repo 에 `git push`/);
  });

  test("critic/reviewer stage-write 는 verdict 를 전달해요", () => {
    expect(skill).toContain("--verdict");
  });
});
```

- [ ] **Step 2: 테스트 실행**

Run: `bun test tests/migrate-skill-contract.test.ts`
Expected: PASS (Task 5·6·7 반영 후). Task 5-7 이전에 먼저 작성했다면 FAIL 이 정상 — SKILL 수정 후 green.

- [ ] **Step 3: 커밋**

```bash
git add tests/migrate-skill-contract.test.ts
git commit -m "test: migrate SKILL flow hardening 계약 assert 추가"
```

---

### Task 10: 전체 검증

- [ ] **Step 1: Rust 테스트**

Run: `cargo test -p axhub-helpers`
Expected: PASS

- [ ] **Step 2: Bun 테스트 + 타입체크**

Run: `bun test && bunx tsc --noEmit`
Expected: ≥498 pass / 0 fail (Phase 18 baseline + 신규), tsc clean

- [ ] **Step 3: SKILL 진단 3종**

Run: `bun run skill:doctor --strict && bun run lint:tone --strict && bun run lint:keywords --check`
Expected: 모두 exit 0

- [ ] **Step 4: 수동 시나리오 점검 (선택, 권장)**

임시 디렉터리에서 full_consensus run scaffold 를 만들고 stage-write 를 순서대로/역순으로 호출해 (a) 고정 파일명 5개 유지, (b) critic iterate 후 reviewer 거부, (c) seal 거부 메시지를 눈으로 확인해요.

- [ ] **Step 5: 릴리즈 여부 결정**

helper binary 변경이라 plugin 배포에 반영하려면 CLAUDE.md 의 2단계 release flow (`bun run release` → narrative amend → `bun run release:tag`) 가 필요해요 — 사용자 지시 시 진행.

---

## 정직 섹션 (한계)

1. **자유 텍스트 hex prefix 는 regex 로 못 막아요.** `a57b33...` 같은 무표식 조각은 패턴이 없어요. 방어선: agent rule(Task 8) → redact backstop 은 알려진 포맷(Slack/GH/AWS/OpenAI/Bearer/webhook)만 → 최종은 reviewer stage 의 secret 점검 항목.
2. **mutation 레벨 hard gate 는 불가능해요.** `axhub apps create` 등은 외부 CLI 라 helper 가 가로챌 수 없어요. stage/seal 레벨은 helper 가 hard 강제(Task 3·4), mutation 레벨은 SKILL 문구 + contract 테스트(Task 5·9) 가 최대치예요. 잔여 리스크: 모델이 SKILL 을 무시하면 여전히 뚫림 — 단, 이번 드라이런의 돌파 지점(승인 발화 해석, iterate 직행)은 helper 게이트가 직접 차단해요.
3. **wave(parallel) 경로 미검증.** `stages/05-reviewer-a.md` 류 wave write-target 은 이름 충돌이 없어 호환으로 판단했지만, Task 10 의 `cargo test` 가 wave 테스트를 함께 돌려 확인해요. 깨지면 wave write-target 경로도 drafts 계약에 맞춰 후속 수정.
4. **기존 진행 중 run 과의 호환.** 고정 ordinal 은 새 run 부터 적용돼요. incremental ordinal 로 생성된 과거 run 에 stage-write 를 이어 부르면 고정 경로로 새 파일이 생겨요 — 과거 run 재개는 지원하지 않고 새 run 시작을 안내 (SKILL 에 이미 run 단위 격리 존재).
