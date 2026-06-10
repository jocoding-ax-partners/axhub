//! clap CLI layer — Phase 2 scaffold (specs/001-helpers-clap-refactor).
//!
//! 전환기(transition) 설계: 단일 `Passthrough` external-subcommand variant 이
//! 모든 subcommand 를 main.rs 의 legacy `match` dispatcher(`crate::legacy_dispatch`)
//! 로 라우팅해요. 덕분에 명령별 typed 이관이 wave 로 진행되는 동안 동작이
//! byte-identical 로 유지돼요. `version`/`help`/no-args 는 clap 이전에 pre-intercept
//! 해서 정확한 legacy 출력을 보존해요(FR-004 + FR-006 전환기). passthrough·USAGE
//! pre-intercept 는 Polish(T036-T037)에서 제거해요.

use clap::Parser;

pub(crate) mod args;

#[derive(Parser)]
#[command(
    name = "axhub-helpers",
    bin_name = "axhub-helpers",
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    // ── US1 (P1 hook 진입점) — 무인자 hook 명령, typed unit variant ──
    SessionStart,
    PromptRoute,
    CommitGate,
    TddInject,
    TestClassifier,
    VerifyDeployArtifact,
    /// spec 006 — session-start eager-infra marker gate. exit 0 = run eager
    /// infra (token-init/warmup/quality-context), exit 1 = skip (zero-footprint).
    /// Consumed by the session-start shell wrapper before token-init/warmup.
    SessionEagerGate,
    /// spec 006 §57/§68 — shared routing-decision gate for the deploy SKILL
    /// preflight (Step 0). Emits `{decision, marker, authed, …}` JSON so the
    /// SKILL proceeds only when `decision == axhub`; `yield`/`ignore`/`ask`
    /// block axhub deploy before any auth/resolve work.
    RouteDecision(args::RouteDecisionArgs),

    // ── US1 — flag-bearing hook 명령 (typed args) ──
    ClassifyExit(args::ClassifyExitArgs),
    StateUpdate(args::StateUpdateArgs),
    AutowireStatusline(args::AutowireCliArgs),

    // ── US2 — 데이터/auth 명령 (typed args) ──
    TokenInit(args::TokenArgs),
    TokenImport(args::TokenArgs),
    TokenGate,
    Verify(args::VerifyArgs),
    Trace(args::TraceArgs),
    Doctor(args::DoctorArgs),
    RepairPath(args::RepairPathArgs),
    SettingsMerge(args::SettingsMergeCliArgs),

    // ── US3 — 분석/유지보수 명령 (typed args) ──
    PostInstall(args::PostInstallArgs),
    AuditClarify(args::AuditClarifyArgs),
    ListDeployments(args::ListDeploymentsCliArgs),
    MigratePlan(args::MigratePlanArgs),
    MigrateStageWrite(args::MigrateStageWriteArgs),
    MigrateWavePlan(args::MigrateWavePlanArgs),
    MigrateApprove(args::MigrateApproveArgs),
    RoutingStats(args::RoutingStatsArgs),
    Diagnose {
        #[command(subcommand)]
        sub: DiagnoseSub,
    },

    /// 정적 AST 패턴 검사 (Track H, 신규). 편집된 사용자 코드를 tree-sitter 로 파싱해
    /// SDK 데이터/HTTP 계약 위반(or()/not() 비-pushable 필터, after:/before: 커서,
    /// /api/v1 prefix 누락 등)을 정적으로 검출해요. exit 0=클린/advisory, 1=block
    /// 위반, 파싱 실패는 exit 0+경고(fail-open).
    ///
    /// 의미 분리(혼동 금지): `validate` = **정적 AST 패턴**(편집 코드 계약 검사).
    /// `verify`/`verify-deploy-artifact` = **배포·런타임 검증**(배포 산출물·런타임
    /// 상태). 서로 대체 관계가 아니에요.
    #[cfg(feature = "ast")]
    Validate(args::ValidateArgs),

    /// PostToolUse hook 진입점 (Track H). stdin 의 PostToolUse payload 에서 편집된
    /// 파일 경로를 뽑아 `validate` 엔진으로 검사해요. **항상 exit 0**(fail-open) —
    /// block 위반은 systemMessage(warn-only)로만, `AXHUB_AST_VALIDATE=block` opt-in
    /// 시 additionalContext 교정 지시도 함께. 사용자 직접 호출이 아니라 hook 전용.
    #[cfg(feature = "ast")]
    AstValidate,

    /// 변환 사이트 스캐너 (Track H §H2, 신규). migrate 플로우용 finder — raw HTTP
    /// client 직타, 직접 DB driver, 하드코딩 API URL 을 6언어 AST 로 찾아
    /// `{file,line,kind,snippet}` JSON 으로 내요. 항상 exit 0(gate 아님).
    #[cfg(feature = "ast")]
    ScanSites(args::ScanSitesArgs),

    /// stdio MCP 서버 (Track H frontend 3). validate/scan-sites 엔진을 MCP tool 로
    /// 노출해요. `.mcp.json` 의 local 항목이 이 명령을 stdio 로 띄워요. 클라이언트
    /// 연결이 닫힐 때까지 실행돼요.
    #[cfg(feature = "mcp")]
    McpServe,

    /// `.mcp.json` 설치/머지 (Track H §D.2). 대상 프로젝트의 `.mcp.json` 에 axhub
    /// local(stdio mcp-serve) + remote(ax-mcp) 두 항목만 추가/갱신하고 기존 사용자
    /// 항목은 보존해요. init/migrate SKILL 이 호출해요.
    #[cfg(feature = "mcp")]
    McpInstall(args::McpInstallArgs),

    /// 전환기 raw passthrough — legacy dispatch 로 위임. Polish 에서 제거.
    #[command(external_subcommand)]
    Passthrough(Vec<String>),
}

/// fail-open 분류 (D3, Clarifications 2026-05-29). argv[1] subcommand 토큰 기준
/// (registry 이름 무관). `FailOpenHook` = parse 실패해도 exit 0.
///
/// Parity scope (FR-001): flag-bearing hook(`state-update`)은 **유효 hook-flag
/// 경로**만 fail-open 이고 malformed/비-hook 입력은 handler 의 기존 exit code(64)를
/// 보존해요. classify()→FailOpenHook 는 "clap parse 실패 시 비정상 종료 금지"를
/// 의미하지, "무조건 exit 0" 이 아니에요 — typed handler 가 자체 exit code 를 반환해요.
/// `diagnose` 중첩 subcommand (현재 hitl 만).
#[derive(clap::Subcommand)]
enum DiagnoseSub {
    Hitl(args::DiagnoseHitlArgs),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HookClass {
    FailOpenHook,
    Normal,
}

fn classify(token: Option<&str>) -> HookClass {
    // FailOpenHook = legacy 가 잘못된/알 수 없는 인자에 exit 0 을 주던 명령만
    // (무인자 또는 unknown-flag-ignore). 이들은 typed 후 clap parse 실패 → exit 0 이
    // legacy 동작과 일치해요.
    //
    // 제외(=Normal, parse error→64): `state-update` 는 legacy 가 잘못된 flag 에
    // **exit 64** 를 반환하므로(parity guard, FR-001) parse 실패도 64 여야 해요.
    // `autowire-statusline` 은 SessionStart wrapper 경유라 hook 맥락에서 방어적
    // fail-open(parse error→0) 으로 분류해요. `token-gate` 는 Normal (SKILL gate,
    // exit 0/65 의미 보존).
    match token {
        Some(
            "session-start"
            | "prompt-route"
            | "commit-gate"
            | "tdd-inject"
            | "test-classifier"
            | "verify-deploy-artifact"
            | "classify-exit"
            | "autowire-statusline"
            | "karpathy-inject"
            | "ast-validate",
        ) => HookClass::FailOpenHook,
        _ => HookClass::Normal,
    }
}

/// D5: version 의도를 clap 이전에 가로채요. `version`/`--version`/`-v` 가 argv[1]
/// 일 때만, `--quiet` 는 그 뒤 어디든. quiet ⇒ 빈 stdout+stderr+exit 0.
fn version_intercept(args: &[String]) -> Option<i32> {
    let first = args.first().map(String::as_str)?;
    if !matches!(first, "version" | "--version" | "-v") {
        return None;
    }
    let quiet = args[1..].iter().any(|a| a == "--quiet");
    if !quiet {
        println!(
            "axhub-helpers {} (plugin v{}, schema {})",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION"),
            crate::HOOK_SCHEMA_VERSION
        );
    }
    Some(0)
}

/// 전환기 help pre-intercept: `help`/`--help`/`-h` 가 argv[1] 이면 legacy USAGE
/// (stdout, exit 0). Polish(T037)에서 clap 자동 생성 help 로 교체.
fn help_intercept(args: &[String]) -> Option<i32> {
    match args.first().map(String::as_str) {
        Some("help" | "--help" | "-h") => {
            println!("{}", crate::USAGE);
            Some(0)
        }
        _ => None,
    }
}

/// D4: clap try_parse 에러 분기. help → stdout+0, version → 0(pre-intercepted),
/// usage error → hook 이면 0(fail-open), 아니면 stderr + 64(clap 기본 2 아님).
fn handle_parse_error(e: clap::Error, hook_class: HookClass, command_token: Option<&str>) -> i32 {
    use clap::error::ErrorKind;
    match e.kind() {
        ErrorKind::DisplayHelp => {
            print!("{e}");
            0
        }
        ErrorKind::DisplayVersion => 0,
        kind => {
            if hook_class == HookClass::FailOpenHook {
                0
            } else {
                if command_token == Some("routing-stats")
                    && matches!(kind, ErrorKind::UnknownArgument)
                {
                    eprintln!("axhub-helpers routing-stats: 알 수 없는 flag");
                    return 64;
                }
                // legacy per-command parity: generic 메시지만 출력하고 **인자 값은 echo
                // 안 해요** (토큰류 값 누출 방지 — cli_e2e do_not_echo_token 테스트).
                // clap raw 에러(`e.print()`)는 unrecognized arg 를 echo + "unexpected
                // argument" 문구라 legacy "unknown option" 계약을 깨요(SC-001 은 top-level
                // wording 만 변경 허용).
                let msg = if matches!(
                    kind,
                    ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand
                ) {
                    "missing option"
                } else {
                    "unknown option"
                };
                eprintln!("axhub-helpers: {msg}");
                64
            }
        }
    }
}

/// legacy_dispatch 의 `anyhow::Result<i32>` → process exit code (기존 main() 의
/// 에러 경로 동일: Err → stderr + exit 1).
fn run_result(r: anyhow::Result<i32>) -> i32 {
    match r {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn dispatch(command: Commands) -> i32 {
    match command {
        // US1 무인자 hook — 기존 handler 직접 호출(로직 0 변경).
        Commands::SessionStart => run_result(crate::cmd_session_start()),
        Commands::PromptRoute => run_result(crate::cmd_prompt_route()),
        Commands::CommitGate => run_result(crate::cmd_commit_gate()),
        Commands::TddInject => run_result(crate::cmd_tdd_inject()),
        Commands::TestClassifier => run_result(crate::cmd_test_classifier()),
        Commands::VerifyDeployArtifact => run_result(crate::cmd_verify_deploy_artifact()),
        Commands::SessionEagerGate => run_result(crate::cmd_session_eager_gate()),
        Commands::RouteDecision(a) => {
            run_result(crate::cmd_route_decision(&a.user_utterance, a.explicit))
        }
        Commands::ClassifyExit(a) => run_result(crate::cmd_classify_exit(a.exit_code, &a.stdout)),
        Commands::StateUpdate(a) => {
            let argv = [a.chosen_flag().to_string()];
            run_result(crate::cmd_state_update(&argv))
        }
        Commands::AutowireStatusline(a) => run_result(crate::cmd_autowire_statusline(
            a.scope.as_deref(),
            a.silent,
            a.child,
            a.command_path.map(std::path::PathBuf::from),
        )),
        Commands::TokenInit(a) => run_result(crate::cmd_token_init(a.json)),
        Commands::TokenImport(a) => run_result(crate::cmd_token_import(a.json)),
        Commands::TokenGate => run_result(crate::cmd_token_gate(&[])),
        Commands::Verify(a) => run_result(crate::cmd_verify(a.app_id, a.json)),
        Commands::Trace(a) => run_result(crate::cmd_trace(a.deploy_id, a.app, a.json)),
        Commands::Doctor(a) => run_result(crate::cmd_doctor(a.json, a.no_cooldown)),
        Commands::RepairPath(a) => run_result(crate::cmd_repair_path(a.json, a.dir)),
        Commands::SettingsMerge(a) => run_result(crate::cmd_settings_merge(a)),
        Commands::PostInstall(a) => run_result(crate::cmd_post_install(
            a.target_name,
            a.bin_dir,
            a.link_path,
            a.repo_root,
        )),
        Commands::AuditClarify(a) => {
            run_result(crate::cmd_audit_clarify(a.hash, a.prompt, a.chosen))
        }
        Commands::ListDeployments(a) => run_result(crate::cmd_list_deployments(a.app_id, a.limit)),
        Commands::MigratePlan(a) => {
            let mut argv = Vec::new();
            if let Some(dir) = a.dir {
                argv.push("--dir".to_string());
                argv.push(dir);
            }
            if let Some(app_path) = a.app_path {
                argv.push("--app-path".to_string());
                argv.push(app_path);
            }
            if a.persist_planning {
                argv.push("--persist-planning".to_string());
            }
            if a.json {
                argv.push("--json".to_string());
            }
            run_result(axhub_helpers::migrate_plan::run_migrate_plan(&argv))
        }
        Commands::MigrateStageWrite(a) => {
            let mut argv = vec![
                "--run-json".to_string(),
                a.run_json,
                "--stage".to_string(),
                a.stage,
                "--markdown-file".to_string(),
                a.markdown_file,
            ];
            if let Some(summary) = a.summary {
                argv.push("--summary".to_string());
                argv.push(summary);
            }
            if let Some(run_state) = a.run_state {
                argv.push("--run-state".to_string());
                argv.push(run_state);
            }
            if let Some(approval_state) = a.approval_state {
                argv.push("--approval-state".to_string());
                argv.push(approval_state);
            }
            if a.json {
                argv.push("--json".to_string());
            }
            run_result(axhub_helpers::migrate_planning::run_migrate_stage_write(
                &argv,
            ))
        }
        Commands::MigrateWavePlan(a) => {
            let mut argv = vec![
                "--run-json".to_string(),
                a.run_json,
                "--wave-id".to_string(),
                a.wave_id,
                "--stage-scope".to_string(),
                a.stage_scope,
            ];
            for participant in a.participants {
                argv.push("--participant".to_string());
                argv.push(participant);
            }
            for depends_on in a.depends_on {
                argv.push("--depends-on".to_string());
                argv.push(depends_on);
            }
            for artifact in a.artifacts {
                argv.push("--artifact".to_string());
                argv.push(artifact);
            }
            for write_target in a.write_targets {
                argv.push("--write-target".to_string());
                argv.push(write_target);
            }
            for proof in a.independence_proofs {
                argv.push("--independence-proof".to_string());
                argv.push(proof);
            }
            if let Some(state) = a.state {
                argv.push("--state".to_string());
                argv.push(state);
            }
            if a.json {
                argv.push("--json".to_string());
            }
            run_result(axhub_helpers::migrate_planning::run_migrate_wave_plan(
                &argv,
            ))
        }
        Commands::MigrateApprove(a) => {
            let mut argv = vec![
                "--run-json".to_string(),
                a.run_json,
                "--approved-by".to_string(),
                a.approved_by,
            ];
            if let Some(note) = a.approval_note {
                argv.push("--approval-note".to_string());
                argv.push(note);
            }
            if a.json {
                argv.push("--json".to_string());
            }
            run_result(axhub_helpers::migrate_planning::run_migrate_approve(&argv))
        }
        Commands::RoutingStats(a) => {
            run_result(crate::cmd_routing_stats(a.since, a.json, a.top, a.confused))
        }
        Commands::Diagnose { sub } => match sub {
            DiagnoseSub::Hitl(h) => {
                run_result(crate::cmd_diagnose_hitl(h.session, h.prompts, h.output))
            }
        },

        #[cfg(feature = "ast")]
        Commands::Validate(a) => {
            run_result(axhub_helpers::ast_validate::run_validate(&a.paths, a.json))
        }

        // hook 진입점 — run_hook 은 항상 i32(0) 반환(fail-open). run_result 경유 X
        // (Err 변환으로 인한 비정상 exit code 회피).
        #[cfg(feature = "ast")]
        Commands::AstValidate => axhub_helpers::ast_validate::run_hook(),

        #[cfg(feature = "ast")]
        Commands::ScanSites(a) => {
            run_result(axhub_helpers::site_scan::run_scan_sites(&a.paths, a.json))
        }

        #[cfg(feature = "mcp")]
        Commands::McpServe => run_result(axhub_helpers::mcp_serve::run_mcp_serve()),

        #[cfg(feature = "mcp")]
        Commands::McpInstall(a) => {
            run_result(axhub_helpers::mcp_config::run_mcp_install(a.dir, a.command))
        }

        Commands::Passthrough(tokens) => {
            let Some((cmd, rest)) = tokens.split_first() else {
                eprintln!("{}", crate::USAGE);
                return 64;
            };
            // feature-off 변형(--no-default-features): `AstValidate` variant 가 없어
            // ast-validate 가 여기로 떨어져요. hook 진입점은 fail-open 계약(전 경로
            // exit 0)이라 legacy USAGE+64 대신 stdin drain 후 조용히 no-op 해요 —
            // ast 미탑재 바이너리에 hook 이 걸려도 main 흐름을 막지 않아요.
            #[cfg(not(feature = "ast"))]
            if cmd == "ast-validate" {
                let mut sink = String::new();
                let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut sink);
                return 0;
            }
            run_result(crate::legacy_dispatch(cmd, rest.to_vec()))
        }
    }
}

/// CLI 진입점. `main()` 은 `enable_utf8_console()` 후 이 함수만 호출해요.
pub fn run() -> i32 {
    let argv: Vec<String> = std::env::args().collect();
    let rest = &argv[1..];

    // no subcommand → legacy USAGE stderr + 64 (parity)
    if rest.is_empty() {
        eprintln!("{}", crate::USAGE);
        return 64;
    }
    if let Some(code) = version_intercept(rest) {
        return code;
    }
    if let Some(code) = help_intercept(rest) {
        return code;
    }

    let command_token = argv.get(1).map(String::as_str);
    let hook_class = classify(command_token);
    match Cli::try_parse_from(std::env::args_os()) {
        Ok(cli) => dispatch(cli.command),
        Err(e) => handle_parse_error(e, hook_class, command_token),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_failopen_set() {
        // legacy bad-arg→0 명령만 (무인자/unknown-ignore). parse 실패→exit 0.
        for t in [
            "session-start",
            "prompt-route",
            "commit-gate",
            "tdd-inject",
            "test-classifier",
            "verify-deploy-artifact",
            "classify-exit",
            "autowire-statusline",
            "karpathy-inject",
            "ast-validate",
        ] {
            assert_eq!(
                classify(Some(t)),
                HookClass::FailOpenHook,
                "{t} must classify fail-open"
            );
        }
    }

    #[test]
    fn classify_normal_set() {
        // state-update·autowire-statusline = legacy bad-arg→64 (parity guard) → Normal.
        // token-gate = Normal (SKILL gate, exit 0/65 의미 보존).
        for t in [
            "state-update",
            "token-gate",
            "deploy-prep",
            "sync",
            "doctor",
            "bogus-subcommand",
        ] {
            assert_eq!(
                classify(Some(t)),
                HookClass::Normal,
                "{t} must classify Normal"
            );
        }
        assert_eq!(classify(None), HookClass::Normal);
    }

    #[test]
    fn version_intercept_arg_orders() {
        assert_eq!(
            version_intercept(&["version".into(), "--quiet".into()]),
            Some(0)
        );
        assert_eq!(
            version_intercept(&["--version".into(), "--quiet".into()]),
            Some(0)
        );
        assert_eq!(version_intercept(&["--version".into()]), Some(0));
        assert_eq!(version_intercept(&["-v".into()]), Some(0));
        assert_eq!(version_intercept(&["deploy-prep".into()]), None);
        assert_eq!(version_intercept(&[]), None);
    }
}
