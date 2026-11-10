# axhub plugin 에이전트 행동 정책

axhub plugin 스킬들이 지켜야 하는 행동 규칙을 한곳에 모은 기준 문서예요. 여러 문서에 같은 규칙이 다르게 적혀 있으면 이 문서가 기준이에요 — SKILL.md·README·CLAUDE.md 의 서술과 다르면 이 문서를 따라요. 규칙 블록의 `적용:`/`invariant:` 형식은 `tests/policy-parity.test.ts` 가 자동으로 읽어 검증해요.

블록 형식과 용어를 먼저 풀어둘게요:

- `적용:` — 규칙 복사본을 반드시 담아야 하는 파일 경로예요. 복수면 쉼표로 구분해요.
- `invariant:` — 각 적용 파일에 문자 그대로 존재해야 하는 핵심 문구예요. 쌍따옴표로 감싸고 쉼표로 구분해요. 자동 검사(parity 테스트)가 이 문구가 실제로 있는지 확인해서, 정책과 스킬 문서가 어긋나면 테스트가 실패해요.
- 규칙을 바꿀 때는 이 문서를 먼저 고치고, 적용 파일을 따라 고쳐요. parity 테스트가 어긋남을 잡아요.
- **스킬(skill)** — 플러그인이 상황별로 꺼내 쓰는 작업 설명서예요. `skills/*/SKILL.md` 파일 하나가 스킬 하나예요.
- **headless** — 사람이 대화로 답할 수 없는 자동 실행 환경이에요 (예: 예약 실행, 백그라운드 작업). 이 환경에서는 확인이 필요한 일을 실행하지 않고 멈춰요.
- **preview / dry-run** — 실제로 실행하기 전에 "이렇게 될 거예요"를 미리 보여주는 단계예요.

## AP-1 deploy 성공 선언 (deployment-record lane)
- 규칙: 일반 배포(deployment-record 방식 — 배포 한 건마다 기록 id 가 생기는 방식)의 성공은 그 id 에 대해 `axhub deploy verify` (배포가 정말 끝나서 접속 가능한지 확인하는 명령)가 최종 성공을 돌려줄 때만 선언해요. 배포 시작 명령의 화면 출력, 중간 상태 조회, watch 화면, "가장 최근 배포" 재검색 같은 간접 근거로는 성공이라고 말하지 않아요. 파일을 업로드하는 정적 파일 앱은 확인 경로가 달라서 AP-22 가 따로 맡아요 — 이 규칙은 deployment-record 배포에만 적용되고, Axhub self-hosted 저장소에 push 로 배포하는 정적 앱도 여기에 들어가요.
- 적용: skills/deploy/SKILL.md, skills/up/SKILL.md
- invariant: "axhub deploy verify"

## AP-2 배포 미리보기 (재배포는 진행 안내)
- 규칙: 실제 배포를 실행하기 전에 무엇이 어떻게 배포되는지(앱·환경·브랜치·커밋 또는 올릴 파일) 보여줘요. 환경은 `STAGING_ENABLED` 로 `운영`/`스테이징` 을 골라요. 이미 AXHub 에 있는 앱의 재배포는 이 미리보기를 질문이 아닌 진행 안내로 보여주고 확인을 받지 않아요 — 맞출 일은 AP-29 가 맡고, 사용자가 리허설을 원할 때만 dry-run 에서 멈춰요. 새로 만드는 배포(생성·가져오기)의 확인은 AP-12 가 맡아요. (2026-10 spec 681 전에는 재배포도 preview-confirm 승인을 받았어요.)
- 적용: skills/deploy/SKILL.md, skills/up/SKILL.md
- invariant: "STAGING_ENABLED", "승인 카드를 띄우지 않아요"

## AP-3 파괴적 변경 승인
- 규칙: 삭제, 롤백(이전 버전으로 되돌리기), `--force`/`--yes`/`--execute` 같은 강제 실행 옵션이 붙는 명령(파괴적 명령 — 실행하면 되돌리기 어려운 변경)은 대화로 승인을 1회 받은 뒤에만 실행해요. 목록·상태·로그 같은 단순 조회는 확인 없이 바로 실행해요. headless 에서는 파괴적 명령을 실행하지 않고 preview/summary 로 멈춰요. 이미 AXHub 에 있는 앱의 재배포(`deploy create --execute`·`up --execute`·`env set --from-dotenv --execute`)는 배포 이력으로 되돌릴 수 있어 이 승인의 예외이고 AP-12·AP-29 를 따라요.
- 권한을 새로 주는 grant 생성(커넥터 대상별 `read`/`write` 부여 등)처럼 **남의 접근 범위를 넓히는 변경**도 같은 승인 게이트를 써요 — 나중에 회수할 수 있어도 조회는 아니라서, 부여 대상·레벨을 한국어로 보여준 뒤 승인을 받아요. 부여에 쓸 정확한 플래그는 문서 카탈로그가 아니라 그 leaf 의 라이브 `--help` 로 확정해요.
- 적용: skills/clarity/SKILL.md
- invariant: "파괴적 변경은 승인"

## AP-4 diagnosis 읽기 전용
- 규칙: diagnosis 스킬은 배포가 왜 실패했는지 읽기 전용(조회만 하고 아무것도 바꾸지 않음)으로 진단해요. 재배포·롤백·새 배포 생성을 직접 실행하지 않고, 실행이 필요한 다음 행동은 담당 스킬이 이어받도록 자연어 안내(handoff)만 남겨요.
- 적용: skills/diagnosis/SKILL.md
- invariant: "읽기 전용", "절대 직접 실행하지 않아요"

## AP-5 development write 게이트
- 규칙: development 스킬의 코드 생성은 데이터 읽기(read)를 기본으로 해요. 테이블 구조 변경(스키마 변경) 같은 쓰기(write — 데이터나 구조를 바꾸는 일)는 미리보기 승인과 대상 존재 확인을 거친 뒤에만 실행해요. headless 에서는 아무것도 바꾸지 않아요.
- 적용: skills/development/SKILL.md
- invariant: "read 기본, write 게이트"

## AP-6 CLI preflight 게이트
- 규칙: bootstrap·deploy 스킬은 시작할 때 `axhub` CLI 가 설치돼 있는지, 사전 점검 명령(plugin-support preflight — 비행 전 점검처럼 필요한 기능이 도는지 미리 확인)이 동작하는지 확인해요. CLI 가 없거나 점검이 실패하면 멈추고 설치/업그레이드 방법을 안내하며, 이 확인을 절대 건너뛰지 않아요. 버전 숫자를 직접 비교하지 않고 "필요한 기능이 있는지"로 판단해요.
- 적용: skills/bootstrap/SKILL.md, skills/deploy/SKILL.md
- invariant: "plugin-support preflight"

## AP-7 skill 양보 라우팅
- 규칙: 각 스킬은 자기 담당 밖의 요청을 그 일을 맡은 스킬로 넘겨요(양보). 요청이 섞여 있으면 자기 몫만 끝내고 나머지는 담당 스킬로 넘겨요. 예를 들어 진단 중에 재배포가 필요해지면 진단은 여기서 끝내고 배포는 deploy 스킬이 이어받아요.
- import: 빈 폴더는 bootstrap, CLI 점검 실패는 setup, 배포 실패는 diagnosis 로 보내지만 `양보` 라는 낱말을 쓰지 않아서 아래 적용 목록에서 빠져요.
- 적용: skills/clarity/SKILL.md, skills/deploy/SKILL.md, skills/development/SKILL.md, skills/diagnosis/SKILL.md, skills/up/SKILL.md
- invariant: "양보"

## AP-8 setup 자동 bootstrap 금지
- 규칙: setup(CLI 설치·버전 업데이트·로그인·회사·git 점검) 스킬은 빈 폴더나 앱 설정 파일(manifest)이 없는 폴더를 발견해도 새 앱 생성(bootstrap)이나 배포를 자동으로 실행하지 않아요. 점검 결과 표로 끝내고, 다음 행동은 사용자가 정해요.
- 적용: skills/setup/SKILL.md
- invariant: "앱 만들기·배포는 하지 않아요"

## AP-9 clarity 공개 표면만
- 규칙: clarity 스킬은 숨김 내부 명령 그룹(`axhub plugin-support` — plugin 전용 통로라 예고 없이 바뀔 수 있는 명령들)을 탐색하거나 실행하지 않아요. 누구나 `--help` 로 볼 수 있는 공개 명령 표면(`--json-schema`/`--help`)만 사용해요.
- 적용: skills/clarity/SKILL.md
- invariant: "공개 표면만"

## AP-10 telemetry 옵트인
- 규칙: AI 활용 기록(`axhub axrouter` — 내 Claude Code 프롬프트·응답·툴콜을 팀 워크스페이스로 보내는 수집 기능)은 스킬이 묻거나 켜지 않아요. 사용자가 `axhub axrouter monitor` 로 직접 켜고, 워크스페이스 콘솔 동의 없이는 켜지지 않아요. (setup 이 onboarding 을 대체하기 전에는 onboarding 마무리에서 한 번 물었어요.)
- 적용: POLICY.md
- invariant: "AI 활용 기록", "동의 없이 켜지지 않아요"

## AP-11 비-axhub 맥락 라우팅 가드
- 규칙: axhub 를 명시하지 않은 일반 발화("배포해"·"업데이트해줘"·"로그 보여줘" 같은 generic 동사)는 axhub 맥락(대화의 axhub 언급·현재 폴더의 axhub 연결 manifest·직전 axhub 작업)이 있을 때만 스킬이 진행해요. 맥락이 없으면 실행·안내로 밀어붙이지 않고 axhub 사용 의사를 한 번 묻거나 종료해요 — 다른 axhub 스킬로 넘기지도 않아요. 배포할 곳을 말하지 않은 배포 발화만은 deploy 가 맥락 없이도 받아서, 이 "한 번 묻기"를 배포할 곳 카드로 해요 (AP-27). 이미 preview-confirm 승인(`지금 만들고 배포까지 진행할까요?`)이 backstop 인 bootstrap 은 frontmatter 게이트로만 적용하고 본문 질문은 생략해요 — 그래서 아래 invariant 검사 대상에서 빠져요. headless 에서는 묻지 않고 멈춰요.
- import: frontmatter 가 기존 앱을 axhub 에 올리는 발화로만 받고, 확인 한 번(`지금 이대로 고쳐서 axhub 에 올릴까요?`)이 backstop 이라 bootstrap 처럼 frontmatter 게이트로만 적용해요 — 그래서 아래 적용 목록에서 빠져요.
- 적용: skills/plugins/SKILL.md, skills/deploy/SKILL.md, skills/development/SKILL.md, skills/diagnosis/SKILL.md, skills/clarity/SKILL.md, skills/up/SKILL.md
- invariant: "axhub 맥락"

## AP-12 axhub 진입 확인 (preview 통합 게이트)
- 규칙: axhub 프로젝트가 확정된 상태에서 axhub 에 아직 없는 것을 새로 만드는 일(생성·가져오기·첫 연결·plugin App 게시)은 실행하기 전에 interactive preview 승인 하나가 axhub 진입 확인을 겸해요. 이미 AXHub 에 있는 앱의 재배포(`deploy`·`up`)는 이 승인을 받지 않아요 — AP-29 의 맞추기를 하고 물을 일이 있을 때만 그것을 물어요. 재배포는 배포 이력으로 되돌릴 수 있고, 답이 사실상 "응" 하나뿐인 확인은 막는 것 없이 마찰만 더해서예요(spec 681). headless 재배포는 물을 일이 없으면 실제로 배포하고, 있으면 아무것도 바꾸지 않고 멈춰요. `plugins`의 목록·다운로드는 read-only/no-clobber라 별도 실행 승인을 요구하지 않고, publish만 rights attestation과 execute를 결합해요. headless publish·생성은 preview에서 멈추고 어떤 lane에서도 승인을 조용히 건너뛰지 않아요.
- host 별 승인 프리미티브: 네이티브 선택 UI 는 Claude 판의 프리미티브이고, Codex 판의 프리미티브는 명시 텍스트 승인 1회예요 — 유효 승인은 preview 를 본 뒤 사용자가 안내된 canonical 승인 문구를 byte-exact 로 입력한 경우만이에요. preview 전에 미리 넣어 둔 문구, 유사 표현, 무응답은 승인이 아니에요. 이 선주입 무효 조항은 실측 A/B(조항 없는 게이트는 선주입 문구를 3/3 승인으로 오인, 조항 있으면 3/3 정지)로 필수 확인됐고, codex 파생 번들의 승인 문안에 기계 잠금돼요.
- bootstrap: 승인 질문 `지금 만들고 배포까지 진행할까요?` 와 headless 정지(물어야 할 게 생긴 순간 아무것도 만들지 않고 멈춤)를 본문 자기 문장으로 둬요. 공용 invariant 문구는 없어서 아래 적용 목록에서 빠지고, codex 판 invariant 로 검사해요.
- import: 확인 질문 `지금 이대로 고쳐서 axhub 에 올릴까요?`(올리기 / 그만두기)·빈 답 미승인·headless 는 `--headless` 미리보기까지만 하고 바꾸지 않고 멈춤을 본문 자기 문장으로 둬요. 공용 invariant 문구는 없어서 아래 적용 목록에서 빠지고, codex 판 invariant 로 검사해요.
- 적용: skills/plugins/SKILL.md
- invariant: "axhub 진입 확인", "승인을 조용히 건너뛰지 않아요"
- 적용(codex): plugins/axhub-codex/skills/bootstrap/SKILL.md, plugins/axhub-codex/skills/import/SKILL.md, plugins/axhub-codex/skills/plugins/SKILL.md
- 빈 답변 fail-closed (codex): 네이티브 선택 카드가 켜진 세션에서 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요. 이 규칙은 정책과 실행 스킬 본문의 첫 8,000B 두 채널이 겹쳐 소유해요 — 본문은 8,000B 에서 절단되니 규칙은 그 안에 둬요.
- invariant(codex): "미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요", "카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요"

## AP-16 상태 폴링 예산
- 규칙: 배포·생성 상태를 반복 확인하는 tool call(`deploy status`·`deploy verify`·`apps bootstrap-status` 재호출)은 한 요청당 폴링 예산 **최대 30회 또는 10분** 중 먼저 닿는 쪽까지만 반복해요. 예산에 닿으면 실패로 선언하지 않고 "아직 진행 중이에요" 와 이어서 확인할 명령을 안내하는 재개 요약으로 응답을 끝내며, deployment id/bootstrap id 는 그 안내에 보존해요. 이 예산 종료는 "terminal 전 응답 종료 금지" 규칙의 유일한 예외예요. CLI 자체의 `--watch` 상한과 별개로, 스킬 레벨 반복에는 항상 이 예산이 적용돼요.
- 대기 수단 우선: preflight 가 `capabilities.import.verify_wait` 를 true 로 보고하면 스킬 레벨 반복 대신 `deploy verify --wait --wait-interval 20s --wait-timeout 9m` 단일 호출로 이 예산을 CLI 안에서 소화해요. 이 호출은 명령 도구 timeout 을 600000ms 로 올려 실행해요 — 도구 기본 timeout(2분)에 먼저 끊기면 CLI 가 판정 exit 를 돌려주지 못하고, CLI 대기 9분은 도구 최대 10분 안에서 끝나요. exit 6·10 이나 도구 timeout 끊김은 실패가 아니라 재개 요약과 같은 verify 명령 한 줄로 끝내요. 스킬은 `sleep`·shell loop 를 쓸 수 없으므로 대기 수단 없이 같은 명령을 연달아 호출하면 30회 예산이 몇 초 만에 소진되고, 사용자 화면에는 같은 exit 6 이 실패한 명령처럼 도배돼요. 반복 호출은 그 capability 가 없는 구 CLI 의 fallback 이에요.
- bootstrap: 반복 호출 없이 `apps bootstrap-status --watch --watch-timeout 9m` 과 `deploy verify --wait` 단일 호출(명령 시간 제한 10분)로 기다리고, exit 6·10 은 실패가 아니라 같은 명령으로 다시 확인할 수 있다고 알리고 끝내요. 반복 폴링이 없어서 아래 적용 목록에서 빠져요.
- import: execute 한 번 뒤 `deploy verify --wait --wait-timeout 9m` 단일 호출(명령 시간 제한 10분)로 기다리고, 끊기면 다시 실행하지 않고 같은 확인 명령으로 이어가며, exit 6·10 은 실패가 아니라 같은 명령으로 다시 확인할 수 있다고 알리고 끝내요. 반복 폴링이 없어서 아래 적용 목록에서 빠져요.
- 적용: skills/deploy/SKILL.md, skills/up/SKILL.md
- invariant: "폴링 예산", "최대 30회 또는 10분", "--wait --wait-interval 20s --wait-timeout 9m", "600000ms"

## AP-17 CLI 경로 해석 (설치 여부 오판 금지)
- 규칙: bare `axhub` 호출 실패(command not found·exit 127)는 미설치 판정 근거가 아니에요. 부모 앱(Claude Desktop·VS Code·터미널 앱)이 물려준 오래된 PATH 때문에 설치된 CLI 를 못 찾는 상태가 macOS·Linux·Windows 모두에서 흔해요. 모든 스킬의 CLI 가드는 (1) `command -v axhub`, (2) 위치 파일 `~/.axhub/bin-path`(CLI 0.24.8+ 가 자기 설치 위치를 기록), (3) canonical 경로 `~/.axhub/bin/axhub`(Windows Git Bash 는 `.exe`) 순서로 실행 파일을 찾아요. 디스크에서 찾으면 재설치·setup 으로 돌려보내지 않고 그 절대경로로 `plugin-support repair-path --json` 을 실행해 영속 PATH 를 고친 뒤, 같은 세션의 남은 명령은 반환된 `bin_path` 절대경로로 이어가요 (이미 열린 셸의 PATH 는 OS 설계상 밖에서 못 고쳐요). 구 CLI 라 `bin_path` 가 없으면 찾은 절대경로를 그대로 써요. 세 경로 모두에서 실행 파일을 못 찾을 때만 미설치로 보고 setup 을 안내해요.
- repair-path 오류 결과: `repair-path --json` 이 `repaired:false` 와 `error` 를 돌려줘도(예: 샌드박스가 셸 rc 쓰기를 막은 `error_code: "write_denied"`, `sandbox: true`) `bin_path` 가 있으면 막힘이 아니에요 — 리포트·재설치 없이 그 절대경로로 남은 명령을 이어가요. exit 는 0 이에요(fail-open). 사용자가 PATH 를 영구 등록하고 싶다고 할 때만 `manual_line`(셸 rc 에 넣을 정확한 한 줄)을 알려 줘요. `bin_path` 까지 없을 때만 위 3-경로 탐색으로 찾은 절대경로를 그대로 써요.
- bootstrap·import: preflight 가 안 되면 setup 으로 보내고 멈춰요 — 경로 해석·`repair-path` 는 setup 이 맡아서 아래 적용 목록에서 빠져요.
- 적용: skills/plugins/SKILL.md, skills/clarity/SKILL.md, skills/deploy/SKILL.md, skills/development/SKILL.md, skills/diagnosis/SKILL.md, skills/up/SKILL.md, hooks/session-auto-update.sh, CLAUDE.md
- invariant: "bare `axhub` 실패는 미설치가 아니에요", "repair-path"

## AP-18 device flow 코드 선노출
- 규칙: GitHub device flow 가 필요한 순간에는 코드 노출이 사용자 행동의 전부예요. 코드를 몇 분씩 도는 saga 명령(`apps bootstrap --execute` 등) 안에만 두지 않아요 — 그 tool call 이 실패·거부·중단되면 stdout 이 사라져 사용자는 브라우저의 빈 코드 입력 화면만 보게 돼요. 코드가 안 보인 채 saga 가 끝나면 같은 `--execute` 를 다시 실행하지 않아요 — 새 device code 가 발급돼 이미 받은 코드가 무효가 돼요. 대신 즉시 끝나는 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link` 로 코드를 받아 본문에 `인증 URL:` 과 `입력 코드:` 두 줄로 먼저 노출하고, 승인 확인 뒤 `--resume-last` 로 이어가요. 재시도 구간도 같아요 — `--resume-last` 의 `device_code_pending` 응답은 `user_code`·`verification_uri`·`expires_at` 을 실어 주므로 재시도마다 그 두 줄을 다시 써요. 승인만이 이 루프를 끝내니, 코드 없이 `아직 대기 중` 만 반복하는 재시도는 사용자가 할 수 있는 일이 없어요. 이 필드가 없는 구 CLI 에서는 재노출을 건너뛰고 재시도만 이어가요.
- 코드 유실·만료 재발급: `github link` 는 저장된 pending device link 를 그대로 이어 줘요 — 사용자가 코드를 놓쳤거나 코드가 만료된 뒤 같은 명령을 다시 실행하면 이미 죽은 코드가 그대로 돌아와서, 아무리 승인해도 연동이 영영 안 풀려요. 코드를 못 봤다·사라졌다·만료됐다는 신호가 있거나 같은 코드로 승인 확인이 계속 pending 이면 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh` 로 새 코드를 발급해 두 줄을 다시 써요. 반대로 사용자가 지금 보고 있는 코드가 아직 유효하면 `--fresh` 를 붙이지 않아요 — 그 플래그가 그 코드를 무효로 만들어요. `--fresh` 를 모르는 구 CLI 는 exit 64(사용법)로 거부하니 플래그 없이 한 번만 다시 실행하고 setup 스킬로 CLI 업그레이드를 안내해요.
- bootstrap: saga 밖에서 `axhub github link --json` 으로 코드를 먼저 받아 `https://github.com/login/device 에서 코드 <코드> 를 입력해 주세요.` 로 알리고, `axhub github link --wait-approval 90s --json` 으로 기다리며, 만료되면 `--fresh` 로 새로 받아요. 문구가 달라 아래 적용 목록에서 빠져요.
- 적용: skills/clarity/SKILL.md
- invariant: "입력 코드:", "github link", "github link --fresh"

## AP-15 앱 소유자·계정 불일치 비판정
- 규칙: 앱을 만든 계정과 지금 로그인한 계정이 달라 보여도(앱 정보의 owner 표시, 멤버 목록, git 커밋 이메일 등), 스킬은 그 불일치를 스스로 판정해 막거나 "앱 소유자에게 물어보세요" 같은 확인 절차를 만들어내지 않아요. 배포 권한(인가 — 이 계정이 이 앱을 배포할 수 있는지)의 판정은 CLI/백엔드 몫이에요. 사용자의 구두 승인을 권한 근거로 쓰지 않아요 — "소유자가 배포해도 된대요" 같은 말로는 권한이 생기지 않아요. CLI 가 권한 부족(exit 8, `axhub_app_forbidden` — 앱 owner/admin 권한 검사 실패)으로 막을 때만 앱 소유자/관리자에게 멤버 권한 부여를 요청하도록 안내하고 멈추며, 권한이 실제로 생겼는지는 말이 아니라 같은 명령의 재시도 성공으로만 확인해요.
- 적용: skills/deploy/SKILL.md, skills/deploy/references/error-empathy-catalog.md
- invariant: "axhub_app_forbidden", "구두 승인을 권한 근거로 쓰지 않아요"

## AP-20 host-derivation (claude-first 소스)
- 규칙: 스킬·훅·정책 문안의 source of truth 는 claude-first 단일 트리(`skills/`·`hooks/`·루트 문서)예요. codex 표면은 `scripts/build-plugin-bundle.ts` 의 `--host codex` transform(치환 테이블 `CODEX_SUBSTITUTIONS` + `codex-overrides/` 스왑)이 소유하고, 파생 번들(`plugins/axhub-codex/`)은 생성물이라 직접 수정하지 않아요 — 고칠 것은 소스나 override 를 고치고 재생성해요. 소스에 host-종속 문안(제품명·host 명령·host UI 언급)을 새로 추가할 때는 같은 변경에서 치환 테이블 또는 override 를 갱신할 의무가 있어요 — codex 번들 FORBIDDEN 게이트(`tests/codex-bundle.test.ts`)가 누락을 잡아요. wrapper 훅 스크립트의 로직 변경은 CHANGELOG 에 명시해요 — codex 의 훅 신뢰 대상이 command(스크립트 경로)라 wrapper 본문 갱신은 재신뢰 프롬프트 없이 배포되기 때문이에요 (KTD6).
- codex 훅 변환: codex 는 hooks.json 의 `shell` 필드를 런타임에서 무시하고 훅 command 를 기본 셸 `-c` 로 실행해요 (U1 실측 0.148). 우리 wrapper 는 `bash "…/session-*.sh"` 명시 호출이라 무영향이고, Windows 는 파생 번들의 `commandWindows` 가 `exit 0` 으로 훅을 조용히 건너뛰어요 — Windows codex 는 commandWindows 를 Windows PowerShell 5.1 로 바로 실행해서(2026-11-03 실측) `&&`·`||` 가드가 파서 오류로 매번 Hook failed 를 냈고, bash 훅은 PowerShell·WSL bash 어느 쪽으로도 돌릴 수 없어요.
- 적용: CLAUDE.md, package.json
- invariant: "plugins/axhub-codex"

## AP-21 app-backed plugin marketplace
- 규칙: `plugins` 스킬은 deploy_method=plugin인 일반 App의 canonical AppID·slug만 사용해 목록·exact download·host install·publish·게시 심사 신청·상태 확인을 처리해요. 모든 mode 전에 `axhub plugin submit --help` 성공과 `axhub plugin publish --help` 의 `--submit` 으로 CLI 0.50.0 이상을 확인하고, 없으면 `axhub update apply --execute --yes` 와 최소 버전을 안내한 뒤 setup으로 양보해요. 목록은 `plugin.current_servable_version` summary를 페이지 단위로 읽고 download 결과에는 version_id를 만들지 않아요. `axhub plugin install`은 download 요청과 구분해 exact version·`claude|codex` host를 고른 offline preview 뒤, 목록의 제공 버전·`risk_inventory` 를 보여주고 새 승인에서만 `--execute --yes`를 붙여요. CLI는 traversal·symlink·duplicate path·archive bomb·manifest identity를 검사하고 AxHub secret을 host process에서 제거한 뒤 `~/.axhub/plugins/` local marketplace와 host 공식 plugin CLI로 설치해요. 목록·다운로드·설치는 OAuth 또는 broad PAT, publish·submit execute는 `plugins:read` + `plugins:write` scoped PAT·rights attestation·명시 승인만 허용하고, PAT를 미리 발급하지 않고 로그인한 채 publish를 실행해요 — `plugin_pat_required` 면 CLI `next_action` 대로 `login` 은 `axhub auth login` 뒤 재실행, `issue_plugin_pat` 은 token을 화면에 내지 않는 `axhub auth pat issue … --use` 를 그대로 실행해요. 성공은 `upload_complete`(또는 `publish --submit` 의 `review_submitted`)·installable=false이며, `axhub plugin submit`(또는 App Console)으로 게시 심사를 신청하고 Console Review 승인으로 `axhub plugin manage status` 의 상태가 `servable` 이 되기 전에는 release 완료가 아니에요.
- 적용: skills/plugins/SKILL.md, POLICY.md
- invariant: "plugin.current_servable_version", "`version_id`를 만들거나 보고하지 않아요", "download 요청과 install 요청을 구분", "--execute --yes", "Publish execute에는 OAuth나 broad PAT 대신", "PAT를 미리 발급하지 않", "submit_plugin_version_for_review", "upload_complete", "axhub plugin submit"

## AP-22 static 앱 성공 선언
- 규칙: 파일을 업로드하는 정적 파일 앱(GitHub 작업공간의 deploy_method=static — 서버 없이 파일만 올리는 방식)의 배포 성공은 activate 결과의 `active_release_id` (새 버전이 실제로 켜졌다는 표시)로 선언하고 `deploy verify` 는 부르지 않아요 — 업로드 static 은 deployment-record 가 아니라 release 라 verify 가 404 예요. Axhub self-hosted 저장소의 정적 앱은 소스를 push 해 서버가 빌드하는 deployment-record 라 AP-1 을 따라요. 이 규칙은 static lane 을 실제로 소유한 skill 에만 적용해요. AP-1 에서 떼어낸 이유는, 두 lane 의 성공 선언을 한 규칙에 묶어 두면 static lane 이 없는 skill 을 그 규칙에 넣을 수 없기 때문이에요.
- 주소·인증서: static 앱(두 lane 모두)의 성공 안내는 비어 있지 않은 정적 사이트 주소만 최종 주소로 보여줘요 — 사이트가 아직 없을 때의 앱 도메인 주소(`*.axhub.app` 등)는 정적 앱 주소가 아니에요. self-hosted static 은 verify 성공 뒤(import 는 evidence 가 `public_url_pending: true` 이거나 `--verify-wait none` early return 일 때) `axhub apps static site get --app <app> --tenant <t> --json` 을 한 번 읽어 `data.public_url`·`data.tls_ready` 를 써요 — 첫 게시 전 404(exit 5)는 실패가 아니라 아직 준비 중이에요. 나중에 주소를 물어도 `apps get` 의 `access_url` 이 아니라 이 명령을 다시 읽어요. `tls_ready: false`·`tls_pending: true` 는 실패가 아니라 HTTPS 인증서 발급 중(약 2~3분)이에요 — 그 사이 브라우저 인증서 경고를 배포 실패로 보지 말고 잠시 뒤 열린다고 안내해요.
- 주소 충돌: static 배포가 다른 앱(삭제 뒤 30일 안이라 되돌릴 수 있는 앱 포함)이 쥔 주소(`apply.static_host_taken`, 구 서버는 `apply.static_release_failed` + detail `app_static_sites_host_key`)나 영구 삭제 뒤 격리 기간(기본 7일) 중인 주소(`apply.static_host_quarantined`, 구 서버 detail `static host quarantined`)로 실패하면 같은 배포를 다시 하지 않아요 — 재시도로는 안 풀려요. 분기는 `deploy verify --json` 의 `failure_hint` 가 null 이 아닌지로 해요(verify 에는 실패 code 가 없어요; `deploy status --json` 은 `failure_reason.code`/`detail` 과 같은 `failure_hint` 를 함께 줘요). import 는 `recovery_action` 을 따라요. 첫 게시 전이라 주소를 바꿀 수 있으면 다른 주소를 제안해 승인 뒤 `axhub apps update <app> --subdomain <new>` 로 바꾸고 한 번만 다시 배포해요 — self-hosted 는 실패한 커밋이 이미 원격에 있어 push 할 게 없으니 승인받은 빈 커밋(`git commit --allow-empty`)을 push 해 새 배포를 만들어요. 주소가 잠겼으면(412 `subdomain_locked`, 또는 주소를 자동 생성하는 workspace) 승인 뒤 다른 주소로 새 앱을 만들어요(격리 중인 주소는 격리 기간이 끝난 뒤 다시 배포해도 돼요).
- import: 배포는 `plugin-support import --mode execute` 한 번이 맡고 본문은 `deploy verify --wait` 확인과 CLI `message_ko`·`recovery_action` 안내만 둬요. 업로드 static 의 `active_release_id`·정적 사이트 주소·주소 충돌 처리를 본문에 따로 적지 않아서 아래 적용 목록에서 빠져요.
- 적용: skills/deploy/SKILL.md
- invariant: "active_release_id"

## AP-23 app git backend 선판정
- 규칙: 저장소 provider 대사나 mutation보다 먼저 public CLI JSON으로 effective backend를 확인해요. resume/existing은 `axhub apps get <app> --json`, fresh bootstrap은 read-only `axhub apps git-backend --tenant <tenant> --json`을 쓰고 app row를 먼저 만들지 않아요(fresh 판정 문구는 AP-24 가 검사해요). 기존 코드 import는 `plugin-support import`가 같은 입력으로 경로를 정하고(`selfhosted_repo_push`) 읽기 실패면 GitHub로 추측하지 않아요. 판정 입력은 top-level `git_backend.backend`·`git_backend.source`뿐이며(fresh 는 서버가 주는 `git_backend.decision` 도 — AP-24) Gitea API·C1 HTTP·remote URL을 보지 않아요. selfhosted는 계정 연동·device flow·GitHub App 설치 대사를 노출하지 않고, deploy는 static을 포함해 일반 `git push`의 webhook deployment id를 exact verify해요 — 빈·무관한 폴더만 `axhub repo clone`하고, 이 앱 저장소에 안 붙은 로컬 소스 폴더(`axhub repo clone <app> .`가 `repository.destination_exists`)는 clone 대신 `import`로 넘겨요. self-hosted static은 preflight `capabilities.import.selfhosted_static`이 true여야 하고 `axhub.yaml`에 `build.deploy_method: static`과 명시적 `build.static_output_dir`(루트면 `.`)을 둬요. GitHub와 `legacy_github`는 static upload lane을 포함해 기존 gate/upload/create 경로를 유지해요.
- 적용: skills/deploy/SKILL.md
- invariant: "axhub apps get <app> --json", "git_backend.backend=selfhosted"
- 적용(codex): plugins/axhub-codex/skills/deploy/SKILL.md
- invariant(codex): "axhub apps get <app> --json", "git_backend.backend=selfhosted"

## AP-24 fresh git backend 명시 선택
- 규칙: 서버가 `git_backend.decision`(spec 706 — 회사 모드 무조건 Axhub 저장소·무조건 GitHub·둘 다)을 주면 fresh 저장 위치는 그 판정만 따라요. `use` 는 묻지 않고, `ask`(둘 다 모드에서 GitHub 를 연결한 사람)만 `코드를 어디에 저장할까요?` 를 `options` 순서·`recommended` 그대로 묻고, `link_github` 는 `axhub github link` 로 연결한 뒤 다시 읽어요. 내 GitHub 계정이 둘 이상일 때만 `어느 GitHub 계정에 둘까요?` 를 물어요. 고른 key 는 preview·execute 에 `--git-choice <key>`(+`--github-owner`)로 넘기고 `remembered`·추천값은 쓰지 않아요. 판정에 없는 저장 위치를 말하면 바꾸지 않고 멈춰요. preflight `capabilities.self_hosted_git.git_policy_decision` 이 없는 CLI 는 업데이트로 보내요. 물어야 하는데 답할 수 없는 headless 는 아무것도 만들지 않고 멈춰요. 판정이 없는 구 서버용 `--git-backend` 분기와 `SELECTED_GIT_BACKEND`·`init-resume` 이어하기는 2026-11 bootstrap·scaffold 통합 때 없앴어요 — 이어 하기는 같은 이름의 내 앱이 이미 있으면 생성 단계를 건너뛰는 것으로 대신해요.
- 적용: skills/bootstrap/SKILL.md
- invariant: "`git_backend.decision`", "git_policy_decision", "코드를 어디에 저장할까요?", "--git-choice"
- 적용(codex): plugins/axhub-codex/skills/bootstrap/SKILL.md
- invariant(codex): "`git_backend.decision`", "git_policy_decision", "코드를 어디에 저장할까요?", "--git-choice"

## AP-25 배포 환경 표시 (스테이징을 운영으로 오인 금지)
- 규칙: 스테이징 옵트인 앱(`apps get` 의 `staging_enabled=true` — spec 016)은 push/manual 배포가 스테이징에만 반영되고 운영은 심사 승인 뒤 promote 로만 바뀌어요. 그래서 배포 환경은 CLI profile 이 아니라 `staging_enabled` 로 판단해요. preview 카드의 환경 라벨은 `staging_enabled` 가 true 면 `스테이징`, 아니면 `운영` 이에요. verify 성공(exit 0) 뒤 성공 요약은 `deploy verify --json` 의 `environment` 를 읽어요 — `staging` 이면 "스테이징에 반영됐고 운영에는 아직 반영되지 않았어요" 와 다음 단계(심사 신청 `axhub publish --app <app> --deployment-id <id> --execute`, 승인은 Console Review, 승인 뒤 promote 자동)를 같은 성공 블록에서 안내하고 "운영 반영 완료" 라고 말하지 않아요. `environment` 가 없거나 null(구 CLI·앱 조회 실패)이면 운영으로 간주하지 않고 `apps get` 의 `staging_enabled` 로 같은 판단을 해요. 심사 신청은 사용자가 요청할 때만 실행해요.
- import: 결과 표의 주소 줄에 `(스테이징이면 운영 반영은 심사 뒤)` 로만 적고 `environment` 분기·심사 신청 명령 안내는 두지 않아서 아래 적용 목록(공용·codex)에서 빠져요. 확인하지 못한 항목은 됐다고 표시하지 않아요.
- 적용: skills/deploy/SKILL.md, skills/up/SKILL.md
- invariant: "staging_enabled", "스테이징", "axhub publish --app"
- 적용(codex): plugins/axhub-codex/skills/deploy/SKILL.md, plugins/axhub-codex/skills/up/SKILL.md
- invariant(codex): "staging_enabled", "스테이징", "axhub publish --app"

## AP-26 조용한 백그라운드 자동 업데이트 (묻지 않음)
- 규칙: auto-update 훅은 사용자에게 묻지 않고 `"async": true` 백그라운드 worker(`hooks/session-auto-update.sh`)로 axhub CLI 와 플러그인을 적용해요. 훅 계열 중 유일하게 네트워크·바이너리 교체를 하는 예외이고, 다른 전경 훅은 여전히 네트워크 0 이에요. worker 는 kill switch(`AXHUB_NO_AUTO_UPDATE` / `~/.axhub/config/no-auto-update`) → dev 가드 → AP-17 3-경로 → 24h throttle → `mkdir` lock 순으로 gate 를 밟고, `axhub update check --field-expr` 1회로 판정해 `has_update && !disabled && !is_downgrade` 면 `axhub update apply --execute --yes --json`, `plugin.has_update` 면 host 의 plugin update 를 실행해요. 옛 레포 서명만 믿는 0.44.1 이하 CLI 는 서명 신원 브리지 포인터(`AXHUB_UPDATE_FEED_URL=https://cli.axhub.ai/identity-bridge`)로 두 서명을 다 믿는 0.44.2 를 먼저 받고 같은 run 에서 최신으로 이어요 — 버전 포인터만 바꾸고 서명 검증은 우회하지 않아요. 0.46.10 미만 CLI 는 다운로드 전체에 전역 timeout(기본 30초)을 걸어 느린 회선에서 exit 10 이 반복되므로(#386) 그 CLI 가 받는 hop(브리지·본 apply)에만 `AXHUB_TIMEOUT=10m` 을 붙여요 — 0.46.10 이상은 아카이브에 10분 하한이 있어 그대로 둬요. 알림은 실제로 바뀐 게 있을 때만 세션당 최대 한 줄이고 질문이 없어요 — 에이전트는 다음 응답 앞머리에 그 한 줄만 말하고 버전 확인 명령을 실행하지 않아요. 보안 검증 실패(exit 14/66)는 반드시 1회 안내하고 같은 버전은 재시도하지 않아요. 침묵을 최신으로 읽지 않아요 — 실패는 `~/.axhub/cache/auto-update.log` 에 남겨요. 플러그인 적용 확인은 restart-confirm 훅이 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 의 로드 버전을 marker 와 비교해 에이전트 명령 없이 닫아요. 이 훅 경로에서 에이전트가 실행하는 명령은 0개예요.
- 적용: hooks/session-auto-update.sh, hooks/session-restart-confirm.sh, CLAUDE.md, POLICY.md
- invariant: "묻지 않", "no-auto-update", "AXHUB_NO_AUTO_UPDATE"
- 적용(codex): plugins/axhub-codex/hooks/session-auto-update.sh, plugins/axhub-codex/hooks/session-restart-confirm.sh, plugins/axhub-codex/POLICY.md
- invariant(codex): "묻지 않", "no-auto-update", "AXHUB_NO_AUTO_UPDATE"

## AP-27 배포할 곳 추천 카드 (다른 곳으로 알아서 배포 금지)
- 규칙: 배포할 곳을 말하지 않은 배포 요청("배포해줘"·"올려줘")은 axhub 맥락이 없어도 deploy 스킬이 받아요. 폴더가 axhub 에 연결돼 있으면 기존 preview 카드가 그 확인을 겸하고, 연결돼 있지 않으면 선택 카드로 `어디에 배포할까요?` 를 한 번 물어요 — `axhub (추천)`·`다른 곳`. 사용자가 고르거나 배포할 곳을 직접 말하기 전에는 GitHub Pages·Vercel 같은 다른 배포를 알아서 시작하지 않아요. 교육 현장에서 배포 요청이 묻지 않고 GitHub Pages 로 나가 수강생마다 결과가 갈리던 문제를 막아요. 사용자가 다른 배포 대상을 직접 말하면 deploy 는 진입하지 않고, headless 에서는 묻지 않고 멈춰요.
- 적용: skills/deploy/SKILL.md
- invariant: "어디에 배포할까요?", "알아서 시작하지 않아요"
- 적용(codex): plugins/axhub-codex/skills/deploy/SKILL.md
- invariant(codex): "어디에 배포할까요?", "알아서 시작하지 않아요"

## AP-29 배포 전 맞추기 (재배포는 묻지 않고 맞춰서 올림)
- 규칙: 이미 AXHub 에 있는 앱을 다시 올릴 때(`deploy`·`up`) 승인 카드 대신 배포 전 맞추기를 해요. (1) CLI 확정 판정 `axhub deploy check --app <app> --json` 을 받고 (2) preflight 의 `capabilities.readiness.context_url` 배포 기준 문서를 한 번 읽고(못 받으면 deploy 의 `references/deploy-rules.md` 요약으로 대신) (3) 그 규칙으로 앱 코드를 보고 빌드 단계가 있으면 로컬 빌드를 한 번 돌려요 — 테스트·예시·README 흔적은 문제로 보지 않아요. 묻는 일은 셋뿐이에요: 어디서도 값을 찾을 수 없는 필수 설정값(값을 대화로 받지 않고 `.env` 에 넣어 달라고 부탁해요), 안 되는 6가지나 문서의 `물어봄` 규칙(외부 DB·메일·웹훅·AI 등)에 처음 해당한 것, 처음 올릴 때 새로 정해지는 것. 나머지 — `.env` 값 등록(`axhub env set --from-dotenv … --only … --execute`, AI 는 값을 읽지 않아요)·포트·`0.0.0.0` 바인딩·상태 확인 경로·빌드 오류 수정·커밋·push — 는 묻지 않고 해요. GitHub 에 연결된 앱은 push 하는 순간 자동 배포가 시작돼 CLI 검사를 거치지 않으므로, 설정값 등록과 수정 커밋을 반드시 push 전에 끝내요. 접속 주소가 없는 앱(`deploy check` 의 `subdomain_not_configured` 항목, 이때 `error.subcode` 는 `deploy.subdomain_not_configured`)은 push 해도 배포가 시작되지 않아요 — 주소는 첫 운영 배포 뒤 바꿀 수 없어 맞출 일이 아니라 따로 확인받아 정하는 일이고(위 셋과 별개인 복구 질문), 정하기 전에는 push 하지 않아요. "그대로" 답은 앱 폴더 `.axhub/deploy-answers.json` 의 `keep` 에 남겨 다음 배포에 다시 묻지 않고, 이 파일은 커밋하지 않아요. headless 는 물을 일이 하나라도 있으면 아무것도 바꾸지 않고 멈춰요. 끝나면 `이렇게 맞춰서 올렸어요` 와 맞춘 것을 알려요. preflight 에 `capabilities.readiness.deploy_check` 가 없는 구 CLI 면 업데이트를 안내하고 멈춰요.
- 적용: skills/deploy/SKILL.md, skills/up/SKILL.md
- invariant: "승인 카드를 띄우지 않아요", "--from-dotenv", "context_url", ".axhub/deploy-answers.json", "이렇게 맞춰서 올렸어요"
- 적용(codex): plugins/axhub-codex/skills/deploy/SKILL.md, plugins/axhub-codex/skills/up/SKILL.md
- invariant(codex): "승인 카드를 띄우지 않아요", "--from-dotenv", "context_url", ".axhub/deploy-answers.json", "이렇게 맞춰서 올렸어요"

## AP-30 첫 올리기 점검·개발 중 배포 기준
- 규칙: 처음 올리는 앱(`import`)은 확인 전에 AP-29 와 같은 점검을 해요 — preflight `capabilities.readiness.context_url` 기준 문서를 `curl -fsSL` 로 원문 그대로 한 번 읽고(요약하는 웹 읽기 도구는 쓰지 않아요, 기능 문서는 `https://docs.axhub.ai/llms.txt` 목차로 찾아요), CLI 미리보기·`deploy check`·다른 서비스 의존(외부 DB·로그인 등) 찾기·로컬 빌드 1회 뒤 찾은 것을 바로 고칠 것·물을 것·사용자가 직접 할 일로 나눠요. 한 화면에 앱 이름·주소·저장 위치·배포 방식과 그 셋을 보여주고 물을 것을 모아 확인은 1회예요(올리기 / 그만두기). 외부 DB·로그인은 그대로 연결하지 않고 axhub 것으로 바꾸기(추천)·그대로 쓰기를 물어요. 확인 뒤 덩어리마다 커밋하며 고치고, 다시 비교(`deploy check`·로컬 빌드)를 통과해야 올려요. `.env` 에 있는 설정값은 execute 의 `--env-from-dotenv .env --env-only <이름,…>` 로 넘겨 CLI 가 앱을 만든 직후·첫 배포 전에 등록해요(첫 배포가 설정값 누락으로 실패하지 않게). 앱 기능을 만드는 `development` 는 코드를 쓰기 전에 같은 기준 문서를 같은 방법으로 읽고 안 되는 6가지를 처음부터 피해요(예: 데이터 저장은 AXHub DB). development 는 문서를 못 받으면 같은 요약 파일로 대신해요.
- 적용: skills/import/SKILL.md, skills/development/SKILL.md
- invariant: "capabilities.readiness.context_url", "curl -fsSL"
- 적용(codex): plugins/axhub-codex/skills/import/SKILL.md, plugins/axhub-codex/skills/development/SKILL.md
- invariant(codex): "capabilities.readiness.context_url", "curl -fsSL"

## AP-31 에이전트 샌드박스 쓰기 거절
- 규칙: 홈 폴더(`$HOME`) 아래를 쓰는 명령 — `auth login`, `axhub --tenant <tenant> git setup --json`, `plugin-support repair-path --json`, self-hosted `plugin-support import --mode execute`(안에서 git setup 을 해요) — 은 에이전트 샌드박스(Codex workspace-write 등) 안이면 첫 실행부터 권한 상승(샌드박스 밖)으로 실행해요. `repo clone` 은 `git setup` 이 저장한 자격증명을 읽고 대상 폴더에만 써서 미리 권한 상승하지 않아요. subcode `local.write_denied`(exit 64, 구 CLI 는 exit 1 `Operation not permitted`)는 CLI 버그가 아니라 예상된 거절이에요. 샌드박스 안에서 막혔을 때만(hint 가 에이전트 샌드박스를 말해요, `CODEX_SANDBOX`·`CODEX_SANDBOX_NETWORK_DISABLED` 설정) 같은 명령을 권한 상승으로 한 번만 다시 실행해요 — 샌드박스 안 재실행은 다시 거절돼요. 이미 권한 상승이었거나 hint 에 `check that you own <path>` 가 있으면 재실행하지 않고, 그 경로의 소유자·권한(예: `sudo axhub` 가 남긴 파일)을 고치도록 사용자에게 안내해요. hint 앞에 맥락(`failed to save PAT: cannot write <path>: …`)이 붙을 수 있어 시작 문구가 아니라 subcode 로 판정해요. import 는 `recovery_action` 이 이 구분을 담아요. 같은 세션에서 `git setup` 이 이미 성공했으면 push 전에 다시 하지 않아요(push 는 저장된 자격증명을 읽기만 해요).
- bootstrap: 에이전트 샌드박스 안이면 `git setup` 을 처음부터 권한 상승으로 실행한다고만 적고 subcode 재시도 규칙은 두지 않아서 아래 적용 목록에서 빠져요.
- 적용: skills/deploy/SKILL.md, skills/import/SKILL.md
- invariant: "local.write_denied"
