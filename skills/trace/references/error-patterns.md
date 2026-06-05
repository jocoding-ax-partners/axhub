# Trace Error Pattern Catalog (4-Part Korean)

Phase 25 PR 25.4 (R3γ 갱신) — `axhub:trace` skill 이 **런타임 로그**(현행 `axhub deploy logs`)에서 매칭하는 8 + 패턴. 순수 빌드타임 패턴(dependency_install_failed / docker_image_pull_failed)은 빌드 단계 실패 시 런타임 로그가 비므로 event_log `failure_reason` 경로로 안내돼요.
각 entry 는 `deploy/references/error-empathy-catalog.md` 의 4-part empathy
구조 (감정 / 원인 / 액션 / 다음 버튼) 를 따라요. `trace_helper.rs` 의
`match_error_patterns` 가 event_log `failure_reason`(authoritative) + 런타임
로그의 ERROR/WARN 라인을 매칭한 뒤 여기 키 (예: `env_not_found`) 를 SKILL 에 넘겨요. SKILL 은
매칭 key 의 entry 를 그대로 사용자에게 출력해요.

---

## env_not_found

**감정:** 잠깐만요. 환경변수가 빠진 것뿐이에요. 앱은 안전해요.

**원인:** 실패 로그에서 `env: <KEY> not found` 가 발견됐어요. axhub env 에 해당 키가 등록 안 된 상태로 배포가 시작돼서 빌드가 막혔어요.

**해결:** `axhub env set <KEY>=<값>` 으로 등록한 뒤 다시 배포해주세요. 키 이름이 정확한지 확인하고 production / staging 환경별로 따로 set 해야 해요.

**버튼:** ["환경변수 추가", "환경변수 보여줘", "값 확인하기"]

---

## oom

**감정:** 잠깐만요. 빌드가 메모리를 초과했어요. 앱은 안전해요.

**원인:** 빌드 중 OOM (out of memory) killed 가 발생했어요. 이미지 빌드가 인스턴스 메모리 한계 (보통 1-2GB) 를 넘었거나 한꺼번에 너무 많은 파일을 처리했을 때 생겨요.

**해결:** profile 을 더 큰 인스턴스로 바꿔서 (예: `axhub profile use prod-large`) 다시 배포해주세요. 또는 빌드 캐시를 활용해서 동시 처리 양을 줄여요.

**버튼:** ["profile 변경", "캐시 사용 확인", "도와주세요"]

---

## module_not_found

**감정:** 잠깐만요. 패키지가 누락됐어요. 앱은 안전해요.

**원인:** 빌드 중 `Cannot find module '<pkg>'` 가 발견됐어요. 보통 `package.json` 에 등록 안 된 패키지를 import 한 경우거나, lock 파일이 outdated 인 경우예요.

**해결:** 로컬에서 `npm install <pkg>` 또는 `bun add <pkg>` 한 뒤 lock 파일까지 commit 후 다시 배포해주세요. 이미 추가했는데도 실패하면 lock 파일 commit 누락 가능성 있어요.

**버튼:** ["package.json 확인", "lock 파일 commit", "도와주세요"]

---

## network_timeout

**감정:** 잠깐만요. 일시적인 통신 문제 같아요. 앱은 안전해요.

**원인:** 빌드 중 `connection refused` 또는 `network timeout` 발견됐어요. 외부 패키지 레지스트리 (npm registry / pip / cargo) 가 일시적으로 응답 안 했거나, 회사 네트워크 정책으로 차단됐을 가능성이 있어요.

**해결:** 1-2 분 뒤 다시 배포해 보세요. 그래도 실패하면 패키지 이름이 정확한지, 회사 mirror registry 가 설정돼 있는지 확인해요.

**버튼:** ["다시 배포", "mirror 설정 확인", "도와주세요"]

---

## dependency_install_failed

**감정:** 잠깐만요. 의존성 설치가 막혔어요. 앱은 안전해요.

**원인:** `npm err!` 또는 `dependency install failed` 가 발견됐어요. 패키지 버전 충돌, peer dep 부재, native build (gyp / cargo) 실패 같은 사유로 install 단계가 끝나기 전에 빌드가 멈췄어요.

**해결:** 로컬에서 `npm install` (또는 `bun install`) 깨끗하게 한 번 돌려보고 실패하는 패키지를 확인해주세요. 보통 lock 파일 재생성으로 해결돼요.

**버튼:** ["로컬 install 시도", "lock 파일 재생성", "도와주세요"]

---

## docker_image_pull_failed

**감정:** 잠깐만요. base 이미지를 받지 못했어요. 앱은 안전해요.

**원인:** `docker pull` 또는 `image pull failed` 가 발견됐어요. 베이스 이미지 (예: `node:20-alpine`) 가 registry 에서 사라졌거나, 회사 사내 registry 인증이 만료됐어요.

**해결:** `Dockerfile` 의 FROM 라인을 확인해서 image tag 가 존재하는지 확인해요. 사내 registry 라면 회사 IT 담당자에게 인증 정보 확인 부탁해요.

**버튼:** ["Dockerfile 확인", "IT 담당자에 연락", "도와주세요"]

---

## port_already_in_use

**감정:** 잠깐만요. 포트 충돌이에요. 앱은 안전해요.

**원인:** `EADDRINUSE` 또는 `address already in use` 가 발견됐어요. 컨테이너 내부에서 같은 포트를 두 프로세스가 잡으려고 시도했거나, 이전 배포가 깔끔하게 종료되지 않아서 포트 점유 상태로 남았어요.

**해결:** `axhub recover` 로 직전 배포 정리 후 다시 배포해 보세요. 또는 `axhub.yaml` 의 port 설정이 컨테이너 내부 포트와 일치하는지 확인해요.

**버튼:** ["복구해줘", "axhub.yaml port 확인", "도와주세요"]

---

## build_command_failed

**감정:** 잠깐만요. 빌드 스크립트가 실패했어요. 앱은 안전해요.

**원인:** `build command failed` 또는 `exit code 1` 이 발견됐어요. 보통 `npm run build` / `cargo build` / `bun run build` 같은 사용자 스크립트가 컴파일/타입체크 에러로 실패한 경우예요.

**해결:** 로컬에서 같은 build 명령을 돌려서 발생하는 raw 에러를 확인해주세요. linter / type checker 가 실패하면 그 출력에 정확한 라인 정보가 있어요.

**버튼:** ["로컬 build 시도", "type check 통과 확인", "도와주세요"]

---

## (no_pattern_match)

매칭되는 pattern 없을 때 SKILL 이 사용하는 generic fallback:

**감정:** 잠깐만요. 정확한 원인을 자동으로 추적 못 했어요.

**원인:** 런타임 로그의 마지막 ERROR/WARN 을 보여드릴게요 (최대 5 줄). raw 라인 인용은 Vibe Coder Visibility 원칙상 검열 없이 그대로 보여드려요.

**해결:** 1) 에러 메시지 한 줄을 그대로 검색해보거나, 2) `axhub deploy logs <deployment-id> --app <app> --source build` 로 전체 로그를 받아서 직접 살펴봐주세요.

**버튼:** ["전체 런타임 로그 받기", "도와주세요", "취소"]
