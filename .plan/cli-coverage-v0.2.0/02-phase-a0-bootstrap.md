# Phase A0 — Helper Bootstrap (4 Subcommand × 3 OS)

> Phase A0 의 두 번째 큰 chunk. helper Rust binary 가 install/fetch 모든 책임 = single source of truth. SKILL 은 NL wrap 만.

---

## 결정 배경 (사용자 통찰)

> "skill로 할 생각 말고 rust 코드로된 helper에서 설치하게 하면 되는거 아니야?"

**rationale**:
- helper 는 이미 multi-arch (5 binary: darwin-arm64/amd64, linux-arm64/amd64, windows-amd64) 출하
- helper 는 cosign 서명 + auto-download 인프라 운영 중
- SKILL = thin NL wrap (AskUserQuestion + helper subcommand 호출 + progress narration)
- install logic = Rust (typed, cargo test, cosign 서명, single source of truth)
- TS shadow 박멸 (v0.2.0)

**Plugin scope identity 보존**: plugin = NL deploy/manage. install 책임이 SKILL 안에 들어가면 scope drift (CEO codex "match CLI surface" warning 동일 패턴).

## OS scope: mac + linux + windows v0.2.0 모두

사용자 결정: 세 OS 모두 v0.2.0 ship. ~+10시간 effort. 100% feature parity.

## 신규 helper subcommand 4

### B0-1. `axhub-helpers bootstrap`

cold customer one-shot. plugin install 후 첫 init SKILL Step 3 가 호출.

**책임**:
1. node detect → 없으면 install
2. git skip (tarball 방식이라 불필요)
3. ax-hub-cli detect → 없으면 install (이미 `bin/install.sh` logic 활용)
4. sudo 필요 시 OS 별 prompt (Mac touchID / Linux pkexec / Windows UAC)
5. corporate proxy detect (`HTTPS_PROXY` env / `~/.npmrc` proxy / system proxy) → respect
6. progress JSON stream (stderr) — 30초 마다 SKILL workflow narration

**Input**: `--stack <slug>` (init SKILL Step 2 의 사용자 선택), `--no-confirm` (subprocess), `--json`

**Output (JSON)**:
```json
{
  "node_installed": true,
  "node_version": "20.18.0",
  "node_install_method": "volta",
  "axhub_cli_installed": true,
  "axhub_cli_version": "0.10.2",
  "elapsed_sec": 234,
  "warnings": ["nvm detected, volta installed alongside (no conflict)"]
}
```

**OS-specific install path**:

| OS | node install | 권한 |
|---|---|---|
| Mac | volta install (no sudo, ~/.volta) | user-local |
| Linux | volta install (no sudo, ~/.volta) | user-local |
| Windows | volta install (no admin, %USERPROFILE%/.volta) | user-local |

volta 자체 install:
- Mac/Linux: `curl https://get.volta.sh \| bash` (script 검증 helper 가 직접 download + cosign-style hash check)
- Windows: `volta-windows-installer.msi` download + 자동 install

**Volta vs nvm/asdf 충돌 처리**:
- nvm/asdf detect → warn but proceed (volta 가 fallback PATH 우선)
- 사용자 명시 reject 시 → 수동 install 안내 link 출력 후 exit 64

### B0-2. `axhub-helpers fetch-template <slug>`

examples repo (jocoding-ax-partners/examples) 의 template tarball download + extract.

**책임**:
1. `https://codeload.github.com/jocoding-ax-partners/examples/tar.gz/main` GET
2. cwd 의 빈 directory 검증 (axhub.yaml 이미 있으면 abort)
3. tarball 안의 `templates/<slug>/` 디렉토리만 extract (cwd 에 root)
4. examples repo 의 `LICENSE` / `README.md` 는 skip
5. tarball size cap = 100 MB (ax-hub-cli `MaxArchiveSize` 동일)
6. path traversal 방지 (existing `ExtractTarGz` 패턴)

**Input**: `<slug>` (positional), `--dest <path>` (default cwd)

**Output (JSON)**:
```json
{
  "template_slug": "nextjs-axhub",
  "files_extracted": 47,
  "bytes_extracted": 3245678,
  "elapsed_sec": 4
}
```

**git X**: tarball download = HTTPS GET. git binary 불필요. cold customer 진입장벽 추가 0.

### B0-3. `axhub-helpers install-deps`

template root 의 dep manifest detect + install.

**책임**:
1. cwd scan (depth 1):
   - `package.json` → `npm install` (또는 `yarn install` if `yarn.lock`, `pnpm install` if `pnpm-lock.yaml`)
   - `requirements.txt` → `pip install -r requirements.txt`
   - `pyproject.toml` → `pip install -e .` 또는 `uv sync` if `uv.lock`
   - `go.mod` → `go mod download`
   - `Gemfile` → `bundle install`
   - `Cargo.toml` → `cargo build`
2. 미지원 manifest = warn + 사용자가 수동 install 안내
3. progress JSON stream

**Input**: `--manifest auto|npm|pip|go|bundle|cargo` (default auto detect)

**Output (JSON)**:
```json
{
  "manifest": "package.json",
  "installer": "npm",
  "packages_installed": 234,
  "elapsed_sec": 87,
  "warnings": []
}
```

### B0-4. `axhub-helpers list-templates`

examples repo 의 `templates.json` manifest fetch.

**책임**:
1. `https://raw.githubusercontent.com/jocoding-ax-partners/examples/main/templates.json` GET
2. JSON parse
3. cache (`~/.cache/axhub-plugin/templates.json`, TTL = 1시간)
4. cache hit 시 stale-while-revalidate
5. fetch fail 시 fallback = ax-hub-cli builtin 5

**Output (JSON)**:
```json
{
  "schema_version": "templates/v1",
  "fetched_at": "2026-05-03T01:00:00Z",
  "source": "remote",
  "templates": [
    {
      "slug": "nextjs-axhub",
      "framework": "nextjs",
      "stack": ["node", "react", "tailwind"],
      "description": "Next.js + axhub deploy ready",
      "min_node": "18.0.0"
    }
  ]
}
```

**Input**: `--no-cache` (force fetch), `--json`

## Cross-platform smoke test matrix

`bun smoke:full` 확장:

```
helper smoke (5 binary × 3 OS = 15 cases):
  ✓ darwin-arm64: bootstrap (no node) → volta install → node v20 → ax-hub-cli download
  ✓ darwin-amd64: 동일
  ✓ linux-arm64 (alpine + ubuntu): 동일
  ✓ linux-amd64: 동일
  ✓ windows-amd64: 동일

  ✓ fetch-template nextjs-axhub (모든 5 binary)
  ✓ install-deps npm/pip/go (각 detect path)
  ✓ list-templates remote + cache hit + fallback
```

CI = 5 OS runner matrix (existing) + 4 신규 subcommand × 5 binary smoke = ~20 신규 test step.

## Risks (cross-platform 특이)

| Risk | OS | Mitigation |
|---|---|---|
| Mac touchID prompt = AppleScript wrapper, blocking | Mac | progress narration 명시 "지문 인식 창 떴어요" |
| Linux pkexec polkit absent | Linux | sudo fallback + "polkit 없으면 sudo 비밀번호 한 번" |
| Windows UAC = elevation token request | Windows | volta = no admin, but path 변경은 새 shell session 후 |
| corporate antivirus 가 helper Rust binary 차단 | All | cosign verify 결과 stderr + "회사 IT 에 'binary signed by axhub' 알림" |
| volta install script 가 corporate proxy block | All | `HTTPS_PROXY` env 명시 검증 + 못 받으면 "회사 IT 에 cli.jocodingax.ai 화이트리스트 요청" 안내 |
| nvm/asdf detect → volta 충돌 | Mac/Linux | warn + sudo X (user-local) + PATH 우선순위 안내 |
| node 가 PATH 에 있는데 너무 옛 버전 (v14 등) | All | min_node 검증 + volta 로 v20 install (기존 node 유지, PATH 우선만) |
| 회사 GitHub Enterprise = examples repo 접근 X | All | fallback to ax-hub-cli builtin 5 + "회사 GH proxy 설정 확인" 안내 |

## Effort

- helper bootstrap subcommand (Rust): ~4시간 (volta script wrap + OS-specific install + sudo handling)
- helper fetch-template + list-templates: ~2시간 (codeload tarball + cache + fallback)
- helper install-deps: ~1.5시간 (manifest detect + dispatch)
- cross-platform smoke matrix: ~2시간
- **Total: ~9.5시간**

## Validation gate before Phase B

- [ ] 5 binary 모두 cosign 서명 ship 가능
- [ ] cross-platform smoke 15 case PASS (3 OS × 5 binary 일부)
- [ ] examples repo 의 `templates.json` 가 fetch + parse 가능
- [ ] bootstrap 가 fresh Mac/Linux/Windows VM 에서 PASS (manual smoke)
- [ ] sudo handling smoke (Mac touchID prompt screenshot, Linux sudo prompt, Windows UAC dialog)
