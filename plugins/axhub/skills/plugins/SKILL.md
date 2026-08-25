---
name: plugins
description: 'axhub plugin marketplace 게시·목록·exact download skill. "플러그인 올려줘", "plugin publish", "플러그인 목록 보여줘", "플러그인 1.2.0 내려 받아", "여러 스킬을 하나의 플러그인으로 올려"에 사용해요. Plugin은 category=plugin인 일반 App이고 App Console·Console Review를 써요. 앱 배포는 deploy, 앱 연결은 import, 업데이트는 update로 양보해요. axhub 맥락 없으면 tenant marketplace인지 확인해요.'
examples:
  - utterance: "사용자들이 올린 플러그인 목록 보여줘"
    intent: "list app-backed marketplace plugins"
  - utterance: "scaffold 플러그인 1.23.0을 이 폴더에 받아줘"
    intent: "download one exact immutable plugin version"
  - utterance: "이 폴더의 여러 스킬을 하나의 플러그인으로 올려줘"
    intent: "publish a multi-skill plugin app"
  - utterance: "앱 배포해줘"
    intent: "yield to deploy"
allows-dependency-execution: false
model: sonnet
---

# Plugins — app-backed marketplace

axhub 맥락(대화·현재 연결·직전 작업)이 없으면 axhub tenant marketplace 대상인지 한 번 확인해요. headless에서는 묻지 않고 멈춰요.

> **CLI 경로(AP-17):** bare `axhub` 실패는 미설치가 아니에요. `command -v` → `~/.axhub/bin-path` → `~/.axhub/bin/axhub`(.exe)로 찾고 `repair-path --json`을 실행해요. 이후 명령은 반환된 `bin_path`(없으면 발견한 절대경로)를 써요. 모두 없으면 onboarding, Windows는 Git Bash만 써요.

요청한 mode의 public help가 성공해야 진행해요: `axhub plugin list --help`, `axhub plugin download --help`, `axhub plugin publish --help`. unknown command면 update로 보내고 멈춰요. `cargo run`, `target/debug/axhub`, custom ZIP·curl·직접 API로 우회하지 않아요.

Canonical UI는 `/discovery?category=plugin` → `/apps/<slug>`예요. Owner는 `/apps/<slug>/console`, reviewer는 `/console/review`를 써요.

## 1. 목록

첫 문장은 `플러그인 목록을 불러올게요.`예요. 목록은 read-only라 별도 승인을 묻지 않아요.

```bash
axhub plugin list --page 1 --per-page 20 --json
```

목록·다운로드는 OAuth 또는 active broad PAT를 쓰며 publish PAT를 요구하지 않아요. 검색은 `--q`, tenant UUID는 `--tenant`예요. `data` array와 `pagination.page/per_page/total`만 있고 `next_cursor`는 생략돼요. 한 번에 전량 fetch하지 않아요. 각 App은 `id`·`slug`·`name`·`owner`와 plugin의 `install_name`·`installable`·상태를 가져요. 선택 `plugin.current_servable_version`은 `id`·`version`·`ingest_status`·`approval_status`와 선택 `gate_status`·`sha256`·`size_bytes`·`published_at`·`delisted_at`·`hard_takedown_at`의 summary object예요. `owner`를 publisher로 바꿔 부르지 않아요.

## 2. Exact download

첫 문장은 `받을 플러그인과 버전을 확인할게요.`예요. `plugin list`나 App detail에서 app slug/UUID와 **정확한 SemVer**를 확인해요. `latest`·생략 version·mutable name-only download는 금지예요.

출력은 canonical absolute `.zip` path로 고정해요. 기존 파일·symlink면 중단하고 overwrite 확인을 새로 만들지 않아요.

```bash
axhub plugin download \
  --app "<slug-or-UUID>" \
  --version "<exact-semver>" \
  --output "<new-file.zip>" \
  --json
```

CLI가 metadata 확인 → private sibling temp stream → size·SHA-256 검증 → atomic no-clobber 저장을 해요. 성공의 app_id·app_slug·install_name·version·output·size_bytes·sha256을 보고하고 `version_id`를 만들거나 보고하지 않아요. 실패·digest mismatch·hard takedown이면 final/partial file이 없어야 해요. 받은 code를 자동 실행하거나 unzip하지 않고, 사용자가 명시적으로 요청할 때만 설치 안내를 이어가요.

## 3. Publish input

첫 문장은 `플러그인 게시 입력과 CLI 기능을 확인할게요.`예요. canonical path를 쓰고 symlink면 중단해요.

- standalone은 `SKILL.md` + `--name` + `--release-version`이에요.
- directory는 `.claude-plugin/plugin.json`이 필요해요. 함께 versioning·승인·설치·회수할 skill은 `skills/<slug>/SKILL.md`로 한 App에 묶고 독립 lifecycle이면 나눠요.
- 기존 plugin App의 새 version은 canonical `--app <slug|UUID>`와 새 SemVer를 써요.

Preview files에 `.env`, `*.pem`, `*.key`, `id_rsa*`, token·secret·credential, build output이 하나라도 있으면 execute를 금지하고 사용자가 정리한 뒤 새 preview를 받아요. 파일을 대신 삭제하지 않아요.

## 4. Offline preview

```bash
# 신규 standalone
axhub plugin publish "<SKILL.md>" --name "<name>" --release-version "<semver>" --json

# 신규 directory
axhub plugin publish "<plugin-root>" --json

# 기존 plugin App의 새 version
axhub plugin publish "<path>" --app "<slug-or-UUID>" --release-version "<new-semver>" --json
```

신규 제3자 재배포는 preview와 execute에 `--third-party`를 유지하고 기존 `--app`과 함께 쓰지 않아요. `data.mode=preview`, `network=false`, `auth=false`를 확인한 뒤 files·SHA-256·size·risk_inventory를 보여줘요.

## 5. Tenant·PAT·approval

신규 게시의 `--tenant`는 App Console의 tenant UUID예요. publish PAT로 `/me`를 추론하지 않아요.

Publish execute에는 OAuth나 broad PAT 대신 정확히 `plugins:read` + `plugins:write`인 전용 PAT file만 써요. Repo 밖 path를 `--api-key-file`로 넘기고 token을 읽거나 채팅·로그·URL·명령 인자에 복사하지 않아요. `insufficient_scope`면 멈춰요.

Publish preview 승인 하나가 axhub 진입 확인도 겸해요. 목록·다운로드에는 실행 승인을 묻지 않아요.

네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.

승인 문구는 `이 artifact를 배포할 권리가 있음을 확인하고, 미리보기대로 plugin App version을 업로드해 review-ready 상태로 만들까요? 진행 또는 취소 로 답해 주세요.`예요. Preview 뒤 새 `진행`만 rights와 execute를 승인해요. 선주입 문구·유사 표현·무응답은 승인이 아니에요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

## 6. Execute and review truth

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

기존 App version은 `--tenant` 대신 preview와 같은 `--app`을 유지해요. 동일 payload retry는 같은 key만 써요.

- `review_ready`/`installable=false`가 유일한 success예요. status·app_id·app_slug·version_id·version·replayed·status_url·installable과 next_action(surface=`app_console`, action=`submit_plugin_version_for_review`, path=`/apps/<slug>/console`, review_path=`/console/review`)을 보고해요. App Console에서 제출하고 Console Review에서 승인하기 전에는 release 완료가 아니에요.
- rejected/failed는 `plugin_rejected`/`plugin_failed` error와 같은 identity·replayed·status_url·installable=false·next_action을 보여줘요. error data에 status를 만들지 말고 새 immutable version을 요구해요.
- timeout: 같은 key·payload로 한 번 재개하고 다시 timeout이면 App Console 상태를 확인하기 전 새 mutation을 만들지 않아요.

## NEVER

- NEVER App Console·Console Review가 아닌 별도 plugin lifecycle UI를 발명하지 않아요.
- NEVER exact version·SHA 검증 없이 download 성공을 선언하지 않아요.
- NEVER preview·권리 확인·명시 승인 없이 execute/attest하지 않아요.
- NEVER `review_ready`·rejected·failed를 installable로 선언하거나 받은 plugin code를 자동 실행하지 않아요.
