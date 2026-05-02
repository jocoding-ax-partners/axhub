# Phase B — 7 신규 SKILL

> Phase A0 의 helper foundation 위에 SKILL = NL wrap. 모든 install/fetch/mutation logic = helper Rust subcommand 호출.

---

## SKILL 작성 규칙 (project CLAUDE.md 강제)

모든 신규 SKILL = `bun run skill:new <slug>` scaffold 사용. 직접 작성 금지.

frontmatter `multi-step` + `needs-preflight` 명시. AskUserQuestion 마다 `tests/fixtures/ask-defaults/registry.json` channel 등록. 한국어 = 해요체. nl-lexicon trigger 어구 = `description:` 에만.

---

## B-1. `init` SKILL (DX v4 — zero-install all-in-one)

**파일**: `skills/init/SKILL.md`

**frontmatter**:
```yaml
---
name: init
description: 이 스킬은 사용자가 새 axhub 앱을 만들고 싶어할 때 사용합니다. 다음 표현에서 활성화: "새 앱 만들어줘", "결제 앱 만들어줘", "아이디어 있는데 시작해보자", "프로젝트 만들어", "init", "scaffold", "axhub.yaml 만들어줘", "Next.js 앱 만들어줘", "FastAPI 앱 만들어줘", 또는 빈 디렉토리에서 새 앱 시작 의도. 기술 스택 선택, 모든 의존성 (node, ax-hub-cli) 자동 설치, examples repo 템플릿 복제, npm install 자동까지 한 번에 처리해요.
multi-step: true
needs-preflight: false
---
```

**workflow**:

```
Step 0  TodoWrite checklist (7 step 진행 시각화)

Step 1  preflight (auth 무관 — init 가 cold customer 진입로)
        helper preflight --no-auth-check --json

Step 2  AskUserQuestion: stack 선택
        helper list-templates --json
        → preview card: "어떤 기술 스택으로 만들까요?"
          [Next.js / FastAPI / Django / Go / React+Vite]
          options 에 stack description + min_node 표시

Step 3  helper bootstrap --stack <slug>
        - node detect → 없으면 volta install (sudo ask)
          AskUserQuestion: "node 설치할까요? (sudo 권한 필요할 수 있어요)"
          consent-mint action="bootstrap_install_node"
        - ax-hub-cli detect → 없으면 install
        - progress narration 30초 마다

Step 4  helper fetch-template <slug>
        - examples repo tarball download (codeload)
        - cwd 에 extract
        - axhub.yaml 자동 생성 (ax-hub-cli init agent path 활용)

Step 5  AskUserQuestion: "의존성 설치할까요? (npm install 등)"
        consent-mint action="install_deps"
        helper install-deps --manifest auto

Step 6  AskUserQuestion plain: "axhub apps create 도 할까요?" (E9 결정)
        - 사용자 선택 시 apps create chain (별도 turn)

Step 7  AskUserQuestion plain: "GitHub repo 연결할까요?"
        - 사용자 선택 시 github connect chain (별도 turn)
```

**non-interactive guard (D1)**: subprocess (`claude -p`, CI) 시 모든 AskUserQuestion 건너뛰고 `--detect-only` mode 로 framework detect 결과만 출력. 실제 install/scaffold = abort.

**registry**:
```json
"init": {
  "어떤 기술 스택으로 만들까요?": {
    "safe_default": "abort",
    "rationale": "Stack 선택 = 사용자 의도. subprocess 자동 선택 위험 (잘못된 stack)."
  },
  "node 설치할까요?": {
    "safe_default": "abort",
    "rationale": "sudo + system mutation. 명시 동의 없으면 안 함."
  },
  "의존성 설치할까요?": {
    "safe_default": "abort",
    "rationale": "npm install = network + disk write. subprocess 자동 X."
  },
  "axhub apps create 도 할까요?": {
    "safe_default": "abort",
    "rationale": "backend mutation. 별도 ask + 사용자 explicit."
  },
  "GitHub repo 연결할까요?": {
    "safe_default": "abort",
    "rationale": "github connect = backend webhook 생성. 별도 ask."
  }
}
```

**effort**: ~1.5시간 (helper subcommand 다 있어서 wrap 만)

---

## B-2. `env` SKILL

**파일**: `skills/env/SKILL.md`

**frontmatter**:
```yaml
---
name: env
description: 이 스킬은 사용자가 axhub 앱의 환경변수를 보거나 추가하거나 삭제하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "환경변수 뭐 있어", "환경변수 추가", "환경변수 추가해", "DB URL 추가", "API 키 등록", "secret 추가", "환경 변수 확인", "env 봐", "env 추가", "env 삭제", 또는 axhub 앱의 env var 조회/변경 의도. set 은 --from-stdin 으로만 받아서 argv 노출 방지해요.
multi-step: true
needs-preflight: true
---
```

**workflow**:

```
Step 0  TodoWrite

Step 1  preflight + resolve (current_app 확인)

Step 2  의도 분기 AskUserQuestion: "list/get/set/delete 중 뭐 할까요?"
        (또는 "환경변수 추가해" → set 자동, "뭐 있어" → list 자동)

Step 3a (list/get):
        axhub env list --app <id> --json
        Korean rendering, 값은 마스킹 (KEY=v***)

Step 3b (set):
        AskUserQuestion: "키 이름?"
        AskUserQuestion: "값?" (입력 후 즉시 모자이크 'v***' 표시)
        consent-mint action="env_set" context={app_id, key}
        echo "$VALUE" | axhub env set <KEY> --app <id> --from-stdin --json
        # ★ argv 에 value 절대 안 들어감
        검증: ps aux 시 value 평문 노출 X

Step 3c (delete):
        AskUserQuestion preview: "삭제할 KEY = ___, prod = force+confirm 필요"
        prod 환경 시:
          AskUserQuestion: "키 이름 정확히 입력하세요" → confirm match
        consent-mint action="env_delete" context={app_id, key}
        axhub env delete <KEY> --app <id> --force --confirm=<KEY> --json

Step 4  결과 narration + 다음 액션 안내
```

**SECURITY MUST**:
- value 가 argv 에 절대 없음 (--from-stdin pipe 강제)
- helper telemetry funnel event 시 value 절대 X (KEY 만)
- terminal scrollback 에 입력 후 즉시 마스킹 (1자리도 노출 X)
- debug log 활성 시도 value redact

**effort**: ~1.5시간

---

## B-3. `github` SKILL

**파일**: `skills/github/SKILL.md`

**frontmatter**:
```yaml
---
name: github
description: 이 스킬은 사용자가 axhub 앱과 GitHub repo 를 연결하거나 끊고 싶어할 때 사용합니다. 다음 표현에서 활성화: "GitHub 연결", "repo 연결", "GitHub repo 연결해", "내 repo 붙여", "git 연결", "repo 끊어", "github disconnect", 또는 GitHub 연동 의도. AppHub GitHub App 사전 설치 안 됐으면 install URL 안내해요.
multi-step: true
needs-preflight: true
---
```

**workflow**:

```
Step 0  TodoWrite

Step 1  preflight + resolve (current_app)

Step 2  AskUserQuestion: "connect / disconnect / repos list 중 뭐 할까요?"

Step 3a (connect):
        axhub github repos list --json
        - install_not_found (exit 67) → AppHub install URL 안내:
          "https://github.com/apps/axhub/installations/new 먼저 install 해주세요. 끝나면 '다 됐어' 라고 말씀해주세요."
        - account 다중 시 AskUserQuestion 선택
        - repo AskUserQuestion 선택
        - branch 입력 (default = main)
        consent-mint action="github_connect" context={app_id, repo, branch}
        axhub github connect <slug> --account <X> --repo <Y> --branch <Z> --json
        - already_exists (exit 64) → "이미 연결됨, disconnect 후 재연결?" ask

Step 3b (disconnect — destructive):
        AskUserQuestion preview: "정말로 <repo> 와 연결을 끊을까요? webhook 삭제됨, 복원 어려움"
        AskUserQuestion: "앱 슬러그 정확히 입력" → confirm match
        consent-mint action="github_disconnect" context={app_id, slug}
        axhub github disconnect <slug> --force --confirm=<slug> --json

Step 3c (repos list — read-only):
        axhub github repos list --json
        Korean rendering: "조직 / repo 목록"
```

**effort**: ~1.5시간

---

## B-4. `open` SKILL

**파일**: `skills/open/SKILL.md`

**frontmatter**:
```yaml
---
name: open
description: 이 스킬은 사용자가 배포된 axhub 앱을 브라우저에서 열어보고 싶어할 때 사용합니다. 다음 표현에서 활성화: "결과 봐", "라이브 봐", "브라우저로 열어", "프로덕션 열어", "deploy URL 봐", "open", "open in browser", "metrics 봐", "logs 페이지", 또는 배포 결과 확인 의도. 현재 디렉토리의 axhub.yaml 자동 감지해요.
multi-step: false
needs-preflight: false
---
```

**workflow**:

```
Step 1  axhub open [slug-or-id] [--logs|--metrics] --json
        - no axhub.yaml in cwd (exit 64) → "init 안내"
        - app deleted (exit 67) → apps list 라우팅
Step 2  결과 narration ("브라우저 열었어요")
```

**effort**: ~30분

---

## B-5. `whatsnew` SKILL

**파일**: `skills/whatsnew/SKILL.md`

**frontmatter**:
```yaml
---
name: whatsnew
description: 이 스킬은 사용자가 axhub 의 최신 변경사항을 알고 싶어할 때 사용합니다. 다음 표현에서 활성화: "axhub 뭐 새로 나왔어", "release notes", "changelog", "신규 기능", "v0.10 뭐 바뀌었어", "whatsnew", "최신 변경", 또는 release highlights 조회 의도.
multi-step: false
needs-preflight: false
---
```

**workflow**:

```
Step 1  axhub whatsnew --json
Step 2  Korean+English text 그대로 노출 (CLI 가 이미 Korean 포함)
```

**effort**: ~30분

---

## B-6. `profile` SKILL

**파일**: `skills/profile/SKILL.md`

**frontmatter**:
```yaml
---
name: profile
description: 이 스킬은 사용자가 axhub backend endpoint (회사별/환경별) 를 보거나 바꾸고 싶어할 때 사용합니다. 다음 표현에서 활성화: "현재 endpoint 뭐야", "회사 endpoint 바꿔", "profile 바꿔", "endpoint 바꿔", "다른 회사로", "profile list", "profile use", 또는 multi-tenant endpoint switching 의도. add 는 *.jocodingax.ai 도메인 allowlist 만 허용해요.
multi-step: true
needs-preflight: false
---
```

**workflow**:

```
Step 1  preflight (auth 무관)

Step 2  AskUserQuestion: "list / current / use / add 중 뭐?"

Step 3a (list/current — read-only):
        axhub profile list --json / current --json

Step 3b (use):
        AskUserQuestion: profile name 선택 (current list 에서)
        consent-mint action="profile_use" context={profile_name}
        axhub profile use <name>
        - name_not_found (exit 67) → list 라우팅

Step 3c (add — endpoint allowlist gate):
        AskUserQuestion: "profile name?"
        AskUserQuestion: "endpoint URL?"
        # PLUGIN-SIDE allowlist check
        if endpoint domain ∉ ["*.jocodingax.ai", "localhost"]:
          AskUserQuestion warn: "사내 도메인 아닌데 진행?"
        consent-mint action="profile_add" context={name, endpoint}
        axhub profile add <name> --endpoint <URL> --json
```

**effort**: ~1시간

---

## B-7. `admin` SKILL (DX Codex F7)

**파일**: `skills/admin/SKILL.md`

**frontmatter**:
```yaml
---
name: admin
description: 이 스킬은 회사 admin 사용자가 vibe coder 가 진입할 axhub team / member / token / sandbox app 을 자동 셋업하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "팀 셋업", "신규 직원 axhub 추가", "vibe coder 등록", "admin onboarding", "회사 axhub 셋업", "team 만들어", "member 추가", 또는 admin 권한 backend onboarding 의도. admin RBAC scope 필요해요.
multi-step: true
needs-preflight: true
---
```

**workflow**:

```
Step 0  TodoWrite

Step 1  preflight + admin scope 검증
        - scope 에 'admin' 없으면 → "admin 권한 필요. IT 에 요청" 안내 + abort

Step 2  AskUserQuestion: "team 만들기 / member 추가 / sandbox app 만들기 / token 발급 중?"

Step 3a (team 만들기):
        AskUserQuestion: "team 이름?"
        AskUserQuestion preview 5필드 (이름 / endpoint / region / SSO 연동 / 결제)
        consent-mint action="admin_setup_team"
        axhub teams create --name <X> --json (CLI 신규 endpoint 가정 — 별도 design pass)

Step 3b (member 추가):
        AskUserQuestion: "이메일?"
        consent-mint action="admin_setup_team" context={email, role}
        axhub teams members add --email <X> --role member --json

Step 3c (sandbox app):
        helper resolve sandbox preset
        axhub apps create --from-file sandbox-preset.yaml --json

Step 3d (token 발급):
        scope ask + TTL ask
        consent-mint action="admin_setup_team" context={scope, ttl}
        axhub auth tokens create --scope <X> --ttl <Y> --json
```

**WARNING**: admin SKILL 은 **별도 design pass** 필요해요. 이 SKILL.md = skeleton. 본 design pass:
- `axhub teams create` / `axhub teams members add` / `axhub auth tokens create` 명령어가 ax-hub-cli 에 있는지 검증 (없으면 sibling repo PR 또는 별도 admin API 통한 호출)
- admin RBAC scope JWT 발급 path
- multi-tenant team isolation
- audit log 통합

별도 design pass 권장 시기: Phase A0 완료 후 Phase B 진입 전.

**effort**: ~2시간 (skeleton SKILL 만, design pass 별도)

---

## Phase B 총 effort

- 7 신규 SKILL × 평균 1.2시간 = ~8.5시간
- (admin 의 design pass 별도 ~3시간 사전)
- **Total Phase B: ~6시간 (admin skeleton 만, design pass 후속)**

## Validation gate before Phase B-test

- [ ] 7 SKILL `bun run skill:doctor --strict` exit 0
- [ ] 7 SKILL frontmatter 의 nl-lexicon trigger 가 `lint:keywords --check` baseline 통과
- [ ] 7 SKILL 의 AskUserQuestion 마다 registry channel 등록
- [ ] `bun run lint:tone --strict` 0 err (해요체 강제)
- [ ] admin SKILL = skeleton 임을 plan/PR 에 명시 + design pass 일정 약속
