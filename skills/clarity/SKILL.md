---
name: clarity
description: 'axhub 의 onboarding(첫 셋업·로그인)·init(새 앱)·deploy(배포)·development(앱 코드 생성)이 담당하지 않는 나머지 모든 axhub CLI 작업을 처리하는 만능(catch-all) 라우팅 대상이에요. 공개 명령을 라이브로 찾아(--json-schema→--help) 무확인 실행해서 별도 스킬이 없는 기능도 다 쓸 수 있게 해요. 활성화 예: 환경변수·시크릿("환경변수 설정해줘", "env 추가", "시크릿 넣어줘"), 로그·상태("로그 보여줘", "배포 상태 확인"), 앱 관리·생성 제외("앱 목록", "앱 삭제/멈춰/복제/이름 바꿔"), 롤백("롤백해줘", "이전 버전으로"), 데이터·테이블·커넥터("테이블 만들어줘", "컬럼 추가", "DB 연결", "데이터 조회"), 테넌트·팀·권한("테넌트 바꿔줘", "팀 전환", "권한 줘"), 비용·프로필·공개("비용 보여줘", "프로필 바꿔줘", "마켓에 공개"), 그리고 "axhub로 ~해줘"처럼 axhub 관련이나 의도가 불명확한 모든 발화. 경계: 첫 셋업=onboarding, 새 앱=init, 배포=deploy, 기존 앱에 화면·페이지·대시보드 코드 생성=development, CLI·플러그인 버전 업데이트=update 라 그 의도가 분명할 때만 양보하고 그 외 axhub CLI 운영은 다 받아요. 앱 코드는 안 짜고 axhub 명령만 실행해요.'
---

# axhub clarity 브리지

핵심 3스킬(온보딩·새 앱 만들기·배포)에 명확히 안 맞거나 **의도가 불분명한** axhub 요청을 여기서 해소해요. 작업→명령 카탈로그는 없어요 — **매번 라이브 CLI 의 `--help` 트리를 탐색**해 맞는 명령을 찾고, 찾으면 **확인 질문 없이 바로 실행**한 뒤 결과를 한국어로 요약해요.

## 원칙

- **카탈로그 금지.** 이 문서에 작업→명령 매핑을 적지 않아요. CLI 가 릴리즈될 때마다 표면이 변하니 진실은 항상 라이브 `axhub --json-schema` (없으면 `axhub ... --help`) 예요. (본문의 `axhub env` 류는 절차 예시일 뿐 매핑이 아니에요.)
- **사용법 선숙지 강제 (--help gate).** 명령을 찾았다고 바로 실행하지 않아요. 실행 전 그 정확한 leaf 명령(서브커맨드 포함)의 `--help` 를 반드시 1회 읽어 사용법(positional 인자 순서·필수/선택 플래그·파괴적 실행 플래그)을 확정하고, 거기 나온 인자·플래그만 써요. 사용법을 안 읽은 명령은 실행 금지예요.
- **무확인 자동 실행.** 조회·설정 변경·파괴적 작업(삭제·롤백) 구분 없이, 명령을 찾으면 즉시 실행해요. "실행할까요?" 를 묻지 않아요.
- **공개 표면만.** `axhub plugin-support ...` (hidden 그룹) 는 plugin 내부 프로토콜이라 이 스킬의 탐색·실행 대상이 아니에요.
- **지어내지 않기.** 탐색으로 못 찾은 기능은 "axhub 에 그 기능은 없어요" 라고 정직하게 말하고, 가장 가까운 명령을 제안해요.

**대표 정직성 계약.** `clarity` 는 hidden `plugin-support` 를 탐색하지 않아요. 공개 `--json-schema` / `--help` 트리에서 맞는 leaf 를 찾지 못하면 존재하지 않는 명령을 만들지 말고, "axhub 에 그 기능은 없어요" + 가장 가까운 공개 명령만 말해요. 상태 확인·로그·환경변수처럼 대표 여정 뒤 작업은 이 경로로 이어가요.

## Anti-Patterns (하지 말 것)

원칙 위반이 실전에서 드러나는 구체 형태예요:

- ❌ `--json-schema` (270KB) 를 통째로 읽기 — 반드시 `--field-expr` 로 필요 부분만 슬라이스해요. 통째 로드는 context 낭비.
- ❌ `--help` 를 안 읽고 인자를 추측 조립 — leaf 명령 `--help` 1회 선숙지(--help gate) 후에만 실행. 추측 인자는 exit 64.
- ❌ 1단계 탐색에서 못 찾자 포기 — 두 단계 깊이까지 탐색한 뒤에만 "기능 없음" 을 선언해요.
- ❌ 탐색 출력(schema/help 본문)·raw stdout/stderr·secret·내부 id 를 chat 에 echo — 사용자에겐 한국어 요약만.
- ❌ 못 찾은 기능을 비슷한 명령으로 조용히 대체 실행 — 정직하게 부재를 알리고 가장 가까운 명령을 "제안"만 해요 (무단 실행 금지).
- ❌ `plugin-support` hidden 표면을 탐색·실행 (공개 표면만 원칙 위반).
- ❌ deploy/init/onboarding/development 담당 의도를 가로채기 (아래 경계표 위반 — 해당 의도는 양보). 특히 앱 코드(페이지·화면·대시보드·엔드포인트) 생성은 development 양보 — clarity 는 axhub 명령 실행만 해요.

## 진행 상황 알림 (Progress Reporting)

각 단계를 시작할 때 친근한 한국어 한 줄로 지금 뭐 하는 중인지 알려줘요 — vibe coder 가 멈춘 게 아니라 진행 중인 걸 알 수 있게 해요. 형식은 `[현재/전체] ○○ 하는 중이에요…`, 끝나면 `○○ 됐어요` 처럼 한 줄로 확인해요.

- 사람이 알아들을 요약만 알려요 — secret·내부 id·raw 출력·schema 본문은 chat 에 넣지 않아요 (위 원칙 그대로).
- 한 번에 끝나는 단순 조회(예: 목록 한 번 보기)는 굳이 단계별로 안 알리고 결과만 줘도 돼요 — 탐색이 여러 단계로 길어질 때 알려요.

단계 이름 (announce 용 한국어):
- `[1/4] 무엇을 찾는지 파악하는 중이에요`
- `[2/4] 기능 찾아보는 중이에요`
- `[3/4] 실행하는 중이에요`
- `[4/4] 결과 정리하는 중이에요`

### TodoWrite 체크리스트 (2+ 태스크일 때만 · 있을 때만)

요청이 **2개 이상의 axhub 작업으로 쪼개질 때만** TodoWrite 로 태스크를 보여줘요 (예: "테이블 만들고 env 추가하고 로그 봐줘"). 한 번에 끝나는 단순 조회·단일 명령은 TodoWrite 없이 위 한 줄 알림만 해요 — 1줄짜리 체크리스트는 만들지 않아요. TodoWrite 도구가 host 에 노출됐을 때만 호출하고, 없으면 조용히 진행해요 (도구 가용성은 언급 안 해요).

clarity 는 카탈로그가 없어서 todos 도 **고정 목록이 아니라 요청을 쪼갠 실제 태스크에서 도출**해요 — 사용자 발화를 axhub 작업 단위로 나눠 한 항목씩 만들어요. 참고 shape ("테이블 만들고 env 추가해줘"):

```typescript
TodoWrite({ todos: [
  { content: "테이블 생성",   status: "in_progress", activeForm: "테이블 만드는 중" },
  { content: "환경변수 추가", status: "pending",     activeForm: "env 추가하는 중" }
]})
```

**태스크 하나가 끝날 때마다**(그 태스크의 탐색→실행→결과까지 끝나면) 전체 todos 배열로 다시 호출해 끝난 항목은 `completed`, 다음 항목은 `in_progress` 로 갱신해요 — 끝에 한꺼번에 말고 매 태스크 직후에요. 이전 스킬 todo 가 남아 있으면 patch 하지 말고 위 배열 전체로 교체해요. 종료 시 미완료 todo 0 개.

## Workflow

1. **CLI 가드.** `command -v axhub` 가 실패하면 멈추고 안내해요: "axhub CLI 가 아직 없네요. 온보딩부터 진행할게요" → onboarding 스킬로 넘겨요. raw 에러는 chat 에 노출하지 않아요.

1a. **버전 체크 (맨 처음, best-effort · 비차단 · 10분 TTL).** CLI 가 있으면 본 작업 전에 axhub CLI·플러그인 새 버전이 있는지 한 번 가볍게 확인해요. 매 호출 네트워크를 피하려 10분 캐시하고, 실패·구 CLI 면 조용히 건너뛰어요 — 작업을 막지 않아요.

   ```bash
   STAMP="${TMPDIR:-/tmp}/axhub-update-check.stamp"
   if [ -z "$(find "$STAMP" -mmin -10 2>/dev/null)" ]; then
     : > "$STAMP"
     PLUGIN_VER=$(grep -o '"version"[^,]*' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json" 2>/dev/null | head -1 | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
     UPD=$(axhub update check ${PLUGIN_VER:+--plugin-version "$PLUGIN_VER"} --json 2>/dev/null)
   fi
   ```

   `UPD` 의 `has_update`(CLI) / `plugin.has_update`(플러그인) 중 하나라도 true 면 한 줄만 안내한 뒤 이어가요. 둘 다 false 거나 `UPD` 가 비면(캐시 hit·네트워크 실패·구 CLI) 아무것도 안 보여줘요.
   - CLI 새 버전: "axhub CLI 새 버전(`latest`)이 나왔어요 — '업데이트 해줘'라고 하면 적용할게요."
   - 플러그인 새 버전: "axhub 플러그인 새 버전(`plugin.latest`)이 있어요 — `/plugin update` 로 받을 수 있어요."

2. **의도 좁히기 (clarify).** 발화가 모호하면 먼저 핵심 동사·명사를 잡아요. 그래도 후보 동작이 여럿이면 한 번만 짧게 되물어요 — 단, 되묻기는 마지막 수단이고 대개는 다음 탐색으로 스스로 판별해요.

3. **탐색 (discover).** axhub 는 **에이전트용 기계가독 표면** `--json-schema` 를 제공해요 — `--help` prose 를 긁는 것보다 안정적이니 이걸 우선 써요. 단 전체 schema 는 ~270KB 라 **반드시 `--field-expr` 로 필요한 부분만 슬라이스**하고 통째로 읽지 않아요.

   ```bash
   # 1단계: 최상위 명령 목록만 (작아요)
   axhub --json-schema --field-expr '.commands | keys[]'
   # 2단계: 후보 명령의 구조 (서브커맨드·플래그·alias) — 그 명령만 슬라이스
   axhub --json-schema --field-expr '.commands["<후보>"]'
   ```

   - 예: "환경변수 설정해줘" → keys 에서 `env` 발견 → `--field-expr '.commands.env'` 로 set/list/get/delete 와 플래그 확인 → 인자 조립.
   - `--json-schema` 가 없거나 비면(구 CLI) `--help` 트리로 폴백해요: `axhub --help` → `axhub <후보> --help` → 필요하면 더 깊이.
   - 후보가 여럿이면 description 으로 판별하고, 탐색 출력(schema/help 본문)은 chat 에 붙이지 않아요 — 판단 재료로만 써요.

3b. **사용법 선숙지 (--help gate) — 실행 전 필수, 건너뛰기 금지.** leaf 명령(서브커맨드까지)을 정했으면 조립·실행 전에 **그 정확한 명령의 `--help` 를 반드시 1회 읽어** 사용법을 숙지해요: positional 인자 순서, 필수/선택 플래그, 파괴적 실행 플래그(`--execute`/`--yes`/`--force`), 그리고 예시. 추측으로 인자를 조립해 바로 실행하지 않아요.

   ```bash
   # 고른 정확한 leaf 명령의 사용법 (서브커맨드 포함) — 실행 전 필수
   axhub <명령> <서브커맨드> --help
   ```

   - 여기서 확인한 인자·플래그만 Step 4 에서 써요. help 에 없는 플래그·인자는 지어내지 않아요.
   - `--help` 가 비거나 없으면(구 CLI) `axhub --json-schema --field-expr '.commands["<명령>"]'` 의 해당 서브커맨드 노드로 같은 정보(인자·플래그)를 확정해요.
   - help 본문은 chat 에 echo 하지 않아요 — 읽고 사용법만 내재화해요.

4. **실행 (execute).** Step 3b 사용법 확정을 통과한 명령만 조립해 바로 실행해요 (사용법 미확인 명령 실행 금지).

   - 기계 파싱이 필요하면 `--json` (global flag) 을 붙여요.
   - help 가 `--execute` / `--yes` / `--force` 같은 명시 실행 플래그를 요구하는 파괴적 명령이면 **그 플래그를 알아서 부가해** 한 번에 실행해요. dry-run 으로 멈춰 확인을 구하지 않아요.
   - 인자가 부족하면(앱 이름 등) 먼저 조회 명령으로 채울 수 있는지 시도하고, 정말 사용자만 아는 값일 때만 물어요.
   - help 의 어떤 플래그가 **플러그인 자신의 설치 버전**을 요구하면(예: `update check` 의 `--plugin-version`), `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 의 `version` 을 읽어 채워요 — 그러면 업데이트 확인에서 CLI·플러그인 최신 여부를 한 번에 알려줘요 (`plugin.has_update` 면 `/plugin update` 안내).

   ```bash
   OUT=$(mktemp)
   axhub <명령> <인자...> > "$OUT" 2>&1
   EXIT=$?
   # raw 출력은 chat 에 cat 하지 않아요 — 읽고 아래 규칙대로 요약해요.
   ```

5. **결과 제시.** exit 0 이면 무엇이 어떻게 됐는지 한국어 한두 문장으로 요약해요 (URL·이름 같은 사용자 가치 정보만, 내부 id·raw JSON 생략). 비-0 이면:
   - 인증 계열(exit 4 등) → "axhub 로그인이 만료됐어요. 다시 로그인할까요?"
   - 사용법 오류(exit 64) → Step 3b 로 돌아가 그 명령의 `--help` 를 다시 읽고 인자를 고쳐 1회 재시도, 그래도 실패하면 정직하게 설명.
   - 그 외 → 원인을 한국어로 풀어 설명하고 다음 행동을 제안. raw stderr 는 노출하지 않아요.

6. **기능 부재 처리.** 두 단계 깊이까지 탐색해도 맞는 명령이 없으면: "axhub 에 그 기능은 없어요. 가장 가까운 건 `axhub <근접 명령>` 인데 이걸로 할까요?" — 존재하지 않는 명령을 만들어 실행하지 않아요.

## 경계

| 요청 | 담당 |
|---|---|
| 처음 셋업·CLI 설치·로그인부터 | onboarding |
| 새 앱 만들기·템플릿·스캐폴드 | init |
| 배포 실행·배포 검증 | deploy |
| **그 외 전부 + 의도가 불분명한 axhub 발화** | **clarity (이 스킬)** |

경계가 섞여 오면(예: "배포하고 로그 봐줘") 배포는 deploy 에 양보하고 로그 부분만 이 스킬이 이어받아요.

## 다음 단계 이어주기

조회 결과가 앱으로 이어질 만한 리소스(connector·table·데이터 카탈로그 등)면, 결과 요약 끝에 다음 단계를 한 줄로 권해요 — 예: "이 데이터로 앱 만들래요? '이걸로 앱 만들어줘' 라고 하면 돼요." 순수 안내 문장이에요. 이때도 `axhub plugin-support` 같은 hidden 표면을 호출하거나 state 를 쓰지 않아요 — clarity 는 그대로 공개 표면만 탐색·실행하고, 실제 앱 생성은 init 이 같은 대화 맥락을 이어받아 처리해요.

## Visibility

- 탐색의 `--help` 호출·명령 본문·raw stdout/stderr 는 chat 에 echo 하지 않아요.
- 사용자에게 보이는 건 무엇을 했는지 한 줄 + 결과 요약 + (있으면) 다음 행동 제안 — 전부 해요체.
