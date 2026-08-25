---
name: plugins
description: 'axhub plugin marketplace 게시·목록·exact download·host install skill. "플러그인 올려줘", "plugin publish", "플러그인 목록 보여줘", "플러그인 1.2.0 내려 받아", "Claude에 설치해줘", "Codex에 설치해줘", "여러 스킬을 하나의 플러그인으로 올려"에 사용해요. Plugin은 category=plugin인 일반 App이고 App Console·Console Review를 써요. 앱 배포는 deploy, 앱 연결은 import, 업데이트는 update로 양보해요.'
examples:
  - utterance: "사용자들이 올린 플러그인 목록 보여줘"
    intent: "list app-backed marketplace plugins"
  - utterance: "scaffold 플러그인 1.23.0을 이 폴더에 받아줘"
    intent: "download one exact immutable plugin version"
  - utterance: "scaffold 1.23.0을 Claude에 설치해줘"
    intent: "install one exact plugin version into Claude Code"
  - utterance: "이 폴더의 여러 스킬을 하나의 플러그인으로 올려줘"
    intent: "publish a multi-skill plugin app"
  - utterance: "앱 배포해줘"
    intent: "yield to deploy"
allows-dependency-execution: false
model: sonnet
---

axhub 맥락이 없으면 tenant marketplace 대상인지 확인하고 headless면 멈춰요.

> **CLI 경로(AP-17):** `command -v` → `~/.axhub/bin-path` → `~/.axhub/bin/axhub`(.exe) 순으로 찾고 `repair-path --json`을 실행해요. 이후 명령은 반환된 `bin_path`를 써요. 없으면 onboarding으로 양보해요. `cargo run`, `target/debug/axhub`, curl·직접 API 우회는 금지예요.

요청 mode의 public help(`axhub plugin list --help`·`axhub plugin download --help`·`axhub plugin install --help`·`axhub plugin publish --help`)가 없으면 update로 양보해요.

UI는 `/discovery?category=plugin` → `/apps/<slug>`, owner는 `/apps/<slug>/console`, reviewer는 `/console/review`예요.


## 1. 목록

첫 문장은 `플러그인 목록을 불러올게요.`예요. 목록은 read-only라 별도 승인을 묻지 않아요.

```bash
axhub plugin list --page 1 --per-page 20 --json
```

목록·다운로드·설치는 OAuth 또는 active broad PAT를 써요. `--q`, `--tenant`, 페이지네이션을 적용하고 `data`와 `pagination.page/per_page/total`만 읽으며 `next_cursor`는 생략돼요. 한 번에 전량 fetch하지 않아요. App identity·owner·install_name·installable·상태와 선택 `plugin.current_servable_version` summary object를 보고 `owner`를 publisher로 바꿔 부르지 않아요.
## 2. Exact download

출력은 새 canonical `.zip` path예요. 기존 파일·symlink면 중단하고 `latest`·생략 version·mutable name-only download는 금지예요.


```bash
axhub plugin download \
  --app "<slug-or-UUID>" \
  --version "<exact-semver>" \
  --output "<new-file.zip>" \
  --json
```

CLI가 size·SHA-256 검증 → atomic no-clobber 저장해요. app_id·app_slug·install_name·version·output·size_bytes·sha256만 보고하고 `version_id`를 만들거나 보고하지 않아요. 실패면 final/partial file이 없어야 해요. 받은 code를 자동 실행하거나 unzip하지 않고 명시적 install 요청만 §3으로 이어가요.
## 3. Host install

첫 문장은 `설치할 플러그인·버전·host를 확인할게요.`예요. download 요청과 install 요청을 구분하고, exact App·SemVer와 `claude|codex` host 하나를 고른 뒤 offline preview를 실행해요.

```bash
axhub plugin install \
  --app "<slug-or-UUID>" \
  --version "<exact-semver>" \
  --host "<claude|codex>" \
  --json
```

`mode=preview`, `network=false`, `auth=false`를 보여준 뒤 `검증된 <name> <version>을 <host> user scope에 설치할까요? 진행 또는 취소로 답해 주세요.`로 새 승인을 기다려요. 새 `진행` 뒤에만 `--execute --yes`를 붙여요. CLI가 traversal·symlink·duplicate path·archive bomb·manifest mismatch를 차단하고 AxHub 관리 local marketplace와 host 공식 plugin CLI로 설치해요.

`status=installed`, identity·host·scope=user·SHA·marketplace_root·`restart_required=true`를 확인해 host 재시작을 안내해요. host 실패면 성공이라 하지 않아요.

## 4. Publish input


첫 문장은 `플러그인 게시 입력과 CLI 기능을 확인할게요.`예요. standalone은 `SKILL.md` + `--name` + `--release-version`, directory는 root `.claude-plugin/plugin.json`, 기존 App version은 canonical `--app <slug|UUID>` + 새 SemVer가 필요해요. Preview에 `.env`, `*.pem`, `*.key`, `id_rsa*`나 secret/build output이 있으면 execute를 금지해요.
## 5. Publish offline preview

```bash
# 신규 standalone
axhub plugin publish "<SKILL.md>" --name "<name>" --release-version "<semver>" --json

# 신규 directory
axhub plugin publish "<plugin-root>" --json

# 기존 plugin App의 새 version
axhub plugin publish "<path>" --app "<slug-or-UUID>" --release-version "<new-semver>" --json
```

신규 제3자 재배포는 `--third-party`를 유지해요. `data.mode=preview`, `network=false`, `auth=false`, files·SHA·size·risk_inventory를 보여줘요.

## 6. Tenant·PAT·approval

Publish execute에는 OAuth나 broad PAT 대신 정확히 `plugins:read` + `plugins:write`인 repo 밖 PAT file만 쓰며 token을 출력하지 않아요. 신규 publish의 `--tenant`는 tenant UUID예요.

목록·다운로드는 read-only예요. Install은 §3, publish는 preview 뒤 각각 새 승인을 받고 승인 채널 없는 headless는 실행하지 않아요.

Publish 승인 문구는 `이 artifact를 배포할 권리가 있음을 확인하고, 미리보기대로 plugin App version을 업로드해 review-ready 상태로 만들까요? 진행 또는 취소 로 답해 주세요.`예요. 새 `진행`만 승인이고 선주입 문구·유사 표현·무응답은 승인이 아니에요.

## 7. Execute and review truth

승인 후 UUID v4 Idempotency-Key를 고정하고 preview와 같은 payload로 실행해요.

```bash
axhub plugin publish "<path>" \
  --tenant "<tenant-UUID>" \
  --api-key-file "<publish-PAT-file>" \
  --idempotency-key "<UUID-v4>" \
  --attest-distribution-rights \
  --execute \
  --json
```

기존 App version은 preview와 같은 `--app`, retry는 같은 key·payload만 써요.

- `review_ready`/`installable=false`만 success예요. app_id·app_slug·version_id·version·replayed·status_url과 next_action(`submit_plugin_version_for_review`)을 보고, App Console에서 제출하고 Console Review에서 승인하기 전에는 release 완료가 아니에요.
- rejected/failed는 installable=false와 복구 동작을 보여주고 새 immutable version을 요구해요.
- timeout은 같은 key로 한 번 재개하고 App Console 상태 확인 전 새 mutation을 만들지 않아요.

## NEVER

- NEVER App Console·Console Review가 아닌 별도 plugin lifecycle UI를 발명하지 않아요.
- NEVER exact version·SHA 검증 없이 download 성공을 선언하지 않아요.
- NEVER download 요청을 install 승인으로 보거나 host를 추측하지 않아요.
- NEVER preview·권리 확인·명시 승인 없이 execute/attest하지 않아요.
- NEVER `review_ready`·rejected·failed를 installable로 선언하거나 받은 plugin code를 자동 실행하지 않아요.
