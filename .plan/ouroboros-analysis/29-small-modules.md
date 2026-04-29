# 29. Small Modules — 미확인 함수 detail 정독

> US-002 deep-dive. 1차 라운드에서 시그니처만 grep 한 11 모듈 본문 정독. 각 모듈의 핵심 알고리즘 + 상수 + 주의점.

## 29.1 `evolution/wonder.py` (306 LOC) — WonderEngine

> Socratic Wonder phase. "What do we still not know?"

### 데이터 모델

```python
class WonderOutput(BaseModel, frozen=True):
    questions: tuple[str, ...]
    ontology_tensions: tuple[str, ...]
    should_continue: bool = True
    reasoning: str = ""

@dataclass
class WonderEngine:
    llm_adapter: LLMAdapter
    model: str = field(default_factory=get_wonder_model)
```

### LLM 호출 파라미터

| 파라미터 | 값 |
|---|---|
| temperature | 0.7 (탐색적) |
| max_tokens | 2048 |
| 모델 | `get_wonder_model()` 환경/config |

### System prompt 핵심 (인용)

```
You practice Socratic questioning:
not just asking "what went wrong" but "what assumptions are we making?"

JSON 응답 (no markdown):
{
    "questions": ["question 1", ...],
    "ontology_tensions": ["tension 1", ...],
    "should_continue": true/false,
    "reasoning": "..."
}

SCOPE GUARD — this is critical:
- Only ask questions REQUIRED to satisfy the seed's goal and constraints.
- Do NOT propose ontology fields unrelated to the seed's goal.
- Concepts IMPLIED by the seed ARE allowed.
- An ontology is ALWAYS incomplete — that is normal, not a gap to fill.
- Prefer deepening existing fields over adding new ones.
- If the current ontology covers the seed's AC AND evaluation shows no regressions, set should_continue=false.

Focus on ONTOLOGICAL questions (what IS the thing?) not implementation questions.
```

### Prompt 본체 구성 순서

1. **Seed Scope** (boundary): goal + constraints + AC list
2. **Current Ontology**: name + description + fields (name/type/description)
3. **Evaluation Results**: approved / score / drift / failure_reason / **feedback_metadata** (severity + code + max_depth + affected_count) / **failed AC list** + pass rate
4. **REGRESSIONS** (lineage ≥ 2 generations 일 때): RegressionDetector.detect() → "WHY did these previously-passing ACs start failing?"
5. **Execution Output** (truncated head/tail)
6. **Evolution History** (last 3 generations): Gen N: schema name + field count + Wonder questions[:2]

### 두 단계 fallback

#### 1) LLM 실패 시 — `_degraded_output()`

```python
if not eval_summary.final_approved:
    questions.append(f"What requirement is the current ontology missing{scope_hint}?")
if eval_summary.drift_score > 0.3:
    questions.append("Why has the implementation drifted from the original intent?")
    tensions.append("The ontology describes one thing but execution produces another")
if eval_summary.failure_reason:
    questions.append(f"What ontological gap caused: {eval_summary.failure_reason}?")
if len(ontology.fields) < 3:
    questions.append(f"Are there concepts implied by the seed goal that are not yet modeled?")

should_continue = bool(questions) or (eval_summary and not eval_summary.final_approved)
```

#### 2) JSON parse 실패 시 — fallback

```python
return WonderOutput(
    questions=(f"What assumptions remain untested{scope_hint}?",),
    should_continue=True,
    reasoning=f"Parse error, using seed-scoped fallback: {e}",
)
```

→ `should_continue=True` 보존 — 진화 loop 가 멈추지 않음.

## 29.2 `evolution/reflect.py` (339 LOC) — ReflectEngine

> 진화 핵심. "execution + wonder → 다음 generation 의 Seed". Gen 1 만 인터뷰, Gen 2+ 는 Reflect 자동 처리.

### 데이터 모델

```python
class OntologyMutation(BaseModel, frozen=True):
    action: MutationAction         # add | modify | remove
    field_name: str
    field_type: str | None = None
    description: str | None = None
    reason: str = ""

class ReflectOutput(BaseModel, frozen=True):
    refined_goal: str
    refined_constraints: tuple[str, ...]
    refined_acs: tuple[str, ...]
    ontology_mutations: tuple[OntologyMutation, ...]
    reasoning: str = ""
```

### LLM 파라미터

| 파라미터 | 값 |
|---|---|
| temperature | 0.5 (Wonder 보다 보수적) |
| max_tokens | 3000 |

### System prompt 핵심

```
- If Wonder questions exist, you MUST propose at least one ontology_mutation that addresses them
- If evaluation score >= 0.8 and approved, keep changes focused but still evolve based on Wonder insights
- If evaluation score < 0.8 or not approved, propose more aggressive mutations
- Each mutation must have a clear reason tied to evaluation findings or wonder questions
- Do NOT change things that are working well -- only evolve what needs evolution
- action must be exactly one of: "add", "modify", "remove"
- An empty ontology_mutations list is ONLY acceptable when there are no Wonder questions
```

### Stagnation 경고 (prompt 본문 자동 첨부)

```python
stagnant_count = 0
for i in range(len(gens) - 1, 0, -1):
    if OntologyDelta.compute(gens[i - 1].ontology_snapshot,
                              gens[i].ontology_snapshot).similarity >= 0.99:
        stagnant_count += 1
    else:
        break

if stagnant_count >= 1:
    parts.append(f"""
## WARNING: STAGNATION DETECTED
The ontology has NOT changed for {stagnant_count} consecutive generation(s).
Previous Reflect phases produced ZERO effective mutations.
You MUST propose concrete ontology mutations based on the Wonder questions above.
Translate each Wonder question into at least one add/modify/remove mutation.
""")
```

→ 1 generation 이라도 ontology similarity ≥ 0.99 이면 즉시 prompt 에 STAGNATION 경고 주입 — LLM 이 게으르게 mutations=[] 반환 못 하도록 강제.

### Per-AC 분해 (PRIORITY 강조)

```
Per-AC Breakdown:
  AC 1 [PASS]: ...
  AC 2 [FAIL]: ...
  AC 3 [PASS]: ...

PRIORITY: Fix N failing AC(s) while preserving passing ones.
```

### REGRESSIONS section (CRITICAL 강조)

```
## REGRESSIONS (count)
  - AC 3 (Gen 4 → Gen 7): ...
CRITICAL: These ACs previously passed. Preserve their behavior while fixing other issues.
```

### Parse 실패 처리

```python
def _parse_response(self, content, current_seed) -> ReflectOutput | None:
    try:
        # ```fence 제거 후 json.loads
        # action 무효 시 default MODIFY
        return ReflectOutput(...)
    except (JSONDecodeError, KeyError, TypeError) as e:
        logger.warning("reflect.parse_failed", raw_content=content[:1000])
        return None  # → 호출자가 retry 또는 propagate
```

→ Wonder 와 다르게 Reflect 는 fallback 안 함. parse 실패 시 ProviderError 반환.

## 29.3 `evolution/regression.py` (113 LOC) — RegressionDetector

> AC 회귀 감지. 새 storage 없이 lineage history 만 스캔.

### 알고리즘

```python
def detect(self, lineage: OntologyLineage) -> RegressionReport:
    if len(lineage.generations) < 2:
        return RegressionReport()

    latest = lineage.generations[-1]

    # Build per-AC history: ac_index → [(gen_number, passed), ...]
    ac_history: dict[int, list[tuple[int, bool]]] = {}
    for gen in lineage.generations:
        for ac in gen.evaluation_summary.ac_results:
            ac_history.setdefault(ac.ac_index, []).append((gen.generation_number, ac.passed))

    regressions = []
    for ac in latest.evaluation_summary.ac_results:
        if ac.passed:
            continue  # 통과 → 회귀 아님

        history = ac_history.get(ac.ac_index, [])

        # last passed generation 찾기
        last_passed_gen = None
        for gen_num, passed in reversed(history):
            if passed:
                last_passed_gen = gen_num
                break

        if last_passed_gen is None:
            continue  # 한 번도 통과 못함 → 회귀 아니라 persistent failure

        # consecutive failures 카운트
        consecutive = 0
        for _, passed in reversed(history):
            if not passed:
                consecutive += 1
            else:
                break

        regressions.append(ACRegression(
            ac_index=ac.ac_index,
            ac_text=ac.ac_content,
            passed_in_generation=last_passed_gen,
            failed_in_generation=latest.generation_number,
            consecutive_failures=consecutive,
        ))
```

→ "한 번도 통과 못 했음" vs "통과했다가 failing 시작함" 구별. 후자만 regression.

## 29.4 `verification/verifier.py` (294 LOC) — SpecVerifier

> 4 tier 검증 시스템 중 T1 (constant/config) + T2 (structural). T3/T4 skip.

### 상수 (보안 가드)

```python
MAX_FILE_SIZE = 50 * 1024            # 50 KB per file
MAX_FILES_PER_HINT = 100             # glob 결과 cap
MAX_PATTERN_LENGTH = 200             # ReDoS 방지 (LLM-generated regex)
```

### 4 Tier

```python
class VerificationTier(StrEnum):  # verification/models.py 추정
    T1_CONSTANT = ...    # 상수/설정값 (regex search + value extract)
    T2_STRUCTURAL = ...  # 파일/클래스/함수 존재 (filename + content)
    T3 = ...             # skip (verifier 외 다른 곳)
    T4 = ...             # skip
```

### T1 검증 흐름 (`_verify_constant`)

```python
files = self._find_files(assertion.file_hint)  # glob + path traversal guard
if not files:
    return verified=True, "No files matched hint"  # trust agent

pattern = self._safe_compile(assertion.pattern)  # 200 chars cap

for file_path in files:
    content = self._read_file(file_path)         # 50 KB cap
    match = pattern.search(content)
    if match:
        actual = self._extract_value_after_match(content, match)
        # = / : / ( 따라가 quoted/unquoted 값 추출
        if assertion.expected_value:
            verified = expected_value in actual
        else:
            verified = True
        return SpecVerificationResult(...)

return verified=False, "Pattern not found"
```

### T2 검증 흐름 (`_verify_structural`)

```python
files = self._find_files(assertion.file_hint)

# 1차: 파일명 자체 매칭
name_pattern = self._safe_compile(assertion.pattern, re.IGNORECASE)
for file_path in files:
    if name_pattern.search(os.path.basename(file_path)):
        return verified=True, file_path

# 2차: 파일 내용에서 class/function/interface 검색
content_pattern = self._safe_compile(assertion.pattern)
for file_path in files:
    content = self._read_file(file_path)
    if content_pattern.search(content):
        return verified=True, file_path

return verified=False, "Structure not found"
```

### Path traversal 방지 (`_find_files`)

```python
real_project = os.path.realpath(self.project_dir)

filtered = [
    f for f in glob.glob(pattern, recursive=True)
    if os.path.realpath(f).startswith(real_project + os.sep)
    and not any(skip in f for skip in ("__pycache__", ".git", "node_modules", ".venv", ".tox"))
]

return filtered[:MAX_FILES_PER_HINT]
```

→ `realpath()` 후 prefix 검증 — `../../etc/*` 같은 traversal 시도 거부.

### 값 추출 (`_extract_value_after_match`)

```python
end = match.end()
rest = content[end : end + 100]    # 100 chars 만 분석

# = / : / ( 따라가는 패턴
value_match = re.match(r'\s*[=:]\s*["\']?([^"\'\s,;)\]}{]+)["\']?', rest)
if value_match:
    return value_match.group(1)

# (...) 형태
paren_match = re.match(r'\s*\(\s*["\']?([^"\'\s,;)]+)["\']?\s*\)', rest)
if paren_match:
    return paren_match.group(1)

return rest.strip()[:50]
```

## 29.5 `secondary/scheduler.py` (469 LOC) — SecondaryLoopScheduler

> Primary goal 달성 후 TODO 일괄 처리.

### Status enum

```python
class BatchStatus(StrEnum):
    COMPLETED = "completed"     # 모든 TODO 처리 (일부 fail 가능)
    PARTIAL = "partial"          # timeout 등으로 조기 종료
    SKIPPED = "skipped"          # 사용자 skip
    NO_TODOS = "no_todos"        # 처리할 TODO 없음
```

### 데이터 모델

```python
@dataclass(frozen=True, slots=True)
class TodoResult:
    todo_id: str
    description: str
    success: bool
    error_message: str | None = None
    duration_ms: int = 0

@dataclass(frozen=True, slots=True)
class BatchSummary:
    status: BatchStatus
    total: int
    success_count: int
    failure_count: int
    skipped_count: int
    results: tuple[TodoResult, ...]
    started_at: datetime
    completed_at: datetime

    @property
    def duration_ms(self) -> int: ...
    @property
    def success_rate(self) -> float: ...    # 0.0–1.0
    @property
    def failed_todos(self) -> tuple[TodoResult, ...]: ...
```

### `should_activate` 규칙

```python
def should_activate(self, primary_completed: bool, skip_flag: bool = False) -> bool:
    if skip_flag:
        log.info("secondary_loop.skipped.user_flag")
        return False
    if not primary_completed:
        log.info("secondary_loop.skipped.primary_incomplete")
        return False
    return True
```

→ Primary 미완 시 → 자동 skip. `--skip-secondary` 플래그 → 명시적 skip.

### 핵심 처리 흐름 (`process_batch`)

```python
async def process_batch(self, limit: int | None = None) -> Result[BatchSummary, ...]:
    pending_result = await self._registry.get_pending(limit=batch_limit)
    todos = pending_result.value  # priority 순 정렬

    for todo in todos:
        result = await self._process_single_todo(todo)
        # 한 TODO 실패해도 다음 TODO 계속

    return BatchSummary(...)
```

### 단일 TODO 처리 (`_process_single_todo`)

```python
await self._registry.update_status(todo.id, IN_PROGRESS)

try:
    exec_result = await self._executor(todo)

    if exec_result.is_ok:
        await self._registry.update_status(todo.id, DONE)
        return TodoResult(success=True, ...)
    else:
        error_msg = str(exec_result.error)
        await self._registry.update_status(todo.id, FAILED, error_msg)
        return TodoResult(success=False, error_message=error_msg, ...)

except Exception as e:
    error_msg = f"Unexpected error: {e}"
    await self._registry.update_status(todo.id, FAILED, error_msg)
    return TodoResult(success=False, error_message=error_msg, ...)
```

→ 3-way 처리: Result.ok / Result.err / unexpected exception. 모두 isolation.

### 상수

| 상수 | 값 | 위치 |
|---|---|---|
| `_max_todos_per_batch` | 50 | `SecondaryLoopScheduler` 필드 |
| Default executor | `_default_executor` (no-op for 테스트) | 모듈 상단 |

### `skip_all_pending(reason)` — 전체 skip

→ 사용자가 다음 세션에 미루고 싶을 때. 모든 pending → SKIPPED status (failure 아님).

## 29.6 `core/git_workflow.py` (143 LOC) — GitWorkflowConfig

> CLAUDE.md 파싱으로 PR/branch workflow 자동 감지.

### 데이터 모델

```python
@dataclass(frozen=True, slots=True)
class GitWorkflowConfig:
    use_branches: bool = False
    branch_pattern: str = "ooo/{task}"        # {lineage_id}, {task} 지원
    auto_pr: bool = False
    protected_branches: tuple[str, ...] = ("main", "master")
    source: str = ""                           # 어느 파일에서 detect 했는지
```

### PR-workflow 감지 패턴 (8 regex)

```python
_PR_WORKFLOW_PATTERNS = (
    re.compile(r"pr[- ]based\s+workflow", re.IGNORECASE),
    re.compile(r"always\s+create\s+(?:a\s+)?(?:pull\s+request|pr)", re.IGNORECASE),
    re.compile(r"never\s+(?:commit|push)\s+(?:directly\s+)?to\s+main", re.IGNORECASE),
    re.compile(r"never\s+(?:commit|push)\s+(?:directly\s+)?to\s+master", re.IGNORECASE),
    re.compile(r"create\s+(?:a\s+)?(?:feature\s+)?branch", re.IGNORECASE),
    re.compile(r"open\s+(?:a\s+)?(?:pull\s+request|pr)", re.IGNORECASE),
    re.compile(r"feature\s+branch\s+workflow", re.IGNORECASE),
    re.compile(r"branch\s+and\s+(?:open\s+)?(?:a\s+)?pr", re.IGNORECASE),
)
```

### Protected branch 감지

```python
_PROTECTED_BRANCH_PATTERN = re.compile(
    r"(?:never|don'?t|do\s+not)\s+(?:commit|push)\s+(?:directly\s+)?to\s+(\w+)",
    re.IGNORECASE,
)

protected = set()
for match in _PROTECTED_BRANCH_PATTERN.finditer(claude_md_content):
    protected.add(match.group(1).lower())

if use_branches and not protected:
    protected = {"main", "master"}  # PR workflow 검출시 default
```

### Auto-PR 감지 (보수적)

```python
auto_pr = bool(re.search(
    r"auto(?:matically)?\s+(?:create|open)\s+(?:a\s+)?(?:pull\s+request|pr)",
    claude_md_content,
    re.IGNORECASE,
))
```

→ "auto" 키워드 필수 — "create a PR" 정도로는 auto_pr 안 켜짐 (false positive 방지).

### 검색 위치 우선순위

```python
for candidate in [
    project_root / "CLAUDE.md",
    project_root / ".claude" / "CLAUDE.md",
]:
    if candidate.exists():
        claude_md_content = candidate.read_text(encoding="utf-8")
        source = str(candidate)
        break
```

### `is_on_protected_branch()`

```python
result = subprocess.run(
    ["git", "rev-parse", "--abbrev-ref", "HEAD"],
    capture_output=True, text=True, cwd=project_root, timeout=5,
)
current_branch = result.stdout.strip()
return current_branch in config.protected_branches
```

→ `timeout=5` 한정. git 호출 안전.

## 29.7 `core/file_lock.py` (56 LOC) — Cross-platform file lock

> Windows (msvcrt) + POSIX (fcntl) 동시 지원.

### 구현

```python
@contextmanager
def file_lock(file_path: Path, exclusive: bool = True) -> Iterator[None]:
    lock_path = file_path.with_suffix(file_path.suffix + ".lock")
    lock_path.parent.mkdir(parents=True, exist_ok=True)

    with lock_path.open("a+", encoding="utf-8") as handle:
        _ensure_lockfile_content(handle)  # 빈 파일이면 "0" 쓰기 (msvcrt 가 byte 필요)
        _acquire_lock(handle, exclusive=exclusive)
        try:
            yield
        finally:
            _release_lock(handle)
```

### Platform 분기

```python
if os.name == "nt":
    import msvcrt
    # exclusive: msvcrt.LK_LOCK
    # shared:    msvcrt.LK_RLCK
    # release:   msvcrt.LK_UNLCK
else:
    import fcntl
    # exclusive: fcntl.LOCK_EX
    # shared:    fcntl.LOCK_SH
    # release:   fcntl.LOCK_UN
```

→ heartbeat 메커니즘은 본 모듈 외부 (parallel_executor 의 `HEARTBEAT_INTERVAL_SECONDS = 30` 별도). 본 모듈은 단순 lock primitive.

## 29.8 `core/security.py` (356 LOC) — InputValidator + 시크릿 마스킹

### 상수

```python
MAX_INITIAL_CONTEXT_LENGTH = 50_000   # 50 KB
MAX_USER_RESPONSE_LENGTH   = 10_000   # 10 KB
MAX_SEED_FILE_SIZE         = 1_000_000 # 1 MB
MAX_LLM_RESPONSE_LENGTH    = 100_000  # 100 KB
```

### API key pattern

```python
_API_KEY_PATTERNS = {
    "openai":     re.compile(r"^sk-[a-zA-Z0-9_-]{20,}$"),
    "anthropic":  re.compile(r"^sk-ant-[a-zA-Z0-9_-]{20,}$"),
    "openrouter": re.compile(r"^sk-or-[a-zA-Z0-9_-]{20,}$"),
    "google":     re.compile(r"^AIza[a-zA-Z0-9_-]{35}$"),
}
```

### 민감 필드명 화이트리스트

```python
SENSITIVE_FIELD_NAMES = frozenset({
    "password", "api_key", "apikey", "api-key", "secret", "token",
    "credential", "auth", "key", "private", "bearer", "authorization",
})

SENSITIVE_PREFIXES = ("sk-", "pk-", "api-", "bearer ", "token ", "secret_", "AIza")
```

### 핵심 함수

| 함수 | 기능 |
|---|---|
| `mask_api_key(key, visible_chars=4)` | `sk-...cdef` 형태로 마스킹 |
| `validate_api_key_format(key, provider=None)` | 형식만 검증 (실 호출 X) |
| `is_sensitive_field(name)` | 필드명에 sensitive token 포함 여부 |
| `is_sensitive_value(value)` | `sk-` / `AIza` 등 prefix 매칭 |
| `mask_sensitive_value(value, field_name)` | 컨텍스트별 마스킹 |
| `sanitize_for_logging(data: dict)` | 로깅 전 dict 재귀 마스킹 |
| `truncate_input(text, max_length, suffix="...")` | 길이 제한 |

### `InputValidator` 정적 메서드

```python
class InputValidator:
    @staticmethod
    def validate_initial_context(context: str) -> tuple[bool, str]:
        # empty / whitespace-only / > 50 KB 거부
        ...

    @staticmethod
    def validate_user_response(response: str) -> tuple[bool, str]:
        # > 10 KB 거부
        ...

    @staticmethod
    def validate_seed_file_size(file_size: int) -> tuple[bool, str]:
        # > 1 MB 거부
        ...

    @staticmethod
    def validate_path_containment(path, allowed_root) -> tuple[bool, str]:
        # resolve() 후 is_relative_to() 검증 → traversal 방지
        ...

    @staticmethod
    def validate_llm_response(response: str) -> tuple[bool, str]:
        # > 100 KB 거부 (empty 는 OK)
        ...
```

### Path traversal 방지 (canonical)

```python
@staticmethod
def validate_path_containment(path, allowed_root) -> tuple[bool, str]:
    try:
        resolved = Path(path).resolve()
        root = Path(allowed_root).resolve()
    except (OSError, ValueError) as exc:
        return False, f"Path resolution failed: {exc}"

    if not resolved.is_relative_to(root):
        return False, f"Path escapes allowed root: {resolved} is not under {root}"
    return True, ""
```

→ `resolve()` 가 symlink + ../ 모두 풀어주고 그 후 `is_relative_to()` 검증.

## 29.9 `mcp/server/security.py` (688 LOC) — SecurityLayer (Auth + Authz + RateLimit + Validation)

> 단일 인터페이스로 4 보안 메커니즘 통합.

### Auth method

```python
class AuthMethod(StrEnum):
    NONE = "none"
    API_KEY = "api_key"
    BEARER_TOKEN = "bearer_token"
```

### Permission 4 levels

```python
class Permission(StrEnum):
    READ = "read"
    WRITE = "write"
    EXECUTE = "execute"
    ADMIN = "admin"
```

### Auth 흐름

#### NONE → 항상 인증 통과

```python
if self._config.method == AuthMethod.NONE:
    return Result.ok(AuthContext(authenticated=True, permissions=frozenset(Permission)))
```

→ NONE 모드는 `required` 무시 + 모든 Permission 부여.

#### API_KEY → SHA-256 hash 비교

```python
@staticmethod
def _hash_key(key: str) -> str:
    return hashlib.sha256(key.encode()).hexdigest()

self._hashed_keys = frozenset(self._hash_key(key) for key in config.api_keys)
# ...
if hashed in self._hashed_keys:
    return Result.ok(AuthContext(
        authenticated=True,
        client_id=hashed[:16],          # hash prefix 가 client ID
        permissions=frozenset(Permission),
    ))
```

→ Plaintext API key 비교 안 함. SHA-256 해시 후 compare.

#### BEARER_TOKEN → HMAC-SHA256 + timestamp

```python
# 토큰 형식: "client_id:timestamp:signature"
parts = token.split(":")
if len(parts) != 3:
    return Result.err(...)

client_id, timestamp_str, signature = parts

expected = hmac.new(
    self._config.token_secret.encode(),
    f"{client_id}:{timestamp_str}".encode(),
    hashlib.sha256,
).hexdigest()

if not hmac.compare_digest(signature, expected):
    return Result.err("Invalid token signature")

# 시간 검증
now = time.time()
timestamp = int(timestamp_str)
if timestamp > now + 60:                # 60s 미래 허용 (clock skew)
    return Result.err("Token timestamp is in the future")
if now - timestamp > 3600:              # 1 hour expiry
    return Result.err("Token expired")
```

### `Authorizer` (per-tool permission + role)

```python
def authorize(self, tool_name: str, auth_context: AuthContext) -> Result[None, ...]:
    permission = self._tool_permissions.get(tool_name)

    # 미등록 도구 → authenticated 만 통과
    if permission is None:
        return Result.ok() if auth_context.authenticated else Result.err(...)

    # required_permissions ⊆ user permissions
    if not permission.required_permissions.issubset(auth_context.permissions):
        missing = permission.required_permissions - auth_context.permissions
        return Result.err(f"Missing permissions: {missing}")

    # role 체크 (role 정의된 경우만)
    if permission.allowed_roles and not permission.allowed_roles.intersection(auth_context.roles):
        return Result.err(f"Role not authorized")

    return Result.ok()
```

### `RateLimiter` (token bucket)

```python
def __init__(self, requests_per_minute, burst_size):
    self._rate = requests_per_minute / 60.0
    self._burst_size = burst_size
    self._buckets: dict[str, tuple[float, float]] = {}  # client_id → (tokens, last_update)

async def check(self, client_id: str) -> bool:
    async with self._lock:
        now = time.monotonic()
        tokens, last_update = self._buckets.get(client_id, (self._burst_size, now))

        elapsed = now - last_update
        tokens = min(self._burst_size, tokens + elapsed * self._rate)

        if tokens >= 1:
            self._buckets[client_id] = (tokens - 1, now)
            return True
        else:
            self._buckets[client_id] = (tokens, now)
            return False
```

→ Per-client token bucket. Burst 가능 + 평균 RPM 제한.

### `InputValidator` (FREETEXT 화이트리스트)

#### 위험 패턴 (전 필드 검사)

```python
dangerous_patterns = [
    "__import__", "subprocess", "os.popen", "os.system",
    "eval(", "exec(", "compile(", "open(",
]
path_traversal_patterns = ["../", "..\\"]
```

#### 셸 메타문자 (FREETEXT_FIELDS 만 면제)

```python
shell_metacharacters = [";", "|", "&&", "||"]

FREETEXT_FIELDS = {
    "artifact", "quality_bar", "reference", "seed_content",
    "current_output", "prompt", "initial_context", "answer",
    "current_approach", "problem_context", "acceptance_criterion",
    "message", "content", "desc", "entry", "reason",
}
```

→ 자연어 prose 필드는 `;` 등 metachar 정상이므로 면제. 다른 필드는 strict 검증.

#### 재귀 검증 (`_collect_strings`)

```python
def _collect_strings(obj, prefix="") -> list[tuple[str, str]]:
    if isinstance(obj, str):
        return [(prefix, obj)]
    elif isinstance(obj, dict):
        return [(child_key, ...)
                for k, v in obj.items()
                for child_key in [f"{prefix}.{k}" if prefix else k]
                for ... in _collect_strings(v, child_key)]
    elif isinstance(obj, (list, tuple)):
        return [(... )
                for i, v in enumerate(obj)
                for ... in _collect_strings(v, f"{prefix}[{i}]")]
    return []
```

→ 중첩 dict/list 끝까지 재귀 — `[0.13.3]` fix 대응 (1차 라운드 Section 12 참조).

### `SecurityLayer.check_request` 4 단계 chain

```python
async def check_request(self, tool_name, arguments, credentials) -> Result[AuthContext, ...]:
    # 1. Authenticate
    auth_result = self._authenticator.authenticate(credentials)
    if auth_result.is_err: return Result.err(auth_result.error)
    auth_context = auth_result.value

    # 2. Rate limit (enabled 시만)
    if self._rate_limiter and auth_context.client_id:
        if not await self._rate_limiter.check(auth_context.client_id):
            return Result.err(MCPServerError("Rate limit exceeded", is_retriable=True,
                                              details={"retry_after": 60}))

    # 3. Authorize
    authz_result = self._authorizer.authorize(tool_name, auth_context)
    if authz_result.is_err: return Result.err(authz_result.error)

    # 4. Validate input
    valid_result = self._validator.validate(tool_name, arguments)
    if valid_result.is_err: return Result.err(valid_result.error)

    return Result.ok(auth_context)
```

### Middleware factory

```python
def create_security_middleware(security_layer):
    async def middleware(tool_name, arguments, credentials, handler):
        check_result = await security_layer.check_request(tool_name, arguments, credentials)
        if check_result.is_err:
            return Result.err(check_result.error)
        return await handler(arguments)
    return middleware
```

## 29.10 `routing/escalation.py` (341 LOC) — EscalationManager

> 2 연속 실패 시 tier 상향 (Frugal → Standard → Frontier → Stagnation event).

### 상수

```python
FAILURE_THRESHOLD = 2     # 2 연속 실패 → escalation
```

### 데이터 모델

```python
@dataclass
class FailureTracker:
    consecutive_failures: int = 0
    current_tier: Tier = Tier.FRUGAL
    last_failure_time: datetime | None = None

@dataclass(frozen=True, slots=True)
class EscalationAction:
    should_escalate: bool
    is_stagnation: bool         # Frontier 도 실패 → True
    target_tier: Tier | None
    previous_tier: Tier
    failure_count: int
```

### Escalation path

```python
escalation_path = {
    Tier.FRUGAL:    Tier.STANDARD,
    Tier.STANDARD:  Tier.FRONTIER,
    Tier.FRONTIER:  None,        # 더 위 없음 → stagnation
}
```

### `record_failure` 흐름

```python
def record_failure(self, pattern_id, current_tier) -> Result[EscalationAction, None]:
    tracker = self._get_or_create_tracker(pattern_id, current_tier)
    tracker.record_failure()
    failure_count = tracker.consecutive_failures

    if failure_count >= FAILURE_THRESHOLD:        # 2 연속
        next_tier = self._get_next_tier(current_tier)

        if next_tier is not None:
            tracker.consecutive_failures = 0      # ← reset on escalation
            return Result.ok(EscalationAction(
                should_escalate=True,
                target_tier=next_tier,
                ...
            ))
        else:
            # Frontier 도 실패 → stagnation
            return Result.ok(EscalationAction(
                is_stagnation=True,
                target_tier=None,
                ...
            ))

    return Result.ok(EscalationAction(should_escalate=False, ...))
```

### StagnationEvent

```python
class StagnationEvent(BaseEvent):
    def __init__(self, pattern_id, failure_count, **kwargs):
        super().__init__(
            type="escalation.stagnation.detected",
            aggregate_type="routing",
            aggregate_id=pattern_id,
            data={
                "pattern_id": pattern_id,
                "failure_count": failure_count,
                "tier": Tier.FRONTIER.value,
                **kwargs,
            },
        )
```

→ `escalation.stagnation.detected` 이벤트 → resilience 시스템이 lateral thinking 으로 전환.

### `record_success` — counter reset

```python
def record_success(self, pattern_id):
    if pattern_id in self._trackers:
        tracker = self._trackers[pattern_id]
        tracker.reset_on_success()  # consecutive=0, last_failure_time=None
```

### `clear_tracker` — pattern 제거

→ 사용자 explicit reset 또는 패턴 변경 시.

## 29.11 `routing/downgrade.py` (662 LOC) — DowngradeManager

> 5 연속 성공 시 tier 하향 (Frontier → Standard → Frugal). Jaccard ≥ 0.80 으로 패턴 유사 시 inherit.

### 상수

```python
DOWNGRADE_THRESHOLD  = 5      # 5 연속 성공 → downgrade
SIMILARITY_THRESHOLD = 0.80   # Jaccard ≥ 0.80 → 같은 패턴
```

### 데이터 모델

```python
@dataclass
class SuccessTracker:
    _success_counts: dict[str, int]
    _tier_history: dict[str, Tier]

    def record_success(pattern_id, tier) -> int: ...
    def reset_on_failure(pattern_id) -> None: ...
    def get_success_count(pattern_id) -> int: ...
    def get_tier(pattern_id) -> Tier | None: ...
    def clear() -> None: ...

@dataclass(frozen=True, slots=True)
class DowngradeResult:
    should_downgrade: bool
    current_tier: Tier
    recommended_tier: Tier
    consecutive_successes: int
    cost_savings_factor: float    # Frontier 30x → Standard 10x = 3.0x 절감
```

### Downgrade path

```python
def _get_lower_tier(tier: Tier) -> Tier:
    tier_order = [Tier.FRUGAL, Tier.STANDARD, Tier.FRONTIER]
    current_index = tier_order.index(tier)
    if current_index > 0:
        return tier_order[current_index - 1]
    return tier  # Frugal → Frugal (이미 최저)
```

### Cost savings 계산

```python
def _calculate_cost_savings(from_tier, to_tier) -> float:
    if from_tier == to_tier:
        return 1.0
    return from_tier.cost_multiplier / to_tier.cost_multiplier
    # Frontier (30x) → Standard (10x) = 3.0x
    # Standard (10x) → Frugal (1x) = 10.0x
```

### `record_success` 흐름

```python
def record_success(self, pattern_id, tier) -> Result[DowngradeResult, None]:
    success_count = self._tracker.record_success(pattern_id, tier)

    should_downgrade = (success_count >= 5) and (tier != Tier.FRUGAL)

    if should_downgrade:
        recommended_tier = _get_lower_tier(tier)
        cost_savings = _calculate_cost_savings(tier, recommended_tier)
    else:
        recommended_tier = tier
        cost_savings = 1.0

    return Result.ok(DowngradeResult(
        should_downgrade=should_downgrade,
        current_tier=tier,
        recommended_tier=recommended_tier,
        consecutive_successes=success_count,
        cost_savings_factor=cost_savings,
    ))
```

### `PatternMatcher` (Jaccard)

#### Tokenize (whitespace + punctuation strip)

```python
def _tokenize(self, text: str) -> set[str]:
    words = text.lower().split()
    cleaned_words = set()
    for word in words:
        cleaned = word.strip(".,;:!?\"'()-[]{}/<>")
        if cleaned:
            cleaned_words.add(cleaned)
    return cleaned_words
```

#### Jaccard similarity

```python
def calculate_similarity(self, pattern_a: str, pattern_b: str) -> float:
    tokens_a = self._tokenize(pattern_a)
    tokens_b = self._tokenize(pattern_b)

    if not tokens_a and not tokens_b:
        return 1.0  # Both empty
    if not tokens_a or not tokens_b:
        return 0.0  # One empty

    return len(tokens_a & tokens_b) / len(tokens_a | tokens_b)
```

→ "fix typo in README" + "fix typo in docs" → tokens 공통 {fix, typo, in} = 3, union {fix, typo, in, README, docs} = 5 → 0.6 → 80% 미달.

#### `find_similar_patterns`

```python
def find_similar_patterns(self, target_pattern, candidate_patterns):
    similar = []
    for candidate in candidate_patterns:
        sim = self.calculate_similarity(target_pattern, candidate)
        if sim >= self._similarity_threshold:
            similar.append((candidate, sim))
    similar.sort(key=lambda x: x[1], reverse=True)
    return similar
```

### Pattern 학습 inherit

```python
def get_recommended_tier_for_pattern(self, pattern_description, default_tier=Tier.FRUGAL) -> Tier:
    tracked_patterns = self._tracker.get_all_patterns()
    similar_patterns = self._pattern_matcher.find_similar_patterns(
        pattern_description, tracked_patterns,
    )

    if not similar_patterns:
        return default_tier  # 우선 FRUGAL — 비용 절감 옵티미즘

    best_match, best_similarity = similar_patterns[0]
    matched_tier = self._tracker.get_tier(best_match)
    return matched_tier or default_tier
```

→ **새 task 가 들어오면 default 가 Frugal**. 비슷한 과거 task 가 더 높은 tier 에서 성공한 적 있다면 그 tier 로 시작.

### `apply_downgrade` (실 적용)

```python
def apply_downgrade(self, pattern_id):
    current_tier = self._tracker.get_tier(pattern_id)
    self._tracker._success_counts[pattern_id] = 0  # reset 카운터

    if current_tier:
        new_tier = _get_lower_tier(current_tier)
        self._tracker._tier_history[pattern_id] = new_tier
```

### `get_cost_savings_estimate`

```python
def get_cost_savings_estimate(self, pattern_id) -> float:
    current_tier = self._tracker.get_tier(pattern_id)
    if current_tier is None or current_tier == Tier.FRUGAL:
        return 1.0
    new_tier = _get_lower_tier(current_tier)
    return _calculate_cost_savings(current_tier, new_tier)
```

## 29.12 모듈 간 협력

```
EscalationManager (실패 카운트)        ─┐
                                        ├─→ Tier 결정
DowngradeManager (성공 카운트)         ─┘

DowngradeManager.PatternMatcher (Jaccard ≥ 0.80) → 새 task 의 starting tier inherit

WonderEngine → ReflectEngine → SeedGenerator (다음 generation Seed)
                ↑
     RegressionDetector.detect(lineage) → REGRESSIONS prompt section

SpecVerifier (T1+T2) ─→ ChecklistVerifyHandler MCP 도구 ↘
                                                          → SecurityLayer 검증 (FREETEXT_FIELDS 면제)
core/InputValidator (DoS limits) ────────────────────────↗

SecondaryLoopScheduler ←── Primary 완료 신호 ── 메인 워크플로

GitWorkflowConfig (CLAUDE.md detect) → ralph/run skill 의 PR 작성 분기
file_lock (cross-platform) → ~/.ouroboros/data/ 파일 동시 접근 방지
```

## 29.13 미발견 영역 (이번 라운드도 못 본 것)

- `evolution/loop.py` (61 KB!) — EvolutionaryLoopConfig + 30 generation 메인 루프 + convergence/oscillation 감지
- `evolution/convergence.py` (14.6 KB) — ≥0.95 similarity 알고리즘 detail (1차 라운드 magic number 만 cover)
- `evolution/projector.py` (11.4 KB) — lineage state 투영
- `secondary/todo_registry.py` (13.6 KB) — TodoRegistry CRUD + EventStore 연계
- `verification/extractor.py` + `models.py` — assertion 추출 LLM 프롬프트 + Pydantic 모델
- `mcp/server/adapter.py` (52.9 KB!) — FastMCP 어댑터 본체
- `routing/router.py` (6.5 KB) + `complexity.py` (8.3 KB) — 라우팅 결정 본체

이들은 별도 라운드에서 cover 또는 시간 허용 시 추가 정독.
