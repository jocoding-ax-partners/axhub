# Phase C — 5 기존 SKILL Polish

> 신규 SKILL 안 만들고, 기존 SKILL 안에서 subcommand 확장만. 위험 작아요.

---

## C-1. `apps` SKILL polish

**파일**: `skills/apps/SKILL.md`

**현재**: list 만 (read-only).

**추가**:

### apps create
- AskUserQuestion: `--interactive` vs `--from-file` 분기
- interactive: 필드 dynamic ask (name/framework/runtime.port/start_command)
- from-file: yaml path 입력
- consent-mint action="apps_create"
- axhub apps create [--interactive | --from-file <path>] --json

### apps delete (destructive)
- **STEP 1**: 무조건 `--dry-run` 먼저 호출
  - axhub apps delete <slug|id> --dry-run --json
  - 결과 미리보기: "삭제 대상 = paydrop (id=42), 마지막 배포 12분 전, 환경변수 5개"
- AskUserQuestion: "정말로 삭제할까요? slug 정확히 입력"
- confirm match → consent-mint action="apps_delete"
- axhub apps delete <slug|id> --yes --json

### apps update (dynamic field)
- 사용자 의도 NL 분석 → AskUserQuestion: "어떤 필드 변경?"
  - options 동적: name / framework / runtime.port / health_check / start_command / Other
- 선택 후 AskUserQuestion: "새 값?"
- consent-mint action="apps_update" context={field, value}
- axhub apps update <slug|id> --field <key>=<value> --json
- multi-field 변경 시 반복

### apps get
- read-only
- axhub apps get <slug|id> --json
- Korean rendering

### apps open delegation
- 사용자가 "이 앱 열어" 시 → top-level `open` SKILL 으로 위임 안내
- "axhub apps open 대신 'axhub open <slug>' 쓸게요. open SKILL 으로 갑니다."

**effort**: ~30분

---

## C-2. `apis` SKILL polish

**파일**: `skills/apis/SKILL.md`

**현재**: list 만.

**추가**:

### apis schema (read-only)
- axhub apis schema <endpoint-id> --json
- Korean rendering JSON schema

### apis test (read-only)
- axhub apis test <endpoint-id> --method GET --header ... --json
- 결과 narration

### apis call (write — full consent gate, deploy 동일)
- **codex ENG finding F10**: read-only polish 아님. write scope. consent gate 필수.
- AskUserQuestion preview card:
  ```
  다음 API 를 호출할게요:
  ① endpoint: <id>
  ② method:   POST
  ③ payload:  { ... }   ← AskUserQuestion 으로 입력
  ④ side effect: 백엔드 mutation 가능
  ⑤ 예상:     ~2초
  
  진행할까요? [네 / 아니요 / 미리보기만 (--dry-run)]
  ```
- consent-mint action="apis_call" context={endpoint_id, method, body_hash}
- 4-dim preview schema 확장: payload + side_effect + auth_scope + idempotency 추가
- axhub apis call <endpoint-id> --method <X> --body-file <path> --json

**effort**: ~30분

---

## C-3. `deploy` SKILL polish

**파일**: `skills/deploy/SKILL.md`

**현재**: deploy create + status (이미 있음).

**추가**:

### deploy cancel (destructive)
- AskUserQuestion preview: "진행 중 배포 cancel — app=paydrop, deployment=dep_xxx"
- consent-mint action="deploy_cancel" context={app_id, deployment_id}
- axhub deploy cancel <deployment-id> --app <X> --yes --json
- post-cancel status 확인

### deploy list (pagination)
- read-only
- axhub deploy list --app <X> --page-size 10 --json
- 사용자 "더 보기" 시 --page 다음 페이지
- AskUserQuestion 개별 deployment 선택 시 deploy status 라우팅

**effort**: ~20분

---

## C-4. `doctor` SKILL polish

**파일**: `skills/doctor/SKILL.md`

**현재**: 5-row 진단 (helper / CLI / 인증 / profile / endpoint).

**추가**:

### doctor audit subdiagnostic
- 의도 분기: 일반 doctor 결과 끝에 "agent observability stack 도 점검할까요?"
- 동의 시 axhub doctor audit --json
- 4-row 추가:
  - migration_applied
  - endpoint_reachable
  - role (admin/member/unknown)
  - export_permission
- 결과 표 합쳐서 출력

**effort**: ~15분

---

## C-5. `update` SKILL polish

**파일**: `skills/update/SKILL.md`

**현재**: axhub CLI 업데이트 (apply/check).

**추가**:

### cosign WARN→ENFORCE 사전 알림
- 현재 v0.9.1+ 가 WARN mode (검증 fail 시 stderr 알림 후 설치 진행)
- v0.9.2 부터 ENFORCE mode 예정 (검증 fail 시 설치 중단)
- preflight 시 CLI 가 v0.9.2 이상으로 업데이트 가능하면:
  - "v0.9.2+ 부터 cosign 서명 검증 강제예요. 회사 IT 가 sigstore.dev 차단 시 install 막힐 수 있어요. AXHUB_ALLOW_UNSIGNED=1 환경변수로 우회 가능 (비추)."
- AskUserQuestion: "그래도 업데이트할까요?"

**effort**: ~10분

---

## Phase C 총 effort

- 5 SKILL × 평균 ~20분 = ~1.5시간

## Validation gate before Phase D

- [ ] 5 polish SKILL `bun run skill:doctor --strict` exit 0
- [ ] apps delete 의 `--dry-run first` 패턴 검증 (E2E case)
- [ ] apis call 의 consent-mint cycle 검증 (mint → bash run → verify)
- [ ] deploy cancel consent gate 검증
- [ ] doctor audit subdiagnostic E2E (mock-hub agent_audit endpoint)
