# 배포 기준 요약 (기준 문서를 못 받았을 때)

preflight 의 `capabilities.readiness.context_url` 문서(`https://docs.axhub.ai/llms-deploy.ko.txt`)가 정본이에요. 받지 못했을 때만 이 요약으로 점검해요. 처리 표시는 문서와 같아요 — **바로 고침**(묻지 않고 고치고 끝에 알려요) · **물어봄**(한 번만 묻고 답을 기억해요) · **알림**(한 번 알리고 진행해요).

## 앱이 뜨는 조건 — 바로 고침

- 포트는 `PORT` 환경변수로 열고 `0.0.0.0` 에 붙어요. 60초 안에 상태 확인 경로가 200·3xx 로 응답해야 해요.
- `.env` 는 서버로 올라가지 않아요. 필요한 값은 `axhub env set --from-dotenv` 로 등록해요. `localhost`·사설 IP 값은 등록하지 않아요.
- `PORT`·`DATABASE_URL`·`STORAGE_*`·`AXHUB_*` 는 AXHub 가 넣어 줘요 — 실행용으로 등록하지 않아요.

## 정적 사이트 — 바로 고침

- AXHub 저장소(self-hosted)에 push 해서 배포하는 정적 사이트는 `axhub.yaml` 에 `build.deploy_method: static` 과 `build.static_output_dir` 를 적어요. `static_output_dir` 는 빌드 뒤 `index.html` 이 있는 폴더예요 — 최상위에 있으면 `.`, 아니면 `dist`·`build`·`out` 처럼 실제 빌드 결과 폴더예요. 둘 중 하나라도 없으면 서버가 정적 사이트로 빌드하지 않아요.
- 서버가 올라온 소스로 빌드해요(`build.install`·`build.build`, 없으면 lockfile 기준 설치와 `package.json` 이 있을 때 `npm run build`). 그래서 빌드 결과 폴더는 커밋하지 않아도 돼요.

## 데이터 — 물어봄

- 서버 폴더(SQLite 파일, JSON 파일, `./uploads`)는 재배포·재시작·절전 때 처음 상태로 돌아가요. 데이터는 `database: engine: postgres`(`DATABASE_URL`), 파일은 `storage: enabled: true`(`STORAGE_*`)로 옮길지 물어요. 사용자에게는 `다시 배포해도 내용이 남게 바꾸기 (추천)`·`그대로 올리기`처럼 결과로 묻고 DB·파일 이름은 말하지 않아요.
- AXHub DB 는 PostgreSQL 만이고 빈 상태로 시작해요. 테이블은 앱이 켜질 때 만들어요(바로 고침).
- 바깥 고객 여럿의 데이터를 한 DB 에 담거나 자체 회원·비밀번호 표가 있는 앱은 `database.roles: [runtime, auth]` 로 계정을 나눠요(`DATABASE_URL_RUNTIME`·`DATABASE_URL_AUTH` 자동 주입, 권한·RLS 는 마이그레이션에서 `GRANT`, FORCE RLS 면 `owner_bypass_rls: true`). 자세히는 docs `features/data` 의 「DB 계정을 나눠 쓸 때」(알림).

## 네트워크 — 물어봄

- 밖으로는 공인 인터넷의 TCP 443·80·465·587·993·995·22·3306·5432·6379·27017 만 나가요. 25 번 메일 발송, 그 밖의 포트, 사설 IP·`localhost` 의 다른 프로그램은 막혀요. 막힌 연결은 오류 없이 시간 초과로 끝나요.
- 비공개 앱 주소로 오는 요청은 모두 AXHub 로그인을 거쳐요. 외부 서비스의 웹훅은 `axhub relay create --app <앱> --name <창구> --target-path <경로>` 로 만든 접수창구로 받고, 전달된 요청은 `AXHUB_RELAY_SIGNING_SECRET` 으로 확인해요.
- AI 는 로컬 모델(Ollama 등) 대신 외부 API 로 불러요. 먼저 `axhub axrouter status --tenant <회사> --json` 의 `data.available` 을 봐요. `true` 일 때만 AXRouter(OpenAI 호환)를 권해요 — `axhub axrouter keys issue --name <이름>` 키를 `LLM_API_KEY`(비밀)·`LLM_BASE_URL=https://axrouter.ai/v1` 로 등록하고, 대화·임베딩·받아쓰기 모두 같은 주소예요. `false`·확인 실패면 AXRouter 를 말하지 않고 사용자 본인 키(`OPENAI_API_KEY`·`ANTHROPIC_API_KEY` 등)를 `axhub env set <키> --app <앱> --secret --from-stdin` 으로 넣게 해요.
- 요청 본문은 50MB, 첫 응답은 120초까지예요. 큰 파일은 보관함에 바로 올리고, 오래 걸리는 일은 나누거나 흘려보내요.

## 리소스 — 알림

- 기본 사양은 CPU 0.5코어·메모리 1GB 예요. 무거운 앱은 `axhub apps update <앱> --resource-tier M` 처럼 올려요(XS·S·M·L·XL, 회사 상한 안에서).
- 예약 실행(cron) 기능은 따로 없어요. 앱 안에서 스케줄러를 띄우고, 레플리카가 2 이상이면 두 번 돈다는 걸 알려요. 72시간 접속이 없으면 절전되고 그동안 예약 작업도 멈춰요.

## 안 되는 6가지 — 물어봄

내 컴퓨터의 파일·프로그램 사용, 회사 내부 시스템 직결, 앱 안에서 AI 모델 직접 실행, 웹이 아닌 앱, 엑셀·한글 프로그램 자동화, 입력한 내용을 서버 파일에 저장. 테스트·예시 코드·README 에만 있는 흔적은 문제로 보지 않아요.
