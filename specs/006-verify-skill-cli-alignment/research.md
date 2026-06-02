# Phase 0 Research: verify 스킬 ↔ ax-hub-cli v0.17.2 + axhub-helpers

전체 audit (Clarifications Q2). 원천: live `axhub deploy {status,list,logs} --help`, `axhub/src/commands/deploy/logs.rs`, `crates/axhub-api/src/deploy.rs`, `crates/axhub-helpers/src/verify_helper.rs`, `cli/args/mod.rs`, `skills/recover/SKILL.md`.

---

## D1. helper verify 출력 (VerifyResult)

- **Decision**: 스킬은 `axhub-helpers verify` 출력을 `VerifyResult` 스키마로 다뤄요 — `verdict` ∈ {`live`,`suspect`,`not_live`}(snake_case), `state`, `last_deploy_id`, `last_deploy_age_secs`, `errors`, `reasons`. verdict 3값을 ✅/⚠️/❌ 로 매핑하고 `reasons` 를 verdict 아래 그대로 출력.
- **Rationale**: `verify_helper.rs:19-37` 의 `Verdict` enum + `VerifyResult` struct.
- **Alternatives**: (기각) 스킬 CI 예시 `{"verdict":"passed"}` — `"passed"` 는 없는 값. `"live"` 로 교체.

## D2. deploy status `.status` = 백엔드 free string (닫힌 enum 아님)

- **Decision**: `.status` 를 닫힌 enum 으로 매칭하지 않아요. live 판정 = helper `LIVE_STATES` (`live`/`running`/`deployed`/`active`/`ok`/`succeeded`) 미러, 그 외는 미라이브(진행중/실패)로 휴리스틱 분류. `.current_stage` 로 단계 안내.
- **Rationale**: `axhub-api/src/deploy.rs` 의 `pub status: Option<String>` / `String` — 백엔드 공급 free string (테스트 fixture: `failed`/`succeeded`). 타입 enum 없음.
- **Alternatives**: (기각) 스킬의 `pending/building/deploying/stopped` 를 "CLI enum" 처럼 적은 것 — 그런 enum 은 없음. 휴리스틱 라벨로 재서술 + `ok` 포함한 LIVE_STATES 와 일치시킴.

## D3. deploy logs = app-level (deployment_id legacy, --source passthrough)

- **Decision**: 로그는 `axhub deploy logs --app <APP> --json` (app-level) 로 받아요. `<DEPLOYMENT_ID>` 는 legacy(스코핑에 안 씀). `--source` 는 free passthrough 문자열이라 "pod" 가정 제거 — source 없이(또는 명시적 label 만) app-level 로그를 받고 client-side 마지막 ~50줄 trim 후 ERROR/FATAL grep.
- **Rationale**: `logs.rs:19-20` "Legacy deployment id. Runtime logs are now scoped to the app-level backend logs route." + 구현이 `deploy::list_app_logs(client, app, query)` (app 스코프). `--source <value>` 는 `Option<String>` free string, pod/runtime/build enum 부재(grep 무결과).
- **Alternatives**: (기각) 스킬의 `deploy logs <DEPLOY_ID> --app --source pod` per-deploy pod-log 모델 — deployment_id 스코핑 + `pod` 고정값은 stale. `--follow` 는 비-TTY 단일 스냅샷 degrade (logs.rs 의 bounded streaming) — 유지하되 verify 는 단발 스냅샷만.

## D4. helper verify / list-deployments 인자

- **Decision**: helper `verify` / `list-deployments` 는 **primary `--app-id`, alias `--app`**. 둘 다 수용. 본문 설명을 이 관계로 정정(스킬은 거꾸로 적음). `list-deployments` 는 `--limit` 지원.
- **Rationale**: `cli/args/mod.rs:107,196` `#[arg(long = "app-id", visible_alias = "app")]`.
- **Alternatives**: (기각) 스킬 "`--app-id 도 alias`" — 반대. `--app` 이 alias.

## D5. deploy status / list surface

- **Decision**: `axhub deploy status [DEPLOYMENT_ID] --app <APP> --json` (5s timeout) — positional id + `--app` + `--json` (+`--watch`/`--watch-interval` 미사용). `axhub deploy list --app <APP> --json` — **`--limit` 없음**(최신은 출력에서 client-side 선택).
- **Rationale**: live `deploy status --help` / `deploy list --help`.
- **Alternatives**: (확인) 스킬이 `deploy list` 에 --limit 안 붙임 — OK. list-deployments(helper)만 --limit.

## D6. recover error_code 표 (참조 정본)

- **Decision**: helper error_code 분기는 `skills/recover/SKILL.md` §canonical 표를 cross-link — 정정 불요. 코드: `auth.token_invalid`(65), `resource.app_not_found`(67), `transport.timeout`/`transport.cli_missing`/`response.invalid_json`/`response.error_envelope_unknown_shape`(1). `auth_error_code`(preflight): `cli_not_found`/`cli_unavailable`→install-cli, `cli_config_corrupted`→auth, `cli_too_old`→upgrade.
- **Rationale**: recover SKILL.md:140-150 이 정본("이 SKILL 이 정본; 다른 SKILL 은 cross-link"). 현행.
- **Alternatives**: (기각) verify 에 표 복제 — drift 위험. cross-link 유지.

## D7. 보존 대상 (변경 금지)

- **Decision**: frontmatter `description:`(트리거 byte-lock), in-body CANONICAL_PREFLIGHT_BLOCK(needs-preflight:true), D1 비대화형 가드, TodoWrite Step 0, health_endpoint AskUserQuestion(`health_endpoint_setup` safe_default=skip) 보존.
- **Rationale**: CLAUDE.md skill-authoring 계약 + `ux-ask-fallback-registry` test. verify 의 preflight/AskUserQuestion 은 CLI 계약과 무관한 skill UX 계약.

## D8. CLI 버그 없음 → skill-only

- **Decision**: audit 에서 status/logs/verify 의 불일치는 전부 **스킬 문서가 stale** 한 것(CLI/helper 는 현행 올바름). 따라서 Rust(verify_helper.rs)·ax-hub-cli 변경 0. SKILL.md rewrite 만.
- **Rationale**: helper VerifyResult 는 이미 옳은 verdict emit, status=free-string 은 설계, logs app-level 은 의도된 라우트 변경. 스킬만 안 따라옴.
- **남은 미확인**: `--source` 의 백엔드 인식값(pod 무효 여부)은 live 호출로만 100% 확정 — quickstart 에서 `deploy logs --app --json` 단발 호출로 spot-check (단, source 없이도 app-level 로그가 와서 비차단).

---

## Gap 요약 (현 스킬 → v0.17.2 실제)

| 항목 | 현 스킬 | 실제 | 조치 |
|---|---|---|---|
| helper verdict | `"passed"` | `live`/`suspect`/`not_live` | 교체 + reasons/last_deploy_id 활용 |
| status 분기 | enum 가정(pending/building/...) | free string + LIVE_STATES 휴리스틱 | LIVE_STATES 미러(+ok), 그외=미라이브 |
| logs | `<DEPLOY_ID> --source pod` per-deploy | app-level `--app`, source passthrough | `--app` app-level + pod 가정 제거 |
| --app-id/--app | "--app-id 도 alias" | --app-id primary, --app alias | 정정 |
| deploy list | (--limit 안 씀) | --limit 없음 ✓ | 유지 |
| error_code | recover 표 참조 | 현행 ✓ | 유지 |

## 검증 가능성

`~/.axhub/bin/axhub` v0.17.2 + repo `axhub-helpers` 빌드로 live 대조 가능. `deploy status/logs/list --help` + `axhub-helpers verify --help` + verify_helper.rs 소스.
