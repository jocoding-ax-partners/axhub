# Import Manifest 보강 (authoring 규칙)

import SKILL 의 Manifest 보강 단계가 로드하는 내부 reference 예요. `manifest_create` 가 있는 대화형 execute 직전에만 이 규칙대로 axhub.yaml 을 작성해요.

- **언제:** `가져오기 시작` 승인 직후, execute 호출 직전에만 해요. `manifest_create` 가 없으면(=axhub.yaml 이 이미 있으면) 절대 건드리지 않아요. headless 에서는 실행하지 않아요.
- **무시 파일 선행 정리:** axhub.yaml 을 쓰기 전에 반드시 먼저 `cd "<absolute APP_DIR>" && git check-ignore -q axhub.yaml` 로 ignore 여부를 확인해요. 무시 중이면 `.gitignore` 또는 `.git/info/exclude` 에서 `axhub.yaml` 을 직접 무시하는 줄을 제거하고, 다시 `git check-ignore -q axhub.yaml` 이 실패하는지 확인해요. 아직 무시되면 axhub.yaml 을 쓰거나 `axhub deploy --explain --json` 검증을 진행하지 말고 남은 ignore 출처를 정리해요. 이 변경은 앱 설정을 첫 배포에 반영하기 위한 manifest 보강의 일부이므로, `--commit-manifest` 경로를 선택했다면 `.gitignore` 변경도 같은 커밋에 들어가야 해요.
- **근거(grounding):** preview envelope 의 `detected_state.manifest_hints`(포트·health·build·start 와 그 출처)와 실제 프로젝트 파일만 봐요 — Dockerfile, docker-compose·compose, package.json(name·scripts·deps), requirements.txt·pyproject.toml·go.mod, 프레임워크 설정(next.config.\*, vite.config.\* 등), .env.example. 직접 근거가 있는 값만 적고, 불확실하면 비워요. 작고 정확한 manifest 가 크고 틀린 manifest 보다 나아요 — 비운 필드는 backend 가 자동 감지해요.
- **채우는 필드(axhub.yaml 정규 스키마):**
  - `version: axhub/v1` (필수)
  - `name`: 표시 이름(package.json name 또는 폴더 이름)
  - `runtime`: `port`, `health_path` (EXPOSE·HEALTHCHECK·compose·hints 근거)
  - `build`: `framework`, `install`, `build`, `start`, `dockerfile`, `compose_file`, `static_output_dir`, `deploy_method` (deploy_method 는 detect 가 정한 값을 그대로 써요)
  - `env`: `required`/`optional` 아래 `- name:`(+ 필요하면 `scope:`) — **키 이름만**
  - `database`: `engine` (분명히 감지될 때만)
- **보안(엄수):** env 값은 절대 적지 않아요. .env.example 키나 compose `environment:` 키처럼 값 없는 출처에서 key 이름만 가져오고, 비밀처럼 보이는 값은 건너뛰어요. axhub.yaml 에 secret·토큰·비밀번호를 쓰지 않아요.
- **검증 게이트:** 작성한 뒤 반드시 deploy 와 같은 파서로 검증해요.

  ```bash
  cd "<absolute APP_DIR>" && axhub deploy --explain --json
  ```

  exit 0 이고 `status` 가 `ok` 이면 그대로 진행해요. 실패하면 typed error 가 가리키는 필드만 고쳐 최대 2회까지 다시 검증하고, 그래도 실패하면 작성한 axhub.yaml 을 지워 CLI 가 execute 때 최소 manifest 를 쓰게 두고, 최소 설정으로 진행한다고 한 줄로 알려요. `deploy --explain` 의 raw JSON 은 chat 에 붙이지 않아요.
- **이후:** execute 는 axhub.yaml 이 있으면 최소 manifest 를 새로 쓰지 않고 이 보강본을 그대로 둬요. 첫 배포는 현재 git HEAD 를 빌드하므로 이 보강본은 커밋해서 HEAD 에 들어간 뒤(또는 이후 `deploy`)부터 빌드에 반영돼요 — 그래서 정확하고 풍부한 manifest 를 프로젝트에 남기는 게 이 단계의 목적이에요. 아래 commit+push 옵션을 고르지 않으면 첫 배포 자체는 기존 HEAD 와 앱의 deploy_method 로 진행돼요.
- **무시 파일 사후 확인:** 검증 통과 직후, commit manifest 확인 질문을 띄우기 전에도 `git check-ignore -q axhub.yaml` 이 실패하는지 다시 확인해요. 아직 무시되면 execute 를 호출하지 말고 남은 ignore 출처를 정리해요. 실패한 execute 뒤에 뒤늦게 고치는 복구 흐름으로 두지 않아요.
- **첫 배포까지 반영 (옵션, commit+push):** `axhub plugin-support preflight` 의 `capabilities.import.commit_manifest` 가 true 이고 GitHub 기반 첫 배포(docker/compose, 또는 preview 가 `github_repo_create`/`github_connect`/`first_deploy` 를 요구하는 경우)라면, 검증 통과 직후 한 번 더 물어요 — 보강본을 커밋하고 배포 브랜치에 push 해서 첫 배포부터 반영할지. **local_only 앱은 아직 git remote 가 없을 수 있지만, execute 중 CLI 가 repo/remote 를 만들 수 있으므로 remote 없음만으로 이 질문을 건너뛰지 않아요.** 동의하면 execute 를 `--commit-manifest` 로 호출해요(Workflow 7). capability 가 없거나(구 CLI), static lane 이 repo 없이 진행되거나, 사용자가 거절하면 이 옵션을 빼고 기본 경로(커밋 없이, 이후 deploy 부터 반영)로 가요. commit+push 는 외부·되돌리기 어려운 동작이라 반드시 이 별도 동의를 받고, 강제 push 는 절대 안 하고, headless 에서는 제공하지 않아요.
