# axhub plugin 에이전트 행동 정책

axhub plugin 스킬들이 지켜야 하는 행동 규칙을 한곳에 모은 기준 문서예요. 여러 문서에 같은 규칙이 다르게 적혀 있으면 이 문서가 기준이에요 — SKILL.md·README·CLAUDE.md 의 서술과 다르면 이 문서를 따라요. 규칙 블록의 `적용:`/`invariant:` 형식은 `tests/policy-parity.test.ts` 가 자동으로 읽어 검증해요.

블록 형식과 용어를 먼저 풀어둘게요:

- `적용:` — 규칙 복사본을 반드시 담아야 하는 파일 경로예요. 복수면 쉼표로 구분해요.
- `invariant:` — 각 적용 파일에 문자 그대로 존재해야 하는 핵심 문구예요. 쌍따옴표로 감싸고 쉼표로 구분해요. 자동 검사(parity 테스트)가 이 문구가 실제로 있는지 확인해서, 정책과 스킬 문서가 어긋나면 테스트가 실패해요.
- 규칙을 바꿀 때는 이 문서를 먼저 고치고, 적용 파일을 따라 고쳐요. parity 테스트가 어긋남을 잡아요.
- **스킬(skill)** — 플러그인이 상황별로 꺼내 쓰는 작업 설명서예요. `skills/*/SKILL.md` 파일 하나가 스킬 하나예요.
- **headless** — 사람이 대화로 답할 수 없는 자동 실행 환경이에요 (예: 예약 실행, 백그라운드 작업). 이 환경에서는 확인이 필요한 일을 실행하지 않고 멈춰요.
- **preview / dry-run** — 실제로 실행하기 전에 "이렇게 될 거예요"를 미리 보여주는 단계예요.

## AP-1 deploy 성공 선언
- 규칙: 일반 배포(deployment-record 방식 — 배포 한 건마다 기록 id 가 생기는 방식)의 성공은 그 id 에 대해 `axhub deploy verify` (배포가 정말 끝나서 접속 가능한지 확인하는 명령)가 최종 성공을 돌려줄 때만 선언해요. 배포 시작 명령의 화면 출력, 중간 상태 조회, watch 화면, "가장 최근 배포" 재검색 같은 간접 근거로는 성공이라고 말하지 않아요. 정적 파일 앱(deploy_method=static — 서버 없이 파일만 올리는 방식)은 확인 경로가 달라서, activate 결과의 `active_release_id` (새 버전이 실제로 켜졌다는 표시)로 성공을 선언하고 verify 는 부르지 않아요.
- 적용: skills/deploy/SKILL.md
- invariant: "axhub deploy verify", "active_release_id"

## AP-2 deploy preview-confirm gate
- 규칙: 실제 배포를 실행하기 전에 무엇이 어떻게 배포되는지 미리보기(preview)를 보여주고 사용자 확인을 받아요. headless 에서는 실행하지 않고 dry-run/preview 로 멈춰요.
- 적용: skills/deploy/SKILL.md
- invariant: "preview-confirm"

## AP-3 파괴적 변경 승인
- 규칙: 삭제, 롤백(이전 버전으로 되돌리기), `--force`/`--yes`/`--execute` 같은 강제 실행 옵션이 붙는 명령(파괴적 명령 — 실행하면 되돌리기 어려운 변경)은 대화로 승인을 1회 받은 뒤에만 실행해요. 목록·상태·로그 같은 단순 조회는 확인 없이 바로 실행해요. headless 에서는 파괴적 명령을 실행하지 않고 preview/summary 로 멈춰요.
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
- 적용: skills/clarity/SKILL.md, skills/deploy/SKILL.md, skills/development/SKILL.md, skills/diagnosis/SKILL.md, skills/import/SKILL.md, skills/update/SKILL.md
- invariant: "양보"

## AP-8 onboarding 자동 bootstrap 금지
- 규칙: onboarding(첫 설정 안내) 스킬은 빈 폴더나 앱 설정 파일(manifest)이 없는 폴더를 발견해도 새 앱 생성(bootstrap)을 자동으로 실행하지 않아요. 지금 환경이 얼마나 준비됐는지 정리한 Ready card 로 끝내고, 다음 행동은 사용자가 정해요.
- 적용: skills/onboarding/SKILL.md
- invariant: "No automatic bootstrap"

## AP-9 clarity 공개 표면만
- 규칙: clarity 스킬은 숨김 내부 명령 그룹(`axhub plugin-support` — plugin 전용 통로라 예고 없이 바뀔 수 있는 명령들)을 탐색하거나 실행하지 않아요. 누구나 `--help` 로 볼 수 있는 공개 명령 표면(`--json-schema`/`--help`)만 사용해요.
- 적용: skills/clarity/SKILL.md
- invariant: "공개 표면만"

## AP-10 telemetry 옵트인
- 규칙: AI 활용 기록(`axhub axrouter` — 내 Claude Code 프롬프트·응답·툴콜을 팀 워크스페이스로 보내는 수집 기능)은 onboarding 이 무엇이 수집되는지 설명하고 물어본 뒤, 사용자가 켜기를 고를 때만 켜요 — 동의 없이 켜지 않아요. 거절하면 같은 온보딩에서 다시 묻지 않고 나중에 켜는 방법만 알려줘요. headless(사람이 답할 수 없는 자동 실행 환경)에서는 묻지도 켜지도 않아요.
- 적용: skills/onboarding/SKILL.md
- invariant: "AI 활용 기록", "동의 없이 켜지 않아요"

## AP-11 비-axhub 맥락 라우팅 가드
- 규칙: axhub 를 명시하지 않은 일반 발화("배포해"·"업데이트해줘"·"로그 보여줘" 같은 generic 동사)는 axhub 맥락(대화의 axhub 언급·현재 폴더의 axhub 연결 manifest·직전 axhub 작업)이 있을 때만 스킬이 진행해요. 맥락이 없으면 실행·안내로 밀어붙이지 않고 axhub 사용 의사를 한 번 묻거나 종료해요 — 다른 axhub 스킬로 넘기지도 않아요. 이미 preview-confirm 승인이 backstop 인 bootstrap 은 frontmatter 게이트로만 적용하고 본문 질문은 생략해요. headless 에서는 묻지 않고 멈춰요.
- 적용: skills/onboarding/SKILL.md, skills/bootstrap/SKILL.md, skills/deploy/SKILL.md, skills/import/SKILL.md, skills/development/SKILL.md, skills/diagnosis/SKILL.md, skills/clarity/SKILL.md, skills/update/SKILL.md
- invariant: "axhub 맥락"

## AP-12 axhub 진입 확인 AUQ
- 규칙: axhub 프로젝트가 확정된 상태에서 배포·생성·가져오기를 실행하기 전에, interactive 에서는 "axhub로 진행할지"를 AskUserQuestion 으로 한 번 더 확인해요 — "무엇을·어떻게"를 묻는 기존 preview 승인과 별개인 진입 게이트예요. deploy·import 는 preview card 앞에 별도 AUQ 를 두고, byte 예산이 포화한 bootstrap 은 기존 preview 승인 문구에 axhub 확인을 통합해요(별도 AUQ 대신 1회 승인). headless 에서는 이 AUQ 를 생략해요 — AUQ 0회 계약을 그대로 지켜요.
- 적용: skills/deploy/SKILL.md, skills/bootstrap/SKILL.md, skills/import/SKILL.md
- invariant: "axhub 진입 확인"

## AP-13 Windows 실행 계약 (Git Bash 전용)
- 규칙: Windows 에선 모든 axhub CLI 명령을 Git Bash 전용으로 실행해요 — PowerShell 로 실행하지 않아요. PowerShell 에는 `$HOME` 과 repair 된 PATH 가 없어서 credential·auth 조회가 false-negative 나요. PATH 가 없으면 `SetEnvironmentVariable` 이나 `$env:PATH` prepend 로 수동 등록하지 않고, canonical on-disk 경로(`~/.axhub/bin/axhub`(.exe))로 `plugin-support repair-path` 를 실행해 영속 등록을 고쳐요 — 그 상태에선 bare `axhub` 가 127 이에요. 같은 세션은 repair-path JSON 의 `bin_path` 절대경로로 이어가고, bin_path 가 없는 구 CLI 면 새 터미널 안내로 대체해요 — 이미 열린 셸은 OS 설계상 밖에서 PATH 를 못 고치므로 새 터미널은 다음 세션용 보조 안내예요 (VS Code 통합터미널은 앱 재시작 후에야 새 PATH 를 봐요). `auth status` 는 `auth login` 을 실행한 그 셸에서 검증해요 — HOME 없는 셸이 "미로그인" 이라고 해도 로그인 실패로 보지 않아요. 로그인은 단일 self-polling `axhub auth login --json` 1 회로 하고 background 로 재실행하지 않아요 (device code 소진). 이 계약은 Windows(`$OS`=Windows_NT) 세션에서 SessionStart hook (`hooks/hooks.json`) 이 매 세션 always-on 으로 emit 해요 — skill 본문 로드 여부와 무관하게 free-form 실행 경로까지 덮어요. non-Windows 세션에서는 조용히 건너뛰어요. **예외(명문):** 이 계약이 막는 것은 auth·credential·repair 된 PATH 에 의존하는 명령이에요. npx 기반 셋업(`npx axhub@latest setup` → `axhub setup claude`)은 auth 무접촉이고 npx 가 바이너리를 직접 해석해 PATH 에 의존하지 않아서, 사용자가 PowerShell 에서 실행해도 계약 위반이 아니에요 (참고: npm `axhub` 패키지는 아직 미출시 placeholder 라 이 npx lane 은 현재 실재하지 않아요 — 출시 전까지 문서상 예외로만 남아요). 단 setup 이후의 로그인·`auth status` 검증 등 auth·PATH 의존 명령부터는 그대로 Git Bash 계약을 따라요.
- 적용: hooks/hooks.json, CLAUDE.md
- invariant: "Git Bash 전용"

## AP-14 update-first Code-mode router
- 규칙: Claude Desktop Code 모드에서 사용자가 `axhub` 와 최신성 키워드(최신·버전·업데이트·latest·up to date·version check·update·upgrade)를 함께 말하면, 전역 axhub App/MCP 도구보다 `update` 스킬을 먼저 타야 해요. Code-mode update router guard 는 SessionStart fallback 과 UserPromptSubmit match 로 라우팅 문맥만 추가하고 명령 실행·네트워크·앱 목록 조회는 하지 않아요. 사용자에게 보이는 첫 문장은 `현재 버전을 확인할게요.` 예요. 이 guard 는 `AXHUB_NO_UPDATE_ROUTER=1` 또는 marker 파일 `~/.axhub/config/no-update-router` 로 끌 수 있어요 — 모든 훅 kill switch 는 env 에서 `AXHUB_` 를 뗀 소문자-하이픈 이름의 marker 파일 counterpart 를 가져요 (profile export 가 닿지 않는 Windows GUI 세션에서도 확실한 채널). 업데이트 뒤 같은 요청의 앱 현황 확인을 이어갈 때는 존재하지 않는 `axhub app list` 단수 명령을 추측하지 말고 `axhub apps --help` 로 plural 표면을 확인한 뒤 정확히 `axhub apps list --json` 읽기 전용 명령으로 시작해요. `| head`, `2>/dev/null`, `grep`, `sed`, `awk` 같은 shell 후처리는 붙이지 않아요.
- 적용: hooks/hooks.json, CLAUDE.md, POLICY.md, README.md
- invariant: "AXHUB_NO_UPDATE_ROUTER", "현재 버전을 확인할게요"

## AP-15 앱 소유자·계정 불일치 비판정
- 규칙: 앱을 만든 계정과 지금 로그인한 계정이 달라 보여도(앱 정보의 owner 표시, 멤버 목록, git 커밋 이메일 등), 스킬은 그 불일치를 스스로 판정해 막거나 "앱 소유자에게 물어보세요" 같은 확인 절차를 만들어내지 않아요. 배포 권한(인가 — 이 계정이 이 앱을 배포할 수 있는지)의 판정은 CLI/백엔드 몫이에요. 사용자의 구두 승인을 권한 근거로 쓰지 않아요 — "소유자가 배포해도 된대요" 같은 말로는 권한이 생기지 않아요. CLI 가 권한 부족(exit 8, `axhub_app_forbidden` — 앱 owner/admin 권한 검사 실패)으로 막을 때만 앱 소유자/관리자에게 멤버 권한 부여를 요청하도록 안내하고 멈추며, 권한이 실제로 생겼는지는 말이 아니라 같은 명령의 재시도 성공으로만 확인해요.
- 적용: skills/deploy/SKILL.md, skills/deploy/references/error-empathy-catalog.md
- invariant: "axhub_app_forbidden", "구두 승인을 권한 근거로 쓰지 않아요"
