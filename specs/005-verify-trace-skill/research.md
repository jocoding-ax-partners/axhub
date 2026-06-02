# Phase 0 Research: Trace evidence-source 재설계

**Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

소스 확인(ax-hub-cli + axhub-helpers)으로 모든 NEEDS CLARIFICATION 이 해소됐어요. 핵심 결정:

---

## D1 — Evidence source = 런타임 로그 (γ)

- **Decision**: trace 를 **런타임 로그 기반**으로 재정의. `axhub deploy logs` (app-level 런타임 NDJSON)의 `message` 를 매칭. 빌드 단계 실패는 로컬 `event_log.failure_reason` 으로 안내.
- **Rationale**: 소스 확정 — `axhub-api/src/deploy.rs` 에 로그 라우트는 `list_app_logs`(`GET /api/v1/apps/{app}/logs`) 하나뿐, build-log 엔드포인트 부재. `commands/deploy/logs.rs` 는 `--source`·deploy-id 무시. 즉 build-log 는 가져올 방법이 없음. 사용자가 γ 로 결정(2026-06-02).
- **Alternatives**:
  - α 로컬 event_log reason 매칭 — CLI 비의존이나 reason 변별력 불확실(D5).
  - β deployment status API(`get_deployment` stage/reason) — 네트워크 의존, 빌드 단계만 커버.
  - δ backend build-log API 대기 — 로드맵에 없으면 무기한 degrade.
  - → γ 선택: 런타임 실패를 실시간 추적하고 빌드 실패는 event_log 로 보완하는 게 현행 backend 에서 가장 실효적.

## D2 — helper probe NDJSON 파싱

- **Decision**: `RealTraceProbes::axhub_build_log` (→ 의미상 runtime-log probe)이 `axhub --json deploy logs --app <app> [--limit N]` 출력(NDJSON, line 당 `{type,source,ts,container,insert_id,message}`)을 받아 **각 라인을 파싱해 `message` 만 추출**해서 trace_helper 에 전달. deploy-id/`--source build` 인자는 현행 CLI 가 무시하므로 **제거**(또는 호환용 유지하되 의존 안 함).
- **Rationale**: 현재는 raw stdout(NDJSON 통째)을 `extract_error_lines` 에 먹여서 (a) 사용자에게 raw JSON 노출(NEVER 위반 소지), (b) needle 이 JSON escape 와 충돌. `AppLogLine` 구조(`message: String`)가 확정돼 파싱 대상 명확.
- **Edge**: 파싱 실패 라인은 `serde_json` 에러를 **panic 없이** skip + `runtime_log_parse_warning` 누적. 빈 응답(앱 미기동)은 `build_log_probe_unavailable`(→ `runtime_log_unavailable`) warning + generic fallback.
- **Alternatives**: jq-식 외부 파싱(불필요), 라인 grep(취약) — 기각.

## D3 — 매칭과 표시(display) 분리 (NEVER 규칙 보존)

- **Decision**: `matched_patterns` 는 **파싱된 전체 message 집합**에 대해 `match_error_patterns` 실행(5-라인 display 필터와 분리). `build_log_errors`(표시)는 기존대로 ERROR/FATAL/WARN-태그 message 중 **max 5** 유지.
- **Rationale**: F2 의 reachability 결함은 "매칭이 추출된 5라인만 본다"는 점. 매칭을 전체 message 로 옮기면 태그 없는 라인(`env: …`, `npm ERR! …`)도 매칭됨. 동시에 display 는 ERROR/WARN max5 유지 → SKILL NEVER 규칙("raw stderr 미노출, ERROR/WARN max 5") 보존. (advisor 지적 반영: display 를 넓히지 않음.)
- **Alternatives**: `extract_error_lines` 자체를 union 으로 확장(태그 OR needle) — display 에 비태그 라인이 새어 NEVER 위반 → 기각. (만약 매칭된 라인을 사용자에게 보여주려면 SKILL NEVER 규칙 자체를 바꾸는 별도 결정 필요 — 이 plan 범위 밖.)

## D4 — needle 정밀화 (false-positive 차단)

- **Decision**: 전체 로그 매칭으로 오탐 표면이 커지므로 substring needle 강화:
  - `oom` (3자) → 단어 경계/`oomkilled`·` oom `·`out of memory` 로 한정 (`room`/`zoom`/`bedroom`/`zoom-sdk` 오탐 차단).
  - bare `exit code 1` → `exit code 1` 이 `exit code 127` 의 접두로 오탐 가능 — 경계 처리(뒤가 숫자면 제외) 또는 정확 매칭.
  - `env: ` 등 나머지는 충분히 구체적이라 유지, 단 리뷰.
- **Rationale**: 런타임 로그는 컴파일 출력/경로/패키지명이 섞여 substring 충돌이 잦음.
- **Test**: 오탐 회귀 — `"zoom meeting"`/`"src/room/"` 라인이 `oom` 매칭 **안 됨**을 assert.

## D5 — 빌드 단계 실패 fallback (event_log reason)

- **Decision**: `last_phase == Failed` 이고 런타임 로그가 비면(빌드 단계 추정) `event_log.failure_reason` 을 4-part empathy 입력으로 사용. 가능하면 reason 에도 `match_error_patterns` 적용.
- **Rationale**: 빌드 실패 시 앱 미기동 → 런타임 로그 empty. event_log 는 로컬·CLI 비의존이라 항상 가용.
- **Open (이 spec 범위 밖, 후속 권고)**: reason 상세도. e2e fixture 는 `"build command failed"`(coarse). deploy hook 의 reason 작성부가 상세 에러를 기록하면 빌드-단계 8패턴 변별력↑. 별도 작업으로 분리.

## D6 — SKILL/catalog 문구 재정렬 (R3 문서)

- **Decision**: SKILL.md 의 "3-source (event_log + **build_log** + audit)" → "event_log + **runtime_log** + audit" 로 갱신, Step 2 의 `--source build` 전제 제거, 빌드-단계는 event_log reason 안내로 명시. `references/error-patterns.md` 는 런타임 적용 가능 패턴(oom/module_not_found/port_already_in_use/env_not_found/network_timeout) 중심으로 유지, 순수 빌드-타임 패턴(dependency_install_failed/docker_image_pull_failed)은 event_log reason 경로로 라벨.
- **Rationale**: SKILL 계약 문구가 현행 evidence 와 일치해야 drift 재발 방지. `lint:tone`/`lint:keywords`/`skill:doctor` 통과 필수.
- **Constraint**: `description:` 의 nl-lexicon trigger 어구는 **불변**(keyword baseline lock).

## D7 — R1 cosmetic 라벨 (병행)

- **Decision**: SKILL D1 guard 단락의 `trace_target_selection` 라벨을 실제 매칭 기준(registry `trace` 채널 + question text)으로 명확화.
- **Rationale**: repo 어디에도 없는 설명용 라벨(grep 0건) — 독자 오해 방지. 1줄.

---

**Output**: 모든 NEEDS CLARIFICATION 해소. Phase 1(data-model/contracts/quickstart) 진행 가능.
