# Quickstart: 기존 앱 migrate

> 이미 만든 앱(axhub saga 미사용)을 소스 수정 없이 axhub 에 올려요.

## 기본 흐름

```bash
# 기존 앱 디렉터리에서
axhub migrate .
```

1. **감지** — axhub 가 스택/빌드/실행/포트 + 필요한 env 를 감지해요(Railpack).
   ```
   감지: Next.js
   build: npm run build · start: npm run start · port 3000
   필요 env: DATABASE_URL(runtime), NEXT_PUBLIC_API_URL(build)
   신뢰도: 높음 — 이대로 진행할까요? [네 / 아니요]
   ```
   - 신뢰도 낮으면 진행을 막고 `axhub.yaml`/Dockerfile/직접 입력을 요청해요.

2. **확인** → `네` 면 `axhub.yaml` 을 생성(repo 에 commit 가능).

3. **env 설정** — 감지된 required env 안내:
   ```bash
   axhub env set DATABASE_URL          # 값은 입력으로만(암호화 저장)
   axhub env set NEXT_PUBLIC_API_URL
   ```
   배포 전 미설정이면 막아요(build-scoped 는 빌드 전).

4. **등록 + 연결 + 배포** — axhub 가 자동으로:
   - `apps create --from-file axhub.yaml`
   - GitHub repo 연결(consent)
   - 배포 → 라이브 URL

## 이후 코드 변경 (auto strategy 기본)
```bash
git push   # 코드 바꾸고 push
```
- `strategy: auto` 라 빌드마다 재감지 → manifest 수동 수정 없이 재배포돼요.
- 구조 변경(framework 교체 등)도 자동 적응. `pinned` 으로 고정도 가능.

## 특수 케이스
- **Dockerfile 있음** → 추론 건너뛰고 Dockerfile 우선.
- **docker-compose** → web 서비스(빌드+포트) 자동 식별해 배포. db/redis 는 외부.
- **monorepo** → 배포 가능한 앱 후보를 보여주고 선택. 선택한 앱마다 별도 등록.
- **자체 외부 DB/API** → 그대로 사용(egress). axhub governed 데이터 강제 아님.

## 지원 언어 (v1)
Node · Python · Go · Ruby · Java · Kotlin (그 외는 Dockerfile/compose 로).

## 안 되는 것 (v1)
- axhub-managed DB/redis 프로비저닝(자체 외부 사용) · 임의 원격 URL clone · prebuilt 이미지 · desktop/mobile/순수 batch.
