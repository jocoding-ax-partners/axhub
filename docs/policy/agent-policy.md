# axhub plugin 에이전트 행동 정책

axhub plugin 스킬들이 지켜야 하는 행동 규칙의 canonical 원천이에요. SKILL.md·README·CLAUDE.md 의 서술과 충돌하면 이 문서가 이겨요. 규칙 블록의 `적용:`/`invariant:` 형식은 `tests/policy-parity.test.ts` 가 파싱해 검증해요.

- `적용:` — 규칙 복사본을 반드시 담아야 하는 파일 경로예요. 복수면 쉼표로 구분해요.
- `invariant:` — 각 적용 파일에 문자 그대로 존재해야 하는 핵심 문구예요. 쌍따옴표로 감싸고 쉼표로 구분해요.
- 규칙을 바꿀 때는 이 문서를 먼저 고치고, 적용 파일을 따라 고쳐요. parity 테스트가 어긋남을 잡아요.

## AP-1 deploy 성공 선언
- 규칙: deployment-record 배포의 성공은 bound deployment id 에 대한 `axhub deploy verify` 가 terminal success 를 반환할 때만 선언해요. deploy-create stdout·status snapshot·watch 출력·latest 재탐색으로는 선언하지 않아요. static 앱(deploy_method=static)은 별도 lane — activate 의 `active_release_id` 로 성공을 선언하고 verify 를 호출하지 않아요.
- 적용: skills/deploy/SKILL.md
- invariant: "axhub deploy verify", "active_release_id"

## AP-2 deploy preview-confirm gate
- 규칙: 실제 배포 실행 전에 preview 를 보여주고 사용자 확인을 받아요. headless 에서는 dry-run/preview 로 멈춰요.
- 적용: skills/deploy/SKILL.md
- invariant: "preview-confirm"

## AP-3 파괴적 변경 승인
- 규칙: 삭제·롤백·force/yes/execute 급 파괴적 명령은 대화형 승인 1회 뒤에만 실행해요. 조회(목록·상태·로그)는 확인 없이 실행해요. headless 에서는 파괴적 명령을 실행하지 않고 preview/summary 로 멈춰요.
- 적용: skills/clarity/SKILL.md
- invariant: "파괴적 변경은 승인"

## AP-4 diagnosis 읽기 전용
- 규칙: diagnosis 는 배포 실패 원인을 읽기 전용으로만 진단해요. 재배포·롤백·새 deploy create 를 직접 실행하지 않고, 실행이 필요한 다음 행동은 담당 스킬로 자연어 handoff 만 남겨요.
- 적용: skills/diagnosis/SKILL.md
- invariant: "읽기 전용", "절대 직접 실행하지 않아요"

## AP-5 development write 게이트
- 규칙: development 의 코드 생성은 read 를 기본으로 하고, 스키마 변경 같은 write 는 preview-confirm 승인과 존재 확인 뒤에만 실행해요. headless 에서는 아무것도 바꾸지 않아요.
- 적용: skills/development/SKILL.md
- invariant: "read 기본, write 게이트"

## AP-6 CLI preflight 게이트
- 규칙: init·deploy 는 시작 시 `axhub` 존재와 plugin-support preflight 동작을 확인해요. CLI 가 없거나 preflight 가 안 되면 멈추고 설치/업그레이드를 안내하며, 절대 우회하지 않아요. 버전 숫자를 직접 비교하지 않아요.
- 적용: skills/init/SKILL.md, skills/deploy/SKILL.md
- invariant: "plugin-support preflight"

## AP-7 skill 양보 라우팅
- 규칙: 각 스킬은 자기 경계 밖 요청을 담당 스킬로 양보해요. 경계가 섞이면 자기 몫만 끝내고 나머지는 담당 스킬로 넘겨요.
- 적용: skills/clarity/SKILL.md, skills/deploy/SKILL.md, skills/development/SKILL.md, skills/diagnosis/SKILL.md, skills/import/SKILL.md, skills/update/SKILL.md
- invariant: "양보"

## AP-8 onboarding 자동 init 금지
- 규칙: onboarding 은 빈 폴더나 manifest 없는 폴더를 발견해도 init 을 자동 실행하거나 앱을 자동 생성하지 않아요. Ready card 로 끝내요.
- 적용: skills/onboarding/SKILL.md
- invariant: "No automatic init"

## AP-9 clarity 공개 표면만
- 규칙: clarity 는 hidden `axhub plugin-support` 그룹을 탐색·실행하지 않아요. 공개 `--json-schema`/`--help` 표면만 사용해요.
- 적용: skills/clarity/SKILL.md
- invariant: "공개 표면만"
