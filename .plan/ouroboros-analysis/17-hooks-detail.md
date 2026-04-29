# 17. Hooks System Detail

## 등록 (`hooks/hooks.json`)

```json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "*",
      "hooks": [{"type": "command", "command": "python3 \"${CLAUDE_PLUGIN_ROOT}/scripts/session-start.py\"", "timeout": 5}]
    }],
    "UserPromptSubmit": [{
      "matcher": "*",
      "hooks": [{"type": "command", "command": "python3 \"${CLAUDE_PLUGIN_ROOT}/scripts/keyword-detector.py\"", "timeout": 5}]
    }],
    "PostToolUse": [{
      "matcher": "Write|Edit",
      "hooks": [{"type": "command", "command": "python3 \"${CLAUDE_PLUGIN_ROOT}/scripts/drift-monitor.py\"", "timeout": 3}]
    }]
  }
}
```

타임아웃 짧음 (3-5s) — hook 가 LLM 호출 안 함.

## 1. SessionStart → `scripts/session-start.py` (38 LOC)

### 동작

```python
def main() -> None:
    try:
        checker = _load_version_checker()
        result = checker.check_update() or {}
        if result.get("update_available") and result.get("message"):
            # SessionStart stdout 은 Claude context 로 소비됨
            # → success path silent 유지, stderr 로 알림
            print(result["message"], file=sys.stderr)
            return
    except Exception as e:
        print(f"ouroboros: update check failed: {e}", file=sys.stderr)
```

### `version-check.py` 동적 로드

`_load_version_checker()`:
```python
script_path = Path(__file__).parent / "version-check.py"
spec = importlib.util.spec_from_file_location("version_check", script_path)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)
return checker
```

### `check_update()` 흐름 (`scripts/version-check.py` 207 LOC)

1. `get_installed_version()`:
   - `plugin.json` (`.claude-plugin/plugin.json`) 우선
   - `importlib.metadata.version("ouroboros-ai")` 폴백

2. `get_latest_version(current)`:
   - 24h 캐시 (`~/.ouroboros/version-check-cache.json`) 검사
   - 캐시 hit + valid → 즉시 return
   - 캐시 miss → PyPI 호출

3. `_get_latest_from_pypi(*, include_prerelease)`:
   - `https://pypi.org/pypi/ouroboros-ai/json` 5초 timeout
   - SSL 컨텍스트 생성 실패 → silent skip (cert bundle 없는 환경)
   - prerelease 모드: 모든 release 스캔 → `Version` 파싱 → max
   - stable 모드: `info.version` 직접

4. Atomic 캐시 쓰기:
   ```python
   fd, tmp_path = tempfile.mkstemp(dir=_CACHE_DIR, suffix=".tmp")
   try:
       with open(fd, "w") as f:
           f.write(cache_content)
       Path(tmp_path).replace(_CACHE_FILE)   # atomic rename
   except Exception:
       Path(tmp_path).unlink(missing_ok=True)
       raise
   ```

5. `Version(latest) > Version(current)` 비교 (packaging.version):
   - 있음 → `update_available=True` + 메시지
   - 같음 → silent
   - 파싱 실패 → False (downgrade 위험 회피)

### 출력

업데이트 있음:
```
Ouroboros update available: v0.30.0 → v0.31.0. Run `ooo update` to upgrade.
```

업데이트 없음 / 실패: silent.

### Prerelease 검출

```python
def _is_prerelease(version_str: str) -> bool:
    try:
        from packaging.version import Version
        return Version(version_str).is_prerelease
    except Exception:
        import re
        return bool(re.search(r"(a|b|rc|dev)\d*", version_str))
```

설치 버전이 prerelease 면 prerelease 도 체크 (PyPI `info.version` 은 stable 만 return).

## 2. UserPromptSubmit → `scripts/keyword-detector.py` (238 LOC)

### KEYWORD_MAP (28 항목)

```python
KEYWORD_MAP = [
    # ooo prefix shortcuts (priority)
    {"patterns": ["ooo interview", "ooo socratic"], "skill": "/ouroboros:interview"},
    {"patterns": ["ooo seed", "ooo crystallize"], "skill": "/ouroboros:seed"},
    {"patterns": ["ooo run", "ooo execute"], "skill": "/ouroboros:run"},
    {"patterns": ["ooo eval", "ooo evaluate"], "skill": "/ouroboros:evaluate"},
    {"patterns": ["ooo evolve"], "skill": "/ouroboros:evolve"},
    {"patterns": ["ooo stuck", "ooo unstuck", "ooo lateral"], "skill": "/ouroboros:unstuck"},
    {"patterns": ["ooo status", "ooo drift"], "skill": "/ouroboros:status"},
    {"patterns": ["ooo ralph"], "skill": "/ouroboros:ralph"},
    {"patterns": ["ooo tutorial"], "skill": "/ouroboros:tutorial"},
    {"patterns": ["ooo welcome"], "skill": "/ouroboros:welcome"},
    {"patterns": ["ooo setup"], "skill": "/ouroboros:setup"},
    {"patterns": ["ooo help"], "skill": "/ouroboros:help"},
    {"patterns": ["ooo pm", "ooo prd"], "skill": "/ouroboros:pm"},
    {"patterns": ["ooo qa", "qa check", "quality check"], "skill": "/ouroboros:qa"},
    {"patterns": ["ooo cancel", "ooo abort"], "skill": "/ouroboros:cancel"},
    {"patterns": ["ooo update", "ooo upgrade"], "skill": "/ouroboros:update"},
    {"patterns": ["ooo brownfield"], "skill": "/ouroboros:brownfield"},
    
    # 자연어 트리거
    # PM 우선 (generic interview shadow 방지)
    {"patterns": ["write prd", "pm interview", "product requirements", "create prd"],
     "skill": "/ouroboros:pm"},
    {"patterns": ["interview me", "clarify requirements", "clarify my requirements",
                  "socratic interview", "socratic questioning"],
     "skill": "/ouroboros:interview"},
    {"patterns": ["crystallize", "generate seed", "create seed", "freeze requirements"],
     "skill": "/ouroboros:seed"},
    {"patterns": ["ouroboros run", "execute seed", "run seed", "run workflow"],
     "skill": "/ouroboros:run"},
    {"patterns": ["evaluate this", "3-stage check", "three-stage", "verify execution"],
     "skill": "/ouroboros:evaluate"},
    {"patterns": ["evolve", "evolutionary loop", "iterate until converged"],
     "skill": "/ouroboros:evolve"},
    {"patterns": ["think sideways", "i'm stuck", "im stuck", "i am stuck",
                  "break through", "lateral thinking"],
     "skill": "/ouroboros:unstuck"},
    {"patterns": ["am i drifting", "drift check", "session status", "check drift",
                  "goal deviation"],
     "skill": "/ouroboros:status"},
    {"patterns": ["ralph", "don't stop", "must complete", "until it works", "keep going"],
     "skill": "/ouroboros:ralph"},
    {"patterns": ["ouroboros setup", "setup ouroboros"], "skill": "/ouroboros:setup"},
    {"patterns": ["ouroboros help"], "skill": "/ouroboros:help"},
    {"patterns": ["update ouroboros", "upgrade ouroboros"], "skill": "/ouroboros:update"},
    {"patterns": ["cancel execution", "stop job", "kill stuck", "abort execution"],
     "skill": "/ouroboros:cancel"},
    {"patterns": ["brownfield defaults", "brownfield scan"],
     "skill": "/ouroboros:brownfield"},
]
```

### Word Boundary Matching

```python
def _word_boundary_match(pattern: str, text: str) -> bool:
    return bool(re.search(r"(?:^|\b)" + re.escape(pattern) + r"(?:\b|$)", text))
```

→ "evolve" 가 "evolved" 안에 매치 안 됨.

### Setup Bypass 화이트리스트

```python
SETUP_BYPASS_SKILLS = [
    "/ouroboros:setup",       # 자기 자신
    "/ouroboros:help",         # MCP 없이 동작
    "/ouroboros:qa",           # qa-judge 페르소나 직접 채택 fallback
]
```

### Setup 게이트

```python
def is_mcp_configured() -> bool:
    mcp_path = Path.home() / ".claude" / "mcp.json"
    return mcp_path.exists() and "ouroboros" in mcp_path.read_text()

# main 흐름
if result["detected"]:
    skill = result["suggested_skill"]
    if skill not in SETUP_BYPASS_SKILLS and not is_mcp_configured():
        print(f"""{user_input}

<skill-suggestion>
🎯 REQUIRED SKILL:
- /ouroboros:setup - Ouroboros setup required. Run "ooo setup" first to register the MCP server.
</skill-suggestion>
""")
```

→ MCP 미설치 + 일반 키워드 → setup 으로 강제 redirect.

### First-time 처리

```python
def is_first_time() -> bool:
    prefs_path = Path.home() / ".ouroboros" / "prefs.json"
    if not prefs_path.exists():
        return True
    prefs = json.loads(prefs_path.read_text())
    return not prefs.get("welcomeCompleted", False)

if not result["detected"] and is_first_time():
    print(f"""{user_input}

<skill-suggestion>
🎯 MATCHED SKILLS (use AskUserQuestion to let user choose):
- /ouroboros:welcome - First time using Ouroboros! Starting welcome experience.
IMPORTANT: Auto-triggering welcome experience now. Use AskUserQuestion to confirm or skip.
</skill-suggestion>
""")
```

### Bare "ooo" 처리

```python
if lower in ("ooo", "ooo?"):
    return {"detected": True, "keyword": "ooo", "suggested_skill": "/ouroboros:welcome"}
```

### 출력 포맷

```
<original user prompt>

<skill-suggestion>
🎯 MATCHED SKILLS:
- /ouroboros:<name> - Detected "<keyword>"
</skill-suggestion>
```

(또는 setup redirect / welcome 변형)

## 3. PostToolUse(Write|Edit) → `scripts/drift-monitor.py` (64 LOC)

### 동작

```python
def check_active_session() -> dict:
    ouroboros_dir = Path.home() / ".ouroboros" / "data"
    if not ouroboros_dir.exists():
        return {"active": False}
    
    files = [
        f for f in ouroboros_dir.iterdir()
        if f.suffix == ".json"
        and not f.name.endswith(".lock")
        and f.name.startswith("interview_")
    ]
    if not files:
        return {"active": False}
    
    newest = max(files, key=lambda f: f.stat().st_mtime)
    if newest.stat().st_mtime < time.time() - 3600:    # 1시간 윈도우
        return {"active": False}
    
    return {"active": True, "session_file": newest.name}

def main() -> None:
    session = check_active_session()
    if session["active"]:
        print(f"Ouroboros session active ({session['session_file']}). "
              f"Use /ouroboros:status to check drift.")
    else:
        print("Success")
```

### 의도

Write/Edit 도구로 코드 수정 후 활성 인터뷰 세션 있으면 → 사용자에게 drift 체크 안내.

→ Spec-first 워크플로 위반 감지: "당신 인터뷰 했는데, 코드 수정 중이네요. 시드와 어긋났는지 확인하세요."

### 한계

- LLM 호출 안 함 — 정확한 drift 측정 아님
- 단순 휴리스틱 (1시간 + interview_*.json 존재)
- 정밀 측정 = `/ouroboros:status` MCP 도구

## CLAUDE_PLUGIN_ROOT

`hooks/hooks.json` 의 `${CLAUDE_PLUGIN_ROOT}` — Claude Code 가 plugin 설치 디렉토리 자동 주입.

→ 사용자가 plugin 설치 어디 했든 hook 가 정확한 path 사용.

## 검증

`tests/unit/scripts/`:
- `test_keyword_detector.py`
- `test_session_start.py`
- `test_version_check.py`

`drift-monitor.py` 는 단위 테스트 없음 (확인 못 함).

## 환경 변수 / Kill Switch

확인된 것:
- `DISABLE_OMC` 같은 kill switch — Ouroboros hooks 자체에는 명시 없음
- 사용자가 직접 `hooks/hooks.json` 편집 또는 plugin 비활성화로만 끔

## 보안

- Hook 가 stdin/stdout/stderr 만 사용 (다른 Claude Code 도구 호출 안 함)
- 짧은 timeout 으로 무한 루프 방지
- LLM 호출 없음 → 비용 0
- 외부 네트워크: version-check 만 (PyPI), 5s timeout, 실패 silent
