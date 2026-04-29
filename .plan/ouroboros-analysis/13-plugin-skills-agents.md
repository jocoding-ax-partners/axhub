# 13. Plugin / Skills / Agents Subsystem

## 위치 (`src/ouroboros/plugin/`)

```
plugin/
├─ skills/
│   ├─ registry.py        # SkillRegistry 핫리로드 자동 발견
│   ├─ keywords.py         # MagicKeywordDetector
│   ├─ executor.py         # SkillExecutor
│   ├─ docs.py             # SkillDocumentation 자동 추출
│   └─ __init__.py
├─ agents/
│   ├─ registry.py         # AgentRegistry 동적 발견
│   ├─ pool.py             # AgentPool 로드 밸런싱
│   └─ __init__.py
└─ orchestration/
    ├─ router.py            # ModelRouter PAL 라우팅 + 학습
    ├─ scheduler.py         # Scheduler, TaskGraph
    └─ __init__.py
```

## SkillRegistry

자동 발견:
- `.claude-plugin/skills/` (사용자 정의 우선)
- `skills/` (번들된 기본)

핫리로드 — 재시작 없이 새 skill 추가/수정 반영.

```python
class SkillRegistry:
    def discover(self) -> list[SkillSpec]: ...
    def get(self, name: str) -> SkillSpec | None: ...
    def list(self) -> list[SkillSpec]: ...
    def reload(self) -> None: ...

class SkillSpec:
    name: str                                # frontmatter
    description: str
    mcp_tool: str | None
    mcp_args: dict[str, str] | None
    body: str                                # Markdown 본문
    triggers: list[str]                      # 키워드
    magic_prefix: str | None                 # "ooo:"
```

## MagicKeywordDetector (`plugin/skills/keywords.py`)

`ooo:` prefix 또는 trigger keyword 매칭.

(스크립트 hook `scripts/keyword-detector.py` 와는 별개 — Python 내부 라우터.)

## SkillExecutor

```python
class SkillExecutor:
    async def execute(self, skill: SkillSpec, args: dict, context: ExecutionContext) -> SkillResult: ...
```

이력 추적 + 컨텍스트 인지.

## SkillDocumentation

SKILL.md frontmatter + body → 자동 documentation 생성. `ouroboros help` 가 사용.

## AgentRegistry

동적 발견 우선순위:
1. `OUROBOROS_AGENTS_DIR` env (명시 override)
2. `.claude-plugin/agents/` (사용자 정의)
3. `src/ouroboros/agents/` (번들 기본)

```python
class AgentRegistry:
    def discover(self) -> dict[str, AgentSpec]: ...
    def get(self, name: str) -> AgentSpec | None: ...

class AgentSpec(frozen=True):
    name: str
    role: AgentRole
    capabilities: tuple[str, ...]
    tools: tuple[str, ...]
    model_preferences: tuple[str, ...]
    body: str

class AgentRole(StrEnum):
    ANALYSIS = "analysis"
    PLANNING = "planning"
    EXECUTION = "execution"
    REVIEW = "review"
    DOMAIN = "domain"
    PRODUCT = "product"
    COORDINATION = "coordination"
```

## AgentPool

```python
class AgentPool:
    async def acquire(self, role: AgentRole) -> AgentInstance: ...
    async def release(self, instance: AgentInstance) -> None: ...
    
    def auto_scale(self, target_size: int) -> None: ...
    async def health_check(self) -> dict[str, AgentHealth]: ...
```

로드 밸런싱 + 자동 스케일 + 헬스 모니터링.

## ModelRouter (`plugin/orchestration/router.py`)

Phase 1 의 PAL Router 와 다른 추가 layer — context-aware routing.

```python
class RoutingContext:
    task_type: str
    history: list[RoutingDecision]
    recent_failures: int

class ModelRouter:
    def route(self, context: RoutingContext) -> RoutingDecision: ...
    def learn_from(self, decision: RoutingDecision, success: bool) -> None: ...
```

이력 학습 — 비슷한 context 에서 성공한 모델 우선.

## Scheduler (`plugin/orchestration/scheduler.py`)

```python
class TaskGraph:
    def add_task(self, task: ScheduledTask) -> None: ...
    def add_dependency(self, src: str, dst: str) -> None: ...
    def topological_sort(self) -> list[ScheduledTask]: ...

class ScheduledTask:
    id: str
    payload: Any
    priority: int
    dependencies: tuple[str, ...]
    timeout_seconds: float

class Scheduler:
    async def run(self, graph: TaskGraph) -> dict[str, TaskResult]: ...
```

병렬 실행 + 의존성 해결.

## 19 Skills 카탈로그

| Skill | 라인 | 트리거 | MCP 도구 | 핵심 동작 |
|---|---|---|---|---|
| `setup` | 602 | "ooo setup" | — | 6-step 위저드, env detect, MCP 등록, CLAUDE.md 통합, brownfield 스캔, GitHub star prompt |
| `publish` | 355 | "publish to github" | — | Seed → GitHub Issues (gh CLI), Epic + Task 트리 |
| `interview` | 337 | "interview me" / "ooo interview" | `ouroboros_interview` | 4-PATH 라우팅 + Dialectic Rhythm + Seed-ready Acceptance Guard |
| `run` | 291 | "ooo run", "execute seed" | `ouroboros_start_execute_seed` + `*_job_*` | git PR 감지, 백그라운드 실행, low-token relay loop, 자동 QA |
| `welcome` | 261 | bare "ooo" | — | first-touch onboarding |
| `tutorial` | 230 | "ooo tutorial" | — | interactive 가이드 |
| `brownfield` | 204 | "brownfield" | `ouroboros_brownfield` | 홈 git 레포 스캔, default 설정, mechanical.toml 작성 |
| `ralph` | 193 | "ralph", "don't stop" | `ouroboros_start_evolve_step` + `*_job_*` | 진화 루프 |
| `update` | 187 | "ooo update" | — | PyPI + plugin marketplace 업데이트 |
| `qa` | 176 | "ooo qa", "qa check" | `ouroboros_qa` | 단일 verdict (formal evaluate 와 다름) |
| `seed` | 149 | "crystallize" | — | 인터뷰 → Seed (보통 자동, 수동은 advanced) |
| `help` | 141 | "ooo help" | — | reference |
| `pm` | 129 | "ooo pm", "prd" | `ouroboros_pm_interview` | PM 트랙 인터뷰 → PRD |
| `status` | 121 | "ooo status", "drift check" | `ouroboros_session_status` | 세션 상태 + drift |
| `evaluate` | 120 | "evaluate this" | `ouroboros_evaluate` | 3-stage 트리거 |
| `evolve` | 120 | "ooo evolve" | `ouroboros_start_evolve_step` | 진화 모니터 |
| `resume` | 116 | "ooo resume" | — | 세션 재개 |
| `unstuck` | 105 | "I'm stuck" | `ouroboros_lateral_think` | 5 페르소나 측면 사고 |
| `cancel` | 104 | "ooo cancel" | `ouroboros_cancel_job` + `ouroboros_cancel_execution` | 정체 작업 cancel (Section 12 의 23 tool catalog 참조) |

총 3941 LOC.

## SKILL.md 프론트매터 표준

```yaml
---
name: <skill-name>
description: "<한 줄 설명>"
mcp_tool: <optional MCP 도구 이름>     # 자동 호출
mcp_args:                                # 변수 치환
  arg1: "$1"                              # 첫 인자
  cwd: "$CWD"                             # 현재 디렉토리
---

# /ouroboros:<skill-name>

본문...
```

## 21 Bundled Agents

`src/ouroboros/agents/*.md` — 페르소나 prompt 파일.

### Core 9

| 에이전트 | 역할 | 핵심 질문 |
|---|---|---|
| socratic-interviewer | 질문만 | "What are you assuming?" |
| ontologist | essence 식별 | "What IS this, really?" |
| seed-architect | 인터뷰 → Seed YAML 변환 | "Is this complete and unambiguous?" |
| evaluator | 3-stage 검증 | "Did we build the right thing?" |
| qa-judge | 일반 QA verdict | JSON-only output |
| contrarian | 모든 가정 도전 | "What if the opposite were true?" |
| hacker | 비정통 우회 | "What constraints are actually real?" |
| simplifier | 복잡도 감소 | "Simplest thing that could work?" |
| researcher | 정보 수집 | "What evidence do we have?" |

### Support 12

architect, advocate, judge, breadth-keeper, codebase-explorer, code-executor, consensus-reviewer, semantic-evaluator, ontology-analyst, seed-closer, analysis-agent, research-agent.

## 페르소나 ↔ 페이즈 매핑

- Phase 0: socratic-interviewer + breadth-keeper + ontologist + seed-closer
- Phase 1 routing: 페르소나 없음 (CPU only)
- Phase 2: code-executor + codebase-explorer
- Phase 3: hacker, researcher, simplifier, architect, contrarian (5)
- Phase 4 stage 2: semantic-evaluator
- Phase 4 stage 3: advocate + judge + consensus-reviewer
- QA: qa-judge

## 핵심 페르소나 인용 (간략)

### socratic-interviewer.md (3.1K)
- CRITICAL ROLE BOUNDARIES: "I will implement X" / "Let me build" / "I'll create" 절대 금지
- 도구 직접 접근 없음 — caller (main session) 가 코드 컨텍스트 제공
- BREADTH CONTROL: 시작 시 ambiguity track 추론 + 유지, 한 thread 에 collapse 안 됨
- STOP CONDITIONS: scope/non-goals/outputs/verification 모두 explicit 시 종료 권장

### qa-judge.md (1.6K)
JSON-only 응답:
```json
{
  "score": 0.85,
  "verdict": "pass",
  "dimensions": {
    "correctness": 0.9,
    "completeness": 0.85,
    "quality": 0.85,
    "intent_alignment": 0.85,
    "domain_specific": 0.8
  },
  "differences": ["..."],
  "suggestions": ["..."],
  "reasoning": "..."
}
```

Verdict 규칙:
- `score ≥ pass_threshold (default 0.80)` → `pass`
- `score ≥ 0.40` → `revise`
- `score < 0.40` → `fail`

제약:
- 모든 difference 는 대응 suggestion
- "Five concrete differences beat twenty vague ones"

### contrarian.md (2.2K)
5단계:
1. List every assumption explicit
2. Consider opposite for each
3. Challenge problem statement
4. "What if we did nothing?"
5. Invert obvious approach

### seed-architect.md (2.6K)
출력 스키마 strict (pipe-separated):
```
GOAL: <goal>
CONSTRAINTS: <c1> | <c2> | ...
ACCEPTANCE_CRITERIA: <ac1> | <ac2> | ...
ONTOLOGY_NAME: <name>
ONTOLOGY_DESCRIPTION: <desc>
ONTOLOGY_FIELDS: <name>:<type>:<desc> | ...
EVALUATION_PRINCIPLES: <name>:<desc>:<weight> | ...
EXIT_CONDITIONS: <name>:<desc>:<criteria> | ...
PROJECT_TYPE: greenfield|brownfield
CONTEXT_REFERENCES: <path>:<role>:<summary> | ...
EXISTING_PATTERNS: <p1> | <p2> | ...
EXISTING_DEPENDENCIES: <d1> | <d2> | ...
```

Field types: `string|number|boolean|array|object`. Weights 0.0-1.0.

### evaluator.md (2.2K)
3-Stage 명시 알고리즘:
- Stage 1 mechanical → 모두 통과 필수
- Stage 2 semantic → AC 100% + score weighted
- Stage 3 consensus → triggered (manual / score < 0.8 / high ambiguity / disagreement)

Output 포맷 strict:
```
## Stage 1: Mechanical Verification
[results]
**Result**: PASSED / FAILED

## Stage 2: Semantic Evaluation
[AC analysis]
**AC Compliance**: X%
**Overall Score**: X.XX
**Result**: PASSED / FAILED

## Stage 3: Consensus (if triggered)
[deliberation]
**Approval**: X% (threshold: 66%)
**Result**: APPROVED / REJECTED

## Final Decision: APPROVED / REJECTED
```

## Loader (`agents/loader.py` 233 LOC) — Section 28 deep-dive

### 2-Tier 해상도

```python
@functools.lru_cache(maxsize=64)
def _resolve_agent_path(agent_name: str) -> Path | None:
    # Tier 1: explicit env var
    agents_dir = os.environ.get("OUROBOROS_AGENTS_DIR")
    if agents_dir:
        path = Path(agents_dir) / f"{agent_name}.md"
        if path.exists():
            return path

    # Tier 2: fall through to importlib.resources
    return None
```

→ env override 우선. 못 찾으면 `importlib.resources.files("ouroboros.agents")` fallback.

### LRU 캐시

```python
@functools.lru_cache(maxsize=64)
def load_agent_prompt(agent_name: str) -> str:
    path = _resolve_agent_path(agent_name)
    if path is not None:
        return path.read_text(encoding="utf-8")
    package = importlib.resources.files("ouroboros.agents")
    return package.joinpath(f"{agent_name}.md").read_text(encoding="utf-8")
```

→ CWD 변경되어도 안정. plugin reload 시 `clear_cache()` 호출.

### Section 추출 유틸

```python
def extract_section(content: str, section: str) -> str:
    """## <section> 와 다음 ## 사이 텍스트 (case-insensitive)"""

def extract_list_items(section_content: str) -> tuple[str, ...]:
    """- bullet 추출"""

def _extract_numbered_items(content: str) -> tuple[str, ...]:
    """### N. Title 또는 N. Text 두 형식 지원"""
```

### `PersonaPromptData` 데이터 클래스

```python
@dataclass(frozen=True, slots=True)
class PersonaPromptData:
    system_prompt: str                 # # title 과 첫 ## 사이 단락
    approach_instructions: tuple[str, ...]  # ## YOUR APPROACH 의 numbered items
    question_templates: tuple[str, ...]      # ## YOUR QUESTIONS 의 bullet items

def load_persona_prompt_data(agent_name: str) -> PersonaPromptData: ...
```

→ lateral thinking 페르소나 5 (architect, contrarian, hacker, researcher, simplifier) 가 사용. 각 페르소나의 prompt 구조가 일관되어 자동 파싱 가능.

## 13 Commands Stubs (`commands/*.md`) — 모두 thin wrapper

각 command 파일 = 5–11 LOC stub. 패턴:

```yaml
---
description: "<설명>"
aliases: [<alias1>, <alias2>]    # optional
---

Read the file at `${CLAUDE_PLUGIN_ROOT}/skills/<skill-name>/SKILL.md` using the Read tool and follow its instructions exactly.

## User Input          # optional, only commands taking arguments

{{ARGUMENTS}}
```

### 13 Command 카탈로그

| 파일 | aliases | description |
|---|---|---|
| `cancel.md` | kill, abort | Cancel stuck or orphaned executions |
| `evaluate.md` | eval | Evaluate execution with three-stage verification pipeline |
| `evolve.md` | — | Start or monitor an evolutionary development loop |
| `help.md` | — | Full reference guide for Ouroboros commands and agents |
| `interview.md` | socratic | Socratic interview to crystallize vague requirements |
| `ralph.md` | — | Persistent self-referential loop until verification passes |
| `run.md` | execute | Execute a Seed specification through the workflow engine |
| `seed.md` | crystallize | Generate validated Seed specifications from interview results |
| `setup.md` | — | Guided onboarding wizard for Ouroboros setup |
| `status.md` | drift | Check session status and measure goal drift |
| `tutorial.md` | — | Interactive tutorial teaching Ouroboros hands-on |
| `unstuck.md` | stuck, lateral | Break through stagnation with lateral thinking personas |
| `welcome.md` | — | First-touch experience for new Ouroboros users |

→ 13 commands 모두 `${CLAUDE_PLUGIN_ROOT}/skills/<name>/SKILL.md` 위임. 본체 로직 0. Claude Code 의 slash command 가 SKILL.md 본문 읽고 따라감.

→ 13 commands < 19 skills — 6 skills (qa, publish, brownfield, pm, update, resume) 는 commands 노출 없음 (skill-only or magic keyword 트리거).

## 21 Agent .md 본문 인용 — Section 28 보강

### Lateral thinking 5 페르소나 (Phase 3 / unstuck)

#### `hacker.md` (2.0K)

> "You don't accept 'impossible'—you find the path others miss. Rules are obstacles to route around, not walls to stop at."

4-step approach:
1. Identify constraints (explicit + implicit)
2. Question each constraint (security 진짜, performance 협상 가능, architectural 임의 가능)
3. Look for edge cases
4. Consider bypassing entirely ("API rate limited" → batch 사이드)

#### `architect.md` (2.0K)

> "If you're fighting the architecture, the architecture is wrong. Step back and redesign before pushing forward."

#### `contrarian.md` (2.2K)

> "What everyone assumes is true, you examine. What seems obviously correct, you invert."

5-step: list assumptions → consider opposite → challenge problem statement → "what if we did nothing?" → invert obvious approach

#### `simplifier.md` (2.1K)

> "Every requirement should be questioned, every abstraction justified."

5 heuristics:
- **YAGNI**: You Aren't Gonna Need It
- **Concrete First**: Build the specific case before the general
- **No Abstractions Without Duplication**: Three times before you abstract
- **Data Over Code**: Can data structure replace logic?
- **Worse Is Better**: Simple and working beats perfect and broken

→ "Remove at least 50% of components/features"

#### `researcher.md` (2.0K)

> "Most bugs and blocks exist because we're missing information. Stop guessing—go find the answer."

4-step: define unknown → gather evidence → read documentation (official > Stack Overflow) → form hypothesis

### Phase 4 deliberative (advocate / judge / devil)

#### `advocate.md` (701 B) — JSON-only

```json
{
  "approved": true,
  "confidence": 0.85,
  "reasoning": "..."
}
```

→ "Find STRENGTHS of this solution". 진짜 strength 가 없으면 against 투표 가능 (rare).

#### `judge.md` (1001 B) — JSON-only

```json
{
  "verdict": "approved | rejected | conditional",
  "confidence": 0.85,
  "reasoning": "...",
  "conditions": ["...", "..."] or null
}
```

→ Advocate + Devil 두 입장 weight + ROOT CAUSE vs SYMPTOM 판정.

#### `consensus-reviewer.md` (715 B) — JSON-only

```json
{
  "approved": true,
  "confidence": 0.85,
  "reasoning": "..."
}
```

→ Stage 3 의 generic reviewer (개별 vote, 다른 reviewer 와 합산).

#### `semantic-evaluator.md` (1.5K) — JSON-only Stage 2

```json
{
  "score": 0.85,
  "ac_compliance": true,
  "goal_alignment": 0.85,
  "drift_score": 0.15,
  "uncertainty": 0.2,
  "reasoning": "...",
  "questions_used": ["socratic Q...", "..."],     // anti-reward-hacking 투명성
  "evidence": ["concrete evidence...", "..."]
}
```

→ pass 조건: ac_compliance=true / score≥0.8 / goal_alignment≥0.7 / drift_score≤0.3 / uncertainty≤0.3.

### Ontological 분석 (Phase 0 / Phase 4 Devil's Advocate)

#### `ontologist.md` (1.4K)

4 fundamental questions:
1. **ESSENCE**: "What IS this, really?"
2. **ROOT CAUSE**: "Is this the root cause or a symptom?"
3. **PREREQUISITES**: "What must exist first?"
4. **HIDDEN ASSUMPTIONS**: "What are we assuming?"

→ "Goal is NOT to reject everything, but to ensure we're solving the ROOT problem, not just treating SYMPTOMS."

#### `ontology-analyst.md` (894 B) — JSON-only

```json
{
  "essence": "...",
  "is_root_problem": true,
  "prerequisites": [...],
  "hidden_assumptions": [...],
  "confidence": 0.85,
  "reasoning": "..."
}
```

### Phase 0 인터뷰 보조

#### `socratic-interviewer.md` (3.1K)

CRITICAL ROLE BOUNDARIES (인용):
> - You are ONLY an interviewer. You gather information through questions.
> - NEVER say "I will implement X", "Let me build", "I'll create"
> - Another agent will handle implementation AFTER you finish gathering requirements

TOOL USAGE:
> - You are a QUESTION GENERATOR. You do NOT have direct tool access.
> - The caller (main session) handles codebase reading and provides code context

BROWNFIELD context 분기:
> - Answers prefixed with `[from-code]` describe existing codebase state (factual)
> - Answers prefixed with `[from-user]` are human decisions/judgments
> - Answers prefixed with `[from-research]` contain externally researched information

BREADTH CONTROL:
> - At the start of the interview, infer the main ambiguity tracks
> - If the request contains multiple deliverables, treat those as separate tracks
> - After a few rounds on one thread, run a breadth check
> - If one file/abstraction/bug has dominated several rounds, explicitly zoom back out

#### `breadth-keeper.md` (1.7K)

> "Depth matters, but only after we've preserved the full shape of the problem."

4-step: infer open tracks → detect drift → run breadth checks → keep scope honest

#### `seed-closer.md` (3.2K)

> "A good interview ends on time, but not before unresolved decisions that would change execution are exposed."

CLOSURE GATE SUMMARY:
- 낮은 ambiguity score = "permission to audit closure", NOT "permission to close"
- material blocker 가 있으면 close 안 함
- brownfield/system 작업: ownership/SSoT, protocol/API contract, lifecycle/recovery, migration, cross-client impact, verification 모두 검사

#### `seed-architect.md` (2.6K)

Output 스키마 strict (pipe-separated, 11 fields):
```
GOAL: <goal>
CONSTRAINTS: c1 | c2 | ...
ACCEPTANCE_CRITERIA: ac1 | ac2 | ...
ONTOLOGY_NAME: <name>
ONTOLOGY_DESCRIPTION: <desc>
ONTOLOGY_FIELDS: name:type:desc | ...
EVALUATION_PRINCIPLES: name:desc:weight | ...
EXIT_CONDITIONS: name:desc:criteria | ...
PROJECT_TYPE: greenfield|brownfield
CONTEXT_REFERENCES: path:role:summary | ...
EXISTING_PATTERNS: p1 | p2 | ...
EXISTING_DEPENDENCIES: d1 | d2 | ...
```

→ field types: `string|number|boolean|array|object`. Weights 0.0–1.0.

### Phase 2 실행 보조

#### `code-executor.md` (376 B) — minimal prompt

```
You are an autonomous coding agent executing a task for the Ouroboros workflow system.

## Guidelines
- Execute each acceptance criterion thoroughly
- Use the available tools (Read, Edit, Bash, Glob, Grep)
- Write clean, well-tested code following project conventions
- Report progress clearly as you work
- If you encounter blockers, explain them clearly
```

→ 매우 짧음. 대부분 컨텍스트는 system_prompt build 시점에 추가됨.

#### `codebase-explorer.md` (1.3K) — Read-only brownfield

> "Read-only: Use Read, Glob, Grep to explore. Do NOT use Write, Edit, or Bash."
> "Be concise — under 500 words" (interview context 에 inject 되므로)

Output: tech stack / key types / patterns / protocols & APIs / conventions.

→ 불확실한 패턴은 "appears to" 표기.

### Stage 1/2/3 통합 evaluator

#### `evaluator.md` (2.2K) — 3-stage 명시

Stage 1 (Mechanical, $0): LINT / BUILD / TEST / STATIC / COVERAGE — 모두 통과 필수
Stage 2 (Semantic, Standard tier): AC compliance 100% + Score ≥ 0.8 (weighted)
Stage 3 (Consensus, Frontier tier - triggered): PROPOSER + DEVIL'S ADVOCATE + SYNTHESIZER. ≥ 66% approval.

Stage 3 triggers (4):
- Manual request
- Stage 2 score < 0.8 (but passed)
- High ambiguity detected
- Stakeholder disagreement

→ 1차 라운드 Section 7 (evaluation) 의 evaluator persona 와 align.

### Phase 4 spec verification

#### `qa-judge.md` (1.6K) — generic JSON-only

```json
{
  "score": 0.85,
  "verdict": "pass | revise | fail",
  "dimensions": {
    "correctness": 0.9,
    "completeness": 0.85,
    "quality": 0.85,
    "intent_alignment": 0.85,
    "domain_specific": 0.8
  },
  "differences": ["..."],
  "suggestions": ["..."],
  "reasoning": "..."
}
```

5 dimensions 모두 0.0–1.0.

Verdict 규칙:
- `score ≥ pass_threshold (default 0.80)` → `pass`
- `score ≥ 0.40` → `revise`
- `score < 0.40` → `fail`

제약:
- 모든 difference 마다 대응 suggestion
- "Five concrete differences beat twenty vague ones"

### 보조 minimal agents

#### `analysis-agent.md` (407 B) + `research-agent.md` (500 B)

→ 매우 짧은 generic prompt (~10 LOC). markdown 결과 docs/ 또는 output/ 에 저장. system_prompt 형식 일관.

## 검증

`tests/unit/agents/test_loader.py`
`tests/unit/plugin/agents/test_registry.py`
`tests/unit/plugin/orchestration/test_model_router.py`
`tests/unit/plugin/skills/test_keywords.py`
`tests/unit/plugin/skills/test_registry.py`
`tests/unit/skills/test_skill_artifacts.py`
`tests/integration/plugin/test_orchestration.py`
