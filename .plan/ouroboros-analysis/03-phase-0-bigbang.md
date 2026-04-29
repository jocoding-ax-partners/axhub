# 03. Phase 0 — BIG BANG (Interview → Seed)

## 책임

모호한 사용자 요청을 ambiguity ≤ 0.2 까지 정제 후 immutable Seed 생성. **AI 코딩이 출력이 아니라 입력에서 실패한다는 가설** 의 실증.

## 핵심 모듈 (`src/ouroboros/bigbang/`)

| 파일 | 역할 |
|---|---|
| `interview.py` | `InterviewEngine`, `InterviewState` (Pydantic), `InterviewRound`, `InterviewPerspective(StrEnum)`, `InterviewStatus(StrEnum)`, `prompt_safe_initial_context()`, `_truncate_prompt_safe_context()`, `initial_context_summary_missing()` |
| `ambiguity.py` | `AmbiguityScorer`, `AmbiguityScore`, `ComponentScore`, `ScoreBreakdown`, `AmbiguityMilestone(StrEnum)`, `qualifies_for_seed_completion()`, `is_ready_for_seed()`, `format_score_display()`, `get_milestone()`, `get_next_milestone()`, `get_completion_floor_failures()` |
| `seed_generator.py` | `SeedGenerator`, `load_seed()`, `save_seed_sync()` |
| `brownfield.py` | 기존 코드베이스 탐색 |
| `explore.py` | 코드베이스 매핑 |
| `question_classifier.py` | 질문 카테고리 분류 |
| `pm_interview.py` | PM 트랙 별도 인터뷰 |
| `pm_completion.py` | PM 트랙 종료 |
| `pm_document.py` | PRD 문서 생성 |
| `pm_seed.py` | PM 결과 → Seed |

## Ambiguity 수식

```
Ambiguity = 1 - Σ(clarity_i × weight_i)
```

LLM temperature 고정 0.1 (재현성).

### 가중치 정책

| 차원 | Greenfield | Brownfield |
|---|---|---|
| Goal Clarity | 40 % | 35 % |
| Constraint Clarity | 30 % | 25 % |
| Success Criteria | 30 % | 25 % |
| Context Clarity | — | 15 % |

### 게이트

`Ambiguity ≤ 0.2` → Seed 생성 가능.

이유: 80% weighted clarity 면 남은 unknown 은 코드 레벨에서 해결 가능. 그 이상은 architecture 추측.

### 예시 (Greenfield)
```
Goal: 0.9 × 0.4 = 0.36
Constraint: 0.8 × 0.3 = 0.24
Success: 0.7 × 0.3 = 0.21
                    -----
Clarity = 0.81
Ambiguity = 1 - 0.81 = 0.19  ≤ 0.2  → Ready for Seed
```

## Round 정책 (v0.3.0+)

기존 `MAX_INTERVIEW_ROUNDS = 10` 하드 리밋 제거. 대체:

| Round | 동작 |
|---|---|
| 1–3 | Auto-continue (최소 컨텍스트 수집) |
| 4–15 | "Continue?" 매 round 마다 질문 |
| 16+ | "Continue?" + diminishing returns 경고 |

상수: `MIN_ROUNDS_BEFORE_EARLY_EXIT`, `SOFT_LIMIT_WARNING_THRESHOLD`. `is_complete` = 상태만 검사 (사용자가 종료 결정). `record_response()` 더 이상 max round 시 자동 완료 안 함.

## Skill 통합 — `skills/interview/SKILL.md` (337 LOC, 가장 정교)

### Step 0 — 버전 체크
```bash
curl -s --max-time 3 https://api.github.com/repos/Q00/ouroboros/releases/latest \
  | grep -o '"tag_name": "[^"]*"' | head -1
```
3초 타임아웃. 실패하면 silent skip.

### Step 0.5 — Deferred MCP 도구 강제 로드
```
ToolSearch query: "+ouroboros interview"
```
**중요**: MCP 도구가 deferred 면 즉시 호출 불가. ToolSearch 가 schema 로드 + 호출 가능 상태 만듦.

### Step 1 — Path 분기

**Path A (MCP 모드)**: MCP 가 question generator, main session 이 answerer + router.
**Path B (fallback)**: MCP 없으면 `agents/socratic-interviewer.md` 페르소나로 직접.

### Path A — 4-PATH 라우팅

각 MCP 질문에 대해:

#### PATH 1a — Auto-confirm (높은 신뢰 사실)
모든 조건 충족 시:
- 답이 manifest/config 의 정확한 매치 (`pyproject.toml`, `package.json`, `Dockerfile`, `go.mod`)
- 순수 기술 (description, not prescription)
- 모호성 없음 (단일 답)

→ MCP 에 `[from-code][auto-confirmed] Python 3.12, FastAPI (pyproject.toml)` 즉시 전송. 사용자에게 ℹ️ 알림만 (블록 안 함). 사용자가 "그 거 틀렸어" 말하면 정정 전송.

자동 가능 사실: 프로그래밍 언어, 프레임워크, Python/Node 버전, 패키지 매니저, CI/CD 도구.

#### PATH 1b — Code Confirmation (중간/낮은 신뢰)
코드베이스에서 추론은 됐으나 manifest 정확 매치 없음:
```json
{"questions": [{
  "question": "MCP asks: What auth method does the project use?\n\nI found: JWT-based auth in src/auth/jwt.py\n\nIs this correct?",
  "header": "Q<N> — Code Confirmation",
  "options": [
    {"label": "Yes, correct", "description": "Use this as the answer"},
    {"label": "No, let me correct", "description": "I'll provide the right answer"}
  ]
}]}
```
응답에 `[from-code]` 접두.

#### PATH 2 — Human Judgment (사람만 결정 가능)
goal, vision, AC, business logic, preferences, tradeoffs → AskUserQuestion 직행. `[from-user]` 접두.

#### PATH 3 — Code + Judgment
코드에 사실 있지만 해석 필요. 코드 먼저 읽고 + 질문도 사용자에게. 일부라도 judgment 필요하면 전체 사용자 라우팅.

#### PATH 4 — Research Interlude
3rd-party API, 가격, 라이브러리 호환성, security advisory — 로컬 코드베이스에 없음. WebFetch/WebSearch → 사용자에게 confirmation. `[from-research]` 접두.

**의심 시 PATH 2** — 사용자 묻는 게 추측보다 안전.

## Dialectic Rhythm Guard

비-사용자 답변 (PATH 1a + 1b + 4) 3 회 연속 시 다음 질문은 PATH 2 강제.

이유: Socratic 대화 리듬 유지. Auto-confirm 너무 많으면 사용자가 AI 가 자기 프로젝트에 대해 뭐 가정하는지 인지 못 함. PATH 2/3 응답 시 카운터 reset.

## Seed-ready Acceptance Guard

MCP 가 seed-ready 신호 → main session 이 `agents/seed-closer.md` 기준으로 재검증. 갭 발견 시 override:

```
"MCP says seed-ready, but I am not accepting it yet because <gap>."
```

→ 갭 설명 + 가장 영향 큰 후속 질문 1개 (PATH 2 또는 3) 라우팅.

이유: MCP 는 코드/research 컨텍스트 안 봤음. main session 이 단일 게이트 keeper.

## Retry on Failure

MCP 에러 (`is_error=true` + `meta.recoverable=true`):
1. 사용자에게 "Retrying..." 알림
2. `ouroboros_interview(session_id=...)` 호출 (max 2 retry). 상태 영속 — 진행 안 잃음.
3. 여전히 실패 → Path B 폴백.

## Seed 모델 (`src/ouroboros/core/seed.py`)

frozen Pydantic. 핵심 invariant: **Direction = goal + constraints + acceptance_criteria 는 immutable**.

```python
class Seed(BaseModel, frozen=True):
    goal: str                                    # min_length=1
    task_type: str = "code"                       # "code"|"research"|"analysis"
    brownfield_context: BrownfieldContext = ...   # default greenfield
    constraints: tuple[str, ...]                  # 변경 불가
    acceptance_criteria: tuple[str, ...]          # 변경 불가
    ontology_schema: OntologySchema               # name, description, fields
    evaluation_principles: tuple[EvaluationPrinciple, ...]  # weight 0-1
    exit_conditions: tuple[ExitCondition, ...]
    metadata: SeedMetadata                         # seed_id, version, created_at,
                                                   # ambiguity_score, interview_id,
                                                   # parent_seed_id

class SeedMetadata(BaseModel, frozen=True):
    seed_id: str = f"seed_{uuid4().hex[:12]}"     # auto
    version: str = "1.0.0"
    created_at: datetime = datetime.now(UTC)
    ambiguity_score: float                        # 0-1, ge=0 le=1
    interview_id: str | None
    parent_seed_id: str | None                     # 진화 chain 용

class OntologyField(BaseModel, frozen=True):
    name: str
    field_type: str                                # "string"|"number"|"boolean"|"array"|"object"
    description: str
    required: bool = True
```

수정 시도 → Pydantic ValidationError. 진짜 변경 필요하면 → consensus stage 3 트리거.

## Brownfield 컨텍스트

```python
class BrownfieldContext(BaseModel, frozen=True):
    project_type: str = "greenfield"               # 'greenfield' | 'brownfield'
    context_references: tuple[ContextReference, ...]
    existing_patterns: tuple[str, ...]
    existing_dependencies: tuple[str, ...]

class ContextReference(BaseModel, frozen=True):
    path: str                                       # 절대 경로
    role: str                                       # 'primary' (수정) | 'reference' (read-only)
    summary: str
```

`brownfield/SKILL.md` + `cli/commands/detect.py` 가 자동 감지 → `mechanical.toml` 자동 작성 (단일 AI 호출).

## 출력 — `~/.ouroboros/seeds/seed_*.yaml`

생성 후 `parent_seed_id` 로 진화 체인 추적.

## 의존 컴포넌트

- LLMAdapter (Provider) — 질문 생성
- EventStore — `interview.*` 이벤트 영속화
- AC Tree (`core/ac_tree.py`) — AC 재구성

## Phase 0 ↔ 다른 Phase 연결

- Out → Phase 1 PAL Router (Seed.task_type 으로 strategy 선택)
- Out → Phase 4 Evaluation (acceptance_criteria 가 검증 기준)
- Out → Evolution Loop (parent_seed_id 로 lineage 추적)
- Re-trigger ← Phase 4 Stage 3 Consensus (Seed 수정 트리거)
