# Clarity 실행 가드레일 (anti-patterns · 진행 알림 · TodoWrite)

clarity SKILL 이 로드하는 내부 reference 예요. 탐색·실행 단계에 들어가기 전에 이 anti-pattern 목록과 진행 알림 규칙을 그대로 따라요.

## Anti-Patterns (하지 말 것)

원칙 위반이 실전에서 드러나는 구체 형태예요:

- ❌ `--json-schema` (270KB) 를 통째로 읽기 — 반드시 `--field-expr` 로 필요 부분만 슬라이스해요. 통째 로드는 context 낭비.
- ❌ schema/help/실행 명령에 `2>/dev/null | head -c 2000`, `| grep`, `| jq`, `bash -lc` 같은 shell 후처리 붙이기 — 모든 Desktop-visible clarity 명령은 단일 `axhub ...` 명령이어야 해요. 출력 축소는 더 좁은 `--field-expr` 로만 해요.
- ❌ `--help` 를 안 읽고 인자를 추측 조립 — leaf 명령 `--help` 1회 선숙지(--help gate) 후에만 실행. 추측 인자는 exit 64.
- ❌ 1단계 탐색에서 못 찾자 포기 — 두 단계 깊이까지 탐색한 뒤에만 "기능 없음" 을 선언해요.
- ❌ 탐색 출력(schema/help 본문)·raw stdout/stderr·secret·내부 id 를 chat 에 echo — 사용자에겐 한국어 요약만.
- ❌ `connectors list` / `--enabled-only` tenant-admin 전체 목록을 "내가 조회 가능한 커넥터" 로 표현 — 본인 접근 범위는 `connectors mine` 만 authority.
- ❌ 못 찾은 기능을 비슷한 명령으로 조용히 대체 실행 — 정직하게 부재를 알리고 가장 가까운 명령을 "제안"만 해요 (무단 실행 금지).
- ❌ `plugin-support` hidden 표면을 탐색·실행 (공개 표면만 원칙 위반).
- ❌ deploy/bootstrap/import/onboarding/development/diagnosis/update 담당 의도를 가로채기 (경계표 위반 — 해당 의도는 양보). 특히 기존 앱 첫 연결은 import, 앱 코드(페이지·화면·대시보드·엔드포인트) 생성은 development, 배포 실패 원인 진단은 diagnosis 양보 — clarity 는 axhub 명령 실행만 해요.
- ❌ Claude Desktop 에 노출된 `axhub` App/MCP 도구 호출 — read-only 라도 쓰지 않아요. clarity 는 항상 CLI help gate 뒤 `axhub` 명령으로 실행해요.
- ❌ 읽기 전용 leaf CLI 를 `> /tmp/...`, `2>&1`, `;`, `&&`, `||`, `echo`, `wc`, `jq`, `cat`, `mktemp`, command substitution 같은 shell wrapper 로 감싸기 — Claude Desktop 사용자에게 불필요한 권한 팝업과 임시 파일 흔적이 생겨요. 단일 `axhub ... --json` 호출을 실행하고 tool 결과를 assistant 내부에서 해석해요.
- ❌ `읽는 중 <랜덤>.txt`, `Read /tmp/...`, `파일 읽기` 같은 임시 출력 파일 재읽기 — 사용자는 단일 조회를 기대하므로, 파일 읽기 팝업/단계가 보이면 실패예요. 더 좁은 CLI 조회로 다시 실행해요.
- ❌ 새 앱 생성을 clarity 가 직접 질문 — concept/name/slug/template 추천 질문을 만들지 않아요. 추천 후보와 선택 카드는 bootstrap 책임이에요.

## 진행 상황 알림 (Progress Reporting)

각 단계를 시작할 때 친근한 한국어 한 줄로 지금 뭐 하는 중인지 알려줘요 — vibe coder 가 멈춘 게 아니라 진행 중인 걸 알 수 있게 해요. 형식은 `[현재/전체] ○○ 하는 중이에요…`, 끝나면 `○○ 됐어요` 처럼 한 줄로 확인해요.

- 사람이 알아들을 요약만 알려요 — secret·내부 id·raw 출력·schema 본문은 chat 에 넣지 않아요 (위 원칙 그대로).
- 한 번에 끝나는 단순 조회(예: 목록 한 번 보기)는 굳이 단계별로 안 알리고 결과만 줘도 돼요 — 탐색이 여러 단계로 길어질 때 알려요.

단계 이름 (announce 용 한국어):
- `[1/4] 무엇을 찾는지 파악하는 중이에요`
- `[2/4] 기능 찾아보는 중이에요`
- `[3/4] 실행하는 중이에요`
- `[4/4] 결과 정리하는 중이에요`

## TodoWrite 체크리스트 (2+ 태스크일 때만 · 있을 때만)

요청이 **2개 이상의 axhub 작업으로 쪼개질 때만** TodoWrite 로 태스크를 보여줘요 (예: "테이블 만들고 env 추가하고 로그 봐줘"). 한 번에 끝나는 단순 조회·단일 명령은 TodoWrite 없이 위 한 줄 알림만 해요 — 1줄짜리 체크리스트는 만들지 않아요. TodoWrite 도구가 host 에 노출됐을 때만 호출하고, 없으면 조용히 진행해요 (도구 가용성은 언급 안 해요).

clarity 는 카탈로그가 없어서 todos 도 **고정 목록이 아니라 요청을 쪼갠 실제 태스크에서 도출**해요 — 사용자 발화를 axhub 작업 단위로 나눠 한 항목씩 만들어요. 참고 shape ("테이블 만들고 env 추가해줘"):

```typescript
TodoWrite({ todos: [
  { content: "테이블 생성",   status: "in_progress", activeForm: "테이블 만드는 중" },
  { content: "환경변수 추가", status: "pending",     activeForm: "env 추가하는 중" }
]})
```

**태스크 하나가 끝날 때마다**(그 태스크의 탐색→실행→결과까지 끝나면) 전체 todos 배열로 다시 호출해 끝난 항목은 `completed`, 다음 항목은 `in_progress` 로 갱신해요 — 끝에 한꺼번에 말고 매 태스크 직후에요. 이전 스킬 todo 가 남아 있으면 patch 하지 말고 위 배열 전체로 교체해요. 종료 시 미완료 todo 0 개.
