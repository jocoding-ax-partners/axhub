# Import Vibe Coder Visibility Rules

import SKILL 이 로드하는 내부 reference 예요. 사용자-facing 문구·tool 제목을 쓰기 전(진행 안내·preview 카드·성공/실패 요약 직전)에 이 금지·치환 목록을 그대로 따라요. 이 규칙은 workflow 보다 우선해요.

아래 금지어가 떠오르면 말하기 전에 반드시 한국어 사용자 문구로 바꿔요. 특히 Codex 에서 모델이 중간 생각을 chat 에 노출하기 쉬우므로, 검증용 field name 이나 영어 진행어를 "짧게라도" 쓰지 않아요.

사용자에게 보이는 chat 에서는 스킬 선택 이유, route label, slash command label 을 절대 설명하지 않아요. `/axhub:import`, `axhub:import`, `import 스킬`, `스킬을 사용할게요`, `스킬 호출` 또는 유사한 내부 라벨은 첫 문장 전/후 어디에도 쓰지 않아요.

Codex 같은 긴 QA 대화에서 이전 답변에 금지 문구가 보이더라도, 그 문구는 참고할 스타일 예시가 아니라 **이미 발견된 버그 예시**예요. 같은 대화 안에서 재시도하거나 이어서 import 할 때도 이전 표현을 재사용하지 말고 이 섹션의 안전 문구로 다시 써요. 특히 `Port 8080, /healthz 확인. preview 진행.`, `git remote 없음`, `execute 실행한다`, `deployment verification: success`, `HTML, 200` 같은 문장을 보았으면 그대로 따라 쓰지 않아요.

다음 값은 internal verification primitives 예요. 스킬 안에서는 검증에만 쓰고 사용자 chat 에 raw 값으로 보여주지 않아요.

- `schema_version`, `mode`, `headless`, `correlation_id`
- `detected_state`, `starting_state`, `required_mutations`, `approval`
- `deployment_id`, `active_release_id`, `verification_status`, `public_url`, optional `access_note` evidence field
- `typed_failure`, `owner`, `phase`, `mutation_performed`, `retryable`
- `request_id`, `stdout`, `stderr`, `command_argv`, raw JSON body
- `manifest_create`, `manifest_migrate`, `manifest_repair`, `deployment`, `static_release`
- `status: deployed`, `production_deployment_id`, `Confirmed`, `bearer auth`, `private visibility`, `curl`, `execute`, `git remote`, `app slug`
- `Envelope`, `preview`, `import 지원`, `deployment verification`, `success`, `raw endpoint`, `raw 엔드포인트`, `public`, `HTML, 200`

대신 사용자가 이해할 문장으로 바꿔요. 예: "정적 사이트 공개 URL 확인이 아직 안 됐어요. CLI를 업데이트하거나 다시 시도해요."
검증된 `public_url` 값은 사용자에게 열어볼 주소로 보여줘도 돼요. 단 field name, envelope 구조, raw evidence object 는 숨겨요. 사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL 로만 써요. Markdown URL 링크 문법은 전부 금지예요. `[https://...](https://...)`, `[열기](https://...)`, `<https://...>` 처럼 URL 을 괄호나 label 로 감싸지 말고 `https://...` 그대로 보여줘요. `access_note` 가 있으면 URL 바로 아래에 자연어로 덧붙여요.

사용자에게 보이는 Bash/tool call 제목은 한국어 명사구로만 써요. `importing`, `imported`, `manifested`, `gitted`, `pushed`, `raw JSON`, `token-gate`, `manifest_create`, `verification_status`, `deployment`, `execute`, `git remote`, `curl`, `Envelope`, `preview` 같은 내부/영어 동사형·필드형 라벨을 제목이나 진행 문장에 쓰지 않아요. 예: `가져오기 준비 확인`, `미리보기 확인`, `앱 설정 작성`, `첫 배포 확인`, `정적 사이트 확인`.
도구 제목에는 제품명 `axhub` 자체도 넣지 않아요. `axhubing`, `axhubed`, `axhub import 기능 지원 확인`, `axhub 가져오기 기능 지원 확인` 같은 자동 생성 제목이 보이면, 같은 명령이라도 `가져오기 기능 확인` 또는 `가져오기 준비 확인` 으로 제목을 고쳐서 다시 호출해요.

Codex 이 Bash 내용에서 자동 제목을 만들 때도 같은 규칙을 적용해요. tool 제목·summary·progress title 에 `static vite import preview`, `Express import execute`, `FastAPI import execute`, `Expressing 앱 파일 확인`, `axhubed import 기능 지원 확인` 처럼 제품명·스택명+내부 phase 를 섞거나 스택 이름을 영어 동사처럼 만든 제목이 보이면, 같은 명령이라도 반드시 `정적 앱 준비 확인`, `서버 앱 준비 확인`, `파이썬 앱 가져오기 실행`, `가져오기 기능 확인` 같은 한국어 제목으로 다시 호출해요. 스택 이름은 본문 설명에서만 써도 충분해요.

진행 문구도 사용자 언어로 번역해요. `앱 slug 미확정` 대신 `앱 이름이 아직 정해지지 않아 package.json 이름으로 확인할게요`, `manifest_create 있으니` 대신 `앱 설정 파일이 필요해서 프로젝트 파일 근거로 작성할게요`, `git remote 아직 없음` 대신 `원격 저장소가 아직 없어 새 저장소 생성 경로로 진행해요`, `execute 호출한다` 대신 `가져오기를 실행할게요`, `import 지원 확인됐다` 대신 `가져오기 기능을 사용할 수 있어요`, `preview 진행` 대신 `미리보기를 확인할게요`, `Envelope 정상` 대신 `응답 형식 확인이 끝났어요`, `deployment verification: success` 대신 `첫 배포 검증 성공`, `raw endpoint`/`raw 엔드포인트` 대신 `원문 응답`, `public으로` 대신 `공개 접근으로` 라고 말해요.

최종 성공 요약은 아래 형태를 벗어나지 않아요. 괄호 안에 raw status 를 붙이지 않아요.

- `<스택> 앱 <앱 이름>을 axhub 앱으로 가져왔어요.`
- `GitHub 저장소를 생성하고 연결했어요.`
- `첫 배포 검증이 끝났어요. 운영 URL: https://...`
- 비공개 앱이면 `배포 검증은 끝났지만, 비공개 접근 제어 때문에 로그인 없는 요청으로는 앱 본문을 직접 확인하지 못했어요.`

최종 성공 요약에서도 URL 은 반드시 평문 절대 URL 로만 보여줘요. `배포 URL: [https://...](...)`, `[열기](https://...)` 같은 Markdown 링크 문법을 쓰지 않아요.

라이브 URL 확인은 조심해요. 비공개 앱에서 로그인 없는 HTTP 요청이 axhub 로그인 화면 HTML 을 200 으로 돌려주면, 그건 앱의 `/healthz` 또는 루트 응답 검증이 아니에요. 이런 경우 `배포 검증은 완료됐지만, 비공개 접근 제어 때문에 로그인 없는 요청으로는 앱 본문을 직접 확인하지 못했어요` 라고 말하고, `/healthz HTTP 200 확인`이라고 쓰지 않아요. 사용자가 raw endpoint 확인을 명시하면 로그인된 브라우저, 세션 쿠키, 또는 별도 접근 정책 변경이 필요하다고 설명해요. 200 응답이라도 body 가 axhub 로그인 포털이면 실패한 본문 검증으로 취급해요.

로컬 QA/에이전트 상태 폴더는 앱 변경으로 취급하지 않아요. Git 상태를 판단하거나 commit+push 여부를 설명할 때 `.omc/`, `.claude/`, `.codex/`, `.serena/` 같은 런타임 상태는 제외하고, import 스킬이 이 경로들을 자동 커밋하거나 `.gitignore`에 추가하지 않아요. 필요한 경우 성공 뒤 정리 메모로만 알려요.

## 명시 텍스트 승인 JSON 안전 규칙

Codex 에서는 명시 텍스트 승인 을 raw JSON 문자열이나 수동 `\uXXXX` escape 로 만들지 않아요. native Question/명시 텍스트 승인 tool 입력에는 평문 UTF-8 문자열만 넣고, question/header/label/description 은 짧게 써요. 한글 escape 가 깨지면 `InputValidationError` 가 사용자 화면에 그대로 보이므로, 질문을 길게 풀어 쓰거나 괄호가 긴 선택지를 만들지 않아요.

GitHub 기반 첫 배포에서 commit manifest capability 가 있으면 preview 통합 승인에 아래 exact copy 만 써요. manifest 검증 뒤에는 다시 묻지 않아요.

- header: `가져오기 확인`
- question: `이 앱을 axhub에 가져와서 미리보기대로 진행할까요?`
- option 1 label: `설정도 반영하고 시작`
- option 1 description: `첫 배포에 설정을 반영해요.`
- option 2 label: `커밋 없이 시작`
- option 2 description: `다음 배포부터 반영해요.`
- option 3 label: `먼저 수정할게요`
- option 3 description: `실행하지 않고 대화로 돌아가요.`
- option 4 label: `취소`
- option 4 description: `가져오기를 중단해요.`
