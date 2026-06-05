//! Shared routing-decision logic — the **single source of truth** consumed by
//! both axhub trigger paths:
//!
//! 1. the prompt-route hook (`UserPromptSubmit`, runs first), and
//! 2. the deploy SKILL preflight (Step 0, runs when the skill is selected).
//!
//! Keeping the decision in one pure function makes logic drift between the two
//! paths *structurally impossible*: if both layers call [`decide`], they cannot
//! disagree for the same inputs. This is the mechanism behind the
//! "composition-consistency" guarantee (spec 006 §49-59, "공유 routing-decision
//! 함수"). The classic drift this prevents: the hook yields to `"vercel"` while
//! the preflight sees only `marker_present` and routes to axhub anyway — which
//! reproduces the original "Vercel 쓰고 싶은데 axhub 로 라우팅" complaint on every
//! marker-present repo (i.e. every developer's own repo). See [`decide_from_flags`].
//!
//! ## Design
//!
//! - [`decide_from_flags`] is the **pure ordered priority chain** over already
//!   computed booleans — exhaustively matrix-testable, no I/O. The *order* of
//!   the `if`/`else` arms is the load-bearing logic, not the individual rule
//!   outputs.
//! - [`decide`] is the thin public wrapper that derives the keyword flags from
//!   the raw prompt via the shared detectors ([`axhub_keyword_present`],
//!   [`foreign_keyword_present`]) so both layers derive inputs identically too
//!   (input-construction is itself a drift surface).
//! - Marker presence is modeled as a tri-state [`MarkerStatus`] so a walk-up
//!   that *errors* (fs permission / race) falls open auth-conditionally
//!   (spec 006 §99) instead of silently collapsing the error into "absent".

use std::path::Path;

/// Foreign deploy-target keywords. Hardcoded per spec 006 §45-47 — a
/// slow-changing set, intentionally not externalized. Presence of any of these
/// (with no explicit `"axhub"`) means the user named another target, so axhub
/// yields ("named target wins").
pub const FOREIGN_TARGET_KEYWORDS: &[&str] = &[
    "vercel",
    "netlify",
    "cloudflare",
    "fly",
    "render",
    "railway",
];

/// The literal keyword that marks an explicit axhub intent (marker-independent).
pub const AXHUB_KEYWORD: &str = "axhub";

/// Defensive cap on marker walk-up depth so a pathological filesystem can never
/// spin the hot path. 64 levels is far deeper than any real project tree.
const MAX_WALK_UP_DEPTH: usize = 64;

/// Outcome of the shared routing decision.
///
/// Serialized lowercase (`"axhub"` / `"yield"` / `"ignore"` / `"ask"`) for the
/// routing-audit jsonl + routing-stats skill (spec 006 §94). Intentionally
/// **not** `#[non_exhaustive]`: consumers must handle all four arms, and adding
/// a variant later *should* force them to update their action mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingDecision {
    /// Proceed with axhub routing. hook → neutral (allow skill-selection);
    /// preflight → proceed with deploy.
    Axhub,
    /// Yield to the normal flow (user named a foreign target). hook → silent;
    /// preflight → yield to general flow.
    Yield,
    /// Not an axhub intent (no marker, bare NL). hook → silent (+ once-per-project
    /// grace when authed); preflight → disambiguation.
    Ignore,
    /// Ambiguous (axhub + foreign both named). hook → neutral (cannot run tools;
    /// disambiguation is owned by the preflight); preflight → disambiguation.
    Ask,
}

impl RoutingDecision {
    /// Lowercase wire form, matching the spec's decision literals and the serde
    /// representation. Handy for audit logging without a serde round-trip.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RoutingDecision::Axhub => "axhub",
            RoutingDecision::Yield => "yield",
            RoutingDecision::Ignore => "ignore",
            RoutingDecision::Ask => "ask",
        }
    }
}

/// Tri-state result of the marker walk-up.
///
/// `Unknown` distinguishes a genuine filesystem error (permission / race)
/// from a confirmed `Absent`, so [`decide_from_flags`] can apply the
/// auth-conditional fail-open (spec 006 §99) only on real errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerStatus {
    /// `axhub.yaml` found via cwd→git-root walk-up.
    Present,
    /// Walk-up completed (git root or fs root reached) with no `axhub.yaml`.
    Absent,
    /// Walk-up could not complete because a filesystem stat errored.
    Unknown,
}

/// The pure, ordered routing-decision priority chain (spec 006 §32-43 rules
/// `0`..`e` + the `err` fallback). This function does **no I/O** — feed it
/// already-computed flags. The arm order encodes precedence and is the part
/// AC-16 exists to lock; do not reorder without updating the matrix tests.
///
/// Priority (first match wins):
/// - **0** `explicit_invocation` (slash `/deploy`, `/axhub:deploy`) → `Axhub`
///   (strongest explicit signal; beats every keyword conflict).
/// - **a** `axhub_keyword` AND `foreign_keyword` → `Ask` (disambiguate).
/// - **b** `axhub_keyword` → `Axhub` (explicit, marker-independent).
/// - **c** `foreign_keyword` → `Yield` (named target wins; beats marker).
/// - **d** bare NL + marker `Present` → `Axhub`.
/// - **e** bare NL + marker `Absent` → `Ignore` (grace is consumer-side).
/// - **err** bare NL + marker `Unknown` → `authed ? Axhub : Ignore`.
#[must_use]
pub fn decide_from_flags(
    axhub_keyword: bool,
    foreign_keyword: bool,
    marker: MarkerStatus,
    authed: bool,
    explicit_invocation: bool,
) -> RoutingDecision {
    // rule 0 — slash invocation: strongest explicit, above keyword conflict.
    if explicit_invocation {
        return RoutingDecision::Axhub;
    }
    // rule a — both axhub and a foreign target named: ambiguous, disambiguate.
    if axhub_keyword && foreign_keyword {
        return RoutingDecision::Ask;
    }
    // rule b — explicit "axhub" keyword (no foreign): marker-independent.
    if axhub_keyword {
        return RoutingDecision::Axhub;
    }
    // rule c — foreign target named (no axhub): yield. Beats marker → "named target wins".
    if foreign_keyword {
        return RoutingDecision::Yield;
    }
    // bare NL (no explicit keyword): the marker decides.
    match marker {
        MarkerStatus::Present => RoutingDecision::Axhub, // rule d
        MarkerStatus::Absent => RoutingDecision::Ignore, // rule e (grace handled by consumer)
        // err — marker walk-up errored: fall open auth-conditionally (spec §99).
        // Authed users keep their "배포해"→axhub behavior through transient fs
        // errors; unauthed users stay zero-footprint (pass-through / ignore).
        MarkerStatus::Unknown => {
            if authed {
                RoutingDecision::Axhub
            } else {
                RoutingDecision::Ignore
            }
        }
    }
}

/// Public entry point: derive keyword flags from the raw `prompt` (via the
/// shared detectors) and run [`decide_from_flags`]. Both the hook and the deploy
/// preflight call this so keyword derivation cannot drift between them either.
#[must_use]
pub fn decide(
    prompt: &str,
    marker: MarkerStatus,
    authed: bool,
    explicit_invocation: bool,
) -> RoutingDecision {
    decide_from_flags(
        axhub_keyword_present(prompt),
        foreign_keyword_present(prompt),
        marker,
        authed,
        explicit_invocation,
    )
}

/// True when `prompt` names axhub explicitly. Whole-word match (see
/// [`contains_word`]) so it fires on `"axhub 배포"` and `"axhub.yaml"` but the
/// match is bounded — no surprise substring hits.
#[must_use]
pub fn axhub_keyword_present(prompt: &str) -> bool {
    contains_word(&prompt.to_lowercase(), AXHUB_KEYWORD)
}

/// True when `prompt` names any [`FOREIGN_TARGET_KEYWORDS`] target. Whole-word
/// match so `"render the page"` / `"fly.io"` fire but `"rendered"` /
/// `"butterfly"` do not.
#[must_use]
pub fn foreign_keyword_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    FOREIGN_TARGET_KEYWORDS
        .iter()
        .any(|kw| contains_word(&lower, kw))
}

/// True when a prompt asks for axhub app dynamic-table DDL/DML.
///
/// This is intentionally narrower than general skill routing: it only covers the
/// destructive table surface where native skill matching has proved costly when
/// it drifts into `help`/`apps` and then invents unsupported CLI flags. Catalog
/// data prompts such as "describe snowflake orders table" or "테이블 설명" must
/// stay outside this detector.
#[must_use]
pub fn dynamic_table_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if contains_any_text(
        p,
        &["동적 테이블", "앱 테이블", "dynamic table", "app table"],
    ) {
        return true;
    }

    if contains_any_text(
        p,
        &[
            "테이블 만들",
            "테이블을 만들",
            "테이블 만들어",
            "테이블 생성",
            "create table",
            "table create",
        ],
    ) {
        return true;
    }

    let tableish = contains_any_text(p, &["테이블", "table", "tables"]);
    if tableish
        && contains_any_text(
            p,
            &[
                "drop", "삭제", "지우", "컬럼", "column", "grant", "revoke", "권한",
            ],
        )
    {
        return true;
    }

    contains_any_text(
        p,
        &[
            "행 추가",
            "행 넣",
            "레코드 삽입",
            "insert row",
            "delete row",
            "row insert",
            "row update",
            "row delete",
        ],
    )
}

/// True when the prompt is about AXHub external database connector management.
///
/// Keep this separate from local app database coding. Prompts that mention a
/// hosted app's own `DATABASE_URL`, local package dependencies, or ORM code
/// should stay outside this detector unless they also clearly ask for an AXHub
/// connector.
#[must_use]
pub fn connectors_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    let dbish = contains_any_text(
        p,
        &[
            "db",
            "database",
            "데이터베이스",
            "postgres",
            "postgresql",
            "mysql",
            "mariadb",
            "snowflake",
            "bigquery",
            "외부 db",
        ],
    );
    let connectorish = contains_any_text(p, &["커넥터", "connector"]);
    let actionish = contains_any_text(
        p,
        &[
            "연결",
            "붙여",
            "붙이고",
            "추가",
            "만들",
            "등록",
            "자격증명",
            "credential",
            "credentials",
            "connect",
            "add",
            "create",
            "update",
            "delete",
            "remove",
        ],
    );

    if (dbish || connectorish) && actionish {
        return true;
    }

    contains_any_text(
        p,
        &[
            "외부 데이터 소스",
            "external data source",
            "data connector",
            "database connector",
            "db connector",
        ],
    )
}

/// True when the prompt is about reading/describing AXHub governed data
/// resources rather than creating dynamic app tables or wiring databases into
/// local app code.
#[must_use]
pub fn data_intent_present(prompt: &str) -> bool {
    if dynamic_table_intent_present(prompt) || connectors_intent_present(prompt) {
        return false;
    }

    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    let objectish = contains_any_text(
        p,
        &[
            "데이터",
            "리소스",
            "resource",
            "테이블",
            "table",
            "orders",
            "analytics",
            "부서별",
            "월별",
            "매출",
            "인원",
            "sql",
            "snippet",
        ],
    );
    let readish = contains_any_text(
        p,
        &[
            "조회",
            "읽어",
            "읽는",
            "설명",
            "describe",
            "snippet",
            "스니펫",
            "sql로",
            "인사이트",
            "분석",
            "집계",
            "통계",
        ],
    );

    objectish && readish
}

/// True when the prompt is about organizing AXHub gateway resources.
///
/// This intentionally owns human cleanup phrases such as "리소스 정리하고 싶어"
/// in an AXHub project so Claude Desktop does not reinterpret them as local
/// filesystem cleanup. Pure inventory prompts stay with `my-resources`, and
/// data reads stay with `data_intent_present`.
#[must_use]
pub fn resources_intent_present(prompt: &str) -> bool {
    if dynamic_table_intent_present(prompt) || connectors_intent_present(prompt) {
        return false;
    }

    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if contains_any_text(
        p,
        &[
            "파일",
            "file",
            "디렉토리",
            "directory",
            "folder",
            ".shim",
            ".omc",
            "git ",
            "git status",
            "qa 결과",
        ],
    ) {
        return false;
    }

    let resourceish = contains_any_text(
        p,
        &[
            "리소스",
            "resource",
            "resources",
            "네임스페이스",
            "namespace",
            "tag resource",
            "resource tag",
        ],
    );
    let organizeish = contains_any_text(
        p,
        &[
            "정리",
            "이름 바",
            "이름바",
            "이름 변경",
            "rename",
            "이동",
            "move",
            "네임스페이스",
            "namespace",
            "태그",
            "tag",
            "삭제",
            "지우",
            "remove",
            "delete",
            "등록",
            "bulk register",
            "organize",
            "cleanup",
            "clean up",
        ],
    );

    resourceish && organizeish
}

/// True when the prompt asks about an AXHub app's GitHub repository connection.
///
/// This is deliberately about the AXHub app integration, not generic local git
/// operations. It keeps PR creation/review and plain `git status` outside the
/// hook hint so Claude Desktop can still answer ordinary repository questions
/// normally.
#[must_use]
pub fn github_connection_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if contains_any_text(
        p,
        &[
            "pull request",
            "github pr",
            "gh pr",
            " pr ",
            "pr 올",
            "pr 만들",
            "pr create",
            "git status",
        ],
    ) {
        return false;
    }

    let platformish = contains_any_text(
        p,
        &[
            "github",
            "깃허브",
            "깃헙",
            "git",
            "repo",
            "repository",
            "레포",
            "저장소",
        ],
    );
    let connectionish = contains_any_text(
        p,
        &[
            "연결",
            "붙",
            "끊",
            "connect",
            "disconnect",
            "linked",
            "repo 붙",
            "repo 연결",
            "저장소 연결",
        ],
    );
    let appish = contains_any_text(p, &["이 앱", "앱", "axhub", "배포", "서비스"]);

    platformish
        && connectionish
        && (appish
            || contains_any_text(
                p,
                &[
                    "github 연결",
                    "github connect",
                    "github disconnect",
                    "깃허브 연결",
                    "git 연결",
                    "repo 붙",
                    "repo 연결",
                    "저장소 연결",
                ],
            ))
}

/// True when the prompt asks whether an existing local/repo app can be brought
/// into AXHub. This stays narrower than plain deploy/create so "배포해줘" still
/// belongs to the deploy flow.
#[must_use]
pub fn migrate_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if deploy_create_intent_present(p)
        && !contains_any_text(p, &["기존", "이미 만든", "가져", "옮", "migrate", "import"])
    {
        return false;
    }

    let axhubish = contains_any_text(
        p,
        &[
            "axhub",
            "액스허브",
            "앱",
            "프로젝트",
            "app",
            "project",
            "service",
            "repo",
            "레포",
        ],
    );
    let migrateish = contains_any_text(
        p,
        &[
            "옮길",
            "옮겨",
            "가져올",
            "가져와",
            "가져오기",
            "기존 앱",
            "기존 프로젝트",
            "이미 만든 앱",
            "이미 만든 프로젝트",
            "migrate",
            "migration",
            "import existing",
            "import this",
            "bring this",
        ],
    );

    axhubish && migrateish
}

/// True when the prompt asks to submit an AXHub app for marketplace/public
/// review. This intentionally does not treat bare English "publish <app>" as
/// review submission because that phrase is still deploy-shaped in the corpus.
#[must_use]
pub fn publish_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if deploy_create_intent_present(p)
        && !contains_any_text(p, &["심사", "review", "마켓", "스토어"])
    {
        return false;
    }

    let appish = contains_any_text(
        p,
        &[
            "이 앱",
            "앱",
            "axhub",
            "액스허브",
            "서비스",
            "project",
            "app",
            "service",
        ],
    );
    let reviewish = contains_any_text(
        p,
        &[
            "공개 심사",
            "심사 넣",
            "심사 올",
            "심사 제출",
            "마켓에 올",
            "마켓플레이스",
            "스토어에 올",
            "공개 제출",
            "public review",
            "marketplace review",
            "submit for review",
            "for review",
            "app store review",
        ],
    );
    let make_public = contains_any_text(p, &["make public", "공개해", "공개로"]);
    let marketplaceish = contains_any_text(
        p,
        &[
            "마켓에 올",
            "마켓플레이스",
            "스토어에 올",
            "app store",
            "marketplace",
        ],
    );

    (reviewish && (appish || marketplaceish))
        || (appish
            && make_public
            && contains_any_text(p, &["마켓", "marketplace", "store", "심사"]))
}

/// True when the prompt asks for AXHub workspace team invitations or hosted-app
/// access sharing. This deliberately excludes Claude/OMC multi-agent "team"
/// wording so Desktop does not ask the user to choose between two internal team
/// concepts for ordinary phrases like "팀원 초대해".
#[must_use]
pub fn team_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if contains_any_text(
        p,
        &[
            "omc",
            "claude 팀",
            "클로드 팀",
            "에이전트",
            "agent team",
            "multi-agent",
            "멀티에이전트",
            "subagent",
            "worker",
            "작업 팀",
            "작업팀",
            "코드 작업 팀",
        ],
    ) {
        return false;
    }

    let inviteish = contains_any_text(
        p,
        &[
            "팀원 초대",
            "멤버 초대",
            "사람 추가",
            "협업자 추가",
            "팀에 추가",
            "초대 목록",
            "초대 리스트",
            "초대 취소",
            "초대 다시",
            "invite teammate",
            "team invite",
            "add teammate",
            "add member",
            "invitation list",
            "list invitations",
        ],
    );
    let app_accessish = contains_any_text(
        p,
        &[
            "앱 공유",
            "이 앱 공유",
            "접근 권한",
            "권한 줘",
            "권한 부여",
            "share app",
            "grant access",
            "app access",
            "access invite",
        ],
    );

    inviteish || app_accessish
}

/// Direct quality-review requests from Desktop should enter `axhub-review`
/// immediately. This detector deliberately excludes marketplace/public review
/// submission, because that is owned by `publish_intent_present`.
#[must_use]
pub fn quality_review_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if publish_intent_present(p) {
        return false;
    }

    contains_any_text(
        p,
        &[
            "이 코드 리뷰",
            "코드 리뷰",
            "코드 봐",
            "코드 검토",
            "pr 검토",
            "diff 봐",
            "diff 리뷰",
            "변경사항 리뷰",
            "변경 사항 리뷰",
            "review this diff",
            "code review",
            "review my code",
        ],
    )
}

/// Direct debugging requests that are about local code/test failures. Deploy
/// failure-cause prompts stay with the AXHub deploy trace/log surfaces.
#[must_use]
pub fn quality_debug_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if deploy_trace_intent_present(p)
        || deploy_logs_intent_present(p)
        || deploy_status_intent_present(p)
    {
        return false;
    }

    contains_any_text(
        p,
        &[
            "디버그",
            "왜 테스트",
            "테스트 깨",
            "테스트가 깨",
            "테스트 실패",
            "에러 원인",
            "오류 원인",
            "왜 안 돼",
            "왜 안돼",
            "why is this failing",
            "failed tests",
            "trace this regression",
        ],
    )
}

/// Direct auto-diagnose loop requests. Keep this separate from the lighter
/// doctor/install readiness check and from deploy trace summaries.
#[must_use]
pub fn quality_diagnose_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    contains_any_text(
        p,
        &[
            "loop 돌려",
            "루프 돌려",
            "원인 찾아",
            "원인 찾아줘",
            "재현 가능한 loop",
            "진단 루프",
            "auto diagnose",
            "auto-diagnose",
            "self-repair loop",
            "diagnose loop",
            "5-phase loop",
        ],
    )
}

/// Direct TDD-cycle requests.
#[must_use]
pub fn quality_tdd_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    contains_any_text(
        p,
        &[
            "tdd",
            "테스트 먼저",
            "테스트부터",
            "red green",
            "red-green",
            "실패 테스트 먼저",
            "failing test first",
            "write the failing test",
        ],
    )
}

/// Direct planning requests for code/design changes. This intentionally does
/// not capture first-use/setup guidance or AXHub app migration readiness.
#[must_use]
pub fn quality_plan_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if migrate_intent_present(p) || clarify_intent_present(p) {
        return false;
    }

    contains_any_text(
        p,
        &[
            "플랜 짜",
            "플랜 세",
            "계획 세",
            "변경 계획",
            "큰 구조 변경",
            "아키텍처 변경",
            "architecture change",
            "impact analysis",
            "staged execution",
        ],
    )
}

/// Direct PR/release readiness requests. Bare "ship paydrop" remains deploy
/// intent; this only owns readiness/preparation language.
#[must_use]
pub fn quality_ship_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if publish_intent_present(p) {
        return false;
    }

    contains_any_text(
        p,
        &[
            "pr 만들",
            "pr 준비",
            "pull request 준비",
            "릴리즈 준비",
            "릴리즈 체크",
            "배포 준비",
            "출시 준비",
            "push gate",
            "ship readiness",
            "release readiness",
            "ship review",
        ],
    )
}

/// Narrow deploy/status progress detector for short Korean prompts like
/// "어디까지 됐어" that native skill matching can otherwise treat as generic
/// session progress. These phrases are already owned by `skills/status`; the
/// hook hint only prevents memory-style answers in the Claude CLI E2E path.
#[must_use]
pub fn deploy_restore_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();

    if contains_any_text(
        p,
        &[
            "git ",
            "git reset",
            "git revert",
            "깃",
            "커밋 되돌",
            "commit revert",
            "revert commit",
            "파일 되돌",
            "코드 되돌",
            "설정 되돌",
            "업데이트 되돌",
            "상태바 되돌",
        ],
    ) && !contains_any_text(p, &["배포", "deploy", "deployment"])
    {
        return false;
    }

    let deployish = contains_any_text(
        p,
        &[
            "배포",
            "deploy",
            "deployment",
            "라이브",
            "프로덕션",
            "production",
            "서비스",
        ],
    );
    let restoreish = contains_any_text(
        p,
        &[
            "되돌",
            "롤백",
            "복구",
            "이전 버전",
            "직전 버전",
            "안정 버전",
            "잘 되던 버전",
            "restore",
            "revert",
            "rollback",
            "roll back",
            "undo",
        ],
    );

    if deployish && restoreish {
        return true;
    }

    contains_any_text(
        p,
        &[
            "방금 거 되돌",
            "방금 것 되돌",
            "직전 안정",
            "마지막 정상",
            "잘 되던 버전",
            "restore previous",
            "redeploy previous",
            "undo deploy",
            "rollback 해",
            "rollback 부탁",
        ],
    )
}

#[must_use]
pub fn deploy_status_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    contains_any_text(
        p,
        &[
            "어디까지",
            "어디쯤",
            "진행 상황",
            "어떻게 됐",
            "다 됐",
            "끝났",
            "배포 상태",
            "deploy status",
            "deploy state",
            "is it done",
        ],
    )
}

/// Narrow deploy-log detector for Desktop prompts like "로그 좀 보여줘".
/// In axhub project context, bare log requests should mean the app/deployment
/// logs, not local `.omc`/repo log files. Exclude auth phrases first because
/// Korean "로그인" contains "로그".
#[must_use]
pub fn deploy_logs_intent_present(prompt: &str) -> bool {
    if auth_status_intent_present(prompt) || deploy_trace_intent_present(prompt) {
        return false;
    }
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    contains_any_text(
        p,
        &[
            "로그",
            "빌드 로그",
            "런타임 로그",
            "콘솔",
            "출력",
            "error log",
            "logs",
            "tail logs",
            "runtime log",
            "console log",
            "build output",
        ],
    )
}

/// Narrow failure-cause detector for Desktop prompts like "배포 실패 원인 알려줘".
/// This owns cause/diagnosis questions. Log viewing remains separate so ordinary
/// "로그 좀 보여줘" requests do not become failure analysis.
#[must_use]
pub fn deploy_trace_intent_present(prompt: &str) -> bool {
    if auth_status_intent_present(prompt) {
        return false;
    }
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if contains_any_text(
        p,
        &[
            "테스트",
            "test",
            "tests",
            "spec",
            "unit",
            "코드",
            "repo",
            "레포",
        ],
    ) {
        return false;
    }
    contains_any_text(
        p,
        &[
            "배포 실패 원인",
            "실패 원인",
            "원인 알려",
            "왜 실패",
            "왜 안돼",
            "왜 안 돼",
            "왜 깨졌",
            "왜 죽었",
            "왜 멈췄",
            "디버그",
            "추적해",
            "분석해",
            "what went wrong",
            "why failed",
            "debug deploy",
            "diagnose deploy",
            "trace deploy",
        ],
    )
}

/// Narrow app lifecycle detector for Desktop prompts like
/// "testnextjs 앱 잠깐 멈춰줘" and the human-shortened
/// "testnextjs 다시 켜줘".
///
/// It is intentionally anchored on either an app noun or an app-name-like token
/// plus a lifecycle verb so local process requests ("프로세스 멈춰줘") and
/// failure-cause questions ("왜 멈췄어?") do not get pulled into AXHub app
/// mutation.
#[must_use]
pub fn app_lifecycle_intent_present(prompt: &str) -> bool {
    if deploy_trace_intent_present(prompt)
        || deploy_status_intent_present(prompt)
        || deploy_logs_intent_present(prompt)
    {
        return false;
    }

    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if !app_lifecycle_verb_present(p) {
        return false;
    }

    if contains_any_text(p, &["앱", "app"]) {
        return true;
    }

    !local_process_context_present(p) && app_nameish_token_present(p)
}

fn app_lifecycle_verb_present(prompt: &str) -> bool {
    contains_any_text(
        prompt,
        &[
            "잠깐 멈",
            "잠시 멈",
            "멈춰",
            "멈춰줘",
            "멈춰 줘",
            "일시정지",
            "일시 정지",
            "중지",
            "정지",
            "내려줘",
            "내려 줘",
            "꺼줘",
            "꺼 줘",
            "다시 켜",
            "다시켜",
            "다시 올려",
            "켜줘",
            "켜 줘",
            "재개",
            "살려줘",
            "살려 줘",
            "복제",
            "포크",
            "복사해",
            "pause",
            "stop",
            "suspend",
            "resume",
            "start",
            "fork",
            "clone",
            "copy",
            "pause app",
            "stop app",
            "suspend app",
            "resume app",
            "start app",
            "fork app",
            "clone app",
            "copy app",
        ],
    )
}

fn local_process_context_present(prompt: &str) -> bool {
    contains_any_text(
        prompt,
        &[
            "서버",
            "server",
            "dev server",
            "local server",
            "localhost",
            "127.0.0.1",
            "포트",
            "port",
            "프로세스",
            "process",
            "pid",
            "lsof",
            "ps ",
            "server.js",
            "npm",
            "bun",
            "vite",
            "next dev",
        ],
    )
}

fn app_nameish_token_present(prompt: &str) -> bool {
    prompt.split_whitespace().any(|token| {
        let token = token
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .to_ascii_lowercase();
        if token.len() < 4
            || !token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || !token.chars().any(|c| c.is_ascii_alphabetic())
        {
            return false;
        }

        if APP_NAMEISH_DENY_TOKENS.contains(&token.as_str()) {
            return false;
        }

        token.len() >= 6
            || token.chars().any(|c| c.is_ascii_digit())
            || token.contains('-')
            || token.contains('_')
    })
}

const APP_NAMEISH_DENY_TOKENS: &[&str] = &[
    "app",
    "apps",
    "axhub",
    "api",
    "backend",
    "deploy",
    "deployment",
    "dev",
    "frontend",
    "local",
    "localhost",
    "nextjs",
    "node",
    "npm",
    "prod",
    "production",
    "react",
    "server",
    "staging",
    "vite",
    "web",
];

/// Narrow browser/open detector for Desktop prompts like "라이브 페이지 열어봐".
/// Avoid bare English "open" so GitHub/PR prompts such as "open a pull request"
/// remain outside axhub.
#[must_use]
pub fn open_app_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if contains_any_text(p, &["pull request", " pr ", "github pr"]) {
        return false;
    }
    contains_any_text(
        p,
        &[
            "결과 봐",
            "라이브 봐",
            "라이브 페이지",
            "브라우저로 열",
            "브라우저에서 열",
            "프로덕션 열",
            "deploy url",
            "app url",
            "open in browser",
        ],
    )
}

/// Narrow deploy verification detector for Desktop prompts like
/// "방금 배포 진짜 열리는지 확인해줘".
///
/// Keep this separate from status/open/logs: verify means evidence-based live
/// verdict, not progress polling, browser opening, or log viewing.
#[must_use]
pub fn deploy_verify_intent_present(prompt: &str) -> bool {
    if deploy_logs_intent_present(prompt)
        || open_app_intent_present(prompt)
        || deploy_status_intent_present(prompt)
        || inspect_config_intent_present(prompt)
    {
        return false;
    }
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    contains_any_text(
        p,
        &[
            "방금 배포 진짜",
            "방금 배포 확인",
            "방금 거 확인",
            "진짜 열리는지",
            "진짜 됐",
            "진짜 올라",
            "라이브 됐",
            "정말 됐",
            "확실해",
            "배포 검증",
            "배포 테스트",
            "smoke test",
            "is it live",
            "check live",
            "verify deploy",
        ],
    )
}

/// Narrow routing analytics detector for Desktop prompts like
/// "이번 주 axhub 라우팅 어땠어?".
#[must_use]
pub fn routing_stats_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    contains_any_text(
        p,
        &[
            "라우팅 통계",
            "라우팅 어땠",
            "라우팅 분석",
            "매칭 통계",
            "지난주 매칭",
            "이번 주 routing",
            "이번주 routing",
            "axhub routing",
            "routing stats",
            "routing analytics",
            "usage analytics",
            "audit summary",
        ],
    )
}

/// Narrow read-only env detector for Desktop prompts like "환경변수 뭐 있어?".
///
/// Mutation phrases must stay on the full env skill path because set/delete need
/// consent and stdin-only value handling.
#[must_use]
pub fn env_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if contains_any_text(
        p,
        &[
            "추가", "등록", "넣어", "넣고", "설정", "바꿔", "수정", "삭제", "지워", "없애", "set ",
            " set", "delete", "unset", "remove", "update",
        ],
    ) {
        return false;
    }

    contains_any_text(
        p,
        &[
            "환경변수",
            "환경 변수",
            "env 봐",
            "env 보여",
            "env 확인",
            "env list",
            "env var",
            "environment variable",
            "environment variables",
            "secrets list",
        ],
    )
}

/// Narrow deploy/create detector for bare Korean prompts like "배포해" that can
/// otherwise be interpreted as repo release work in non-interactive Claude E2E.
#[must_use]
pub fn deploy_create_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if deploy_status_intent_present(p) || deploy_restore_intent_present(p) {
        return false;
    }
    contains_any_text(
        p,
        &[
            "배포해",
            "배포 해",
            "배포하",
            "배포 진행",
            "deploy",
            "deploy this",
        ],
    )
}

/// Narrow app-creation detector for Desktop prompts like "새 앱 만들어줘".
/// These should enter `skills/init` and ask for template/name/consent, not answer
/// with internal labels such as `axhub:init` or generic project advice.
#[must_use]
pub fn init_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if browse_template_intent_present(p) || apps_intent_present(p) {
        return false;
    }
    contains_any_text(
        p,
        &[
            "새 앱",
            "앱 만들어",
            "앱 만들",
            "프로젝트 만들어",
            "프로젝트 초기화",
            "초기화해",
            "결제 앱",
            "next.js 앱",
            "nextjs 앱",
            "fastapi 앱",
            "scaffold",
        ],
    )
}

/// Narrow app-list/management detector for "내 앱 목록 보여줘" style prompts.
/// It intentionally excludes marketplace/template discovery and new app creation.
#[must_use]
pub fn apps_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if browse_template_intent_present(p) || contains_any_text(p, &["새 앱", "앱 만들어", "앱 만들"])
    {
        return false;
    }
    contains_any_text(
        p,
        &[
            "내 앱",
            "우리 앱",
            "제 앱",
            "앱 목록",
            "앱 리스트",
            "앱 보여",
            "앱 봐",
            "앱 뭐",
            "등록된 앱",
            "운영 중인 앱",
            "my apps",
            "list apps",
            "app list",
            "which apps",
        ],
    )
}

/// Narrow marketplace/template browsing detector. This owns "템플릿 뭐 있어?"
/// and must not be confused with `apps` (my apps) or `init` (create an app).
#[must_use]
pub fn browse_template_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    let templateish = contains_any_text(p, &["템플릿", "template", "marketplace", "마켓"]);
    templateish
        && contains_any_text(
            p,
            &[
                "뭐", "목록", "보여", "있어", "둘러", "검색", "list", "show", "browse", "search",
            ],
        )
}

/// Narrow clarify/help detector for broad Desktop prompts like "axhub 좀 도와줘".
///
/// This is deliberately smaller than setup/onboarding matching: first-use prompts
/// such as "axhub 처음 쓰는데 뭐부터 하면 돼?" must stay with setup, while broad
/// "help me choose what to do" prompts should get a clean question card without
/// internal route labels.
#[must_use]
pub fn clarify_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str().trim();
    if p == "axhub" {
        return true;
    }
    contains_any_text(
        p,
        &[
            "axhub 좀 도와",
            "axhub 도와",
            "도와줘 axhub",
            "axhub 뭐 도와",
            "axhub 관련해서 도와",
            "axhub로 뭐 해야",
            "help me with axhub",
            "do something with axhub",
            "axhub thing",
        ],
    )
}

/// Narrow doctor/preflight detector for bare health-check prompts like
/// "환경 점검해" that should route to `skills/doctor`, not generic repo checks.
#[must_use]
pub fn doctor_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    contains_any_text(
        p,
        &[
            "환경 점검",
            "환경 체크",
            "설치 상태",
            "설치상태",
            "설치 확인",
            "설치 점검",
            "설치돼",
            "깔렸",
            "cli 상태",
            "cli 설치 상태",
            "진단해",
            "진단 해",
            "doctor",
            "health check",
            "setup check",
            "sanity check",
        ],
    )
}

/// Narrow CLI installer detector for Desktop prompts like "axhub CLI 설치해줘".
///
/// This is intentionally separate from `doctor_intent_present`: install requests
/// should first check whether the CLI is already installed, but if it is missing
/// they may proceed to an installer preview after explicit approval.
#[must_use]
pub fn install_cli_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if contains_any_text(
        p,
        &[
            "설치 상태",
            "설치상태",
            "설치 확인",
            "설치 점검",
            "설치돼",
            "설치 되어",
            "설치되어",
            "잘 깔",
            "깔렸",
            "install status",
            "verify install",
        ],
    ) {
        return false;
    }
    contains_any_text(
        p,
        &[
            "axhub cli 설치해",
            "axhub cli 설치 해",
            "axhub cli 깔아",
            "axhub 설치해",
            "axhub 설치 해",
            "axhub 깔아",
            "cli 설치해",
            "cli 설치 해",
            "cli 깔아",
            "ax-hub-cli 설치",
            "install axhub",
            "axhub install",
            "install cli",
        ],
    )
}

/// Narrow CLI update-check detector for Desktop prompts like "업데이트 필요한지 봐줘".
///
/// Plugin self-update requests must stay with `upgrade`, and install/readiness
/// checks stay with `doctor`/`install-cli`.
#[must_use]
pub fn update_check_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    if contains_any_text(
        p,
        &[
            "플러그인",
            "plugin",
            "claude",
            "클로드",
            "설치 상태",
            "설치상태",
            "설치 확인",
            "설치 점검",
            "설치해",
            "install",
        ],
    ) {
        return false;
    }
    contains_any_text(
        p,
        &[
            "업데이트 필요한지",
            "업데이트 필요",
            "업데이트 있어",
            "업데이트 확인",
            "새 버전",
            "버전 확인",
            "최신인지",
            "최신이야",
            "최신 버전",
            "update check",
            "update available",
            "check version",
            "latest version",
            "new release",
        ],
    )
}

/// Narrow status bar detector for Desktop prompts like "상태바 켜줘".
///
/// This covers the common Korean phrasing that semantic skill matching can route
/// to enable-statusline, then adds a stricter one-helper Desktop contract so the
/// visible answer does not leak raw settings merge internals.
#[must_use]
pub fn statusline_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    contains_any_text(
        p,
        &[
            "상태바",
            "상태 표시줄",
            "상태표시줄",
            "상태줄",
            "statusline",
            "status line",
        ],
    ) && contains_any_text(
        p,
        &[
            "켜", "켜줘", "활성", "보여", "붙여", "설정", "enable", "activate", "show",
        ],
    )
}

/// Narrow API catalog detector for prompts that ask what axhub app APIs or
/// endpoints are available. This avoids generic repository API inspection.
#[must_use]
pub fn apis_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    let apiish = contains_any_text(p, &["api", "apis", "endpoint", "엔드포인트"]);
    apiish
        && contains_any_text(
            p,
            &[
                "뭐",
                "목록",
                "카탈로그",
                "쓸 수",
                "사용 가능",
                "보여",
                "list",
                "available",
                "catalog",
            ],
        )
}

/// Narrow manifest/config inspection detector for Desktop prompts like
/// "매니페스트랑 설정 괜찮은지 봐줘". This should route to `skills/inspect`, not
/// generic repo file reading, because the plugin owns a read-only manifest/config
/// check workflow with redaction and current CLI context.
#[must_use]
pub fn inspect_config_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    let configish = contains_any_text(
        p,
        &[
            "매니페스트",
            "manifest",
            "axhub.yaml",
            "설정",
            "config",
            "configuration",
        ],
    );
    configish
        && contains_any_text(
            p,
            &[
                "봐", "확인", "검증", "괜찮", "점검", "상태", "check", "validate", "review",
            ],
        )
}

/// Narrow auth identity/status detector for prompts like "로그인 돼 있어?" or
/// "토큰 살아있어?" that ask about login/token state. Without this hint the model
/// reads these status-shaped questions as a diagnosis request and routes to the
/// heavier `skills/doctor` (full env card) instead of `skills/auth` (focused
/// identity card). Every phrase is anchored on an auth noun (로그인/토큰/인증/누구/
/// whoami) so deploy-status ("배포 상태"), doctor ("설치돼 있어?"), and apps prompts
/// are never stolen. Login/logout *actions* keep routing via the auth SKILL
/// description; this only fills the status-query gap.
#[must_use]
pub fn auth_status_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let p = lower.as_str();
    contains_any_text(
        p,
        &[
            "로그인 돼",
            "로그인 됐",
            "로그인 되어",
            "로그인했",
            "로그인 상태",
            "로그인 유지",
            "로그인 살아",
            "로그인 만료",
            "로그인 필요",
            "로그인 다시 해야",
            "로그인 다시 해야 해",
            "로그인 다시 해야 돼",
            "다시 로그인해야",
            "재로그인 필요",
            "재로그인 해야",
            "로그인 안 돼",
            "로그인 안돼",
            "토큰 살아",
            "토큰 만료",
            "토큰 유효",
            "토큰 상태",
            "인증 상태",
            "인증 됐",
            "인증 돼",
            "누구로 로그인",
            "누구로 접속",
            "어떤 계정",
            "whoami",
            "who am i",
            "logged in",
            "auth status",
        ],
    )
}

fn contains_any_text(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

/// True when `prompt` is an explicit slash invocation of an axhub command
/// (`/deploy`, `/배포` — the Korean alias in `commands/배포.md`, `/axhub:…`). The
/// deploy preflight detects this from its own invocation context; the hook detects
/// it from the prompt text — both feed the result into [`decide`] as
/// `explicit_invocation`. `/배포` is included because it is a first-class slash
/// command (Korean-first plugin), and the command forwards only `$ARGUMENTS`, so
/// this detector is the fallback when the leading token survives in the text.
#[must_use]
pub fn is_slash_invocation(prompt: &str) -> bool {
    // Match the **exact** leading command token, not a bare prefix: `/deployment`
    // and `/배포해` are different tokens and must not be mistaken for the `/deploy`
    // / `/배포` commands (over-detection would route unrelated slash text to axhub
    // via priority rule 0). The `/axhub:` namespace stays a prefix — any
    // `/axhub:<cmd>` is an explicit axhub invocation.
    let first = prompt.split_whitespace().next().unwrap_or("");
    first == "/deploy" || first == "/배포" || first.starts_with("/axhub:")
}

/// Whole-word substring search. `keyword` is assumed lowercase ASCII; `haystack`
/// must already be lowercased by the caller. A hit must be bounded on both sides
/// by a non-ASCII-alphanumeric byte (or a string edge), so foreign keywords like
/// `"fly"`/`"render"` never fire inside `"butterfly"`/`"rendered"`. Non-ASCII
/// bytes (e.g. Korean) count as boundaries, so `"vercel로"` still matches.
pub(crate) fn contains_word(haystack: &str, keyword: &str) -> bool {
    let kw = keyword.as_bytes();
    let hay = haystack.as_bytes();
    if kw.is_empty() || hay.len() < kw.len() {
        return false;
    }
    let mut i = 0;
    while i + kw.len() <= hay.len() {
        if &hay[i..i + kw.len()] == kw {
            let before_ok = i == 0 || !hay[i - 1].is_ascii_alphanumeric();
            let after = i + kw.len();
            let after_ok = after == hay.len() || !hay[after].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Walk up from `start` looking for `axhub.yaml`, stopping at the first `.git`
/// directory (the git root) and falling back to the filesystem root for
/// non-git trees (spec 006 §23-30). Local fs checks only — never any network.
///
/// Returns [`MarkerStatus::Unknown`] on a filesystem error so the caller can
/// fall open auth-conditionally rather than mistaking an error for `Absent`.
#[must_use]
pub fn find_marker_from(start: &Path) -> MarkerStatus {
    for dir in start.ancestors().take(MAX_WALK_UP_DEPTH) {
        match dir.join("axhub.yaml").try_exists() {
            Ok(true) => return MarkerStatus::Present,
            Ok(false) => {}
            Err(_) => return MarkerStatus::Unknown,
        }
        // Stop the walk-up at the git root: check it AFTER axhub.yaml so a
        // marker living at the git root is still found.
        match dir.join(".git").try_exists() {
            Ok(true) => return MarkerStatus::Absent,
            Ok(false) => {}
            Err(_) => return MarkerStatus::Unknown,
        }
    }
    MarkerStatus::Absent
}

/// [`find_marker_from`] anchored at the current working directory. Returns
/// [`MarkerStatus::Unknown`] if the cwd itself cannot be read.
#[must_use]
pub fn find_marker() -> MarkerStatus {
    match std::env::current_dir() {
        Ok(cwd) => find_marker_from(&cwd),
        Err(_) => MarkerStatus::Unknown,
    }
}

/// Cheap "is the user axhub-authed?" probe: a `.exists()` stat on the helper
/// auth/delegation token-file (`~/.config/axhub-plugin/token`, spec 006 §102).
///
/// MUST stay a pure stat — never spawn `axhub auth status` and never trigger
/// token-init bootstrap, or we create an "auth-read → bootstrap → marker-gate"
/// cycle. Token presence is a *proxy* for authed (a CLI-authed user with no
/// helper token reads as not-authed → pass-through, an accepted under-detection
/// on the error path). NOTE: this is the auth token, distinct from
/// `consent::…` HMAC consent tokens — do not conflate.
#[must_use]
pub fn token_present() -> bool {
    crate::runtime_paths::token_file()
        .map(|path| path.try_exists().unwrap_or(false))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Independent reference implementation of the spec 006 priority table,
    /// written deliberately differently from [`decide_from_flags`] (explicit
    /// nested matches instead of an early-return chain). The exhaustive matrix
    /// test below asserts the production chain agrees with this reference for
    /// *every* input combination — that agreement is the no-drift lock.
    fn reference_decision(
        axhub: bool,
        foreign: bool,
        marker: MarkerStatus,
        authed: bool,
        explicit: bool,
    ) -> RoutingDecision {
        if explicit {
            RoutingDecision::Axhub
        } else if axhub && foreign {
            RoutingDecision::Ask
        } else if axhub {
            RoutingDecision::Axhub
        } else if foreign {
            RoutingDecision::Yield
        } else {
            match (marker, authed) {
                (MarkerStatus::Present, _) => RoutingDecision::Axhub,
                (MarkerStatus::Absent, _) => RoutingDecision::Ignore,
                (MarkerStatus::Unknown, true) => RoutingDecision::Axhub,
                (MarkerStatus::Unknown, false) => RoutingDecision::Ignore,
            }
        }
    }

    /// Exhaustive 2×2×3×2×2 = 48-combo matrix. Locks the full priority ordering
    /// — every collision (0 > all, a > b/c, c > d) is exercised because every
    /// combination is enumerated. If the production chain ever reorders, this
    /// diverges from the reference and fails.
    #[test]
    fn decide_from_flags_matches_reference_for_all_inputs() {
        let markers = [
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ];
        let mut count = 0;
        for &axhub in &[false, true] {
            for &foreign in &[false, true] {
                for &marker in &markers {
                    for &authed in &[false, true] {
                        for &explicit in &[false, true] {
                            let got = decide_from_flags(axhub, foreign, marker, authed, explicit);
                            let want = reference_decision(axhub, foreign, marker, authed, explicit);
                            assert_eq!(
                                got, want,
                                "drift at axhub={axhub} foreign={foreign} marker={marker:?} authed={authed} explicit={explicit}"
                            );
                            count += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(count, 48, "matrix must enumerate every input combination");
    }

    /// rule 0 — slash invocation beats every keyword/marker/auth combination.
    #[test]
    fn rule0_slash_invocation_always_wins() {
        let markers = [
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ];
        for &axhub in &[false, true] {
            for &foreign in &[false, true] {
                for &marker in &markers {
                    for &authed in &[false, true] {
                        assert_eq!(
                            decide_from_flags(axhub, foreign, marker, authed, true),
                            RoutingDecision::Axhub,
                            "slash must win at axhub={axhub} foreign={foreign} marker={marker:?} authed={authed}"
                        );
                    }
                }
            }
        }
    }

    /// THE drift case AC-16 exists to catch (spec §59): a foreign target named
    /// in a marker-present repo must `Yield` (rule c) — never route to `Axhub`
    /// off the marker (rule d). Asserted across marker/auth so the precedence,
    /// not an incidental input, is what holds.
    #[test]
    fn rulec_foreign_keyword_beats_marker_present() {
        for &marker in &[
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ] {
            for &authed in &[false, true] {
                assert_eq!(
                    decide_from_flags(false, true, marker, authed, false),
                    RoutingDecision::Yield,
                    "named-target-wins must hold at marker={marker:?} authed={authed}"
                );
            }
        }
    }

    /// rule a — axhub + foreign both named (no slash) → Ask, regardless of marker/auth.
    #[test]
    fn rulea_axhub_plus_foreign_asks() {
        for &marker in &[
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ] {
            for &authed in &[false, true] {
                assert_eq!(
                    decide_from_flags(true, true, marker, authed, false),
                    RoutingDecision::Ask
                );
            }
        }
    }

    /// rule b — "axhub" keyword alone is marker-independent → Axhub.
    #[test]
    fn ruleb_axhub_keyword_is_marker_independent() {
        for &marker in &[
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ] {
            assert_eq!(
                decide_from_flags(true, false, marker, false, false),
                RoutingDecision::Axhub
            );
        }
    }

    /// rules d / e — bare NL routed purely by marker presence.
    #[test]
    fn ruled_rulee_bare_nl_follows_marker() {
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Present, false, false),
            RoutingDecision::Axhub
        );
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Absent, true, false),
            RoutingDecision::Ignore
        );
    }

    /// err — bare NL + Unknown marker falls open auth-conditionally (spec §99).
    #[test]
    fn err_branch_is_auth_conditional() {
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Unknown, true, false),
            RoutingDecision::Axhub
        );
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Unknown, false, false),
            RoutingDecision::Ignore
        );
    }

    /// The public [`decide`] wrapper must agree with [`decide_from_flags`] fed
    /// the shared detectors' output — i.e. prompt-derived keyword flags don't
    /// drift from the core chain. This is the input-construction half of no-drift.
    #[test]
    fn decide_wrapper_agrees_with_core_over_detectors() {
        let prompts = [
            "배포해",
            "axhub 으로 배포해",
            "vercel 로 배포해",
            "axhub 말고 vercel 로",
            "deploy this to render",
            "그냥 빌드만",
        ];
        for prompt in prompts {
            for &marker in &[
                MarkerStatus::Present,
                MarkerStatus::Absent,
                MarkerStatus::Unknown,
            ] {
                for &authed in &[false, true] {
                    for &explicit in &[false, true] {
                        let via_wrapper = decide(prompt, marker, authed, explicit);
                        let via_core = decide_from_flags(
                            axhub_keyword_present(prompt),
                            foreign_keyword_present(prompt),
                            marker,
                            authed,
                            explicit,
                        );
                        assert_eq!(
                            via_wrapper, via_core,
                            "wrapper/core drift on prompt {prompt:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn axhub_keyword_detection() {
        assert!(axhub_keyword_present("axhub 배포"));
        assert!(axhub_keyword_present("deploy with AXHUB now"));
        assert!(axhub_keyword_present("read axhub.yaml")); // bounded by '.'
        assert!(!axhub_keyword_present("배포해주세요"));
        assert!(!axhub_keyword_present("axhubble")); // no false substring hit
    }

    #[test]
    fn foreign_keyword_detection_is_whole_word() {
        assert!(foreign_keyword_present("vercel 로 올려줘"));
        assert!(foreign_keyword_present("push to render"));
        assert!(foreign_keyword_present("deploy on fly.io")); // bounded by '.'
        assert!(foreign_keyword_present("use Netlify"));
        // No false positives from substrings:
        assert!(!foreign_keyword_present("a butterfly landed")); // contains "fly"
        assert!(!foreign_keyword_present("it rendered fine")); // contains "render"
        assert!(!foreign_keyword_present("just a normal prompt"));
    }

    #[test]
    fn slash_invocation_detection() {
        assert!(is_slash_invocation("/deploy"));
        assert!(is_slash_invocation("  /deploy to prod"));
        assert!(is_slash_invocation("/axhub:deploy"));
        assert!(is_slash_invocation("/axhub:apps"));
        assert!(is_slash_invocation("/배포")); // Korean alias (commands/배포.md)
        assert!(is_slash_invocation("/배포 paydrop"));
        assert!(!is_slash_invocation("deploy"));
        assert!(!is_slash_invocation("please /deploy"));
        assert!(!is_slash_invocation("배포해")); // bare Korean NL is NOT a slash
                                                 // Bounded to the exact command token: a prompt that merely *starts with*
                                                 // "/deploy"/"/배포" but is a different token must NOT be treated as the
                                                 // command (over-detection would route unrelated slash text to axhub via
                                                 // rule 0).
        assert!(!is_slash_invocation("/deployment-plan 설명해줘"));
        assert!(!is_slash_invocation("/deploy-history"));
        assert!(!is_slash_invocation("/배포해")); // "/배포" + 해 = a different token
    }

    #[test]
    fn dynamic_table_intent_detection_is_narrow() {
        for prompt in [
            "ultraqa-app 앱에 orders 동적 테이블 만들고 title:text 컬럼 추가해",
            "앱 테이블 스키마 변경하고 preview 보여줘",
            "orders 테이블 컬럼 추가해",
            "insert row into orders",
            "orders table grant issue",
        ] {
            assert!(
                dynamic_table_intent_present(prompt),
                "expected dynamic table intent for {prompt:?}"
            );
        }

        for prompt in [
            "describe snowflake analytics orders table",
            "이 테이블 읽는 python snippet 만들어줘",
            "테이블 설명해줘",
            "orders 데이터 조회해줘",
        ] {
            assert!(
                !dynamic_table_intent_present(prompt),
                "catalog/data prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn connectors_intent_detection_covers_database_connection_without_stealing_local_app_work() {
        for prompt in [
            "Postgres 데이터베이스 연결하고 싶어",
            "DB 연결해",
            "외부 DB 붙여줘",
            "커넥터 추가하고 싶어",
            "db credentials update",
            "connect database",
        ] {
            assert!(
                connectors_intent_present(prompt),
                "expected connector intent for {prompt:?}"
            );
        }

        for prompt in [
            "server.js에 pg 패키지 붙여줘",
            "DATABASE_URL 환경변수 뭐 있어?",
            "orders 데이터 조회해줘",
            "테이블 만들고 싶어",
            "API 키 등록하고 싶어",
            "로그 좀 보여줘",
        ] {
            assert!(
                !connectors_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn data_intent_detection_covers_data_reads_without_stealing_neighbor_flows() {
        for prompt in [
            "orders 데이터 조회해줘",
            "테이블 설명해줘",
            "이 테이블 읽는 python snippet 만들어줘",
            "describe snowflake analytics orders table",
            "부서별 인원 집계해줘",
        ] {
            assert!(
                data_intent_present(prompt),
                "expected data intent for {prompt:?}"
            );
        }

        for prompt in [
            "Postgres 데이터베이스 연결하고 싶어",
            "DB 연결해",
            "DATABASE_URL 환경변수 뭐 있어?",
            "orders 동적 테이블 만들고 title:text 컬럼 추가해",
            "내 리소스 보여줘",
            "쓸 수 있는 API 뭐 있어?",
        ] {
            assert!(
                !data_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn resources_intent_detection_covers_gateway_organization_without_file_cleanup() {
        for prompt in [
            "리소스 정리하고 싶어",
            "리소스 이름 바꿔",
            "리소스 이동해줘",
            "리소스 태그 정리해줘",
            "resource cleanup",
            "rename resource",
            "bulk register resources",
        ] {
            assert!(
                resources_intent_present(prompt),
                "expected resources intent for {prompt:?}"
            );
        }

        for prompt in [
            "orders 데이터 조회해줘",
            "내 리소스 보여줘",
            "Postgres 데이터베이스 연결하고 싶어",
            "orders 테이블 만들고 title 컬럼도 넣어줘",
            "QA 결과 파일 정리하고 싶어",
            ".shim 로그 정리해줘",
            "git status 보고 미추적 파일 정리해",
        ] {
            assert!(
                !resources_intent_present(prompt),
                "neighbor/local-cleanup prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn github_connection_intent_detection_covers_app_linking_without_stealing_git_or_pr() {
        for prompt in [
            "이 앱 깃허브랑 연결돼 있어?",
            "GitHub 연결 상태 봐줘",
            "내 repo 붙여",
            "repo 연결해줘",
            "저장소 연결 끊어줘",
            "github disconnect",
        ] {
            assert!(
                github_connection_intent_present(prompt),
                "expected github connection intent for {prompt:?}"
            );
        }

        for prompt in [
            "PR 올려",
            "gh pr create 해줘",
            "open a pull request",
            "git status please",
            "로컬 git status 확인해줘",
            "리소스 정리하고 싶어",
        ] {
            assert!(
                !github_connection_intent_present(prompt),
                "neighbor git/pr prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn migrate_intent_detection_covers_import_readiness_without_stealing_deploy() {
        for prompt in [
            "이 프로젝트 axhub로 옮길 수 있어?",
            "기존 앱 가져와",
            "이미 만든 프로젝트 가져올 수 있어?",
            "migrate this repo",
            "import existing app",
            "bring this app into axhub",
        ] {
            assert!(
                migrate_intent_present(prompt),
                "expected migrate/import readiness intent for {prompt:?}"
            );
        }

        for prompt in [
            "배포해줘",
            "지금 진행 중인 배포 어디까지 됐어?",
            "이 앱 깃허브랑 연결돼 있어?",
            "리소스 정리하고 싶어",
            "새 앱 만들어줘",
            "프로덕션으로 올려줘",
        ] {
            assert!(
                !migrate_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn publish_intent_detection_covers_review_submission_without_stealing_deploy() {
        for prompt in [
            "이 앱 공개 심사 넣고 싶어",
            "앱 공개 심사 올려줘",
            "마켓에 올려",
            "스토어에 올려줘",
            "submit this app for review",
            "app store review submission",
        ] {
            assert!(
                publish_intent_present(prompt),
                "expected publish/review intent for {prompt:?}"
            );
        }

        for prompt in [
            "publish paydrop",
            "배포해줘",
            "라이브 페이지 공개해줘",
            "이 프로젝트 axhub로 옮길 수 있어?",
            "이 앱 깃허브랑 연결돼 있어?",
            "리소스 정리하고 싶어",
        ] {
            assert!(
                !publish_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn team_intent_detection_covers_axhub_invites_without_stealing_agent_team() {
        for prompt in [
            "팀원 초대해",
            "멤버 초대하고 싶어",
            "초대 목록 봐",
            "협업자 추가해줘",
            "이 앱 공유해",
            "접근 권한 줘",
            "invite teammate",
            "share app",
        ] {
            assert!(
                team_intent_present(prompt),
                "expected AXHub team/access intent for {prompt:?}"
            );
        }

        for prompt in [
            "Claude 에이전트 팀 띄워줘",
            "OMC 멀티에이전트 작업 팀 만들어줘",
            "코드 작업팀 구성해줘",
            "내가 속한 팀이랑 워크스페이스 보여줘",
            "이 앱 공개 심사 넣고 싶어",
            "리소스 정리하고 싶어",
        ] {
            assert!(
                !team_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn quality_intent_detection_routes_direct_human_prompts_without_stealing_neighbors() {
        assert!(quality_review_intent_present("이 코드 리뷰해줘"));
        assert!(quality_review_intent_present("review this diff"));
        assert!(!quality_review_intent_present("이 앱 공개 심사 넣고 싶어"));
        assert!(publish_intent_present("이 앱 공개 심사 넣고 싶어"));

        assert!(quality_debug_intent_present(
            "왜 테스트가 깨지는지 디버그해줘"
        ));
        assert!(!quality_debug_intent_present("방금 배포 왜 실패했어"));
        assert!(deploy_trace_intent_present("방금 배포 왜 실패했어"));

        assert!(quality_diagnose_intent_present("loop 돌려서 원인 찾아줘"));
        assert!(quality_diagnose_intent_present("재현 가능한 loop 만들어줘"));
        assert!(!quality_diagnose_intent_present("설치 상태 진단해"));

        assert!(quality_tdd_intent_present("테스트 먼저 TDD로 가자"));
        assert!(quality_tdd_intent_present("write the failing test first"));

        assert!(quality_plan_intent_present("큰 구조 변경 계획 세워줘"));
        assert!(quality_plan_intent_present("impact analysis 해줘"));
        assert!(!quality_plan_intent_present(
            "이 프로젝트 axhub로 옮길 수 있어?"
        ));

        assert!(quality_ship_intent_present("PR 만들기 전에 배포 준비 봐줘"));
        assert!(quality_ship_intent_present("release readiness 확인해줘"));
        assert!(!quality_ship_intent_present("ship paydrop"));
    }

    #[test]
    fn deploy_status_intent_detection_covers_short_progress_prompts() {
        for prompt in [
            "어디까지 됐어",
            "지금 어디까지야",
            "진행 상황 알려줘",
            "방금 배포 상태 봐줘",
            "is it done",
            "deploy status",
        ] {
            assert!(
                deploy_status_intent_present(prompt),
                "expected deploy status intent for {prompt:?}"
            );
        }

        for prompt in ["테이블 설명해줘", "오늘 할 일 정리해줘", "그냥 빌드만"]
        {
            assert!(
                !deploy_status_intent_present(prompt),
                "non-deploy status prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn deploy_restore_intent_detection_covers_human_rollback_without_stealing_git() {
        for prompt in [
            "방금 배포 되돌려줘",
            "방금 거 되돌려줘",
            "직전 안정 버전으로 복구해줘",
            "잘 되던 버전으로 돌려",
            "undo deploy",
            "rollback 해줘 직전 거로",
        ] {
            assert!(
                deploy_restore_intent_present(prompt),
                "expected deploy restore intent for {prompt:?}"
            );
        }

        for prompt in [
            "git revert 해줘",
            "방금 커밋 되돌려줘",
            "업데이트 되돌려줘",
            "상태바 설정 되돌려줘",
            "로컬 파일 되돌려줘",
            "배포 상태 봐줘",
        ] {
            assert!(
                !deploy_restore_intent_present(prompt),
                "neighbor restore prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn deploy_create_and_doctor_intent_detection_is_narrow() {
        assert!(deploy_create_intent_present("배포해"));
        assert!(deploy_create_intent_present("deploy this"));
        assert!(!deploy_create_intent_present("배포 상태 봐줘"));
        assert!(!deploy_create_intent_present("deploy status"));
        assert!(!deploy_create_intent_present("undo deploy"));

        assert!(doctor_intent_present("환경 점검해"));
        assert!(doctor_intent_present("axhub CLI 설치 상태 괜찮아?"));
        assert!(doctor_intent_present("axhub 잘 깔렸는지 봐줘"));
        assert!(doctor_intent_present("doctor"));
        assert!(!doctor_intent_present("테이블 점검해"));
        assert!(!doctor_intent_present("axhub 설치해줘"));

        assert!(install_cli_intent_present("axhub CLI 설치해줘"));
        assert!(install_cli_intent_present("axhub 설치해줘"));
        assert!(install_cli_intent_present("install axhub"));
        assert!(!install_cli_intent_present("axhub CLI 설치 상태 괜찮아?"));

        assert!(update_check_intent_present("업데이트 필요한지 봐줘"));
        assert!(update_check_intent_present("새 버전 나왔어?"));
        assert!(update_check_intent_present("check version"));
        assert!(!update_check_intent_present(
            "Claude에 설치된 axhub 플러그인도 최신인지 봐줘"
        ));
        assert!(!update_check_intent_present("axhub CLI 설치 상태 괜찮아?"));

        assert!(apis_intent_present(
            "axhub 앱이 어떤 API 쓸 수 있는지 보여줘"
        ));
        assert!(apis_intent_present("available endpoints"));
        assert!(!apis_intent_present("repo API 코드 리팩토링해"));
    }

    #[test]
    fn inspect_config_intent_detection_is_narrow() {
        assert!(inspect_config_intent_present(
            "매니페스트랑 설정 괜찮은지 봐줘"
        ));
        assert!(inspect_config_intent_present("axhub.yaml 검증해줘"));
        assert!(inspect_config_intent_present("check config"));
        assert!(inspect_config_intent_present("manifest validate"));

        assert!(!inspect_config_intent_present("배포 상태 봐줘"));
        assert!(!inspect_config_intent_present("내 앱 목록 보여줘"));
        assert!(!inspect_config_intent_present("repo 설정 리팩토링해"));
    }

    #[test]
    fn desktop_app_template_intent_detection_is_narrow() {
        assert!(init_intent_present("새 앱 만들어줘"));
        assert!(init_intent_present("nextjs 앱 만들어줘"));
        assert!(init_intent_present("프로젝트 초기화해줘"));
        assert!(!init_intent_present("내 앱 목록 보여줘"));
        assert!(!init_intent_present("템플릿 뭐 있어?"));

        assert!(apps_intent_present("내 앱 목록 보여줘"));
        assert!(apps_intent_present("운영 중인 앱 뭐 있어"));
        assert!(!apps_intent_present("새 앱 만들어줘"));
        assert!(!apps_intent_present("템플릿 뭐 있어?"));

        assert!(browse_template_intent_present("템플릿 뭐 있어?"));
        assert!(browse_template_intent_present("list templates"));
        assert!(!browse_template_intent_present("새 앱 만들어줘"));
        assert!(!browse_template_intent_present("내 앱 목록 보여줘"));
    }

    #[test]
    fn clarify_intent_detection_is_narrow() {
        assert!(clarify_intent_present("axhub 좀 도와줘"));
        assert!(clarify_intent_present("도와줘 axhub"));
        assert!(clarify_intent_present("help me with axhub"));
        assert!(clarify_intent_present("axhub"));

        assert!(!clarify_intent_present("axhub 처음 쓰는데 뭐부터 하면 돼?"));
        assert!(!clarify_intent_present("새 앱 만들어줘"));
        assert!(!clarify_intent_present("배포해줘"));
        assert!(!clarify_intent_present("내 앱 목록 보여줘"));
        assert!(!clarify_intent_present("로그 좀 보여줘"));
    }

    #[test]
    fn auth_status_intent_detection_is_narrow() {
        // login/token/identity status questions route to auth, not doctor
        assert!(auth_status_intent_present("나 지금 로그인 돼 있어?"));
        assert!(auth_status_intent_present("로그인 됐어?"));
        assert!(auth_status_intent_present("로그인 상태 확인해줘"));
        assert!(auth_status_intent_present(
            "지금 로그인 필요한 상태인지 봐줘"
        ));
        assert!(auth_status_intent_present("로그인 다시 해야 해?"));
        assert!(auth_status_intent_present("재로그인 필요해?"));
        assert!(auth_status_intent_present("토큰 살아있어?"));
        assert!(auth_status_intent_present("어떤 계정으로 접속 중이야?"));
        assert!(auth_status_intent_present("who am i"));
        // must NOT steal doctor / deploy-status / apps / generic prompts
        assert!(!auth_status_intent_present("axhub 설치돼 있어?"));
        assert!(!auth_status_intent_present("환경 점검해줘"));
        assert!(!auth_status_intent_present("배포 상태 봐줘"));
        assert!(!auth_status_intent_present("내 앱 목록 보여줘"));
        assert!(!auth_status_intent_present("배포 다 됐어?"));
    }

    #[test]
    fn deploy_logs_intent_detection_owns_log_requests_without_stealing_login() {
        for prompt in [
            "로그 좀 보여줘",
            "빌드 로그 봐",
            "런타임 로그 확인해줘",
            "tail logs",
            "console log",
        ] {
            assert!(
                deploy_logs_intent_present(prompt),
                "expected deploy logs intent for {prompt:?}"
            );
        }

        for prompt in [
            "로그인 상태 확인해줘",
            "지금 로그인 필요한 상태인지 봐줘",
            "로그인 다시 해야 해?",
            "재로그인 필요해?",
            "토큰 살아있어?",
            "내 앱 목록 보여줘",
            "매니페스트랑 설정 괜찮은지 봐줘",
            "배포 실패 원인 알려줘",
            "왜 실패했어",
            "왜 안돼",
        ] {
            assert!(
                !deploy_logs_intent_present(prompt),
                "non-log prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn deploy_trace_intent_detection_owns_failure_cause_requests() {
        for prompt in [
            "배포 실패 원인 알려줘",
            "실패 원인 좀 봐줘",
            "왜 실패했어",
            "왜 안돼",
            "왜 깨졌는지 분석해줘",
            "what went wrong",
            "debug deploy",
        ] {
            assert!(
                deploy_trace_intent_present(prompt),
                "expected trace intent for {prompt:?}"
            );
        }

        for prompt in [
            "로그 좀 보여줘",
            "로그인 상태 확인해줘",
            "방금 배포 진짜 열리는지 확인해줘",
            "지금 진행 중인 배포 어디까지 됐어?",
            "매니페스트랑 설정 괜찮은지 봐줘",
        ] {
            assert!(
                !deploy_trace_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn app_lifecycle_intent_detection_is_narrow() {
        for prompt in [
            "testnextjs 앱 잠깐 멈춰줘",
            "testnextjs 앱 잠시 내려줘",
            "testnextjs 앱 다시 켜줘",
            "testnextjs 다시 켜줘",
            "testnextjs 켜줘",
            "testnextjs 멈춰줘",
            "paydrop 다시 올려줘",
            "testnextjs 앱 복제해",
            "pause testnextjs app",
            "resume app",
        ] {
            assert!(
                app_lifecycle_intent_present(prompt),
                "expected app lifecycle intent for {prompt:?}"
            );
        }

        for prompt in [
            "testnextjs 프로세스 멈춰줘",
            "서버 다시 켜줘",
            "nextjs 서버 다시 켜줘",
            "localhost 다시 켜줘",
            "testnextjs 로그 보여줘",
            "testnextjs 배포해줘",
            "왜 멈췄는지 봐줘",
            "배포 상태 봐줘",
            "내 앱 목록 보여줘",
            "새 앱 만들어줘",
            "로그 좀 보여줘",
        ] {
            assert!(
                !app_lifecycle_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn open_app_intent_detection_covers_browser_requests_without_stealing_prs() {
        for prompt in [
            "라이브 페이지 열어봐",
            "결과 봐",
            "브라우저로 열어줘",
            "프로덕션 열어",
            "deploy url 보여줘",
            "open in browser",
        ] {
            assert!(
                open_app_intent_present(prompt),
                "expected open intent for {prompt:?}"
            );
        }

        for prompt in [
            "open a pull request",
            "GitHub PR 열어줘",
            "PR 만들어줘",
            "브라우저 테스트 코드 고쳐줘",
        ] {
            assert!(
                !open_app_intent_present(prompt),
                "non-open prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn deploy_verify_intent_detection_covers_live_verdict_without_stealing_neighbors() {
        for prompt in [
            "방금 배포 진짜 열리는지 확인해줘",
            "방금 거 확인해줘",
            "라이브 됐어?",
            "배포 검증해줘",
            "smoke test",
            "is it live",
        ] {
            assert!(
                deploy_verify_intent_present(prompt),
                "expected verify intent for {prompt:?}"
            );
        }

        for prompt in [
            "라이브 페이지 열어봐",
            "방금 거 로그 보여줘",
            "지금 진행 중인 배포 어디까지 됐어?",
            "매니페스트 확인해줘",
            "open a pull request",
        ] {
            assert!(
                !deploy_verify_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn routing_stats_intent_detection_covers_desktop_analytics_phrasing() {
        for prompt in [
            "이번 주 axhub 라우팅 어땠어?",
            "라우팅 통계 보여줘",
            "지난주 매칭 어땠어",
            "axhub routing 분석해줘",
            "routing stats",
        ] {
            assert!(
                routing_stats_intent_present(prompt),
                "expected routing-stats intent for {prompt:?}"
            );
        }

        for prompt in [
            "로그 좀 보여줘",
            "라이브 페이지 열어봐",
            "방금 배포 진짜 열리는지 확인해줘",
            "새 앱 만들어줘",
        ] {
            assert!(
                !routing_stats_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn statusline_intent_detection_covers_desktop_status_bar_phrasing() {
        for prompt in [
            "상태바 켜줘",
            "상태표시줄 켜줘",
            "상태줄 활성화",
            "statusline 켜줘",
            "enable status line",
        ] {
            assert!(
                statusline_intent_present(prompt),
                "expected statusline intent for {prompt:?}"
            );
        }

        for prompt in [
            "배포 상태 보여줘",
            "로그인 상태 확인해줘",
            "설치 상태 괜찮아?",
            "매니페스트 설정 봐줘",
        ] {
            assert!(
                !statusline_intent_present(prompt),
                "neighbor prompt must not be captured: {prompt:?}"
            );
        }
    }

    #[test]
    fn env_intent_detection_covers_read_only_without_stealing_mutations() {
        for prompt in [
            "환경변수 뭐 있어?",
            "환경 변수 확인해줘",
            "env 봐",
            "env list",
            "environment variables",
        ] {
            assert!(
                env_intent_present(prompt),
                "expected env read intent for {prompt:?}"
            );
        }

        for prompt in [
            "환경변수 추가해줘",
            "API 키 등록하고 싶어",
            "DB URL 수정해줘",
            "env delete TEST_KEY",
            "secret 삭제해줘",
        ] {
            assert!(
                !env_intent_present(prompt),
                "mutation prompt must stay on full env skill path: {prompt:?}"
            );
        }
    }

    #[test]
    fn marker_found_walking_up_to_git_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // root/.git  + root/axhub.yaml ; cwd = root/a/b/c
        std::fs::create_dir(root.join(".git")).expect("mkdir .git");
        std::fs::write(root.join("axhub.yaml"), "app: demo\n").expect("write marker");
        let nested = root.join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).expect("mkdir nested");
        assert_eq!(find_marker_from(&nested), MarkerStatus::Present);
    }

    #[test]
    fn marker_absent_stops_at_git_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // .git at root but NO axhub.yaml anywhere → Absent (walk stops at git root).
        std::fs::create_dir(root.join(".git")).expect("mkdir .git");
        let nested = root.join("pkg").join("src");
        std::fs::create_dir_all(&nested).expect("mkdir nested");
        assert_eq!(find_marker_from(&nested), MarkerStatus::Absent);
    }

    #[test]
    fn marker_present_at_git_root_itself() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // Marker AND .git both at the git root: marker must still be found
        // (axhub.yaml is checked before the .git stop condition).
        std::fs::create_dir(root.join(".git")).expect("mkdir .git");
        std::fs::write(root.join("axhub.yaml"), "app: demo\n").expect("write marker");
        assert_eq!(find_marker_from(root), MarkerStatus::Present);
    }

    #[test]
    fn decision_wire_strings_match_spec() {
        assert_eq!(RoutingDecision::Axhub.as_str(), "axhub");
        assert_eq!(RoutingDecision::Yield.as_str(), "yield");
        assert_eq!(RoutingDecision::Ignore.as_str(), "ignore");
        assert_eq!(RoutingDecision::Ask.as_str(), "ask");
        // serde representation agrees with as_str().
        assert_eq!(
            serde_json::to_string(&RoutingDecision::Yield).expect("serialize"),
            "\"yield\""
        );
    }
}
