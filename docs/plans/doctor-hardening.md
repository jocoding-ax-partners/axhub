# Doctor 스킬 고도화 플랜

> Windows CLI 세션 영속성 근본 수정 + axhub-helpers 자체 점검 + 환경변수(PATH) 직접 수정 + doctor 완전도 보강
>
> 작성: 2026-06-06 · 모드: SCOPE EXPANSION (구현 범위 C, cross-repo 포함) · 대상 repo 2개

---

## 0. 한 줄 요약

doctor 가 (1) Windows 에서 "다른 세션 = 안 깔림" 오진단을 더 이상 내지 않고, (2) helper 버전까지 검증하며, (3) consent 후 PATH 를 직접 영속화해서 "완벽하게 동작" 하도록 만들어요. 근본 원인은 **두 repo 의 결함 2개**라서 양쪽을 같이 고쳐요.

---

## 1. 문제 정의 (사용자 보고)

> "윈도우에서 한 세션에서 cli 설치했는데, 세션 나가고 다른 세션 열어서 cli 확인해보라고 하면 **설치 안 됐다**고 한다."

3대 요청:
- ① axhub-helpers 까지 점검
- ② 환경변수까지 직접 set 해서 완벽하게 동작
- ③ Windows CLI 세션 영속성을 **근본적으로** 고치고 **doctor 스킬로도** 고칠 수 있게

---

## 2. 근본 원인 분석 (실증 — file:line)

이 버그는 **하나가 아니라 결함 2개**예요. "설치 안 됐다고 함" 이라는 표현 자체가 단서예요 — `cli_present:false` 가 떴다는 뜻이고, 그건 PATH 미등록(=설치는 됨)이 아니라 **스캔이 실제 설치 경로를 놓쳤다**는 의미예요.

### 결함 1 — 오진단: preflight 가 실제 설치 경로를 스캔 안 함

`axhub` (plugin) `crates/axhub-helpers/src/preflight.rs:111` `fallback_axhub_paths()`:

```rust
pub fn fallback_axhub_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "macos") { /* homebrew, /usr/local/bin */ }
    else if cfg!(target_os = "linux") { /* /usr/local/bin, /usr/bin, linuxbrew */ }
    // ❌ cfg!(target_os = "windows") 브랜치가 아예 없음
    if let Some(home) = HOME or USERPROFILE {
        paths.push(home.join(".cargo/bin").join(AXHUB_BIN_NAME));
        paths.push(home.join(".local/bin").join(AXHUB_BIN_NAME));
        // ❌ home.join(".axhub/bin") 가 빠짐 — 이게 공식 installer 의 기본 설치 경로
    }
    paths
}
```

공식 installer 의 **기본 설치 경로**는 `ax-hub-cli/scripts/install.ps1:20` / `install.sh:27`:

```
Windows: %USERPROFILE%\.axhub\bin\axhub.exe
Unix:    $HOME/.axhub/bin/axhub
```

→ `.axhub/bin` 은 **모든 OS 에서** fallback 목록에 없음. Windows 는 cfg 브랜치 자체가 없어서 더 심함. PATH 에 없는 새 세션에서 GUI subprocess(Claude Desktop)는 shell PATH 도 못 물려받아서 → 스캔 실패 → `cli_present:false` → doctor "설치 안 됨" 오보.

### 결함 2 — 진짜 PATH 갭: 공식 installer 가 PATH 를 영속화 안 함

`ax-hub-cli/scripts/install.ps1:71-79`:

```powershell
if (($env:PATH -split ';') -notcontains $InstallDir) {
    Write-Host "note: $InstallDir is not on your PATH yet."
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', ... + ';$InstallDir', 'User')"
    Write-Host "then open a new terminal."
}
```

installer 는 PATH 를 **출력만** 하고 **직접 set 안 해요**. `install.sh:33-53` 도 동일 (`echo 'export PATH=...' >> ~/.bashrc` 안내만). Claude Code 는 이 note 를 사용자에게 안 보여주고 실행도 안 하니, HKCU User PATH 에 영영 안 들어가요 → 새 터미널 = `command not found`. 이게 "근본 원인" 이에요.

### 진단 요약

```
사용자: "session1 에서 설치" → axhub.exe 가 %USERPROFILE%\.axhub\bin 에 생김 (설치 자체는 성공)
              │
              ├─ 결함 2: installer 가 HKCU User PATH 에 .axhub\bin 안 넣음 (print만)
              │
session2(새 터미널): PATH 에 .axhub\bin 없음 → raw `axhub` 실패
              │
doctor 진단: preflight 스캔
              │
              ├─ 결함 1: fallback 에 .axhub\bin 없음 → 디스크의 axhub.exe 못 찾음
              │
              ▼
       cli_present:false → "설치 안 됐어요" ❌ (실제론 디스크에 있음)
```

---

## 3. 범위 결정 (확정)

| # | 결정 | 선택 |
|---|------|------|
| 1 | 구현 범위 | **C — 전체 + cross-repo** (axhub plugin + ax-hub-cli installer 둘 다 수정). 사용자가 `ax-hub-cli` repo 직접 작업 권한 부여 |
| 2 | doctor 직접 수정 env 범위 | **PATH 만** — PROFILE/ENDPOINT 는 null=정상 기본값 설계 존중, 자동 set 안 함 |
| 3 | mutation 코드 위치 | **sibling `repair` 스킬** — doctor 는 감지+제안만(model haiku 유지), 실제 PATH 수정은 새 `repair` 스킬(sonnet) |
| 4 | 추가 기능 | **4개 전부** — keychain 토큰 건강성 / 플러그인 캐시 staleness / self-heal 재진단 루프 / 네트워크·endpoint 프로브 |

### 명시적 비범위 (NOT in scope)

- PROFILE/ENDPOINT 자동 set — 잘못된 서버 지목 위험, 현 설계와 충돌. (사용자가 값을 명시하면 그때 별도 검토)
- fish / exotic 셸(login-only·비표준 rc)의 PATH 자동 쓰기 — zsh/bash 만 auto, 나머지는 advice fallback (rc 다양성이 HKCU 보다 fragile).
- doctor `--fix`/`--dry-run`/`--send-report` Rust stub 노출 — 별도 ralplan (현행 NEVER 유지)
- arm64 Windows — 현 release 미빌드 (별도)

---

## 4. 아키텍처

### 4.1 컴포넌트 맵 (2-repo)

```
┌─────────────────────────── ax-hub-cli repo (근본 수정) ───────────────────────────┐
│  scripts/install.ps1   ── HKCU User PATH 영속화 추가 (print → 실제 set)             │
│  scripts/install.sh    ── shell rc append 실제 수행 (옵션, 동의 기반)               │
│         │ 배포: cli.axhub.ai CDN 동기화 (release 파이프라인)                         │
└─────────┼──────────────────────────────────────────────────────────────────────────┘
          │ 신규 설치자는 여기서 영속 PATH 획득 (근본 해결)
          ▼
┌─────────────────────────── axhub plugin repo (감지 + 복구) ──────────────────────┐
│                                                                                    │
│  crates/axhub-helpers/src/preflight.rs                                             │
│    └ fallback_axhub_paths()  ── .axhub/bin (전 OS) + Windows cfg 브랜치 추가       │
│    └ PreflightReport         ── cli_on_path / cli_on_disk_only 구분 필드 추가      │
│                                                                                    │
│  crates/axhub-helpers/src/main.rs                                                  │
│    └ repair-path  (신규 subcommand) ── HKCU User PATH 영속 쓰기 + WM_SETTINGCHANGE │
│    └ doctor --json / doctor-summary ── helper 버전 self-check + 4 신규 row         │
│                                                                                    │
│  skills/doctor/SKILL.md  (haiku 유지, read-only)                                   │
│    └ "디스크에 있는데 PATH 미등록" 감지 → AskUserQuestion → repair 스킬 라우팅     │
│                                                                                    │
│  skills/repair/SKILL.md  (신규, sonnet, mutate)  ◀── 기존 onboarding 라우팅 패턴   │
│    └ consent 후 `axhub-helpers repair-path` 호출 → self-heal 재진단                │
└────────────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 왜 sibling `repair` 스킬인가 (결정 3)

- doctor 의 "NEVER auto-fix" + `model: haiku` (빠른 read-only) 계약 **보존**.
- 기존 `cli_present:false → Skill("axhub:onboarding")` 라우팅과 **동일 패턴** — 새 멘탈모델 없음.
- mutation(레지스트리 쓰기)은 sonnet + consent-gate + D1 guard 가 필요한 무거운 동작 → 격리.
- 사용자 체감: "진단해줘" → doctor 가 문제 찾고 "고칠까요?" → 고쳐줌. **"doctor 로도 고칠 수 있게"** 충족.

**doctor 는 mutate 하지 않는 orchestrator (codex C5 정합).** doctor 자신은 레지스트리/PATH 를 **직접 안 건드려요** — 감지 + AskUserQuestion + `Skill("axhub:repair")` 라우팅만. 실제 mutation 은 repair(sonnet)가 수행. 이건 기존 `cli_present:false → Skill("axhub:onboarding")` 와 **동일 구조**라 doctor 의 "NEVER auto-fix" 계약(SKILL.md:296)을 깨지 않아요. "NEVER auto-fix" = "consent·sibling 없이 직접 mutate 금지" 의 의미예요. doctor `model: haiku` 유지.

**`repair` 는 신규 스킬 (확인됨).** 기존 `recover` 는 **배포 롤백**(직전 안정 commit 재배포), `rollback` 은 **deploy-id 롤백** 담당이라 env/PATH 복구와 무관해요. `repair` 는 환경 복구 전용 신규 스킬이에요. trigger 어구 충돌 방지: `repair` = "PATH 고쳐줘" / "환경 복구해줘" / "경로 등록해줘" 류만 (롤백/되돌려/이전 버전 류 금지 — `recover`/`rollback` 소유). `lint:keywords` baseline 으로 잠가요.

---

## 5. Pillar A — axhub-helpers 자체 점검 (요청 ①)

현재 doctor 는 helper **버전을 출력만** 하고 검증 안 해요 (CLI 는 range-check 하는데 helper 는 안 함 = 비대칭).

**추가:**
- `doctor --json` 출력에 `helper_version_expected`(plugin 버전, codegen:version 동기값) + `helper_version_ok`(skew 여부) 필드.
- doctor 카드에 row 추가:
  - `✓ helper 바이너리: 정상 (axhub-helpers v0.9.33, plugin 과 일치)`
  - `⚠ helper 버전 불일치: helper v0.9.20 < plugin v0.9.33 — '업데이트 확인해줘'`
- helper **실행 가능성** 점검: `preflight` 가 sane exit code 를 내는지 (EDR 격리/깨진 바이너리 조기 발견).

read-only → doctor haiku 유지.

---

## 6. Pillar B — 환경변수(PATH) 직접 수정 (요청 ②)

### 6.1 신규 helper subcommand: `axhub-helpers repair-path`

```
axhub-helpers repair-path --json [--dir <install_dir>]
```

동작:
1. install dir 결정 — `--dir` > `AXHUB_INSTALL_DIR` > `CARGO_DIST_INSTALL_DIR` > `CARGO_HOME\bin` > `%USERPROFILE%\.axhub\bin`. **installer(`install.ps1:13-21`)와 byte-identical 우선순위** (codex C1: `CARGO_DIST_INSTALL_DIR` 누락·순서 오류 시 repair 가 엉뚱한 dir 을 봐서 복구 대상 사용자에게서 실패).
   - ★ **단일 공유 contract**: 이 해석은 `preflight` fallback(§7.1)과 `repair-path` 가 **공유하는 한 함수** `install_dir_candidates() -> Vec<PathBuf>` 로 추출해요. 두 곳이 각자 구현하면 드리프트 → 같은 클래스 버그 재발. 단위테스트로 installer 우선순위와 1:1 잠금.
2. 그 dir 에 `axhub(.exe)` 존재 확인. 없으면 `{repaired:false, reason:"binary_not_found"}`.
3. **Windows**: HKCU `Environment` 의 `Path` 를 **raw 로 읽고**(REG_EXPAND_SZ 보존) dir 가 없으면 append, **읽은 타입 그대로** write. `WM_SETTINGCHANGE` broadcast 는 **best-effort cosmetic** — 새 셸은 어차피 레지스트리를 새로 읽고, 현재 실행 중인 프로세스는 broadcast 받아도 PATH 갱신 안 됨(env 상속). 실패해도 무시. installer(§7.3)와 동일하게 broadcast 생략 가능 — 일관성 위해 양쪽 다 cosmetic 으로 표기.
4. **Unix (macOS/Linux, 대칭 확장)**: consent 후 rc-persist. 셸 감지(`$SHELL` basename) → zsh→`~/.zshrc`(macOS 기본) / bash→`~/.bashrc`. **idempotent**(이미 dir 있으면 skip, grep 검사) append: `export PATH="$HOME/.axhub/bin:$PATH"` — `$HOME`·`$PATH` **literal 유지**(셸 시작 시 확장, install-time 아님; install.sh:51 동일 계약). marker 주석(`# added by axhub`)으로 식별·제거 가능. **backup**: rc 를 `<rc>.axhub-backup-<ts>` 로 복사 후 편집. **fish/exotic 셸은 advice fallback**(자동 안 씀, 명령만 안내). Windows HKCU 와 동일 안전 바(idempotent + consent + backup).
5. 현재 프로세스 PATH 도 in-memory 로 갱신(즉시 재검증용).
6. fail-open: 모든 실패에서 `exit 0` + `{repaired:false, reason, systemMessage}`. panic 금지 (hook safety 계약).

**의존성**: 새 crate 없이 `windows-sys 0.61` 에 features 추가 — `Win32_System_Registry`(RegGetValueW/RegSetValueExW), `Win32_UI_WindowsAndMessaging`(SendMessageTimeoutW), `Win32_Foundation`. (대안: `winreg` crate — REG_EXPAND_SZ raw 처리가 더 안전·간결. 신규 dep 1개 vs 직접 FFI. 구현 시 결정, winreg 권장.)

### 6.2 ⚠️ CRITICAL 정확성 리스크: REG_EXPAND_SZ 손상

User PATH 레지스트리는 보통 `REG_EXPAND_SZ` 라 `%USERPROFILE%` 같은 미확장 변수를 담아요. 흔한 파괴 패턴:
- `[Environment]::GetEnvironmentVariable('Path','User')` 는 값을 **확장해서** 반환 → 그대로 write 하면 `%VAR%` 가 literal 경로로 박제됨 (다른 앱 PATH 손상).
- `setx` 는 **1024자에서 truncate** (PATH 잘림).

**필수 mitigation:**
- raw 레지스트리 값을 타입 보존해서 읽기 (`winreg::get_raw_value` / PS 는 `(Get-Item 'HKCU:\Environment').GetValue('Path','','DoNotExpandEnvironmentNames')`).
- **읽은 타입 그대로 write** (REG_EXPAND_SZ 면 ExpandString, REG_SZ 면 String). 무조건 ExpandString 강제 금지 (codex C2: §7.3 스니펫이 prose 와 모순됐음). `setx` 금지, 레지스트리 API 직접.
- **멤버십 체크는 raw·expanded 양쪽 비교** (codex C2): 기존 PATH 가 미확장 `%USERPROFILE%\.axhub\bin` 을 담고 있으면, 확장형 `C:\Users\..\.axhub\bin` 하고만 비교 시 **중복 append**. 각 entry 를 `ExpandEnvironmentVariables` 후 정규화(case-insensitive + trailing `\`) 비교.
- **백업은 로그가 아니라 구체 롤백 artifact** (codex C4): write 전 기존 raw PATH 를 `%LOCALAPPDATA%\axhub\path-backup-<ISO8601>.txt` 에 저장 + 복구 명령 1줄 안내. 단순 로그 라인 금지.

이 항목이 이 플랜의 **#1 리스크**예요. 잘못하면 사용자 전체 PATH 를 망가뜨려요.

### 6.3 현재 세션 한계 + DX1 해소 (agent-mediated absolute path)

repair 가 HKCU 를 고쳐도 **이미 실행 중인 Claude/shell 프로세스**의 PATH 는 자동 갱신 안 돼요 (프로세스 시작 시 상속).

**EUREKA (DX-review DX1):** 이 한계는 **raw 셸 사용에만** 적용돼요. axhub 의 모든 CLI 호출은 **agent(Claude)가 mediate** 하니, skill 들이 `axhub` bare 대신 **resolved 절대경로**(`install_dir_candidates()` hit)로 호출하면 **현재 세션도 즉시 동작**해요. helper 는 이미 preflight 에서 `resolve_axhub_path()` 로 절대경로를 씀 — 이걸 skill-driven `axhub` 호출(deploy 등)에도 확장.

결과 분담:
- **영속 PATH 쓰기** → 미래 세션 + 사용자의 raw 셸 사용 해결.
- **agent 절대경로 mediate** → 현재 세션 즉시 해결 (vibe coder 가 "고쳤다는데 안 됨" 안 겪음).
- 둘 합쳐 진짜 **"완벽하게 동작"** 전달.

그래서:
- self-heal 재진단은 PATH 가 아니라 **known-path 직접 호출**로 검증 (결함 1 수정 후 fallback 이 찾아줌).
- repair 성공 카드 (해요체, 정직): "PATH 영속 설정 완료 — **이 창에선 제가 정확한 경로로 대신 실행**하고, 새로 여는 터미널에선 직접 `axhub` 가 바로 동작해요." (raw 셸 한계를 positive 하게 frame, 알람 X)
- on_disk_not_on_path 상태일 때 doctor/repair/deploy skill 의 `axhub` 호출은 resolved 절대경로 사용.

---

## 7. Pillar C — CLI PATH 영속성 (cross-platform, 요청 ③ 근본 + doctor)

> **적용 범위 (사용자 Mac 질문으로 확정):** root cause 는 **전 OS** 동일(install.sh/ps1 둘 다 print-only + fallback `.axhub/bin` 누락). 분담:
> - **defect 1 fix(§7.1) + DX1(§6.3)** → 전 OS — vibe coder(agent-mediated)는 Mac·Windows 둘 다 이걸로 해결.
> - **defect 2 persist** → Windows HKCU(auto) + **Mac/Linux rc-persist(대칭, §6.1 item 4)** — raw 터미널 사용자용.

### 7.1 결함 1 수정 (axhub plugin / preflight.rs)

```rust
pub fn fallback_axhub_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    // installer env 우선 (parity)
    for var in ["AXHUB_INSTALL_DIR", "CARGO_DIST_INSTALL_DIR"] {
        if let Some(d) = std::env::var_os(var) {
            paths.push(PathBuf::from(d).join(AXHUB_BIN_NAME));
        }
    }
    if cfg!(target_os = "macos") { /* 기존 */ }
    else if cfg!(target_os = "linux") { /* 기존 */ }
    else if cfg!(target_os = "windows") {
        if let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            paths.push(home.join(".axhub").join("bin").join(AXHUB_BIN_NAME)); // ★ 공식 기본 경로
        }
        if let Some(ch) = std::env::var_os("CARGO_HOME").map(PathBuf::from) {
            paths.push(ch.join("bin").join(AXHUB_BIN_NAME));
        }
    }
    if let Some(home) = HOME_or_USERPROFILE {
        paths.push(home.join(".axhub/bin").join(AXHUB_BIN_NAME)); // ★ 전 OS 공통 누락분
        paths.push(home.join(".cargo/bin").join(AXHUB_BIN_NAME));
        paths.push(home.join(".local/bin").join(AXHUB_BIN_NAME));
    }
    paths
}
```

### 7.2 진단 상태 3분기 (오진단 → 정확한 메시지)

preflight 에 구분 추가:

| axhub.exe 디스크 | PATH 등록 | 새 상태 | `cli_present` | `cli_on_path`★ | doctor 메시지 |
|---|---|---|---|---|---|
| ✓ | ✓ | `ok` | true | true | ✓ CLI 설치: v… |
| ✓ | ✗ | `on_disk_not_on_path` ★신규 | **true** | false | ⚠ 설치는 됐는데 PATH 미등록 — "PATH 고쳐줘" → repair |
| ✗ | ✗ | `missing` | false | false | ✗ 설치 안 됨 → install-cli |

★신규 상태가 핵심 — 더 이상 "설치 안 됨" 오보 안 냄.

**⚠ blast radius (eng-review F1, confidence 9/10): `cli_present` 의미 보존.** `on_disk_not_on_path` 는 CLI 가 resolved 절대경로로 **실제 동작**하므로 `cli_present=true` 를 **유지**해요. 새 bool `cli_on_path` 만 추가. 이유: `preflight.rs:579` `exit_code = if !cli_present || !in_range { EXIT_USAGE }` 이고 `deploy_prep.rs:57` 이 `preflight_exit_code` 를 소비 → `cli_present=false` 로 바꾸면 동작하는 CLI 로 배포가 차단돼요(statusline 도 동일). doctor 카드는 `cli_on_path` 로 "PATH 미등록" row 를 그리고, `cli_present` 는 건드리지 않아요. **이 backward-compat 은 mandatory regression test (§12).**

### 7.3 결함 2 근본 수정 (ax-hub-cli / install.ps1 + install.sh)

`install.ps1` 의 print-only 블록을 실제 영속화로 교체:

```powershell
# (Install-ZipArchive 내부, "note:" 블록 대체)
$key = Get-Item 'HKCU:\Environment' -ErrorAction SilentlyContinue
$userPathRaw = if ($key) { $key.GetValue('Path', '',
    [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) } else { '' }
$existingType = if ($key) { $key.GetValueKind('Path') } else { 'ExpandString' }
# 멤버십: raw·expanded 양쪽 정규화 비교 (중복 append 방지)
$target = [Environment]::ExpandEnvironmentVariables($InstallDir).TrimEnd('\')
$already = ($userPathRaw -split ';' | Where-Object { $_ } | Where-Object {
    [Environment]::ExpandEnvironmentVariables($_).TrimEnd('\') -ieq $target })
if (-not $already) {
    $newPath = if ($userPathRaw) { "$userPathRaw;$InstallDir" } else { $InstallDir }
    # 기존 타입 보존 write (REG_EXPAND_SZ ↔ ExpandString). setx 금지(1024 truncate)
    Set-ItemProperty -Path 'HKCU:\Environment' -Name 'Path' -Value $newPath -Type $existingType
    $env:PATH = "$env:PATH;$InstallDir"  # 현재 세션
    Write-Host "added $InstallDir to your user PATH (new terminals will see it)"
} else {
    $env:PATH = "$env:PATH;$InstallDir"
}
```

`install.sh` (대칭): `install.sh:33-53` 의 print-only 블록을 **실제 rc-persist** 로 교체 (install.ps1 과 동일 패턴). zsh→`~/.zshrc`/bash→`~/.bashrc` 감지, idempotent append(`export PATH="$HOME/.axhub/bin:$PATH"` literal 유지), marker 주석, fish→안내 fallback. 현재 셸엔 `export PATH=...` eval. installer 는 비대화형이라 consent 없이 append하되 idempotent + marker 로 안전(사용자가 직접 `curl|bash` 실행 = 암묵 동의).

**배포 주의**: 이 스크립트는 `cli.axhub.ai` CDN 으로 서빙돼요. repo 수정 후 **CDN 동기화(release 파이프라인)** 가 돼야 신규 설치자에게 반영돼요. 동기화 메커니즘 확인 필요 (deploy 항목).

---

## 8. 추가 기능 (요청 4개 전부)

| 기능 | 구현 | doctor row 예시 | read/write |
|---|---|---|---|
| keychain 토큰 건강성 | `keychain.rs`/`keychain_windows.rs` 로 토큰 read 가능 여부 probe. "못 읽음(EDR/권한)" vs "미로그인" 구분 | `⚠ 토큰 저장소 접근 불가 (보안 솔루션 격리 가능성)` | read |
| 플러그인 캐시 staleness | `~/.claude/plugins/cache/*/*/bin/` 다중 버전·디스크 스캔. >1 버전이면 경고 + 정리 안내 | `⚠ 캐시에 helper 3개 버전 (디스크 240MB) — 정리 안내` | read |
| self-heal 재진단 루프 | repair 후 자동 재 preflight (known-path 직접) → green 확인. closed-loop | `✓ 수정 완료, 재진단 통과` | read |
| 네트워크/endpoint 프로브 | endpoint 도달성 점검 (`--offline` 반대편). timeout 짧게, fail 시 프록시/방화벽 힌트 | `✗ endpoint 도달 불가 (회사 프록시 가능성)` | read |

전부 read-only → doctor haiku 유지. self-heal 루프만 repair 스킬(sonnet) 종료부에서 실행.

**⚠ probe 배치 (eng-review F2/F3, confidence 8/10): hot path 오염 금지.** 4 probe 는 **`cmd_doctor`(명시적 full doctor)에만** 둬요. `preflight`(statusline·deploy_prep 가 빈번 호출하는 hot path)와 `doctor-summary`(Desktop 단일-call fast path)에는 **넣지 않아요** — keychain/endpoint 는 IO·network 라 매 호출 수백ms 오염. fast path 는 cli 3-state + helper-skew(로컬·값쌈)만. JSON surface 3개 구분: `preflight`(core, lean) / `doctor-summary`(Desktop fast, lean) / `cmd_doctor`(full, probe 포함).

---

## 9. Error & Rescue Map

| codepath | 실패 모드 | rescue | 사용자 노출 |
|---|---|---|---|
| `repair-path` HKCU 읽기 | 레지스트리 키 없음/권한 | 빈 PATH 로 간주, 신규 생성 | "PATH 새로 만들었어요" |
| `repair-path` HKCU 쓰기 | 권한 거부/EDR | `repaired:false` + 수동 명령 안내 | 수동 `SetEnvironmentVariable` 1줄 제시 |
| `repair-path` REG_EXPAND_SZ | 타입 오판 → 손상 위험 | **raw 읽기 강제 + 백업 로깅** | (방지가 핵심) |
| install.ps1 PATH set | HKCU 접근 실패 | try/catch → 기존 print 안내로 fallback | note 출력 (현행 유지) |
| preflight 스캔 | dir 권한 거부 | skip 후 다음 후보 | 영향 없음 |
| keychain probe | PowerShell 부재(Server Core) | `unknown` 상태 | "토큰 상태 확인 불가" |
| endpoint probe | timeout | `unreachable` (fail ≠ error) | 프록시 힌트 |
| self-heal 재진단 | 여전히 fail | 루프 1회 한정, 무한방지 | "수동 확인 필요" |

원칙: 모든 hook/helper 진입점 `exit 0`, panic 금지, `unwrap()` 금지 (`hook_safety` 계약, CLAUDE.md).

---

## 10. Edge Cases (Windows 중심)

- **MAX_PATH 260자**: `.axhub\bin` 경로가 긴 한글 profile 에서 초과 — install.ps1 기존 PathTooLong catch 재사용.
- **PATH 1024자 초과**: 레지스트리 직접 write 로 `setx` truncate 회피.
- **중복 append**: 재실행 시 idempotent (정규화 멤버십 체크).
- **`%USERPROFILE%` 미확장 항목 보존**: REG_EXPAND_SZ 유지 (§6.2).
- **Git Bash on Windows**: bash 경로 유효성 유지 — PowerShell 추가가 bash 동작 안 뺏음 (spec 004 계약).
- **현재 세션 미갱신**: 새 터미널 안내 + self-heal 은 known-path 검증.
- **non-interactive(D1)**: `claude -p`/CI 는 repair AUQ skip → safe_default(수동/나중에). registry.json `repair` 섹션 등록.
- **읽기 전용 HKCU(기업 GPO)**: write 실패 → 수동 안내 fallback.

---

## 11. Security & Threat Model

| 위협 | 가능성 | 영향 | mitigation |
|---|---|---|---|
| 레지스트리 PATH 손상 (REG_EXPAND_SZ) | 중 | 높음 (전 PATH 파괴) | raw 읽기 + 백업 + 타입 보존 (§6.2) |
| PATH injection (악성 dir 주입) | 낮음 | 중 | dir 는 installer 우선순위/known 경로만, 임의 입력 거부 |
| install 스크립트 supply chain | 낮음 | 높음 | `cli.axhub.ai` 단일 채널 + SHA256 pin (현행 유지) |
| consent 우회 | 낮음 | 중 | D1 guard + AUQ, subprocess 자동 write 금지 |
| 토큰 노출 | 낮음 | 높음 | keychain probe 는 read 가능성만, 토큰 내용 절대 echo 안 함 (현행 NEVER) |

---

## 12. Test Plan

**Rust (axhub-helpers):** — eng-review coverage diagram(§18) 기준, 신규 codepath 30개 전부 impl 과 동시 작성.
- ★ **CRITICAL REGRESSION (mandatory, no-ask)**: `on_disk_not_on_path` 에서 `cli_present=true` 유지 → `preflight` exit_code 불변 + `deploy_prep`/`statusline` 동작 불변 assert. (F1 blast radius 증명)
- `install_dir_candidates()`: 우선순위 4분기 (`AXHUB_INSTALL_DIR`>`CARGO_DIST_INSTALL_DIR`>`CARGO_HOME\bin`>`.axhub\bin`) + per-OS cfg. installer 와 1:1 잠금.
- `resolve_axhub_path()`: PATH hit→source=path / fallback hit→source=fallback / miss.
- preflight 3분기 (`ok`/`on_disk_not_on_path`/`missing`) + `cli_on_path` 값.
- `repair-path` (Windows mock): (a) binary found/not, (b) 신규 추가, (c) idempotent 재실행, (d) **REG_EXPAND_SZ 보존**, (e) **REG_SZ(no %var%) 보존** ◀신규, (f) **HKCU key 부재→생성** ◀신규, (g) **membership dedup: 기존이 미확장 `%USERPROFILE%\.axhub\bin`** ◀codex C2, (h) backup artifact write, (i) 권한 실패 fail-open exit 0.
- `repair-path` (Unix mock): zsh→`~/.zshrc`/bash→`~/.bashrc` 감지 / idempotent(이미 dir 있으면 skip) / backup 생성 / literal `$HOME`·`$PATH` 보존(install-time 미확장) / fish→advice fallback(자동 안 씀).
- helper 버전 skew: older/newer/equal/unparseable.
- 4 probe (keychain/cache/endpoint/self-heal): 각 happy + fail-open. **`cmd_doctor` 에서만 호출됨을 assert** (preflight/doctor-summary 엔 없음).

**Installer (ax-hub-cli `install.ps1` + `install.sh`):**
- ps1: fresh install / re-install idempotent(중복 append 0) / PATH 이미 등록(raw+expanded 양형) / HKCU `Path` 부재→생성 / write 실패→print fallback.
- sh: fresh install → rc append / re-install idempotent(중복 0) / zsh·bash rc 선택 / 이미 PATH 등록 시 skip / fish→안내.

**Skill (CLAUDE.md 계약 — 무조건):**
- `bun run skill:new repair` 스캐폴드 (직접 생성 금지)
- `bun run skill:doctor --strict` exit 0 (D1 sentinel / TodoWrite / preflight / step-collision)
- `bun run lint:tone --strict` 0 err (해요체)
- `bun run lint:keywords --check` no diff (trigger 어구는 frontmatter description 만)
- `tests/fixtures/ask-defaults/registry.json` 에 `repair` 섹션 + doctor 신규 AUQ 등록 (`tests/ux-ask-fallback-registry.test.ts`)
- `bun test` ≥498 pass / 0 fail
- `bunx tsc --noEmit` clean
- `tests/hooks-kill-switch.test.ts` (신규 env opt-out 추가 시)

**E2E (수동, Windows VM):**
- session1 설치 → session2 (새 터미널) → "진단해줘" → "설치는 됐는데 PATH 미등록" 정확 표시 → "PATH 고쳐줘" → 새 터미널에서 `axhub` 동작.

---

## 13. Deploy & Rollout (cross-repo 순서)

```
1. axhub plugin: preflight 결함1 수정 (감지 정확화) ── 단독으로도 오진단 해소, 위험 0
   └ release (commit-and-tag-version 2단계 flow)
2. axhub plugin: repair-path helper + repair 스킬 + 4 probe
   └ release:check 5-binary build 통과 필수
3. ax-hub-cli: install.ps1/sh PATH 영속화
   └ CDN(cli.axhub.ai) 동기화 — 신규 설치자 근본 해결
   └ ✅ 배포 경로 확인됨: ax-hub-cli `.github/workflows/release.yml:373` "Publish CDN" job 이
      `scripts/install.ps1`/`install.sh` 를 GCS 버킷(`gs://${BUCKET}/install.ps1`)에 업로드
      (`release-cdn-assets.py` + `gsutil`). **단, stable tag 에서만 fire** — prerelease 로는 안 나감.
```

- **feature flag / 롤백**: preflight 변경은 순수 additive (경로 추가) → git revert 안전. repair 는 신규 스킬 → 미호출이면 영향 0. installer 변경은 try/catch fallback 으로 기존 print 동작 보존.
- **순서 근거**: 1번이 가장 안전·고효과(오진단 즉시 해소). 2번이 doctor 복구. 3번이 cross-repo 근본. 1→2→3 독립 배포 가능.
- env opt-out: `AXHUB_DISABLE_PATH_REPAIR=1` (ADR §10.6 `AXHUB_DISABLE_*` polarity 준수) — 기업 GPO 환경 우회용.

---

## 14. 리스크 등록부

| # | 리스크 | 심각도 | 대응 |
|---|---|---|---|
| R1 | REG_EXPAND_SZ PATH 손상 | **CRITICAL** | raw 읽기/타입 보존/백업 (§6.2), mock 테스트 필수 |
| R2 | installer 수정이 CDN 에 미반영 | 중 (↓완화) | 경로 확인됨(release.yml:373 CDN job). 단 **stable tag 필요** — release flow 에 포함. 1·2번(plugin)이 독립적으로 증상 해소라 fallback 존재 |
| R3 | doctor haiku→증가된 로직으로 느려짐 | 낮음 | probe 는 짧은 timeout, 4 probe 병렬, read-only 유지 |
| R4 | repair 현재 세션 미갱신 혼란 | 중 | "새 터미널" 명시 + self-heal known-path 검증 |
| R5 | skill 계약 회귀 (lint/test) | 중 | scaffold 강제 + self-check 체크리스트 |

---

## 15. 작업 분해 (순서 + 검증 게이트)

### Phase 1 — 오진단 해소 (axhub plugin, 위험 0)
- [ ] **T0 (P1, codex C1)** `install_dir_candidates()` 공유 함수 추출 — installer 우선순위(`AXHUB_INSTALL_DIR > CARGO_DIST_INSTALL_DIR > CARGO_HOME\bin > .axhub\bin`)를 단일 source 로. preflight·repair-path 둘 다 이걸 사용.
  - Verify: `cargo test install_dir_candidates` (installer 우선순위 1:1 잠금)
- [ ] **T1 (P1)** `preflight.rs` `fallback_axhub_paths()` 가 `install_dir_candidates()` 사용 (`.axhub/bin` 전 OS + Windows cfg 포함)
  - Verify: `cargo test fallback_axhub_paths`
- [ ] **T2 (P1)** preflight 3분기 상태(`on_disk_not_on_path` 신규) + **`cli_on_path` bool 신설, `cli_present=true` 유지**(F1)
  - ⚠ `resolve_axhub_path()` 가 현재 first-hit 절대경로만 반환 → **어느 source 가 매칭됐는지**(PATH vs fallback) 같이 반환하도록 확장 필요. PATH 매칭=`cli_on_path=true`, fallback-only 매칭=`on_disk_not_on_path`(`cli_present=true`, `cli_on_path=false`) 판정의 근거.
  - Verify: `cargo test preflight` + **CRITICAL regression** (cli_present 불변), `doctor --json` 출력 확인
- [ ] **T3 (P1)** doctor SKILL: 신규 상태 카드 메시지 + "PATH 고쳐줘" 라우팅 row
  - Verify: `bun run skill:doctor --strict`, `lint:tone --strict`

### Phase 2 — helper self-check + 복구 (axhub plugin)
- [ ] **T4 (P1)** `repair-path` subcommand — `#[cfg(windows)]` HKCU 영속(§6.2 준수) + `#[cfg(unix)]` rc-persist(§6.1 item 4: zsh/bash 감지, idempotent, backup, fish→advice), fail-open
  - Verify: `cargo test repair_path` (Windows mock + Unix rc mock 양쪽)
- [ ] **T5 (P1)** `bun run skill:new repair` 스캐폴드 → consent + repair-path 호출 + self-heal
  - Verify: `skill:doctor --strict`, registry.json `repair` 섹션 등록
- [ ] **T5.5 (P1, codex C6b)** `install-cli` post-install verify 를 raw `axhub --version`(SKILL.md:133) 대신 **known-path**(`install_dir_candidates()` 직접) 로 변경 — same-session child→parent PATH 미전파로 인한 false-fail 제거. PATH 미등록이면 repair 안내로 연결.
  - Verify: `bun run skill:doctor --strict`, Windows VM same-session 검증
- [ ] **T5.6 (P1, DX1)** on_disk_not_on_path 상태에서 doctor/repair/deploy skill 의 `axhub` 호출을 **resolved 절대경로**로 — agent 가 현재 세션 mediate, "고쳤는데 안 됨" 제거. helper 가 resolved path 를 skill 에 노출(JSON `cli_resolved_path`).
  - Verify: on_disk_not_on_path 시 skill 이 절대경로 사용 assert, repair 성공 카드에 정직한 "이 창/새 창" 문구
- [ ] **T5.7 (P2, DX2)** repair 스킬은 doctor 라우팅 시 **재consent 안 함** (single consent handoff) — doctor AUQ 가 THE consent. 단 repair 직접 호출 시엔 자체 AUQ 유지. registry.json `repair` 섹션은 직접-호출 경로용.
  - Verify: doctor→repair 경로 single-consent assert, 직접 repair 경로 AUQ 유지
- [ ] **T6 (P2)** helper 버전 self-check (`helper_version_ok`) + doctor row
  - Verify: `cargo test`, doctor 카드
- [ ] **T7 (P2)** 4 probe (keychain/cache/endpoint) + self-heal 루프 — **`cmd_doctor` 에만**(F2/F3), preflight·doctor-summary hot path 제외
  - Verify: `cargo test` + probe 가 preflight/doctor-summary 출력에 없음 assert, fail-open 확인
- [ ] **T8 (P1)** `AXHUB_DISABLE_PATH_REPAIR` opt-out + `hooks-kill-switch` 테스트
  - Verify: `bun test tests/hooks-kill-switch.test.ts`
- [ ] **T9 (P1)** 전체 게이트: `bun test` ≥498 / `tsc --noEmit` / `release:check`
- [ ] **T10 (P1)** plugin release (2단계 flow + CHANGELOG narrative)

### Phase 3 — 근본 수정 (ax-hub-cli, cross-repo)
- [ ] **T11 (P1)** `scripts/install.ps1` PATH 영속화 (print → set, REG_EXPAND_SZ 보존)
  - Verify: Windows VM 신규 설치 → 새 터미널 `axhub` 동작
- [ ] **T12 (P1, 대칭)** `scripts/install.sh` print-only(33-53)→실제 rc-persist (zsh/bash 감지, idempotent, marker, fish→안내). install.ps1 과 동일 패턴.
  - Verify: macOS VM fresh install → 새 Terminal `axhub` 동작 + 재실행 중복 0
- [ ] **T13 (P1)** ax-hub-cli **stable** release → CDN 자동 publish (release.yml:373 job)
  - Verify: release 후 `irm https://cli.axhub.ai/install.ps1` 이 신규(PATH 영속) 스크립트 서빙 + `scripts/cdn-verify.sh` 통과

---

## 16. Dream State Delta

```
현재                         이 플랜                        12개월 이상
─────────────────────────────────────────────────────────────────────────
doctor = 수동 진단 카드   →  doctor = 감지+복구 closed-loop  →  "환경 문제는
"설치 안 됨" 오보            "설치는 됐는데 PATH" 정확 +          axhub 가 알아서
새 터미널 = 깨짐            consent 후 영속 self-heal           고쳐주는" 자가치유
helper 검증 없음           helper/cache/keychain/net 풀점검    플랫폼
```

---

## 부록 — 추가로 고려했으나 안 물어본 것 (follow-up 후보)

- **hook 건강성 점검** (axhub hook 등록 + fail-open 계약 확인) — maintainer 성격이라 이번 4개에서 제외. 가치 있으면 다음 라운드.
- doctor `--bundle` 지원 리포트 표면화 (이미 구현됨, 안내만 추가)
- settings.json statusLine autowire 건강성

---

## 17. Codex 독립 리뷰 반영 (cross-model)

codex(다른 AI) 가 plan + 실제 소스를 대조해 adversarial 검토. 발견 6개 중 4개 objective 결함 → 즉시 수정, 1개 wording → 정합, 1개 scope tension → 사용자 확인.

| # | codex 발견 | 판정 | 반영 |
|---|---|---|---|
| C1 | repair-path 경로 우선순위가 `CARGO_DIST_INSTALL_DIR` 누락·순서 오류 → 엉뚱한 dir 검사 | **유효 (중요)** | §6.1 우선순위 교정 + `install_dir_candidates()` **공유 함수**로 추출(T0). preflight·repair drift 방지 |
| C2 | §7.3 `-Type ExpandString` 무조건 = prose 와 모순; 멤버십 expanded-only 비교 → 중복 append | **유효** | §6.2·§7.3 타입 보존 write + raw·expanded 양쪽 비교로 수정 |
| C4 | "backup logging" 모호 — PATH write 는 구체 롤백 artifact 필요 | **유효** | §6.2 `path-backup-<ts>.txt` + 복구 명령으로 구체화 |
| C5 | doctor 가 consent 후 즉시 repair 호출하면 "read-only" 아님 | **유효 (wording)** | §4.2 명문화 — doctor 는 직접 mutate 안 함, AUQ→repair 라우팅(기존 onboarding 패턴). "NEVER auto-fix"=직접 mutate 금지 |
| C6b | install-cli post-install verify 가 raw `axhub --version` → same-session false-fail | **유효** | T5.5 추가 — known-path 검증으로 변경 |
| C7 | 4 extras 는 리포트된 버그엔 over-engineering | **scope tension** | 아래 ▼ |

### 누락 cohort 커버리지 (codex C6)

| cohort | 커버 경로 |
|---|---|
| `CARGO_HOME` set 인데 installer 가 `.axhub\bin` 로 default | T0 공유 contract 가 정확 dir 해석 ✓ |
| `CARGO_DIST_INSTALL_DIR` 사용자 | T0 우선순위에 포함 ✓ |
| 구버전 plugin 사용자 | Phase 1·2 plugin update 후 복구 — `update` 스킬 선행 안내 |
| stable tag 전 신규 사용자 | repair 스킬로 즉시 복구(installer 무관) ✓ |
| headless/non-interactive | D1 guard → safe_default(나중에/수동), 진단만 |
| 기업 GPO (HKCU write 차단) | repair fail-open → 수동 명령 안내 (§9) |

### C7 scope tension — 사용자 결정 사항

- **codex 입장**: 최소 수정 = (1) fallback 에 `.axhub/bin`+env dir, (2) "설치됐지만 PATH 미등록" 메시지, (3) install.ps1 PATH 영속화. keychain/cache/endpoint/helper-skew/repair-skill 은 리포트된 버그엔 불필요.
- **사용자 입장**: ① helper 체크 ② env 직접 ③ + "그 외 완벽도 올리는 것" 명시 요청 → AUQ 에서 4 extras 전부 선택 = **의도된 expansion**.
- **정합**: Phase 1 = **codex 의 최소 수정과 동일** → 단독 ship 가능(de-risk). extras 는 Phase 2 로 분리돼 scope 축소 시 잘라내기 쉬움. 양쪽 다 만족.
- 사용자가 "extras 빼고 minimal 만" 으로 바꾸면 Phase 1 만 실행하면 돼요.

---

## 18. Eng Review 결과 (plan-eng-review)

implementation-rigor lens. 아키텍처 결정 2개(확정) + 테스트 갭 + 병렬화 전략.

### 확정된 아키텍처 결정

| # | 결정 | confidence | 선택 |
|---|---|---|---|
| F1 | `on_disk_not_on_path` 의 `cli_present` 의미 | 9/10 | **`cli_present=true` 유지 + `cli_on_path` 신설**. `preflight.rs:579`·`deploy_prep.rs:57` blast radius 0 (§7.2) |
| F2/F3 | 4 probe JSON surface 배치 | 8/10 | **`cmd_doctor` 전용**. preflight·doctor-summary hot path 보호 (§8) |

### 코드 품질 — cross-language DRY 주의 (confidence 7/10)

install-dir 해석 로직이 **3개 언어에 존재**해요: Rust `install_dir_candidates()`(canonical), doctor SKILL bash/PowerShell helper-pick, installer `$InstallDir`(PS). 언어 경계라 코드 DRY 불가 → **drift 위험**(codex C1 이 이미 한 번 잡음). mitigation: ① Rust 가 canonical source 임을 각 mirror 에 주석, ② installer default(`.axhub\bin`)가 `install_dir_candidates()` default 와 일치하는지 검증하는 테스트(T0), ③ SKILL bash/PS cache-scan 은 "helper 위치"(plugin cache)라 "CLI 위치"(install dir)와 별개 — 혼동 금지.

### 테스트 커버리지

신규 codepath **30개**, plan 단계라 coverage 0/30 — 전부 impl 과 동시 작성(§12 확장). **★ CRITICAL regression**: `cli_present` backward-compat(F1) — no-ask mandatory. 신규 갭 추가: REG_SZ 양형, HKCU key 부재, 미확장 `%VAR%` membership dedup, install.ps1 idempotent 재실행.

### 병렬화 전략 (worktree)

| Lane | 범위 | 의존 |
|---|---|---|
| **A** (axhub plugin) | T0→T1→T2→T3 (preflight chain) → T4→T5→T5.5→T6→T7→T8 | T0 가 T1·T4 선행. 같은 crate(`axhub-helpers`) = merge 경합, **lane 내 순차** |
| **B** (ax-hub-cli) | T11→T12 (installer) | **별도 repo, Lane A 와 완전 병렬** — 공유 파일 0 |
| **C** (converge) | T13 (CDN publish) | T11/T12 + **stable tag** 후 |

실행: **Lane A + Lane B 동시**(다른 repo, 충돌 0) → 각 repo merge → Lane C(stable release)로 수렴. plugin(A)은 단독으로 오진단 해소(Phase 1)라 B 안 기다려도 가치 있음.

### Failure modes (§9 확인 — critical gap 0)

모든 신규 codepath fail-open(exit 0) + 사용자 노출 메시지 보유. silent failure 0. R1(REG_EXPAND_SZ 손상)만 CRITICAL 리스크 — raw 읽기/타입 보존/백업 artifact 로 커버.

### NOT in scope / What already exists

§3(비범위)·§7.1·§17 cohort 표에 이미 명시. preflight `resolve_axhub_path`·doctor `preflight --json`·installer `$InstallDir`·`windows-sys 0.61` 재사용(신규 crate 0).

### Eng Review verdict

| 항목 | 결과 |
|---|---|
| Step 0 scope | CEO(C)+codex 에서 확정 — 재논의 안 함. Phase 1 = minimal 격리 |
| Architecture | 2 findings(F1·F2) → 확정 |
| Code quality | 1 finding(cross-lang DRY) → mitigation 명시 |
| Tests | 30 codepath 매핑, 1 CRITICAL regression mandatory, 갭 plan 반영 |
| Performance | F3(probe hot-path 오염) → cmd_doctor 격리로 해소 |
| Critical gaps | **0** |

**VERDICT: ENG CLEARED** — 아키텍처 결정 확정, 테스트 계획 완비, critical gap 0. 구현 준비 완료(Phase 1 진입점: T0→T1→T2).

---

## 19. DX Review 결과 (plan-devex-review)

product type: **Claude Code Skill + CLI**. mode: **DX POLISH**. "developer" = axhub vibe coder.

### Developer Persona

```
Who:       axhub 바이브 코더 — 비전문가, 한국어, Claude Desktop 자연어 조작
Context:   Windows CLI 설치 후 새 세션 → "안 깔렸다" 막힘
Tolerance: 낮음. "진단해줘" 한 번에 답 기대. 2-3 단계 넘으면 이탈
Expects:   말하면 알아서 진단 + 고쳐줌
```

### Magical Moment

`"진단해줘" → 정확 진단 → "PATH 고쳐줘" → 동작` closed loop. self-heal 루프가 구현. **DX1 로 현재 세션까지 즉시 동작** = 진짜 magical.

### Competitive Benchmark

| tool | time-to-diagnosis | DX choice |
|---|---|---|
| `flutter doctor` | ~5s | ✓/✗ + 인라인 fix command (gold) |
| `brew doctor` | ~3s | 문제 나열 + 수동 fix |
| **axhub doctor** | **~3s (doctor-summary 단일 call)** | ✓/✗ + **다음 자연어 phrase** (vibe coder 에겐 flutter 보다 우위) + on_disk_not_on_path 정확진단(오진단 0) |

### DX 발견 (확정)

| # | 발견 | P | 해소 |
|---|---|---|---|
| DX1 | repair 후 현재 세션 raw `axhub` 실패 → "고쳤는데 안 됨" → magical moment 절반 | P1 | **agent 절대경로 mediate**(§6.3) — env 상속 한계는 raw 셸에만, agent 가 resolved path 로 호출하면 현재 세션 즉시 동작. T5.6 |
| DX2 | doctor AUQ → repair 가 재consent = double-ask 마찰 | P2 | **single consent handoff** — doctor AUQ 가 THE consent, 라우팅 시 repair 재질문 안 함. T5.7 |
| DX3 | 신규 상태(on_disk_not_on_path/helper-skew/probe fail) 메시지 품질 | P2 | 기존 4-part empathy catalog(`deploy/references/error-empathy-catalog.md`: 문제+원인+해결+다음 phrase) **준수 필수** — 신규 row 전부 |

### DX Scorecard

```
| Dimension            | Score | 근거 |
|----------------------|-------|------|
| Getting Started/TTHW | 9/10  | doctor-summary 단일 call ~3s, 오진단 0(F1) |
| API/CLI (NL surface) | 8/10  | 자연어 phrase guessable, 해요체, single-consent(DX2) |
| Error Messages       | 8/10  | 4-part catalog 준수(DX3 조건) |
| Upgrade Path         | 8/10  | helper-skew 자동 surface(T6) |
| Dev Environment      | 8/10  | cross-platform(Windows 가 핵심), agent-mediate(DX1) |
| Documentation        | 7/10  | SKILL self-doc, HOOKS.md. 내부 도구라 충분 |
| DX Measurement       | 7/10  | telemetry + doctor 자체가 측정 |
| Community            | N/A   | 내부 진단 스킬 |
|----------------------|-------|------|
| TTHW-equiv           | ~3s 진단 / 2 phrase 수정 | Champion tier |
| Magical Moment       | designed via self-heal + agent-mediate(DX1) |
| Overall DX           | 8/10  | 내부 진단 도구로 strong |
```

### DX Principle Coverage

Zero friction(진단 1 call)✓ · Fight uncertainty(✓/✗ + 다음 phrase)✓ · Decide-for-me+escape(consent-gate + `AXHUB_DISABLE_PATH_REPAIR`)✓ · Magical moment(self-heal + DX1)✓ · Code-in-context(4-part empathy)✓

### DX verdict

**DX CLEARED** — 8/10, magical moment 의 현재-세션 갭(DX1)이 핵심 발견이자 해소됨. consent 마찰 제거(DX2). 신규 메시지는 4-part catalog 준수(DX3).

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 1 | done | mode C(cross-repo), 4 extras accepted, Phase 1 minimal 격리 |
| Codex Review | outside voice | Independent 2nd opinion | 1 | issues_found | 6 found / 4 objective fixed (C1 path precedence·C2 REG type·C4 backup·C6b verify) |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | clean | 2 arch (F1 cli_present blast radius·F2 probe placement), 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | N/A (no UI surface) |
| DX Review | `/plan-devex-review` | Developer experience gaps | 1 | clean | DX 8/10, 3 findings folded (DX1 agent-mediate·DX2 single-consent·DX3 4-part msg) |

- **CODEX:** 객관 오류 4개 즉시 수정 + `install_dir_candidates()` 공유 contract 도출(drift 방지)
- **CROSS-MODEL:** codex C7(extras=over-engineering) ↔ 사용자 explicit opt-in — Phase 1 격리로 양립
- **UNRESOLVED:** 0 (모든 AUQ 응답됨)
- **CROSS-PLATFORM (post-review, 사용자 Mac 질문):** root cause 전 OS 확인. defect-1+DX1 이미 전 OS. defect-2 persist 를 Mac/Linux rc-persist 로 **대칭 확장**(T4 unix 브랜치·T12 install.sh). additive·cfg-gated 라 저위험이나, rc-persist Rust↔sh mirror 가 cross-language DRY watch list(eng)에 합류 — Rust canonical.
- **VERDICT:** **CEO + ENG + DX CLEARED** — 구현 준비 완료. Phase 1 진입점 T0→T1→T2 (위험 0). cross-repo Phase 3 는 ax-hub-cli stable tag 필요. cross-platform persist 포함.
