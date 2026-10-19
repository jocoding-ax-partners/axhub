# 배포 기준 요약 (기준 문서를 못 받았을 때)

preflight 의 `capabilities.readiness.context_url` 문서(`https://docs.axhub.ai/llms-deploy.ko.txt`)가 정본이에요. 받지 못했을 때만 이 요약으로 점검해요. 처리 표시는 문서와 같아요 — **바로 고침**(묻지 않고 고치고 끝에 알려요) · **물어봄**(한 번만 묻고 답을 기억해요) · **알림**(한 번 알리고 진행해요).

## 앱이 뜨는 조건 — 바로 고침

- 포트는 `PORT` 환경변수로 열고 `0.0.0.0` 에 붙어요. 60초 안에 상태 확인 경로가 200·3xx 로 응답해야 해요.
- `.env` 는 서버로 올라가지 않아요. 필요한 값은 `axhub env set --from-dotenv` 로 등록해요. `localhost`·사설 IP 값은 등록하지 않아요.
- `PORT`·`DATABASE_URL`·`STORAGE_*`·`AXHUB_*` 는 AXHub 가 넣어 줘요 — 실행용으로 등록하지 않아요.

## 데이터 — 물어봄

- 서버 폴더(SQLite 파일, JSON 파일, `./uploads`)는 재배포·재시작·절전 때 처음 상태로 돌아가요. 데이터는 `database: engine: postgres`(`DATABASE_URL`), 파일은 `storage: enabled: true`(`STORAGE_*`)로 옮길지 물어요.
- AXHub DB 는 PostgreSQL 만이고 빈 상태로 시작해요. 테이블은 앱이 켜질 때 만들어요(바로 고침).

## 네트워크 — 물어봄

- 밖으로는 공인 인터넷의 TCP 443·80·465·587·993·995·22·3306·5432·6379·27017 만 나가요. 25 번 메일 발송, 그 밖의 포트, 사설 IP·`localhost` 의 다른 프로그램은 막혀요. 막힌 연결은 오류 없이 시간 초과로 끝나요.
- 비공개 앱 주소로 오는 요청은 모두 AXHub 로그인을 거쳐요. 외부 서비스의 웹훅은 `axhub relay create --app <앱> --name <창구> --target-path <경로>` 로 만든 접수창구로 받고, 전달된 요청은 `AXHUB_RELAY_SIGNING_SECRET` 으로 확인해요.
- AI 는 로컬 모델(Ollama 등) 대신 AXRouter(OpenAI 호환)로 불러요 — `axhub axrouter keys issue --name <이름>` 으로 키를 받아 `LLM_API_KEY`(비밀)·`LLM_BASE_URL=https://axrouter.ai/v1` 로 등록해요. 대화·임베딩·받아쓰기 모두 같은 주소예요.
- 요청 본문은 50MB, 첫 응답은 120초까지예요. 큰 파일은 보관함에 바로 올리고, 오래 걸리는 일은 나누거나 흘려보내요.

## 리소스 — 알림

- 기본 사양은 CPU 0.5코어·메모리 1GB 예요. 무거운 앱은 `axhub apps update <앱> --resource-tier M` 처럼 올려요(XS·S·M·L·XL, 회사 상한 안에서).
- 예약 실행(cron) 기능은 따로 없어요. 앱 안에서 스케줄러를 띄우고, 레플리카가 2 이상이면 두 번 돈다는 걸 알려요. 72시간 접속이 없으면 절전되고 그동안 예약 작업도 멈춰요.

## 안 되는 6가지 — 물어봄

내 컴퓨터의 파일·프로그램 사용, 회사 내부 시스템 직결, 앱 안에서 AI 모델 직접 실행, 웹이 아닌 앱, 엑셀·한글 프로그램 자동화, 입력한 내용을 서버 파일에 저장. 테스트·예시 코드·README 에만 있는 흔적은 문제로 보지 않아요.
