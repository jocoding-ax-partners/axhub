# Codex 플러그인 호환 연구

> 2026-08-19 · dynamic workflow 실측 연구 — 병렬 조사 5 lane(격리 샌드박스 실설치 · openai/codex 소스 정독 · repo 문안 전수 인벤토리 · 빌드 파이프라인 · 생태계 선례 포렌식) + 설계 종합 + 적대 검증. 에이전트 7개, 도구 호출 277회.
>
> 검증 기준선: **Codex CLI 0.147.0** (로컬 설치본 실측 + `openai/codex` rust-v0.147.0 태그 소스) × **axhub plugin v1.20.4**.
>
> 신뢰 규약: 주요 주장에 `[confirmed]`(실측 또는 소스 파일:라인 확보) / `[likely]`(실측 기반 산술 추정) / `[unverified]`(실세션 검증 필요)를 달았어요. `~/.codex` 는 전 과정 읽기 전용이었고, 모든 설치 실험은 격리 `CODEX_HOME` 샌드박스에서 했어요 (모델 세션은 열지 않음 — 그래서 "실세션 거동" 항목만 unverified 로 남아요).

---

## 1. 한눈에 보는 결론

**빌드 타임 host transform 으로 codex 전용 파생 번들(`plugins/axhub-codex`)을 만드는 안(B)을 권고해요 — 단, 적대 검증이 찾아낸 CRITICAL 2건(공유 상태 파일 · 8KB 절단)의 보강을 "본체 선행조건"으로 포함해서요.** 소스는 지금처럼 claude-first 단일 트리(`skills/`, `hooks/`)를 유지하고, codex 표면은 치환 테이블 + 소수 override 파일로 기계 생성해요. 문안 병기(단일 번들)는 byte 예산상 산술적으로 불가능하고, 훅 제외(skills-only)는 선례상 근거가 약해요.

결정적 실측 3개:

1. **설치는 이미 돼요.** 격리 샌드박스에서 현행 repo 를 그대로 `codex plugin marketplace add <repo>` → `codex plugin add axhub@axhub` 하면 v1.20.4 가 설치되고 9개 스킬이 전부 codex 프롬프트에 로드돼요 `[confirmed]`. codex 는 legacy `.claude-plugin/marketplace.json` 경로, Claude 형식 `hooks/hooks.json`, `${CLAUDE_PLUGIN_ROOT}` env 를 전부 공식 호환으로 소비해요.
2. **그러나 지금 품질은 "우연 동작" 수준이에요.** codex 는 스킬 본문을 트리거 시 **8,000B 에서 절단**하는데 우리 9개 스킬 전부가 초과예요(최소 scaffold 10,090B ~ 최대 import 28,918B). 승인 게이트·verify 성공 선언·static lane 분기·update apply 명령이 전부 절단선 밖(@8,058~@11,365)에 있어요 `[confirmed]`. update 스킬은 codex 에 존재하지 않는 `claude plugin update --scope` 를 실행하려 들어요.
3. **병기는 산술적으로 불가예요.** skill byte 예산 잔여가 7,063B(202,937/210,000)인데 REWRITE 대상 병기 증가 추정이 +20~35KB 예요 `[confirmed/likely]`. 그래서 "한 파일에 양쪽 문안"이 아니라 "빌드에서 codex 판 파생"이 유일하게 diet 정책과 양립해요.

로드맵 요약: **P0 격리 실세션 스모크(코드 무수정) → P0.5 소스 공통 개선 PR(훅 wrapper 화·테스트 host-fixture 화·문안 미세수정) → P1 transform 본체 + 마켓플레이스 배선 + 게이트 → 파괴 경로 수동 QA 통과 후 codex 지원 선언 → P1.1 (8KB 소스 재구조화·e2e 자동화)**.

---

## 2. 실측으로 확정된 Codex 0.147.0 동작

### 2.1 그대로 되는 것 (이식 비용 0)

| 표면 | 실측 내용 | 근거 |
|---|---|---|
| marketplace 매니페스트 | 발견 순서 `.agents/plugins/marketplace.json` → `.agents/plugins/api_marketplace.json` → **`.claude-plugin/marketplace.json`** → `.cursor-plugin/marketplace.json` (first-match). 현행 repo 구조는 3순위로 정상 소비 | marketplace.rs:20-24 `[confirmed]` |
| 플러그인 설치 | `plugin add axhub@axhub` 성공, 설치본은 `CODEX_HOME/plugins/cache/<mkt>/<plugin>/<ver>/` 에 **frozen 복사**(원본과 diff 무차이), config 에 `[plugins."axhub@axhub"] enabled=true` | 샌드박스 실측 `[confirmed]` |
| 스킬 로드·라우팅 골격 | 9개 스킬 전부 `axhub:<name>` 으로 카탈로그 로드, name/description frontmatter 소비, unknown frontmatter 필드는 조용히 무시 | `codex debug prompt-input` 실측 + parser.rs:4-20 `[confirmed]` |
| 훅 파일 | 플러그인의 `hooks/hooks.json` 을 **Claude 스키마 그대로** 자동 발견(경로도 기본값 동일). 이벤트 11종(PascalCase 동일): PreToolUse·PermissionRequest·PostToolUse·PreCompact·PostCompact·SessionStart·SessionEnd·UserPromptSubmit·SubagentStart·SubagentStop·Stop | loader.rs:69, hook_config.rs:36-59, protocol.rs:1499-1511 `[confirmed]` |
| `"shell": "bash"` 필드 | codex 스키마에 없지만 deny_unknown_fields 미선언이라 **조용히 무시** — 설치·신뢰 발급 모두 안 막음 (superpowers 가 이 필드 포함 채 trusted 실물) | hook_config.rs:147-176 + 로컬 캐시 실물 `[confirmed]` |
| env 호환 | 훅 실행 env 에 `PLUGIN_ROOT`/`PLUGIN_DATA` + **`CLAUDE_PLUGIN_ROOT`/`CLAUDE_PLUGIN_DATA`**(동일 값, "OOTB compat" 주석) 4개 주입, command 내 `${KEY}` 문자열 치환도 지원 | discovery.rs:227-235,533-535 `[confirmed]` |
| update-router stdin | UserPromptSubmit stdin JSON 은 snake_case 그대로이고 사용자 프롬프트 키는 **`prompt`** — `update-router.sh` 의 `"prompt":` 이후 구간 매칭 전략이 codex 에서도 유효 | schema.rs:551-570 `[confirmed]` |
| 훅 동봉 dual-host 선례 | **4개 실존**: security-guidance 2.0.7(4이벤트 스위트) · ralph-loop 1.0.0 · superpowers 6.3.0 · openai-codex 의 codex 1.0.6 — 전부 Claude hooks.json 그대로 per-hook sha256 신뢰 발급받고 enabled | `~/.codex/config.toml` [hooks.state] 실물 `[confirmed]` |
| 문안 verbatim 선례 | Anthropic 공식·OpenAI 자사 플러그인 모두 "Claude Code" 문안을 **중립화 없이 그대로** codex 에 설치·사용 — 제품명 전면 치환은 불급 | 캐시 grep 실물 `[confirmed]` |

### 2.2 다르게 동작하는 것 (대응 필요)

| 차이 | 실측 내용 | 파급 |
|---|---|---|
| **스킬 본문 8,000B 절단** | 트리거 시 본문 통째 주입하되 `MAX_SKILL_PROMPT_BYTES=8000` 에서 절단 + truncated 경고. 9/9 스킬 초과 | 안전 게이트가 절단선 밖 — §4 CRITICAL-2 (core-skills/lib.rs:14, injection.rs:102-149 `[confirmed]`) |
| **카탈로그 예산·절단** | 스킬 목록 예산 = context window 의 2%(미상 시 8,000자), description 라인당 **1,024자 절단**, 초과 시 뒤쪽 root(=플러그인 스킬이 마지막 root) 먼저 omit. `~/.agents/skills` 사용자 스킬과 전역 예산 경쟁 | update(1,575B)·bootstrap(1,108B) description 절단 (render.rs:19-351 `[confirmed]`) |
| **frontmatter `examples` 무시** | 파싱 필드는 name/description/metadata.short-description 뿐 — 7/9 스킬이 가진 `examples` 라우팅 자산이 codex 에서 전량 증발 | 라우팅 회귀 위험 — §4 HIGH (parser.rs:4-20 `[confirmed]`) |
| **suppressOutput 미구현** | 파싱 후 `let _ =` 로 명시적 폐기(SessionStart/UserPromptSubmit/Stop 등). **PreToolUse·PermissionRequest·PostToolUse 에선 suppressOutput 을 보내면 오히려 출력 전체가 invalid 처리** | 훅 문안이 사용자에게 보임 (session_start.rs:272, output_parser.rs:362-392 `[confirmed]`) |
| **additionalContext 상시 노출** | TUI 에 `hook context:` 3줄 preview 로 항상 렌더(+N lines 힌트, transcript 에 전문). 300ms 내 무출력 성공 훅만 미표시. systemMessage 는 절단 없이 전문 표시 | AP-14 의 2.8KB 영어 지침이 화면에 노출 — §4 MEDIUM (hook_cell.rs:58,489-561 `[confirmed]`) |
| **훅 신뢰 모델** | 설치≠신뢰. user config 의 `[hooks.state]."<plugin>@<mkt>:hooks/hooks.json:<event>:<g>:<h>"` 에 `trusted_hash="sha256:..."` — hash 대상은 **command 원문 등 정의부**(스크립트 파일 내용 아님). Untrusted/Modified 는 **에러 없이 조용히 제외**. TUI 는 시작 시 3택 팝업(Review/Trust all/Continue without), headless 는 `codex exec --bypass-hook-trust` 플래그뿐(설정 파일 불가) | 훅 command 가 바뀌는 릴리즈마다 재신뢰 필요 → wrapper 화가 구조적 해법 (discovery.rs:521-568, config_rules.rs:15-66 `[confirmed]`) |
| **Windows 실행 셸** | 기본 `%COMSPEC%`(cmd.exe) `/C`, Unix 는 `$SHELL -lc`(login shell — profile 소싱됨). Git Bash 는 `commandWindows` 필드로 위임해야 함 | AP-13 재설계 필요 (command_runner.rs:166-211 `[confirmed]`) |
| **업데이트 표면** | `codex plugin update` 서브커맨드·`--scope` **없음**. git marketplace 는 `marketplace upgrade` 가 스냅샷 갱신 + configured 플러그인 **ForceReinstall 까지 자동**. local marketplace 는 upgrade 제외(라이브 참조), 갱신 = `plugin add` 재실행(멱등·구버전 디렉토리 삭제). 세션 로드 시 IfVersionChanged 백그라운드 refresh 는 **스냅샷 기준이라 스스로 원격을 못 봄**(startup 자동 git sync 는 OpenAI curated 전용) | update lane 은 치환이 아니라 재작성 대상 (manager.rs:2234-2304, loader.rs:506-681 `[confirmed]`) |
| **버전 감지** | `plugin list --json` 의 `installed[].version` 으로 설치 버전 읽기 가능. available 배열은 "미설치 전용"이라 신버전 신호 아님. config 에 버전 핀 없음(cache 디렉토리명이 진실) | update 스킬 감지 로직 재설계 자료 (샌드박스 실측 `[confirmed]`) |
| **매니페스트 우선순위** | `.agents/plugins/marketplace.json` 이 있으면 legacy 를 **병합 없이 완전히 가림**(legacy 에만 있는 플러그인은 설치 불가). 플러그인 레벨은 `.codex-plugin/plugin.json` > `.claude-plugin/plugin.json`. plugin.json 이 아예 없어도 설치됨(version="local"). **미동봉 시 codex 가 `.codex-plugin/plugin.json` 을 자동 생성**(interface 블록 포함 — 표시 메타 통제권 상실) | 이중 번들 라우팅의 핵심 레버 (샌드박스 합성 marketplace 실측 + caveman 캐시 실물 `[confirmed]`) |
| **AskUserQuestion 등가물** | `request_user_input` 도구는 존재하나 **기본 모드에서 비활성**(Plan 모드 전용, default_mode feature off) | AP-12 승인 프리미티브 재정의 필요 (config_types.rs:669-671 `[confirmed]`) |
| 기타 | matcher 는 UserPromptSubmit/Stop 에서 무시. `async:true` 는 SessionEnd 외 이벤트에서 훅 자체 skip. timeout 기본 600초(SessionEnd 1초/상한 3초). additionalContext 기본 ~2,500 tok 초과 시 디스크 spill. 출력 wire 는 deny_unknown_fields(표준 Claude 필드만 쓰면 무관) | common.rs:110-126, discovery.rs:476-621 `[confirmed]` |
| **운영 wart 2개** | ① 깨진 marketplace 하나가 `codex plugin` 명령 전체를 죽임(로그 무기록, 복구는 config.toml 수술 — 사용자 로컬의 caveman-repo 가 현재 이 상태). ② marketplace 를 먼저 remove 하면 설치 플러그인이 **보이지 않는 활성 고아**로 잔존(list 에 안 나오는데 세션엔 로드) — 제거 순서는 plugin remove 먼저 | 온보딩·지원 문서에 명시 (실물 진단 + 샌드박스 재현 `[confirmed]`) |

### 2.3 우리 repo 쪽 인벤토리 (치환 대상의 실측)

- **host 문자열 분포**: 스킬 139줄 55,958B(스킬 md 총량의 14.8%), hooks 26줄 12,562B(**hooks 총 바이트의 52%**), 정책·README 30줄 12,911B `[confirmed]`. 주요 항목: Claude Desktop 66줄(대부분 Desktop UX 가드 — 잔류 무해), claude plugin 명령류 43줄, AskUserQuestion/AUQ 34줄, 재시작 exact-문장 32줄, `--scope` 10줄.
- **80% 집중**: claude plugin·재시작·scope 매치의 80%가 `skills/update/`(SKILL+references 3파일) + `hooks/`(auto-update-prompt·plugin-restart-confirm-prompt·hooks.json) 6개 파일에 몰려 있어요 — codex 변형의 실질은 "update lane 재작성" 하나가 지배해요 `[confirmed]`.
- **tests 결합**: host 문자열 assert 155줄. 최다는 update-desktop-ux-contract(49줄, exact 한국어 문장 고정)·smooth-behavior(36줄, 훅 command `toBe` exact 포함). 역방향 함정 3계열: `not.toContain("systemMessage")` / `not.toContain("claude plugin update")` / `not.toContain("oh-my-claudecode")` — codex 변형이 문자열을 "추가"하는 방향도 깨질 수 있어요 `[confirmed]`.
- **byte 예산**: SKILL.md 합 202,937B / 한도 210,000B(잔여 7,063B), always-on ~2,123/2,500 tok. 병기 추정 +20~35KB 는 게이트와 양립 불가 `[confirmed/likely]`.
- **빌드 파이프라인**: `build-plugin-bundle.ts` 는 순수 copyFileSync(변환 0) + marketplace source 재작성뿐. drift 방지는 `plugin-bundle.test.ts` 의 byte-for-byte 3-test 가 유일. `.versionrc.json` bumpFiles 5개 하드코딩(codex 번들 미추가 시 릴리즈마다 drift 테스트 파손). lint:tone 은 `skills/*/SKILL.md`+정책 3파일만 스캔. `plugin:budget` 은 `--root` 로 codex 번들 재사용 가능 `[confirmed]`.
- **`.agents` 는 현재 gitignore**: `.gitignore:74` 가 `.agents/` 전체 ignore + 타 도구 산출물(`.agents/skills/`)과 동거 — 신표준 매니페스트 커밋에는 4줄 캐스케이드 부정 규칙이 필요해요(§4 MEDIUM) `[confirmed]`.

---

## 3. 설계안 4개와 스코어

| 안 | 요약 | diet정합 | 유지보수 | 라우팅 | 훅UX | 업데이트 | 구현비용 | 판정 |
|---|---|---|---|---|---|---|---|---|
| A. 단일 번들 + 인라인 host 분기 병기 | CE-식 "AskUserQuestion in Claude Code, request_user_input in Codex" 문장을 소스에 병기 | 2 | 4 | 3 | 2 | 2 | 4 | **기각** — byte 잔여 7,063B vs +20~35KB 로 산술 불가, codex 전용 조정(1,024자 description·8KB 대응) 물리적 불가 |
| **B. 빌드 타임 transform 이중 번들** | `--host codex` 로 `plugins/axhub-codex` 파생, `.agents` 완전-가림을 역이용해 같은 이름 "axhub" 를 host 별 소스로 라우팅, 훅은 wrapper-재설계 다이어트판 전체 이식 | 4 | 4 | 4 | 4 | 5 | 2 | **권고** (적대 검증 보강 조건부 — §4) |
| C. skills-only 경량 lane | B 에서 hooks/ 를 drop | 5 | 4 | 4 | 5 | 3 | 3 | 기각 — 훅 동봉 정상 동작 선례 4개가 실물로 확정돼 포기 근거가 약하고, auto-update·AP-13/14/19 free-form 가드 상실 |
| D. 무변경 + README 병기 스파이크 | 설치 명령 2줄만 병기 | 5 | 5 | 3 | 2 | 1 | 5 | **P0 로 흡수** — update lane 오동작 확정(`claude plugin update --scope` 실행 시도), 지원 선언 불가 품질 |

구조 선례 정합: 현행 root marketplace → `./plugins/axhub` 구조는 openai/codex-plugin-cc·jocoding-ax-partners/jax-plugin-cc 와 동형이라 유지해요. `.codex-plugin/plugin.json` 은 transform 이 생성·동봉(자동 생성에 표시 메타를 뺏기지 않기, 단 manifest 에 hooks 필드 금지 — codex 스캐폴드 지침 문자열 확인). hooks-codex.json 식 host 별 훅 파일 분리는 superpowers 가 폐기한 패턴이라 도입하지 않아요. 스킬 트리 복제는 ouroboros 의 stale 이중 트리(21 vs 22개, 아무도 참조 안 하는 사본)가 안티패턴 실증이에요 — override 는 update lane 최소 파일로 한정해요. `[confirmed]`

---

## 4. 적대 검증 (red team) — 채택 조건이 된 발견들

> 결론: "B 골격은 유효하나 v1 명세 그대로는 출시 불가. CRITICAL 2건 + 보강 5건을 채택 조건으로 반영하면 성립" — 이 발견들은 C/D 로 후퇴해도 사라지지 않는 문제(공유 상태·8KB·모델 QA)를 포함해서, 대안 전환이 아니라 B 보강의 근거예요.

### CRITICAL (설계 수정 완료 전 출시 불가)

**C-1. `~/.axhub` 공유 상태 파일이 host 비구분** — auto-update throttle(`.plugin-update-check`, 24h mtime)과 restart marker(`.plugin-update-restart`, "버전|scope")를 양 host 가 공유하면: (a) 듀얼 host 사용자(가장 기대되는 고객)의 Claude 세션이 매일 throttle 을 touch → codex 자동 업데이트 훅이 영구 미발동인데, git 설치 사용자의 원격 신버전 반영 주체는 그 훅이 실행할 `marketplace upgrade` 뿐이라 **codex 플러그인이 무한 stale**. (b) codex 가 남긴 marker 를 Claude 의 restart-confirm 훅이 읽고 7일간 오안내(역방향 동일). → **fix(채택)**: transform 이 codex 판의 상태 파일 경로를 host-suffix(`-codex`)로 재작성 + marker 에 host 판별자, 각 host 훅은 자기 marker 만 소비. codex-bundle.test 에 "비-suffix 공유 경로 문자열 0건" assert. `[confirmed]`

**C-2. 8KB 절단이 안전 계약을 모델 재량으로 강등** — 9/9 스킬이 8,000B 초과이고 안전 게이트의 byte offset 실측이 전부 절단선 밖이에요: bootstrap AUQ 승인 게이트 first@11,365 · `--execute`@8,956, import AUQ@8,058 · `deploy verify`@11,009, deploy static `active_release_id`@10,444, update apply `--execute`@9,954. "본문이 잘리면 path 파일을 재독하라"는 prepend 1줄에 안전을 위임하면, 모델이 안 따르는 단 한 세션에서 승인 없는 생성 saga·static 앱 404 verify 루프·apply 미인지가 나요. → **fix(채택)**: 실행 4스킬(bootstrap/deploy/import/update)의 **codex 코어 ≤8,000B 저작을 B 본체 선행조건으로 승격**(승인 게이트·성공 선언·static 분기·apply 명령을 첫 8KB 안에, 세부는 references 분리), codex-bundle.test 에 "게이트 문자열이 파일 첫 8,000B 내 존재" byte-offset assert. prepend 재독 지시는 보조 수단. `[confirmed]`

### HIGH (수정 필요 — 전부 fix 채택)

| 공격 | 요지 | 채택 fix |
|---|---|---|
| examples 전량 드랍 | 7/9 스킬의 `examples` 라우팅 자산이 codex 에서 증발 + 카탈로그 전역 2% 예산 경쟁 + 플러그인 스킬(마지막 root)이 먼저 omit | transform 이 codex description 을 "description 핵심 + examples 대표 트리거 병합"으로 재합성(≤1,024자, 양보 규칙·트리거를 앞 200자 전진 배치), 카탈로그 총량 상한 assert, `$axhub:update` 명시 멘션이 확실 경로임을 문서화 |
| AP-12 이행 불가 계약 | deploy 의 "텍스트 번호 선택지 정지 금지"가 codex 유일 프리미티브와 정반대 — 치환 누락 시 모델이 이행 불가능 계약을 받음. FORBIDDEN 목록에 `AskUserQuestion` 부재 | AP-12 승인 문단을 치환이 아니라 **override/host-블록으로 승격**(codex 판: 명시 텍스트 승인 1회 + 정확 응답 대기 + silent skip 금지 — CE 선례), `AskUserQuestion` 을 FORBIDDEN_CLAUDE_STRINGS 에 추가, 첫 8KB 내 배치(C-2 와 결합) |
| 미신뢰 시 조용한 전멸 | "Continue without trusting" 관성 선택·headless 사용자는 6훅 전원 침묵 — 훅에 둔 이유(free-form 커버)가 커버 필요 사용자에게서 꺼지는 역상관 | 커버리지 서술을 "훅은 보강재, 스킬 본문·수동 update 가 1차"로 재정의, update 스킬은 훅 무관 완결 계약 명시, README 에 미신뢰 시 죽는 4개 표면 열거 + `--bypass-hook-trust` 문서화 |
| `.agents` 신표준에 host-특정 번들 | `.agents` 는 다중-host 표준 경로 — Claude/Cursor 가 채택하는 순간 완전-가림 특성으로 그 host 사용자 전원이 codex 문안 번들을 받음 | tripwire 정책화(릴리즈 노트 감지 시 즉시 재배선 + 분기별 재검증) 또는 처음부터 host-중립 구성(axhub=Claude 번들 + axhub-codex=codex 번들 병기, README 가 host 별 설치 id 안내) — §6 결정 1 |
| override 6파일 churn drift | update lane 은 repo 최다 수정 표면인데 소스↔override 의미 동기화 기계 게이트가 없음 | **hash-pin 게이트**: `codex-overrides/SOURCE_HASHES.json` 에 대응 소스 sha256 pin, 소스 변경 시 pin 미갱신이면 테스트 fail — override 재검토 없인 소스 update lane 수정 불가 |
| 파괴 경로 모델-행동 QA 부재 | 4안 공통 맹점 — 안전 봉투(preview-confirm→--execute, verify 단독 성공 선언)는 Claude 모델 튜닝 계약인데 codex 모델군 준수율 측정 계획이 없음 | rollout 에 **파괴 경로 행동 QA 3항목**(승인 전 정지 / dry-run→승인→execute→verify 순서 / static lane 분기)을 codex 실모델 세션으로 각 1회 — 통과를 "codex 지원 선언"의 필요조건으로 dev-policy 명문화 |

### MEDIUM (수정 반영)

- **commandWindows bash 부재 가드**: cmd PATH 에 bash 없으면 실패 훅 entry 가 전문 노출(비-context entry 는 무절단) — `where bash >nul 2>nul && bash "${CLAUDE_PLUGIN_ROOT}/hooks/x.sh" || cd .` 가드 채택. superpowers 의 polyglot run-hook.cmd 패턴도 대안. `[confirmed]`
- **노출 다이어트를 전 훅으로 확장**: 노출원은 AP-14 하나가 아니라 최대 5개(AP-19 ~1.2KB + AP-14 2.8KB 영어 + auto-update + restart-confirm + Windows) — codex 판은 6개 entry 전부 첫 줄 한국어 사용자-가독 요약으로 통일하고 상시 emit 2개(AP-14+AP-19)는 합본 1 entry 로 병합, entry 별 byte 상한 assert.
- **`.gitignore` 4줄 캐스케이드**: `.agents/` 전체 ignore 상태에서 `!.agents/plugins/` 단독 부정은 무효(git 은 ignored 디렉토리로 안 내려감) — `.agents/*` + `!.agents/plugins/` + `.agents/plugins/*` + `!.agents/plugins/marketplace.json` 이 정답. tracked 여부를 `git ls-files` 로 assert. release step 2 의 `--amend -a` 가 미검토 재생성물을 흡수하는 사고 표면도 release-tag 에 diff 검사 추가로 보강.
- **parity 엔진 선행 확장**: 현행 parity 는 "모든 invariant × 모든 적용 파일" 구조라 host 별 명령 문자열을 invariant 로 올리는 순간 한쪽에서 반드시 실패 — **host-scoped invariant 문법**(`- invariant(codex): "..."`) 파서 확장을 codex 파일 편입보다 먼저. P0 체크리스트에 (h) "실물 훅 출력 JSON(특히 outer `continue` 필드)이 codex 에서 Failed 없이 수용되는지" 추가(deny_unknown_fields+flatten 조합의 실거동이 open question).
- **trust hash 갭 의존(생존 판정)**: wrapper 편익("스크립트 내용 변경은 재신뢰 없음")은 upstream 이 닫을 개연성 있는 보안 갭의 역이용 — 정책 문서에 확정 문장으로 쓰지 않고, dev-policy 에 **codex compat matrix**(검증 버전 범위 + 의존 내부 거동 5개: trust hash 대상·8,000B·1,024자·2% 예산·manifest 우선순위) + 분기별 재검증 체크리스트 신설.

### 6개월 drift 예보

1. codex-overrides update lane 의미 drift (hash-pin 없으면 최초 파손 지점 — 2~3 릴리즈 내)
2. codex 0.x 내부 상수·거동 변경 (특히 trust hash 에 파일 내용 포함되는 순간 릴리즈마다 6훅 재신뢰 스팸 재발)
3. `.agents` 신표준의 타 host 채택 (tripwire 없으면 감지 경로가 사용자 리포트뿐)

---

## 5. 최종 권고안 (red team 보강 반영판)

### P0 — 격리 실세션 스모크 (repo 무수정, 반나절)

scratch `CODEX_HOME` 로 현행 repo 를 local marketplace 설치 후 **실제 모델 세션 1회**로 unverified 전부 해소: (a) 훅 trust 3택 팝업과 신뢰 후 6훅 실행 여부(features 의 `plugin_hooks: removed` 의미 확정 포함) (b) additionalContext 3줄 preview 실측과 2.8KB entry 노출 정도 (c) `"shell"` 필드 런타임 무시 (d) `.agents`+legacy 공존 shadow 재확인 + `.agents` 스키마의 로컬 상대경로 source 지원 (e) marketplace 엔트리 version vs 매니페스트 version 우선순위 (f) `codex exec --bypass-hook-trust` headless (g) **Claude Code 가 `.agents/plugins/marketplace.json` 을 무시하는지** (h) 훅 출력 JSON(outer `continue`)의 수용성. — 사용자 전역 `~/.codex` 의 깨진 caveman-repo 는 사용자가 직접 `[marketplaces.caveman-repo]` 를 삭제해야 로컬 검증이 가능해요(우리는 쓰기 금지, 제거 순서 wart 안내 포함).

### P0.5 — 소스 공통 개선 (Claude lane 에도 이득, 독립 PR 3건)

1. **훅 wrapper 화**: SessionStart 인라인 command 5개를 `hooks/*.sh` 로 추출, command 를 `bash "${CLAUDE_PLUGIN_ROOT}/hooks/<x>.sh"` 로 고정(update-router.sh 동형) — codex trust hash 가 command 원문 기반이라 이후 릴리즈의 스크립트 내용 수정이 재신뢰를 안 부름. hook-execution·smooth-behavior 테스트 동반 수정.
2. **테스트 host-fixture 화**: 문자열 결합 ~85% 소유한 update-desktop-ux-contract(49줄)·smooth-behavior(36줄)를 host 별 기대값 테이블로 전환 + 역방향 not.toContain 3계열 체크리스트.
3. **문안 미세수정(+2KB 이내)**: bootstrap SKILL:37 의 "`rtk` 같은 Codex/개발자 전용 래퍼는 이 Claude Desktop skill 에서 절대 쓰지 않아요" host-중립 재작성(치환 시 자기모순 1순위), AUQ 지점에 CE 패턴 1문장("AskUserQuestion(Claude) / request_user_input(Codex·Plan 한정) / 둘 다 없으면 headless — never silently skip"), headless 판정에 `codex exec` 병기.

### P1 — transform 본체

- **`build-plugin-bundle.ts --host codex`** (+180~260 LOC): ① CODEX_SUBSTITUTIONS longest-first 치환 테이블(`claude plugin marketplace update`→`codex plugin marketplace upgrade`, `claude plugin list`→`codex plugin list`, `claude -p`→`codex exec`, `command -v claude`→`command -v codex`, `Claude Code`/`Claude Desktop`→`Codex`, `/oh-my-claudecode:autopilot, `→삭제 — `${CLAUDE_PLUGIN_ROOT}` 는 무치환) ② codex-overrides/ 스왑(update lane 5~6파일 + **실행 4스킬 ≤8KB codex 코어** + AP-12 승인 섹션) ③ transformCodexHooks(JSON 파싱 → shell 키 제거 + commandWindows(bash 가드 포함) 추가 + **상태 파일 경로 host-suffix** + command·additionalContext 내부 치환 + 전 entry 첫 줄 한국어 + AP-14·AP-19 합본 축소) ④ transformCodexManifest + `.codex-plugin/plugin.json` 생성(interface 블록, hooks 필드 금지) ⑤ **description 재합성**(examples 병합, ≤1,024자) ⑥ 직렬화는 version-updater 와 동일 규격 통일.
- **마켓플레이스·버전 배선**: `.agents/plugins/marketplace.json`(P0-(d)(g) 결과 반영, host-중립 구성 여부는 §6-1) + `.gitignore` 4줄 캐스케이드 + `.versionrc.json` bumpFiles +3 + package.json scripts(plugin:bundle:codex 계열·plugin:bundle:all·plugin:budget:codex). 릴리즈 3단계 플로우는 무변경.
- **게이트**: `tests/codex-bundle.test.ts` 신규(byte-for-byte drift / FORBIDDEN_CLAUDE_STRINGS(**AskUserQuestion 포함**) 0건 / longest-first 정렬 / tone 금지 토큰 / invariant 존재 / hooks 변환 검증 / **게이트 문자열 first-8KB offset** / manifest 3종 버전 parity / **SOURCE_HASHES pin**), lint:tone glob 확장, plugin:budget:codex(+description 1,024자 검사), parity host-scoped 문법 선행, ci.yml step 추가.
- **정책·문서**: AP-12(호스트별 승인 프리미티브) · AP-13(codex 훅=cmd.exe, commandWindows 계약, Unix 훅은 login shell 이라 env kill switch 가 훅 레벨에서도 유효하다는 각주) · AP-14(host 별 주입 명령, "첫 3줄 사용자-가독 한국어" 계약화) · AP-19(신뢰 전 미방출 각주) · 신규 AP-20(host-derivation: claude-first 소스, transform 소유, 파생 번들 직접 수정 금지, compat matrix) · POLICY.md(codex 공개 선언 + 훅 신뢰 전 자동 업데이트 미동작 + axrouter 는 Claude 전제라 codex 제외) · dev-policy(DP-5/7 + compat matrix + tripwire) · README(codex 설치 섹션: marketplace add → plugin add → 훅 리뷰·신뢰 → 세션 재시작, 최소 버전 **Codex CLI ≥ 0.147.0**).

### 릴리즈 게이트 — "codex 지원 선언"의 필요조건

격리 tenant 실모델 QA 3항목: ① bootstrap 이 승인 전 정지 ② deploy 가 dry-run→명시 승인→`--execute`→`deploy verify` 순서 준수 ③ static 앱이 `active_release_id` lane 으로 분기. (e2e 자동화 전까지 수동 체크리스트를 dev-policy 에 명문화)

### 롤백 스토리

codex 번들은 파생물 — 문제 시 `.agents/plugins/marketplace.json` 1파일 revert 로 codex 노출만 차단(Claude lane 완전 무영향), 사용자 캐시는 frozen 복사라 즉시 파급 없음.

### P1.1 (후속)

전 스킬 ≤8KB 코어 + references 소스 레벨 재구조화(최대 3.6배 초과 해소), `tests/e2e/codex-cli/` 하네스(codex exec + 격리 CODEX_HOME + --bypass-hook-trust), release.yml Slack 문안 codex 병기, AP-12 확정분 반영.

---

## 6. 남은 결정 사항 (운영자 판단 필요)

1. **`.agents` 매니페스트 구성**: codex 전용(같은 이름 "axhub" host 라우팅, 깔끔하지만 타 host 채택 시한폭탄 — tripwire 필수) vs host-중립 병기(axhub + axhub-codex 두 엔트리, 미래 안전하지만 이름 분리) — P0-(g) 결과와 함께 확정.
2. **AP-12 codex 승인 프리미티브**: 명시 텍스트 승인 1회 완화(권고, CE 선례) vs headless 강제 유지(안전하지만 codex interactive 에서 deploy --execute 불가) — 파괴적 실행 게이트라 운영자 최종 승인 필요.
3. **codex 훅 세트 범위**: 6개 전체 다이어트-이식(권고) vs 축소 4개 — trust 리뷰 팝업 항목 수(사용자 마찰) 트레이드오프.
4. **AP-14 fallback entry**: 축소·한국어화 유지(권고) vs codex 번들에서 제거 — P0-(b) 노출 실측 후.
5. **axrouter(AI 활용 기록) codex 제외** 확정과 POLICY.md 공개 문안 — `~/.claude/settings.json`·Claude Code OTEL 전제라 성립 불가 판단이나 제품 결정.
6. **8KB 소스 재구조화 착수 시점** (P1 은 codex 코어 override 로 우회, 소스 레벨 해소는 P1.1).
7. **codex e2e 자동화 투자 시점** (v1 수동 스모크 vs v1.1 자동화).
8. release.yml Slack·마케팅 채널의 codex 안내 추가 시점.

---

## 부록

### A. 재현 포인터

- workflow 스크립트: `~/.claude/projects/-Users-wongil-Desktop-work-jocoding-axhub/dcaa4c8f-f580-4f3c-a45e-59f76cdae71d/workflows/scripts/codex-compat-research-wf_b1575846-d79.js` (run id `wf_b1575846-d79`, lane 별 원본 결과는 같은 세션 디렉토리의 `subagents/workflows/wf_b1575846-d79/journal.jsonl`)
- 샌드박스 재현 절차: `CODEX_HOME=<임시디렉토리> codex plugin marketplace add <repo>` → `codex plugin add axhub@axhub --json` — 전역 `~/.codex` 완전 격리 확인됨(단 `~/.agents/skills` 는 격리 밖 skill root 로 여전히 로드 — CI 격리 시 전제).
- codex 소스 근거는 `openai/codex` rust-v0.147.0 태그 기준 파일:라인이에요(§2 표 각 행).

### B. `codex plugin list --json` 스키마 (update 스킬 재설계용)

```json
{
  "installed": [{
    "pluginId": "axhub@axhub", "name": "axhub", "marketplaceName": "axhub",
    "version": "1.20.4", "installed": true, "enabled": true,
    "source": {"source": "local", "path": "<repo>/plugins/axhub"},
    "marketplaceSource": {"sourceType": "local", "source": "<repo>"},
    "installPolicy": "AVAILABLE", "authPolicy": "ON_INSTALL"
  }],
  "available": []
}
```

- 감지: `installed[].version`. `available` 은 미설치 전용(신버전 신호 아님 — "업데이트 가능" 필드 자체가 없음).
- 적용: git 설치 사용자 `codex plugin marketplace upgrade axhub`(ForceReinstall 자동) / local marketplace `codex plugin add axhub@axhub` 재실행(멱등). 반영 확인: list --json 의 version 재독. 스킬 반영은 세션 재시작.

### C. 핵심 수치 카드

| 항목 | 값 |
|---|---|
| 스킬 본문 주입 절단 | 8,000B (9/9 스킬 초과, 10,090~28,918B) |
| 카탈로그 예산 | context window 2% (미상 시 8,000자), description 라인 1,024자 |
| 안전 게이트 절단선 밖 offset | import AUQ @8,058 · bootstrap --execute @8,956 · update apply @9,954 · deploy static @10,444 · deploy verify @11,009 · bootstrap AUQ @11,365 |
| skill byte 예산 | 202,937 / 210,000 (잔여 7,063B) |
| host 문자열 | 스킬 55,958B(14.8%) · hooks 12,562B(52%) · tests assert 155줄 |
| 훅 이벤트 | 11종 (Claude PascalCase 동일) |
| 훅 신뢰 | per-hook sha256(command 원문 기반), 미신뢰=조용한 제외, headless 우회는 `--bypass-hook-trust` 플래그뿐 |
| 최소 지원 버전 | Codex CLI ≥ 0.147.0 (전 실측의 검증 기준선) |
| 수기 작업량 추정 | transform+게이트 ~320-430 LOC + override 저작(8KB 코어 4개 포함) + 정책 문서 |
