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
