//! Per-command clap argument structs (wave 단위로 채워져요).
//!
//! US1(P1 hook) → US2(P2 데이터) → US3(P3 분석/hidden) 진행에 따라 추가돼요.
//! 미이관 명령은 `super::Commands::Passthrough` 로 legacy dispatch 를 타요.

/// `classify-exit` flags (empty-stdin fallback path 전용). hook 호출은 stdin
/// payload 를 쓰고 이 flags 를 무시해요. 기본값은 legacy 와 동일(exit_code=1, stdout="").
#[derive(clap::Args, Debug)]
pub(crate) struct ClassifyExitArgs {
    #[arg(long = "exit-code", default_value_t = 1)]
    pub exit_code: i32,
    #[arg(long = "stdout", default_value = "")]
    pub stdout: String,
}

/// `state-update` action flag (정확히 하나 필수). classify()=Normal 이라 parse 실패
/// (무인자/bad flag) → exit 64 로 legacy parity 보존(parity guard, FR-001).
#[derive(clap::Args, Debug)]
#[command(group(
    clap::ArgGroup::new("state_update_action")
        .required(true)
        .multiple(false)
        .args([
            "review_acknowledged",
            "post_commit_promote",
            "debug_acknowledged",
            "shipped",
            "edit_event",
            "pull",
        ])
))]
pub(crate) struct StateUpdateArgs {
    #[arg(long = "review-acknowledged")]
    pub review_acknowledged: bool,
    #[arg(long = "post-commit-promote")]
    pub post_commit_promote: bool,
    #[arg(long = "debug-acknowledged")]
    pub debug_acknowledged: bool,
    #[arg(long)]
    pub shipped: bool,
    #[arg(long = "edit-event")]
    pub edit_event: bool,
    #[arg(long)]
    pub pull: bool,
}

impl StateUpdateArgs {
    /// 선택된 flag 의 legacy 토큰. ArgGroup 가 정확히 하나를 보장하므로 항상 매칭돼요.
    /// legacy `cmd_state_update` 의 `match args.first()` 에 그대로 넘겨 동작을 보존해요.
    pub(crate) fn chosen_flag(&self) -> &'static str {
        if self.review_acknowledged {
            "--review-acknowledged"
        } else if self.post_commit_promote {
            "--post-commit-promote"
        } else if self.debug_acknowledged {
            "--debug-acknowledged"
        } else if self.shipped {
            "--shipped"
        } else if self.edit_event {
            "--edit-event"
        } else {
            "--pull"
        }
    }
}

/// `autowire-statusline` flags. classify()=Normal (legacy bad-arg→64). scope 값
/// 검증·auto 해석은 handler 가 담당(한국어 에러 보존). long_about 으로 한국어
/// help 콘텐츠 보존(D6).
#[derive(clap::Args, Debug)]
#[command(
    long_about = "axhub-helpers autowire-statusline — SessionStart statusLine 자동 설정\n\n\
OPTIONS:\n  --scope user|project|auto   대상 settings.json scope (auto=환경 감지)\n  \
--silent                    stderr 억제 (hook 호출 모드)\n  \
--command-path <p>          statusLine.command 경로 override\n  \
--child                     child 프로세스 플래그 (marker write 안 함)\n\n\
ENV:\n  AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1   전체 skip"
)]
pub(crate) struct AutowireCliArgs {
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long)]
    pub silent: bool,
    #[arg(long)]
    pub child: bool,
    #[arg(long = "command-path")]
    pub command_path: Option<String>,
}

/// `consent-mint` flag. classify()=Normal (bad-arg→64). stdin 으로 JSON binding 읽음.
#[derive(clap::Args, Debug)]
pub(crate) struct ConsentMintArgs {
    #[arg(long = "validate-only")]
    pub validate_only: bool,
}

/// `token-init`/`token-import` 공용 flag. classify()=Normal (bad-arg→64).
#[derive(clap::Args, Debug)]
pub(crate) struct TokenArgs {
    #[arg(long)]
    pub json: bool,
}

/// `verify` flags. app_id 필수 검증은 handler 가 담당(한국어 메시지 보존).
#[derive(clap::Args, Debug)]
pub(crate) struct VerifyArgs {
    #[arg(long = "app-id", visible_alias = "app")]
    pub app_id: Option<String>,
    #[arg(long)]
    pub json: bool,
}

/// `trace` flags. deploy_id 필수 검증은 handler 가 담당.
#[derive(clap::Args, Debug)]
pub(crate) struct TraceArgs {
    #[arg(long = "deploy-id")]
    pub deploy_id: Option<String>,
    #[arg(long)]
    pub app: Option<String>,
    #[arg(long)]
    pub json: bool,
}

/// `doctor` flags.
#[derive(clap::Args, Debug)]
pub(crate) struct DoctorArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long = "no-cooldown")]
    pub no_cooldown: bool,
}

/// `settings-merge` raw flags. mutual-excl(--apply/--dry-run, --migrate/--apply) 검증·dry_run
/// 파생·scope 검증은 handler 가 담당(bail→64 보존). classify=Normal.
#[derive(clap::Args, Debug)]
#[command(
    long_about = "axhub-helpers settings-merge — ~/.claude/settings.json statusLine 병합\n\n\
OPTIONS:\n  --apply           실제 병합 실행 (explicit consent gate)\n  \
--dry-run         결정만 출력, 파일 변경 없음 (기본값)\n  \
--migrate         stale ${CLAUDE_PLUGIN_ROOT} literal 를 orphan stub path 로 교체\n  \
--yes             --migrate 와 함께: 대화형 확인 없이 자동 적용\n  \
--scope <s>       user|project|auto (기본: auto)\n  \
--json            결과를 JSON 으로 출력\n  --silent  stderr 억제\n  \
--command-path    statusLine command 경로 override\n\n\
EXIT CODES: 0 no-op  2 created  3 merged  4 preserved-other  5 invalid-json  6 partial-schema  7 permission-denied"
)]
pub(crate) struct SettingsMergeCliArgs {
    #[arg(long)]
    pub apply: bool,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    #[arg(long)]
    pub migrate: bool,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub silent: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long = "command-path")]
    pub command_path: Option<String>,
}

/// `post-install` flags. 필수(target-name/bin-dir/link-path) 검증은 handler. classify=Normal.
#[derive(clap::Args, Debug)]
#[command(
    long_about = "axhub-helpers post-install — sh/ps1-absorption Phase 3.1 post-install handler\n\nUSAGE:\n  axhub-helpers post-install --target-name <N> --bin-dir <D> --link-path <P> [--repo-root <R>]"
)]
pub(crate) struct PostInstallArgs {
    #[arg(long = "target-name")]
    pub target_name: Option<String>,
    #[arg(long = "bin-dir")]
    pub bin_dir: Option<String>,
    #[arg(long = "link-path")]
    pub link_path: Option<String>,
    #[arg(long = "repo-root")]
    pub repo_root: Option<String>,
}

/// `audit-clarify` flags. hash XOR prompt 검증은 handler. classify=Normal.
#[derive(clap::Args, Debug)]
pub(crate) struct AuditClarifyArgs {
    #[arg(long)]
    pub hash: Option<String>,
    #[arg(long)]
    pub prompt: Option<String>,
    #[arg(long)]
    pub chosen: Option<String>,
}

/// `list-deployments` flags. app_id 필수·limit 파싱/범위 검증은 handler. classify=Normal.
#[derive(clap::Args, Debug)]
pub(crate) struct ListDeploymentsCliArgs {
    #[arg(long = "app-id", visible_alias = "app")]
    pub app_id: Option<String>,
    #[arg(long)]
    pub limit: Option<String>,
}

/// `migrate-plan` local light pre-scan flags. The helper only emits candidate
/// paths, stack hints, container-contract presence, and env names; backend
/// detection remains authoritative.
#[derive(clap::Args, Debug)]
pub(crate) struct MigratePlanArgs {
    #[arg(long)]
    pub dir: Option<String>,
    #[arg(long = "app-path")]
    pub app_path: Option<String>,
    #[arg(long)]
    pub json: bool,
}

/// `routing-stats` flags. since/top 파싱(→64)은 handler. 한국어 PRIVACY help 는
/// 기존 const 를 long_about 으로 보존(D6, FR-006a). classify=Normal.
#[derive(clap::Args, Debug)]
#[command(long_about = crate::ROUTING_STATS_HELP)]
pub(crate) struct RoutingStatsArgs {
    #[arg(long)]
    pub since: Option<String>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub top: Option<String>,
    #[arg(long)]
    pub confused: bool,
}

/// `route-decision` flags (spec 006 §53). deploy SKILL preflight Step 0 가
/// auth/resolve **전에** 호출해서 공유 routing-decision 함수의 결과를 받아요. 입력은
/// 사용자 발화(`--user-utterance`) 와 explicit-invocation 신호(`--explicit`).
/// `--explicit` 는 모델이 `/deploy`·`/axhub:deploy` 슬래시 호출일 때 세워요(슬래시
/// 토큰이 발화에 안 남는 경우가 있어서 — `commands/deploy.md` 는 `$ARGUMENTS` 만
/// 넘겨요). handler 가 발화에서 슬래시를 또 감지하므로 둘 중 하나만 맞아도 explicit.
/// classify=Normal 이지만 출력이 항상 JSON+exit 0 이라 SKILL 이 `.decision` 으로
/// 분기하고, 빈 출력이면 fail-open(axhub 진행)해요.
#[derive(clap::Args, Debug)]
pub(crate) struct RouteDecisionArgs {
    #[arg(long = "user-utterance", default_value = "")]
    pub user_utterance: String,
    #[arg(long)]
    pub explicit: bool,
}

/// `diagnose hitl` flags (nested). session/prompts 필수·TTY 검증은 handler. classify=Normal.
#[derive(clap::Args, Debug)]
pub(crate) struct DiagnoseHitlArgs {
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub prompts: Option<String>,
    #[arg(long)]
    pub output: Option<String>,
}
