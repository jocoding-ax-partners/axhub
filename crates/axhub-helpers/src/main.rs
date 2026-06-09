use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use axhub_helpers::autowire::{autowire_statusline, AutowireArgs};
use axhub_helpers::axhub_cli::run_axhub;
use axhub_helpers::bootstrap::{cmd_bootstrap_dependency_plan, run_bootstrap};
use axhub_helpers::catalog::classify;
use axhub_helpers::config::{config_get, config_set, render_get_json};
use axhub_helpers::deploy_prep::run_deploy_prep;
use axhub_helpers::hook_safety;
use axhub_helpers::init_resume::run_init_resume;
use axhub_helpers::keychain::{parse_keyring_value, read_keychain_token};
use axhub_helpers::list_deployments::{
    run_list_deployments, DeploymentSummary, ListDeploymentsArgs,
};
use axhub_helpers::migrate_plan::{build_migrate_plan, run_migrate_plan};
use axhub_helpers::onboarding_detect;
use axhub_helpers::preflight::{run_preflight, PreflightRun};
use axhub_helpers::quality_gate::{validate_deploy_prep_quality, QualityCheckResult};
use axhub_helpers::redact::redact;
use axhub_helpers::resolve::run_resolve;
use axhub_helpers::runtime_paths::{
    last_deploy_file, set_private_dir_mode, state_dir, token_file, welcome_marker_path,
    write_private_file_no_follow,
};
use axhub_helpers::scaffold::{run_scaffold_detect, run_scaffold_dev};
use axhub_helpers::session_bundle::{
    write_session_bundle, AuthStatusBundle, LastDeployBundle, SessionBundle,
};
use axhub_helpers::settings_merge::{
    merge as run_settings_merge, migrate_stale_command_path, MergeOptions, MergeOutcome,
    MigrateOutcome, Scope,
};
use axhub_helpers::snippet::run_snippet;
use axhub_helpers::statusline::current_statusline;
use axhub_helpers::sync::run_sync;
use axhub_helpers::telemetry::{
    append_phase_marker_to_file, emit_deploy_complete, emit_meta_envelope,
};
use axhub_helpers::tenant::run_tenant_resolve;
use axhub_helpers::{commit_gate, hook_output, quality_state};
use chrono::Utc;
use serde_json::{json, Map, Value};

mod cli;

pub(crate) const HOOK_SCHEMA_VERSION: &str = "v0";
pub(crate) const USAGE: &str = "axhub-helpers - axhub plugin adapter binary (Rust)\n\nUsage:\n  axhub-helpers <subcommand> [args]\n\nSubcommands:\n  session-start\n  session-eager-gate\n  route-decision [--user-utterance <s>] [--explicit]\n  prompt-route\n  resolve\n  preflight\n  onboarding-detect [--json]\n  classify-exit\n  verify-deploy-artifact\n  redact\n  statusline\n  path <token-file|last-deploy-file|state-dir>\n  token-init [--json]\n  token-import [--json]\n  token-gate\n  post-install --target-name <N> --bin-dir <D> --link-path <P> [--repo-root <R>]\n  list-deployments\n  bootstrap [--json] [--dry-run|--plan-only|--auto-chain|--record <event>|dependency-plan]\n  routing-stats [--since <D>] [--json] [--top <N>] [--confused]\n  cleanup-audit [--all] [--yes]\n  audit-clarify (--hash <H>|--prompt <P>) --chosen <S>\n  routing-dashboard [--html]\n  mark <phase_name>\n  emit-deploy-complete [<exit_code> [<command_class>]]\n  deploy-prep --intent <name> [--user-utterance <s>] [--refresh-in-flight] [--json]\n  scaffold-detect --json\n  scaffold-dev start|status|stop --json\n  init-resume get|put|route|clear --json\n  tenant-resolve [--json]\n  deploy-preview-summary [--user-utterance <s>]\n  deploy-approved-run [--user-utterance <s>]\n  migrate-plan --dir <path> [--app-path <candidate>] [--persist-planning] [--json]\n  migrate-stage-write --run-json <path> --stage <name> --markdown-file <file> [--run-state <state>] [--approval-state <state>] [--json]\n  migrate-wave-plan --run-json <path> --wave-id <id> --stage-scope <stage> [--participant <app_key>]... [--depends-on <wave_id>]... [--artifact <path>]... [--write-target <target>]... [--independence-proof <text>]... [--state <planned|running|complete>] [--json]\n  migrate-approve --run-json <path> --approved-by <name> [--approval-note <text>] [--json]\n  migrate-guard --dir <path> [--checkpoint] [--allow-dirty] [--init-ok] [--label <s>] [--json]\n  migrate-summary [--user-utterance <s>]\n  publish-summary [--user-utterance <s>]\n  env-summary [--user-utterance <s>]\n  open-summary [--user-utterance <s>]\n  config get <key> [--json]\n  config set <key> <value>\n  sync [--target <target>|auto] [--out <dir>] [--json] [--no-detail] [--allow-identity-change]\n  snippet --mode A|B --language <lang> --target <target> --connector <name> --path <path> --sql <sql> --allowed-columns <csv>\n  auth-refresh-bg\n  verify --app-id <id> [--json]\n  trace --deploy-id <id> [--app <app>] [--json]\n  doctor [--json] [--no-cooldown]\n  repair-path [--json] [--dir <path>]\n  settings-merge --apply|--dry-run [--scope user|project|auto] [--json]\n  autowire-statusline --scope user|project [--silent] [--command-path <p>] [--child]\n  orphan-stub --install [--verify] | --verify\n  diagnose hitl --session <loop_id> --prompts <prompts.json> [--output <captured.json>]\n  version [--quiet]\n  help";

/// Force Windows console output codepage to UTF-8 (65001).
///
/// Windows console 의 default codepage 가 OEM (Korean=CP949, US=CP437) 이라
/// Rust `println!` 의 UTF-8 한글 출력이 mojibake 로 깨져요. `bin/statusline.ps1`
/// wrapper 가 `[Console]::OutputEncoding=UTF8` 로 흡수하지만 `cmd.exe` 직접
/// 호출 / 다른 wrapper 경로는 보호 못해요.
///
/// Codepage 는 process-attached scope 라 `axhub-helpers.exe` 종료 시 함께
/// destroy 돼요. parent `cmd.exe` 세션 codepage 영향 0. pipe redirect / console
/// 미attach 시 `SetConsoleOutputCP` 가 0 반환 — fail-open 으로 무시해요.
#[cfg(windows)]
fn enable_utf8_console() {
    use windows_sys::Win32::System::Console::SetConsoleOutputCP;
    unsafe {
        SetConsoleOutputCP(65001);
    }
}

#[cfg(not(windows))]
fn enable_utf8_console() {}

fn main() {
    enable_utf8_console();
    std::process::exit(cli::run());
}

pub(crate) fn legacy_dispatch(cmd: &str, rest: Vec<String>) -> anyhow::Result<i32> {
    match cmd {
        "version" | "--version" | "-v" => {
            // --quiet flag silences output (used by SessionStart Gatekeeper
            // warmup on macOS — invoking the binary primes codesign /
            // notarization caches; we don't want stray stdout in the hook).
            let quiet = rest.iter().any(|a| a == "--quiet");
            if !quiet {
                println!(
                    "axhub-helpers {} (plugin v{}, schema {HOOK_SCHEMA_VERSION})",
                    env!("CARGO_PKG_VERSION"),
                    env!("CARGO_PKG_VERSION")
                );
            }
            Ok(0)
        }
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(0)
        }
        "redact" => {
            let raw = read_stdin()?;
            print!("{}", redact(&raw));
            Ok(0)
        }
        "statusline" => {
            print!("{}", current_statusline());
            Ok(0)
        }
        "path" => cmd_path(&rest),
        // token-init/token-import: US2 typed (cli::Commands) — legacy arm 제거.
        "token-gate" => cmd_token_gate(&rest),
        // post-install: US3 typed (cli::Commands::PostInstall) — legacy arm 제거.
        // classify-exit: US1 typed (cli::Commands::ClassifyExit) — legacy arm 제거.
        "preflight" => {
            if let Err(message) = validate_preflight_args(&rest) {
                return legacy_usage_error("preflight", message);
            }
            let run = run_preflight();
            println!("{}", serde_json::to_string(&run.output)?);
            Ok(run.exit_code)
        }
        "onboarding-detect" => {
            // `--json` is the only accepted flag (output is always JSON); accept
            // it as a no-op compat flag like `preflight`.
            for arg in &rest {
                if arg != "--json" {
                    return legacy_usage_error("onboarding-detect", "unknown option");
                }
            }
            onboarding_detect::run()
        }
        "resolve" => {
            if let Err(message) = validate_resolve_args(&rest) {
                return legacy_usage_error("resolve", message);
            }
            let run = run_resolve(&rest);
            println!("{}", serde_json::to_string(&run.output)?);
            Ok(run.exit_code)
        }
        // list-deployments: US3 typed (cli::Commands::ListDeployments) — legacy arm 제거.
        // routing-stats: US3 typed (cli::Commands::RoutingStats) — legacy arm 제거.
        "cleanup-audit" => cmd_cleanup_audit(&rest),
        // audit-clarify: US3 typed (cli::Commands::AuditClarify) — legacy arm 제거.
        "routing-dashboard" => cmd_routing_dashboard(&rest),
        "bootstrap" => cmd_bootstrap(&rest),
        "scaffold-detect" => run_scaffold_detect(&rest),
        "scaffold-dev" => run_scaffold_dev(&rest),
        "init-resume" => run_init_resume(&rest),
        "tenant-resolve" => run_tenant_resolve(&rest),
        "state-show" => cmd_state_show(&rest),
        "state-update" => cmd_state_update(&rest),
        "commit-gate" => cmd_commit_gate(),
        "test-classifier" => cmd_test_classifier(),
        "tdd-inject" => cmd_tdd_inject(),
        "karpathy-inject" => cmd_karpathy_inject(),
        "consent" => cmd_quality_consent(&rest),
        "prompt-route" => cmd_prompt_route(),
        "session-start" => cmd_session_start(),
        "mark" => cmd_mark(&rest),
        "emit-deploy-complete" => cmd_emit_deploy_complete(&rest),
        "deploy-prep" => cmd_deploy_prep(&rest),
        "deploy-preview-summary" => cmd_deploy_preview_summary(&rest),
        "deploy-approved-run" => cmd_deploy_approved_run(&rest),
        "migrate-plan" => run_migrate_plan(&rest),
        "migrate-guard" => cmd_migrate_guard(&rest),
        "migrate-summary" => cmd_migrate_summary(&rest),
        "publish-summary" => cmd_publish_summary(&rest),
        "rollback-summary" => cmd_rollback_summary(&rest),
        "team-summary" => cmd_team_summary(&rest),
        "env-summary" => cmd_env_summary(&rest),
        "github-summary" => cmd_github_summary(&rest),
        "resources-summary" => cmd_resources_summary(&rest),
        "review-scope-summary" => cmd_review_scope_summary(&rest),
        "auth-summary" => cmd_auth_summary(&rest),
        "install-summary" => cmd_install_summary(&rest),
        "update-summary" => cmd_update_summary(&rest),
        "inspect-config-summary" => cmd_inspect_config_summary(),
        "status-summary" => cmd_status_summary(&rest),
        "logs-summary" => cmd_logs_summary(&rest),
        "open-summary" => cmd_open_summary(&rest),
        "verify-summary" => cmd_verify_summary(&rest),
        "trace-summary" => cmd_trace_summary(&rest),
        "doctor-summary" => cmd_doctor_summary(&rest),
        "statusline-summary" => cmd_statusline_summary(&rest),
        "config" => cmd_config(&rest),
        "sync" => run_sync(&rest),
        "snippet" => run_snippet(&rest),
        "auth-refresh-bg" => cmd_auth_refresh_bg(),
        "plugin-latest-fetch-bg" => Ok(axhub_helpers::plugin_update::cmd_plugin_latest_fetch_bg()),
        "plugin-update-check" => Ok(axhub_helpers::plugin_update::cmd_plugin_update_check()),
        "plugin-drift-optout" => Ok(axhub_helpers::plugin_update::cmd_plugin_drift_optout()),
        "cli-latest-fetch-bg" => Ok(axhub_helpers::cli_drift::cmd_cli_latest_fetch_bg()),
        "cli-drift-optout" => Ok(axhub_helpers::cli_drift::cmd_cli_drift_optout()),
        // verify/trace/doctor: US2 typed (cli::Commands) — legacy arm 제거.
        // settings-merge: US2 typed (cli::Commands::SettingsMerge) — legacy arm 제거.
        // autowire-statusline: US1 typed (cli::Commands::AutowireStatusline) — legacy arm 제거.
        "orphan-stub" => cmd_orphan_stub(&rest),
        // diagnose: US3 typed (cli::Commands::Diagnose nested hitl) — legacy arm 제거.
        _ => {
            eprintln!("axhub-helpers: unknown subcommand \"{cmd}\"\n\n{USAGE}");
            Ok(64)
        }
    }
}

fn read_stdin() -> anyhow::Result<String> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;
    Ok(s)
}

fn legacy_usage_error(command: &str, message: &str) -> anyhow::Result<i32> {
    eprintln!("axhub-helpers {command}: {message}");
    Ok(64)
}

fn validate_preflight_args(rest: &[String]) -> Result<(), &'static str> {
    for arg in rest {
        match arg.as_str() {
            // Historical SKILLs invoke `preflight --json`; preflight already
            // renders JSON unconditionally, so keep this as a no-op compat flag.
            "--json" => {}
            _ => return Err("unknown option"),
        }
    }
    Ok(())
}

fn validate_resolve_args(rest: &[String]) -> Result<(), &'static str> {
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            // Historical SKILLs invoke `resolve ... --json`; resolve already
            // renders JSON unconditionally, so keep this as a no-op compat flag.
            "--json" => index += 1,
            "--intent" | "--user-utterance" => {
                if index + 1 >= rest.len() {
                    return Err("missing value");
                }
                index += 2;
            }
            _ => return Err("unknown option"),
        }
    }
    Ok(())
}

fn validate_deploy_prep_args(rest: &[String]) -> Result<(), &'static str> {
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--json" | "--refresh-in-flight" => index += 1,
            "--intent" | "--user-utterance" => {
                if index + 1 >= rest.len() {
                    return Err("missing value");
                }
                index += 2;
            }
            _ => return Err("unknown option"),
        }
    }
    Ok(())
}
fn out_json(v: Value) {
    println!("{}", v);
}

fn cmd_path(args: &[String]) -> anyhow::Result<i32> {
    let Some(kind) = args.first().map(String::as_str) else {
        eprintln!("axhub-helpers path: expected one of token-file, last-deploy-file, state-dir");
        return Ok(64);
    };
    let path = match kind {
        "token-file" => token_file(),
        "last-deploy-file" => last_deploy_file(),
        "state-dir" => state_dir(),
        _ => {
            eprintln!("axhub-helpers path: unknown path kind \"{kind}\"");
            return Ok(64);
        }
    };
    let Some(path) = path else {
        eprintln!("axhub-helpers path: cannot resolve {kind}");
        return Ok(65);
    };
    println!("{}", path.display());
    Ok(0)
}

pub(crate) fn cmd_token_init(json_output: bool) -> anyhow::Result<i32> {
    let (token, source) = match env_token() {
        Some(token) => (token, "env:AXHUB_TOKEN".to_string()),
        None => {
            if cli_auth_status_ok() {
                return store_and_report_token(
                    json_output,
                    "cli-auth-ok\n",
                    "axhub-cli-auth-status",
                );
            }
            let keychain = read_keychain_token();
            match keychain.token {
                Some(token) => (
                    token,
                    keychain
                        .source
                        .unwrap_or_else(|| "platform-keychain".to_string()),
                ),
                None => {
                    return emit_token_error(
                        json_output,
                        keychain.error.unwrap_or_else(|| {
                            "axhub token을 찾을 수 없어요. axhub auth login 또는 AXHUB_TOKEN을 사용해주세요."
                                .to_string()
                        }),
                    );
                }
            }
        }
    };
    store_and_report_token(json_output, &token, &source)
}

fn cli_auth_status_ok() -> bool {
    let axhub_bin = std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string());
    let out = axhub_helpers::axhub_cli::run_axhub_with_timeout(
        &axhub_bin,
        &["--json", "auth", "status"],
        std::time::Duration::from_secs(5),
    );
    out.exit_code == 0 && !out.timed_out && auth_status_stdout_is_authorized(&out.stdout)
}

/// Phase 3.5 token-freshness gate, Rust port of hooks/token-freshness-gate.sh.
///
/// Polls the local axhub compatibility token marker mtime against a
/// "session_ts" anchor (now - 30s) and falls back to an inline
/// `axhub auth status --json` probe on timeout. The
/// SKILL deploy Step 3.5 calls this subcommand directly (NOT through hooks.json),
/// so polling can block up to `AXHUB_GATE_POLL_INTERVAL * AXHUB_GATE_POLL_ITERATIONS`
/// seconds without hitting Claude Code hook timeouts.
///
/// Env contract (preserves hooks/token-freshness-gate.sh fixture semantics):
///   AXHUB_AUTH_BG_REFRESH=0     silent skip + exit 0
///   AXHUB_TOKEN_PATH            override token file path (test injection)
///   AXHUB_GATE_FAKE_NOW         override "now" in seconds (test injection)
///   AXHUB_GATE_POLL_INTERVAL    seconds per poll iteration (default 5)
///   AXHUB_GATE_POLL_ITERATIONS  number of poll iterations (default 6)
///   AXHUB_GATE_AUTH_PROBE       command for inline UNAUTHORIZED check
///                                (default: `axhub auth status --json`). Parsed with
///                                POSIX `shlex` (no `eval`) — safer than the sh
///                                wrapper but breaks pipes/env-assignments overrides.
///
/// Exit codes:
///   0   token marker fresh OR inline probe reports an authorized CLI auth state OR hook disabled
///   65  inline probe completed but did not match `"user_email"` (UNAUTHORIZED)
///   Any other error falls through to exit 0 (fail-open contract).
fn cmd_token_gate(_rest: &[String]) -> anyhow::Result<i32> {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    if hook_safety::is_hook_disabled("token-freshness-gate") {
        return Ok(0);
    }
    if std::env::var("AXHUB_AUTH_BG_REFRESH").as_deref() == Ok("0") {
        return Ok(0);
    }

    let now: i64 = std::env::var("AXHUB_GATE_FAKE_NOW")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
    let session_ts = now - 30;

    let token_path: PathBuf = match std::env::var("AXHUB_TOKEN_PATH").ok() {
        Some(s) => PathBuf::from(s),
        None => match token_file() {
            Some(p) => p,
            None => {
                eprintln!("[token-gate] token path resolve failed — inline auth status check");
                return inline_auth_check();
            }
        },
    };

    fn stat_mtime(p: &std::path::Path) -> i64 {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    if !token_path.exists() {
        eprintln!("[token-gate] token file missing — inline auth status check");
        return inline_auth_check();
    }

    let mtime = stat_mtime(&token_path);
    if mtime > session_ts {
        eprintln!("[token-gate] token mtime > session_ts, fresh");
        return Ok(0);
    }

    let poll_interval = std::env::var("AXHUB_GATE_POLL_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    let poll_iters = std::env::var("AXHUB_GATE_POLL_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(6);

    for poll in 0..poll_iters {
        std::thread::sleep(Duration::from_secs(poll_interval));
        let mtime = stat_mtime(&token_path);
        if mtime > session_ts {
            eprintln!(
                "[token-gate] token refreshed after {}s",
                (poll + 1) * poll_interval
            );
            return Ok(0);
        }
    }

    eprintln!("[token-gate] poll timeout");
    inline_auth_check()
}

fn inline_auth_check() -> anyhow::Result<i32> {
    // SECURITY (Reviewer Issue 4, PR #114): `AXHUB_GATE_AUTH_PROBE` 는 test
    // injection 전용 env 예요. `shlex::split` 가 shell metachar (`|` / `;` / `&&`)
    // 를 차단하지만, parts[0] 자체는 사용자 제어 binary path 예요. 신뢰할 수
    // 없는 환경 (untrusted CI runner, foreign repo) 에서는 이 env 를 설정하지
    // 마세요. 본 함수가 spawn 하는 명령은 호출 컨텍스트 (Claude Code SessionStart
    // hook = trusted boundary) 안에서만 사용자 의도된 probe 와 일치한다고
    // 가정해요. 운영 환경 default = `axhub auth status --json` 그대로 둬요.
    let probe = std::env::var("AXHUB_GATE_AUTH_PROBE")
        .unwrap_or_else(|_| "axhub auth status --json".to_string());
    let parts = match shlex::split(&probe) {
        Some(parts) if !parts.is_empty() => parts,
        _ => {
            eprintln!(
                "[token-gate] AXHUB_GATE_AUTH_PROBE shellwords parse failed — exit 0 fail-open"
            );
            return Ok(0);
        }
    };
    eprintln!("[token-gate] inline auth status check");
    match std::process::Command::new(&parts[0])
        .args(&parts[1..])
        .output()
    {
        Ok(out) => {
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            if auth_status_stdout_is_authorized(&stdout_str) {
                Ok(0)
            } else {
                eprintln!("[token-gate] auth UNAUTHORIZED, exit 65");
                Ok(65)
            }
        }
        Err(_) => {
            eprintln!("[token-gate] auth probe spawn failed — exit 0 fail-open");
            Ok(0)
        }
    }
}

fn auth_status_stdout_is_authorized(stdout: &str) -> bool {
    let Ok(parsed) = serde_json::from_str::<Value>(stdout.trim()) else {
        return stdout.contains("\"user_email\"");
    };
    if parsed
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status.eq_ignore_ascii_case("error"))
    {
        return false;
    }
    let data = if parsed.get("schema_version").is_some() || parsed.get("status").is_some() {
        parsed.get("data").unwrap_or(&parsed)
    } else {
        &parsed
    };
    data.get("user_email")
        .or_else(|| data.get("email"))
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
        || data
            .get("authenticated")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || data
            .get("logged_in")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

/// Phase 3.1 post-install — handles symlink/copy + .gitignore + post-commit hook
/// + disclosure marker on behalf of `bin/install.{sh,ps1}` (sh/ps1-absorption T7).
///
/// The shell wrapper still owns the chicken-and-egg download step (helper IS the
/// download target). Once `$TARGET_PATH` exists, the wrapper invokes this
/// subcommand with explicit `--target-name --bin-dir --link-path` flags so the
/// post-install steps run from a single Rust source of truth.
///
/// Flags:
///   --target-name <N>   downloaded binary filename (e.g. "axhub-helpers-darwin-arm64")
///   --bin-dir <D>       directory containing $TARGET (== plugin/bin)
///   --link-path <P>     stable link path (e.g. plugin/bin/axhub-helpers[.exe])
///   --repo-root <R>     optional git repo root for .gitignore / post-commit hook
///                       install (NULL when caller is not in a git tree)
///
/// Env semantics (codex finding #10 preserved):
///   AXHUB_NO_DISCLOSURE=1      skip disclosure marker write
///   AXHUB_SKIP_AUTODOWNLOAD=1  same effect (suppresses disclosure for CI / test
///                              paths that pipe install output through JSON parsers)
///   AXHUB_POSTCOMMIT_INSTALL=append → opt-in append to existing post-commit hook
pub(crate) fn cmd_post_install(
    target_name: Option<String>,
    bin_dir: Option<String>,
    link_path: Option<String>,
    repo_root: Option<String>,
) -> anyhow::Result<i32> {
    let (Some(target_name), Some(bin_dir), Some(link_path)) = (target_name, bin_dir, link_path)
    else {
        eprintln!(
            "axhub-helpers post-install: --target-name / --bin-dir / --link-path 가 필요해요"
        );
        return Ok(64);
    };
    let bin_dir = PathBuf::from(bin_dir);
    let link_path = PathBuf::from(link_path);
    let repo_root = repo_root.map(PathBuf::from);

    let target_path = bin_dir.join(&target_name);
    if !target_path.exists() {
        eprintln!(
            "axhub-helpers post-install: target binary 없어요: {}",
            target_path.display()
        );
        return Ok(64);
    }

    // sh/ps1-absorption Phase 3.1 (T7): symlink/copy + chmod remain in install.{sh,ps1}
    // wrapper because tests/install.test.sh exercises the OS/arch matrix with
    // stub binaries that cannot execute this subcommand. cmd_post_install owns
    // .gitignore + post-commit + disclosure marker — the parts that benefit
    // from a single Rust source of truth.

    // .gitignore + post-commit hook (only when --repo-root supplied).
    if let Some(repo) = repo_root.as_ref() {
        if repo.exists() {
            install_axhub_state_gitignore(repo);
            install_post_commit_hook(repo);
        }
    }

    // Disclosure marker write (dual-channel with autowire dispatcher per T3
    // codex ADR). Suppressed by AXHUB_NO_DISCLOSURE / AXHUB_SKIP_AUTODOWNLOAD
    // for CI / scripted contexts.
    if std::env::var("AXHUB_NO_DISCLOSURE").as_deref() != Ok("1")
        && std::env::var("AXHUB_SKIP_AUTODOWNLOAD").as_deref() != Ok("1")
    {
        write_disclosure_marker_from_post_install();
    }

    println!(
        "axhub-helpers post-install: {} → {} (target_name={target_name})",
        link_path.display(),
        target_name
    );
    Ok(0)
}

fn install_axhub_state_gitignore(repo: &std::path::Path) {
    let gitignore = repo.join(".gitignore");
    let entry = ".axhub-state/";
    if !gitignore.exists() {
        let body = format!("# axhub quality state (local-only)\n{entry}\n");
        let _ = fs::write(&gitignore, body);
        return;
    }
    let body = match fs::read_to_string(&gitignore) {
        Ok(s) => s,
        Err(_) => return,
    };
    let already_present = body.lines().any(|line| line.trim() == entry);
    if already_present {
        return;
    }
    // Preserve existing line ending — append entry on a new line. When the
    // existing body is non-empty we want a leading `\n` (blank separator if
    // body ends with `\n`, or missing terminator + separator if not). For an
    // empty `.gitignore` (touched but never written) we skip the leading `\n`
    // so the first line isn't a blank — Reviewer Issue 3 (PR #114).
    let separator = if body.is_empty() { "" } else { "\n" };
    let suffix = format!("{separator}# axhub quality state (local-only)\n{entry}\n");
    let _ = fs::write(&gitignore, format!("{body}{suffix}"));
}

fn install_post_commit_hook(repo: &std::path::Path) {
    let hook_path = repo.join(".git/hooks/post-commit");
    let post_commit_line = "\"${CLAUDE_PLUGIN_ROOT:-$HOME/.claude/plugins/axhub}/bin/axhub-helpers\" state-update --post-commit-promote 2>/dev/null || true";
    if hook_path.exists() {
        let body = fs::read_to_string(&hook_path).unwrap_or_default();
        if body.contains("state-update --post-commit-promote") {
            return; // already installed
        }
        if std::env::var("AXHUB_POSTCOMMIT_INSTALL").as_deref() == Ok("append") {
            let appended =
                format!("{body}\n# axhub quality review promotion\n{post_commit_line}\n");
            let _ = fs::write(&hook_path, appended);
            #[cfg(unix)]
            chmod_executable_best_effort(&hook_path);
        } else {
            eprintln!(
                "기존 .git/hooks/post-commit 감지됨. 자동 변경은 건너뛰어요. docs/MANUAL-POSTCOMMIT.md 를 참고해주세요."
            );
        }
        return;
    }
    // Create new hook.
    if let Some(parent) = hook_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = format!(
        "#!/usr/bin/env bash\nset -eu\n[ \"${{AXHUB_DISABLE_POSTCOMMIT:-0}}\" = \"1\" ] && exit 0\n{post_commit_line}\n"
    );
    if fs::write(&hook_path, body).is_ok() {
        #[cfg(unix)]
        chmod_executable_best_effort(&hook_path);
    }
}

#[cfg(unix)]
fn chmod_executable_best_effort(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(path, perms);
    }
}

fn write_disclosure_marker_from_post_install() {
    let Some(state) = axhub_helpers::runtime_paths::state_dir() else {
        return;
    };
    let marker_path = state.join("install-disclosure-shown.txt");
    let version = env!("CARGO_PKG_VERSION");
    let body = format!("v{version}\n");
    let _ = fs::create_dir_all(&state);
    let _ = fs::write(&marker_path, body);
}

pub(crate) fn cmd_token_import(json_output: bool) -> anyhow::Result<i32> {
    let raw = read_stdin()?;
    let Some(token) = extract_token_from_import_payload(&raw) else {
        return emit_token_error(
            json_output,
            "token-import 입력에서 access_token/token 값을 찾을 수 없어요.".to_string(),
        );
    };
    store_and_report_token(json_output, &token, "stdin")
}

fn env_token() -> Option<String> {
    std::env::var("AXHUB_TOKEN")
        .ok()
        .and_then(|value| normalize_token_candidate(&value))
}

fn extract_token_from_import_payload(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(token) = parse_keyring_value(trimmed) {
        return Some(token);
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return token_from_json_value(&value).and_then(normalize_token_candidate);
    }
    normalize_token_candidate(trimmed)
}

fn token_from_json_value(value: &Value) -> Option<&str> {
    if let Some(token) = value.as_str() {
        return Some(token);
    }
    ["access_token", "token", "AXHUB_TOKEN"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
}

fn normalize_token_candidate(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let candidate = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .unwrap_or(trimmed)
        .trim();
    if candidate.len() < 16
        || candidate
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return None;
    }
    Some(candidate.to_string())
}

fn store_and_report_token(json_output: bool, token: &str, source: &str) -> anyhow::Result<i32> {
    let path = store_plugin_token(token)?;
    if json_output {
        out_json(json!({
            "stored": true,
            "source": source,
            "token_file": path,
        }));
    } else {
        println!("axhub token stored at {} ({source})", path.display());
    }
    Ok(0)
}

fn emit_token_error(json_output: bool, message: String) -> anyhow::Result<i32> {
    if json_output {
        out_json(json!({
            "stored": false,
            "error": message,
        }));
    } else {
        eprintln!("{message}");
    }
    Ok(65)
}

fn store_plugin_token(token: &str) -> anyhow::Result<PathBuf> {
    let path = token_file()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve axhub plugin token file path"))?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve axhub plugin token directory"))?
        .to_path_buf();
    fs::create_dir_all(&parent)?;
    set_private_dir_mode(&parent).ok();
    write_private_file_no_follow(&path, token.as_bytes())?;
    Ok(path)
}

fn cmd_bootstrap(args: &[String]) -> anyhow::Result<i32> {
    if args.first().map(String::as_str) == Some("dependency-plan") {
        return cmd_bootstrap_dependency_plan(&args[1..]);
    }
    let stdin = if bootstrap_record_event(args).is_some() {
        Some(read_stdin()?)
    } else {
        None
    };
    let run = run_bootstrap(args, stdin.as_deref());
    println!("{}", serde_json::to_string(&run.output)?);
    Ok(run.exit_code)
}

fn bootstrap_record_event(args: &[String]) -> Option<&str> {
    let index = args.iter().position(|arg| arg == "--record")?;
    let event = args.get(index + 1)?.as_str();
    if event.starts_with("--") || !matches!(event, "apps_create" | "deploy_create") {
        return None;
    }
    Some(event)
}

/// Phase 25 PR 25.7 — nl-trigger-first verify/trace auto-suggest. D4 rule
/// (overview §10.4): print the natural Korean phrase only so vibe coders
/// learn `"확인해"` / `"왜 실패했어"` without dangling slash-command hints.
fn verify_trace_suggestion(command: &str, exit_code: i32) -> Option<String> {
    if command.starts_with("axhub deploy create") && exit_code == 0 {
        return Some("배포 완료. \"확인해\" 라고 말하면 라이브 확인해 드려요.".to_string());
    }
    // Only genuine server-attempt failures are trace-worthy. Client-side
    // pre-attempt gates are NOT: clap usage(2), CLI auth(4), dry-run preview(11),
    // and usage(64). Those never reached the deploy path, so "왜 실패했어" would
    // mislead. Helper auth(65) remains trace-worthy because the frozen helper
    // output contract and existing regression test expect the deploy-failure nudge.
    if command.starts_with("axhub deploy create")
        && exit_code != 0
        && !matches!(exit_code, 2 | 4 | 11 | 64)
    {
        return Some("배포 실패. \"왜 실패했어\" 라고 말하면 원인 추적해 드려요.".to_string());
    }
    if command.starts_with("axhub recover") && exit_code == 0 {
        return Some("복구 완료. \"확인해\" 라고 말하면 라이브 재확인해 드려요.".to_string());
    }
    None
}

pub(crate) fn cmd_classify_exit(arg_exit_code: i32, arg_stdout: &str) -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("classify-exit") {
        out_json(json!({}));
        return Ok(0);
    }
    let raw = read_stdin()?;
    if !raw.trim().is_empty() {
        let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let command = payload
            .pointer("/tool_input/command")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !command.starts_with("axhub ") {
            out_json(json!({}));
            return Ok(0);
        }
        let exit_code = payload
            .pointer("/tool_response/exit_code")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        let stdout = payload
            .pointer("/tool_response/stdout")
            .and_then(Value::as_str)
            .unwrap_or("");

        // Phase 25 PR 25.7 — verify/trace auto-suggest. Surfaces a Korean
        // nl-trigger first (D4 vibe-coder rule) so users learn the natural
        // phrase before discovering the slash command.
        let suggest = verify_trace_suggestion(command, exit_code);

        if exit_code == 0 && !command.starts_with("axhub deploy create") {
            // No empathy catalog entry, but we may still want to nudge.
            if let Some(msg) = suggest {
                out_json(json!({ "systemMessage": msg }));
            } else {
                out_json(json!({}));
            }
            return Ok(0);
        }
        let entry = classify(exit_code, stdout);
        let mut system_message = format!(
            "{}\n\n원인: {}\n\n해결: {}",
            entry.emotion, entry.cause, entry.action
        );
        if let Some(button) = entry.button {
            system_message.push_str(&format!("\n\n선택: {button}"));
        }
        if let Some(msg) = suggest {
            system_message.push_str("\n\n");
            system_message.push_str(&msg);
        }
        out_json(json!({ "systemMessage": system_message }));
        return Ok(0);
    }

    out_json(serde_json::to_value(classify(arg_exit_code, arg_stdout))?);
    Ok(0)
}

pub(crate) fn cmd_verify_deploy_artifact() -> anyhow::Result<i32> {
    const HOOK_NAME: &str = "verify-deploy-artifact";
    const LEGACY_HOOK_NAME: &str = "post-tool-verify-deploy-artifacts";
    if hook_safety::is_hook_disabled(HOOK_NAME) || hook_safety::is_hook_disabled(LEGACY_HOOK_NAME) {
        return Ok(0);
    }

    let raw = match read_stdin() {
        Ok(raw) => raw,
        Err(err) => {
            hook_safety::append_hook_error(HOOK_NAME, &err);
            return Ok(0);
        }
    };
    let payload: Value = match serde_json::from_str(&raw) {
        Ok(payload) => payload,
        Err(_) => return Ok(0),
    };
    let command = payload
        .pointer("/tool_input/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_axhub_deploy_create_command(command) {
        return Ok(0);
    }
    let exit_code = payload
        .pointer("/tool_response/exit_code")
        .and_then(Value::as_i64);
    if exit_code != Some(0) {
        return Ok(0);
    }
    let stdout = payload
        .pointer("/tool_response/stdout")
        .and_then(Value::as_str)
        .unwrap_or("");
    if stdout.trim().is_empty() {
        return Ok(0);
    }

    let result = axhub_helpers::verify_deploy_artifact::verify_user_app_artifact(stdout);
    if result.passed {
        return Ok(0);
    }

    let observed = result.violations.join("; ");
    let system_message = format!(
        "⚠️ 배포 artifact 검증에서 의심 신호를 발견했어요: {observed}. 라이브 결과를 한 번 더 확인해주세요."
    );
    let context = format!(
        "<axhub-deploy-verify>\n[axhub hook | deploy artifact verification]\nObserved: {observed}\nSuggested: run axhub-helpers verify --app-id <app> or inspect axhub deploy logs before claiming the app is live.\nSkip: AXHUB_DISABLE_HOOK=verify-deploy-artifact\n</axhub-deploy-verify>"
    );
    println!(
        "{}",
        hook_output::post_tool_use_context_with_system(&context, &system_message)
    );
    Ok(0)
}

fn is_axhub_deploy_create_command(command: &str) -> bool {
    let Some(rest) = command.trim_start().strip_prefix("axhub") else {
        return false;
    };
    let Some(rest) = strip_required_whitespace(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("deploy") else {
        return false;
    };
    let Some(rest) = strip_required_whitespace(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("create") else {
        return false;
    };
    rest.chars()
        .next()
        .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}

fn strip_required_whitespace(input: &str) -> Option<&str> {
    let trimmed = input.trim_start();
    (trimmed.len() != input.len()).then_some(trimmed)
}

fn cmd_state_show(args: &[String]) -> anyhow::Result<i32> {
    if !args.is_empty() && args != ["--json".to_string()] {
        eprintln!("axhub-helpers state-show: expected --json or no args");
        return Ok(64);
    }
    let repo_root = quality_state::repo_root_from_cwd()?;
    println!("{}", quality_state::state_show_json(&repo_root)?);
    Ok(0)
}

pub(crate) fn cmd_state_update(args: &[String]) -> anyhow::Result<i32> {
    let repo_root = quality_state::repo_root_from_cwd()?;
    match args.first().map(String::as_str) {
        Some("--review-acknowledged") => quality_state::update_review_acknowledged(&repo_root)?,
        Some("--post-commit-promote") => {
            if !hook_safety::is_postcommit_disabled() {
                quality_state::update_post_commit_promote(&repo_root)?;
            }
        }
        Some("--debug-acknowledged") => quality_state::mark_debug_acknowledged(&repo_root)?,
        Some("--shipped") => quality_state::mark_shipped(&repo_root)?,
        Some("--edit-event") => quality_state::update_edit_event(&repo_root)?,
        Some("--pull") => quality_state::mark_pull(&repo_root)?,
        Some(flag) => {
            eprintln!("axhub-helpers state-update: unknown option {flag}");
            return Ok(64);
        }
        None => {
            eprintln!("axhub-helpers state-update: missing option");
            return Ok(64);
        }
    }
    out_json(json!({"ok": true}));
    Ok(0)
}

pub(crate) fn cmd_commit_gate() -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("commit-gate") || hook_safety::is_quality_triggers_disabled() {
        println!("{}", hook_output::pre_tool_use_allow());
        return Ok(0);
    }
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if payload.get("tool_name").and_then(Value::as_str) != Some("Bash") {
        println!("{}", hook_output::pre_tool_use_allow());
        return Ok(0);
    }
    let command = payload
        .pointer("/tool_input/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !commit_gate::is_commit_or_push(command) {
        println!("{}", hook_output::pre_tool_use_allow());
        return Ok(0);
    }
    let repo_root = quality_state::repo_root_from_cwd()?;
    let state = quality_state::QualityState::load_or_init(&repo_root)?;
    match commit_gate::evaluate_bash_command(command, &state, &repo_root) {
        commit_gate::GateDecision::Allow => println!("{}", hook_output::pre_tool_use_allow()),
        commit_gate::GateDecision::Ask(reason) => {
            println!("{}", hook_output::pre_tool_use_ask(&reason))
        }
    }
    Ok(0)
}

pub(crate) fn cmd_test_classifier() -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("test-classifier")
        || hook_safety::is_quality_triggers_disabled()
    {
        out_json(json!({}));
        return Ok(0);
    }
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let command = payload
        .pointer("/tool_input/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    let exit_code = payload
        .pointer("/tool_response/exit_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let failed_event = payload.get("hook_event_name").and_then(Value::as_str)
        == Some("PostToolUseFailure")
        || exit_code != 0;
    if payload.get("tool_name").and_then(Value::as_str) == Some("Bash")
        && failed_event
        && axhub_helpers::test_classifier::is_test_command(command)
    {
        let repo_root = quality_state::repo_root_from_cwd()?;
        quality_state::mark_test_failure(&repo_root)?;
    }
    out_json(json!({}));
    Ok(0)
}

pub(crate) fn cmd_tdd_inject() -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("tdd-inject") || hook_safety::is_quality_triggers_disabled() {
        out_json(json!({}));
        return Ok(0);
    }
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if let Some(ctx) = axhub_helpers::tdd_inject::additional_context_for_payload(&payload) {
        println!("{}", hook_output::pre_tool_use_context(&ctx));
    } else {
        out_json(json!({}));
    }
    Ok(0)
}

fn cmd_karpathy_inject() -> anyhow::Result<i32> {
    if hook_safety::is_karpathy_disabled() {
        out_json(json!({}));
        return Ok(0);
    }
    if let Some(ctx) = axhub_helpers::karpathy_inject::user_prompt_karpathy_inject()? {
        println!("{}", hook_output::user_prompt_context(&ctx));
    } else {
        out_json(json!({}));
    }
    Ok(0)
}

fn cmd_quality_consent(args: &[String]) -> anyhow::Result<i32> {
    let state = match args.first().map(String::as_str) {
        Some("--enable") => Some(true),
        Some("--disable") => Some(false),
        Some("--show") | None => None,
        Some(flag) => {
            eprintln!("axhub-helpers consent: unknown option {flag}");
            return Ok(64);
        }
    };
    let path = state_dir()
        .unwrap_or_else(|| PathBuf::from(".axhub-state"))
        .join("quality-consent.json");
    if let Some(enabled) = state {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({"megaskill_enabled": enabled}))?,
        )?;
    }
    let enabled = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("megaskill_enabled").and_then(Value::as_bool))
        .unwrap_or(false);
    out_json(json!({"megaskill_enabled": enabled}));
    Ok(0)
}

const MAX_LIST_DEPLOYMENTS_LIMIT: usize = 100;

const DYNAMIC_TABLE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 테이블 변경 내용을 확인할게요.
This is an AXHub hosted-app table schema/row/grant request. Do not inspect or edit local app code, local database files, `server.js`, `package.json`, ORM files, `.env`, SQL migrations, or QA result files.
Before any schema/row/grant mutation, check login/current app/table availability/column types, show a Korean preview of the target app, table, action, and columns/row summary, then ask for explicit approval.
The visible preview must not include raw CLI command lines. The actual command belongs only in the Bash tool call after approval.
In Claude Desktop, do not call AskUserQuestion, Question, or any question-card tool for this approval because raw question JSON may become visible. Ask in normal chat: `이대로 만들까요? 진행 또는 취소라고 답해 주세요.` Then stop. If the next user reply clearly says `진행`, continue with the approved execution.
Bash titles must be Korean only, such as `로그인 상태 확인`, `테이블 상태 확인`, `테이블 변경 준비`, or `테이블 변경 실행`.
Do not write route labels, slash commands, skill names, workflow/워크플로, preflight, command names, raw command lines, raw question JSON, raw JSON fields, raw IDs, raw emails, raw app slugs, local file contents, repo inspection, English tool-title fragments, or A/B implementation labels in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const ONBOARDING_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 처음 설정을 확인할게요.
Internal route target: Skill("axhub:onboarding"). This is hidden control text for the selected route; never announce internal routing, never expose the skill name, and never synthesize a separate "Getting started" flow from memory. The installed onboarding skill contract is authoritative. If the host does not expose an explicit Skill surface, follow the onboarding skill body verbatim as the control contract.
This is AXHub first-run onboarding. Do not answer with generic project advice, a menu that tells the user to say another phrase, or a handoff that redirects the user to ask for a new app.
Continue in this same turn using the onboarding detect-first loop: check the current gaps read-only first, handle only the first actionable gap, then re-detect before moving on.
Do not run installers, dependency installs, GitHub approval, app creation, repo creation, deployment, settings writes, or PATH mutation before an explicit user approval for that exact action.
If there is no local app manifest and the directory is empty, ask the onboarding first-app handoff question exactly: `첫 앱 만들래요?` with `네` and `아니요` choices. Only after `네`, transition to the init skill handoff/template flow; do not ask template choices first, and do not run `axhub apps bootstrap --execute` until the user has approved the preview.
Terminal states must be one of VIBE_READY, READY_WITH_USER_ACTION, SAFE_STOP_NONINTERACTIVE, or BLOCKED_UNSUPPORTED.
Bash titles must be Korean only, such as `온보딩 상태 확인`, `설치 상태 확인`, `GitHub 연결 확인`, `앱 준비 확인`, or `최종 점검`.
Do not write route labels, slash commands, Skill("axhub:onboarding"), skill names, workflow/워크플로 labels, TodoWrite availability, preflight internals, raw JSON fields, raw IDs, raw emails, file paths, installer URLs, raw command names, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const CONNECTORS_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 데이터베이스 연결을 준비할게요.
This is an AXHub external database connector setup/management request. Do not inspect or edit local app code, `server.js`, `package.json`, dependency files, ORM files, `.env`, or local DATABASE_URL wiring.
Handle this as an AXHub external database connection setup. If information is missing, ask naturally for the connector name, database engine, workspace, host, port, database name, username, SSL mode, and a safe credentials-file/input path. Do not ask for secret values in chat.
Before any create/update/delete, check the current workspace and existing connectors, show a Korean preview, and ask for explicit approval.
Bash titles must be Korean only, such as `커넥터 상태 확인`, `커넥터 목록 확인`, `커넥터 변경 준비`, or `커넥터 변경 실행`.
Do not write route labels, slash commands, skill names, local file contents, repo inspection, package-install plans, app-code DATABASE_URL setup, raw JSON fields, raw IDs, raw emails, preflight narration, English tool-title fragments, or A/B implementation labels in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const DATA_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 데이터 리소스를 확인할게요.
This is an AXHub governed data read/describe/snippet request. Do not inspect local app code, repo files, `.env`, `server.js`, package files, ORM files, QA result files, or plugin source files.
Use AXHub data-resource lookup only. If no resource is registered, say naturally that no connected data resource was found and that a database connection must be added first.
Never run a live data read until the user explicitly approves it after seeing the target, columns, row limit, and query shape.
Bash titles must be Korean only, such as `로그인 상태 확인`, `데이터 리소스 확인`, `데이터 설명`, `실데이터 확인`, or `스니펫 준비`.
Do not write route labels, slash commands, skill names, workflow/워크플로, preflight, catalog 조회, catalog 비어있음, connector 목록, catalog kinds, raw JSON fields, raw IDs, raw emails, account scopes, raw app slugs, governance/path-guessing jargon, command names, English tool-title fragments, A/B implementation labels, local file contents, or route-conversion narration in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const RESOURCES_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 리소스 정리 방식을 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 리소스 현황 확인
Bash command: `axhub-helpers resources-summary --user-utterance "<latest user sentence>"`
This is an AXHub gateway resource organization request for external DB tables/views/resources. It is not local filesystem cleanup, repo cleanup, QA artifact cleanup, shim cleanup, or git working-tree cleanup.
Check login/workspace/resource availability first. If the user did not name the exact operation or resource, ask naturally which resource organization action they want: 목록 확인, 이름 변경, 이동, 태그 정리, 등록, or 삭제. Do not run any mutation before explicit approval.
Bash titles must be Korean only, such as `로그인 상태 확인`, `리소스 현황 확인`, `리소스 변경 준비`, or `리소스 변경 실행`.
Do not inspect local files, repo files, `.shim`, `.omc`, QA result files, git status, package files, plugin source files, or local cleanup candidates.
After the tool call, copy the Korean stdout as the answer. Do not add command names, JSON field names, internal labels, file contents, English tool-title fragments, or a claim that resource changes are impossible.
Do not write route labels, slash commands, skill names, workflow/워크플로, preflight, catalog kinds, connector/resource, raw question JSON, command names, raw command lines, raw JSON fields, raw IDs, raw emails, local file paths, local artifact names, English tool-title fragments, or terse ambiguity labels such as `모호` in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const GITHUB_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: GitHub 연결 상태를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: GitHub 연결 상태 확인
Bash command: `axhub-helpers github-summary --user-utterance "<latest user sentence>"`
This is an AXHub hosted-app GitHub repository connection request. Do not answer from local git remotes, git config, GitHub CLI, GitHub PR state, repo source files, package files, `.git`, or QA result files.
After the tool call, copy the Korean stdout as the answer. Do not add command names, JSON field names, raw IDs, raw emails, installation IDs, local git remote evidence, file contents, route labels, slash commands, skill names, ToolSearch narration, or English tool-title fragments.
Connect/disconnect/create repo/add remote/push are mutations. Do not run any mutation before showing a Korean preview and receiving explicit approval.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const MIGRATE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 가져오기 상태를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 가져오기 상태 확인
Bash command: `axhub-helpers migrate-summary --user-utterance "<latest user sentence>"`
This is an AXHub existing-app import/migration readiness request. Do not answer from local server checks, package scripts, git release state, generic deployment advice, QA result files, or previous deployment failure state.
After the tool call, copy the Korean stdout as the answer. Do not add command names, raw JSON fields, raw deploy status fields, local server evidence, file contents, route labels, slash commands, skill names, ToolSearch narration, emoji, or English tool-title fragments.
App registration, GitHub connection, env writes, and deployment are mutations. Do not run any mutation before showing a Korean preview and receiving explicit approval.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const PUBLISH_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 공개 심사 준비를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 공개 심사 준비 확인
Bash command: `axhub-helpers publish-summary --user-utterance "<latest user sentence>"`
This is an AXHub marketplace/public review submission request. Do not read quality files, local state files, plugin source, repository files, package files, or prior QA artifacts before showing this preparation summary.
After the tool call, copy the Korean stdout as the answer. Do not add command names, raw JSON fields, raw review status fields, route labels, slash commands, skill names, ToolSearch narration, preflight wording, quality-state details, file contents, English tool-title fragments, or internal workflow labels.
Submission is an external marketplace mutation. Do not submit until the user provides a review note, sees a Korean preview, and explicitly approves.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const DEPLOY_ROLLBACK_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 되돌릴 수 있는 배포를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 배포 되돌리기 확인
Bash command: `axhub-helpers rollback-summary --user-utterance "<latest user sentence>"`
This is an AXHub deployment restore/rollback/recover request. Do not expose whether it maps to rollback or recover; do not call recover, rollback, deploy create, or deploy rollback yet.
After the tool call, copy the Korean stdout as the answer. Do not add command names, route labels, slash commands, skill names, preflight, raw JSON fields, raw deploy IDs, raw commit hashes, raw status names, `commit_not_found`, `no-op`, app IDs/slugs, local file/repo inspection, English tool-title fragments, or workflow labels.
Any rollback/redeploy is destructive/external. Do not mutate until the user sees the Korean preview and explicitly approves.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const TEAM_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 팀 작업을 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 팀 작업 확인
Bash command: `axhub-helpers team-summary --user-utterance "<latest user sentence>"`
This is an AXHub workspace team invitation, invitation-list, or hosted-app access request. Do not reinterpret it as Claude/OMC multi-agent team setup, local code collaboration, project staffing, or general task delegation.
After the tool call, copy the Korean stdout as the answer. Do not add command names, raw JSON fields, raw tenant IDs, raw user IDs, raw emails unless the user typed that email, route labels, slash commands, skill names, ToolSearch narration, preflight wording, tenant/workspace implementation terms, OMC/Claude team comparisons, file contents, English tool-title fragments, or internal workflow labels.
Sending or canceling invitations and changing app access are external permission mutations. Do not mutate until the user provides the target person, sees a Korean preview, and explicitly approves.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const QUALITY_REVIEW_ROUTING_HINT: &str = r#"<axhub-routing-hint>
첫 문장: 코드 리뷰를 시작할게요.
첫 확인: Bash title `변경 범위 확인`, 실행 `axhub-helpers review-scope-summary --user-utterance "<latest user sentence>"`.
그 다음: 확인 요약의 실제 변경 범위를 기준으로 소스/설정 파일을 읽고 한국어 코드 리뷰를 작성.
큰 변경이라고 나오면 파일을 읽기 전에 `변경량이 커서 먼저 범위를 정할게요. 전체를 볼까요, 핵심 파일만 볼까요?`라고 묻고 대기.
마무리: 리뷰가 끝나면 Bash title `리뷰 상태 저장`, 실행 `axhub-helpers state-update --review-acknowledged`.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const QUALITY_DEBUG_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 원인을 좁혀볼게요.
This is a direct code/test debugging request. Use the dedicated AXHub debug workflow now; do not let background quality auto-mode process this as a generic response first.
Do not read `.axhub-state/quality.json` before starting the direct debug workflow. Do not run generic file-listing or repo-survey commands before the workflow preflight/symptom step.
In Claude Desktop, debug directly in the current session first. Do not call Task/Subagent/agent delegation and do not use visible `디버그 위임` unless the user explicitly asks for a separate agent.
Bash titles must be Korean only, such as `문제 신호 확인`, `최근 실패 확인`, or `디버그 상태 저장`.
If auth is OK, visible text must say only `로그인되어 있어요.` or the next natural action. Never display account email, raw user id, account scope, exact expiry, or raw preflight fields.
After finishing the debug pass, update debug state with `axhub-helpers state-update --debug-acknowledged` using the Korean title `디버그 상태 저장`.
Do not write route labels, slash commands, skill names, quality auto-mode, workflow/워크플로 labels, TodoWrite availability, preflight internals, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, file-listing narration, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const QUALITY_DIAGNOSE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 진단 루프를 준비할게요.
This is a direct auto-diagnose loop request. Use the dedicated AXHub diagnose workflow now; do not let background quality auto-mode process this as a generic response first.
Bash titles must be Korean only, such as `진단 루프 준비`, `실패 신호 확인`, or `검증 결과 확인`.
In Claude Desktop, do not expose raw AskUserQuestion JSON. If a hypothesis choice is needed, ask in normal chat with short Korean choices and wait.
If auth is OK, visible text must say only `로그인되어 있어요.` or the next natural action. Never display account email, raw user id, account scope, exact expiry, or raw preflight fields.
Do not write route labels, slash commands, skill names, quality auto-mode, workflow/워크플로 labels, preflight internals, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, file-listing narration, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const QUALITY_PLAN_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 변경 계획을 잡아볼게요.
This is a direct planning request for code/design changes. Use the dedicated AXHub plan workflow now; do not let background quality auto-mode process this as a generic response first.
Bash titles must be Korean only, such as `계획 범위 확인`, `영향 범위 확인`, or `계획 정리`.
Do not implement changes during the planning workflow unless the user later gives a separate execution request.
If auth is OK, visible text must say only `로그인되어 있어요.` or the next natural action. Never display account email, raw user id, account scope, exact expiry, or raw preflight fields.
Do not write route labels, slash commands, skill names, quality auto-mode, workflow/워크플로 labels, TodoWrite availability, preflight internals, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, file-listing narration, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const QUALITY_SHIP_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 출시 준비 상태를 확인할게요.
This is a direct PR/release readiness request. Use the dedicated AXHub ship workflow now; do not let background quality auto-mode process this as a generic response first.
In Claude Desktop, prepare the readiness summary directly in the current session first. Do not call Task/Subagent/agent delegation and do not use visible `ship 위임` unless the user explicitly asks for a separate agent.
Bash titles must be Korean only, such as `출시 준비 확인`, `리뷰 상태 확인`, or `출시 상태 저장`.
If auth is OK, visible text must say only `로그인되어 있어요.` or the next natural action. Never display account email, raw user id, account scope, exact expiry, or raw preflight fields.
After finishing the readiness pass, update ship state with `axhub-helpers state-update --shipped` using the Korean title `출시 상태 저장` only when the ship workflow actually completed.
Do not create a PR, push, publish, release, or deploy before explicit approval. Do not write route labels, slash commands, skill names, quality auto-mode, workflow/워크플로 labels, TodoWrite availability, preflight internals, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, file-listing narration, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const QUALITY_TDD_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 테스트부터 잡아볼게요.
This is a direct TDD-cycle request. Use the dedicated AXHub TDD workflow now; do not let background quality auto-mode process this as a generic response first.
Bash titles must be Korean only, such as `TDD 대상 확인`, `테스트 확인`, or `테스트 실행`.
Start by clarifying the target behavior when it is missing; do not silently pick an unrelated module.
If auth is OK, visible text must say only `로그인되어 있어요.` or the next natural action. Never display account email, raw user id, account scope, exact expiry, or raw preflight fields.
Do not write route labels, slash commands, skill names, quality auto-mode, workflow/워크플로 labels, TodoWrite availability, preflight internals, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, file-listing narration, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const DEPLOY_STATUS_ROUTING_HINT: &str = r#"<axhub-routing-hint>
[axhub hook | deploy status]
Observed: deploy progress/status prompt.
Suggested: use the AXHub status workflow now; do not answer from repo/git memory. If auth missing, say 로그인/토큰 확인.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const DEPLOY_LOGS_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 로그를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 로그 확인
Bash command: `axhub-helpers logs-summary --user-utterance "<latest user sentence>"`
This is an AXHub app deployment log request. Do not inspect local repo log files, `.omc`, `.claude`, git logs, package logs, or plugin cache files.
After the tool call, copy the Korean stdout as the answer. Do not add command names, JSON field names, internal labels, file contents, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const OPEN_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 앱 페이지를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 앱 페이지 확인
Bash command: `axhub-helpers open-summary --user-utterance "<latest user sentence>"`
This is an AXHub app browser/open request. Do not inspect QA result files, `.omc`, `.claude`, plugin cache files, git logs, Chrome MCP state, or browser extension state.
After the tool call, copy the Korean stdout as the answer. Do not add command names, JSON field names, internal labels, file contents, ToolSearch narration, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const VERIFY_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 배포가 실제로 열리는지 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 배포 검증
Bash command: `axhub-helpers verify-summary --user-utterance "<latest user sentence>"`
This is an AXHub live-deploy verification request. Do not narrate routing, preflight, stale cache IDs, JSON fields, deploy IDs, user emails, raw status names, or intermediate fallback attempts.
After the tool call, copy the Korean stdout as the answer. Do not add command names, internal labels, raw JSON field names, file contents, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const TRACE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 배포 기록을 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 배포 기록 확인
Bash command: `axhub-helpers trace-summary --user-utterance "<latest user sentence>"`
This is an AXHub deployment failure-cause request. Do not narrate routing, slash commands, skill names, preflight, deploy IDs, raw status names, JSON field names, failure_reason, matched_patterns, build_log_errors, local QA files, plugin source files, or English tool-title fragments.
After the tool call, copy the Korean stdout as the answer. Do not add a table, raw IDs, command names, internal labels, or another investigation layer.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const ROUTING_STATS_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 라우팅 통계를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 라우팅 통계 확인
Bash command: `axhub-helpers routing-stats --since 7d`
This is an AXHub plugin routing analytics request. Do not inspect QA result files, desktop QA logs, repo files, plugin source files, git history, `.omc`, `.claude`, or local project notes.
After the tool call, summarize the Korean stdout briefly. Do not add command names, internal labels, raw file contents, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const ENV_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 환경변수를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 환경변수 확인
Bash command: `axhub-helpers env-summary --user-utterance "<latest user sentence>"`
This is an AXHub app environment-variable request. Do not inspect shell environment variables, `.env` files, repo files, plugin source files, `.omc`, `.claude`, git history, or QA result files.
After the tool call, copy the Korean stdout as the answer. Do not add command names, internal labels, raw JSON fields, raw values, secret values, preflight narration, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const DOCTOR_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
Internal route target: Skill("axhub:doctor"). This is hidden control text for the selected route; never announce internal routing, never expose the skill name, and never synthesize a separate diagnostic flow from memory. The installed doctor skill contract is authoritative. If the host exposes an explicit Skill surface, say the first visible sentence below, then invoke Skill("axhub:doctor") as the internal diagnostic surface before any Bash tool call. If the host does not expose an explicit Skill surface, follow the fallback Bash contract below verbatim.
First visible sentence, exactly: 설치 상태를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 설치 상태 확인
Bash command: `axhub-helpers doctor-summary --user-utterance "<latest user sentence>"`
This is an AXHub CLI/plugin/auth readiness check. Do not install, update, login, logout, or modify settings unless the user explicitly asks for that action.
After the tool call, copy the Korean stdout as the answer. Do not add command names, slash commands, Skill("axhub:doctor"), skill names, internal labels, raw JSON fields, raw user emails, filesystem paths, preflight narration, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const INSTALL_CLI_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 설치 상태를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 설치 상태 확인
Bash command: `axhub-helpers install-summary --user-utterance "<latest user sentence>"`
This is an AXHub CLI install request. First check whether the CLI is already installed. If it is already installed, stop after the Korean stdout and do not run an installer. If it is missing, ask for explicit approval before any installer command.
After the tool call, copy the Korean stdout as the answer. Do not add command names, slash commands, skill names, internal labels, raw JSON fields, auth status fields, filesystem paths, installer URLs, preflight narration, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const UPDATE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 업데이트를 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 업데이트 확인
Bash command: `axhub-helpers update-summary --user-utterance "<latest user sentence>"`
This is an AXHub CLI update check. Do not run update apply, install, doctor, auth login/logout, plugin update, cache scans, or compatibility diagnostics before this summary. The summary helper itself runs the real read-only CLI update check.
After the tool call, copy the Korean stdout as the answer. Do not add command names, slash commands, skill names, internal labels, raw JSON fields, has_update, filesystem paths, installer URLs, plugin update suggestions, preflight narration, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const STATUSLINE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 상태바 설정을 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 상태바 설정
Bash command: `axhub-helpers statusline-summary --user-utterance "<latest user sentence>"`
This is an AXHub status bar enable request. Preserve an existing third-party status bar by default.
After the tool call, copy the Korean stdout as the answer. Do not add command names, slash commands, skill names, internal labels, raw file paths, raw settings JSON, existing command strings, exit codes, scope fallback narration, statusLine/wire/settings-merge terminology, or English tool-title fragments.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const CLARIFY_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 어떤 걸 도와드릴까요?
Use exactly one question card. Header, exactly: 작업 선택
Question, exactly: 어떤 걸 도와드릴까요?
Visible options, exactly:
- 환경 점검 — 설치, 로그인, 버전 상태를 확인해요
- 앱 배포 — 현재 프로젝트를 올릴 준비를 해요
- 앱과 리소스 조회 — 내 앱, 리소스, 테이블을 확인해요
- 문제 원인 보기 — 상태, 로그, 실패 원인을 확인해요
- 처음부터 안내 — 가능한 작업을 한눈에 보여줘요
If the question-card tool requires option values, set each value to exactly the same Korean text as its visible label. Do not put hidden option values that contain English skill slugs or slash-command forms in the question-card JSON.
After the user chooses an option, do not call the Claude Skill tool, do not invoke any slash command, and do not write a route-transition sentence.
If the user chooses 환경 점검, visible sentence exactly: 설치 상태를 확인할게요. Then use exactly one Bash tool call with description/title exactly: 설치 상태 확인. Bash command: `axhub-helpers doctor-summary --user-utterance "<original broad user sentence>"`. Copy the Korean stdout as the answer.
If the user chooses 앱 배포, visible sentence exactly: 배포 준비를 확인할게요. Before the Bash call, make sure it runs in the user-visible app folder; if an added folder is the only Vite/React app, `cd` there first, and if multiple app folders are plausible, ask which folder and stop. Then use exactly one Bash tool call with description/title exactly: 배포 준비 확인. Bash command: `axhub-helpers deploy-preview-summary --user-utterance "<original broad user sentence>"`. If stdout says `axhub 매니페스트(axhub.yaml)가 없어요.`, show the local choices React/Vite로 초기화, 다른 템플릿 선택, 취소, then stop without deploy approval. Otherwise show the Korean preview and ask for explicit approval before any deploy execution.
If the user chooses 앱과 리소스 조회, visible sentence exactly: 앱과 리소스를 확인할게요.
If the user chooses 문제 원인 보기, visible sentence exactly: 문제 원인을 확인할게요.
Do not say the prompt is vague. Do not append parenthesized English/internal labels to any option. Do not show slash commands, skill names, command mappings, implementation values, route labels, raw tool names, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const INIT_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
Internal route target: Skill("axhub:init"). This is hidden control text for the selected route; never announce internal routing, never expose the skill name, and never synthesize a generic app-ideation flow. The installed init skill contract is authoritative. If the host exposes an explicit Skill surface, say the first visible sentence below, then invoke Skill("axhub:init") as the internal app-creation surface before any Bash tool call. If the host does not expose an explicit Skill surface, follow the fallback Bash contract below verbatim.
First visible sentence, exactly: 새 앱을 만들 수 있는 템플릿을 확인할게요.
This is an AXHub app creation request. It is not generic app ideation, a local coding-project brainstorm, or a general "what kind of app do you want" flow.
Start by checking repo-local resume state with one Bash tool call when possible. Bash description/title, exactly: 앱 생성 상태 확인. Bash command: `axhub-helpers init-resume route --json`
If the resume state says there is an incomplete app creation, ask whether to continue it before showing fresh templates.
If there is no resumable creation, read the backend template registry before asking for a template. Bash description/title, exactly: 템플릿 확인. Bash command: `axhub apps templates list --json`
Ask only from templates returned by that backend registry, using human-friendly labels and at most three explicit visible choices. Do not invent templates that are absent from the backend registry. Do not add an explicit 기타 option because Claude Desktop adds its own free-form 기타/Other option. Do not offer generic choices such as 웹 앱, API/백엔드, CLI 도구, or an `axhub 앱` catch-all option.
After the template is chosen, surface the account-level GitHub App install state once as a read-only step. Bash command: `axhub github accounts list --json`; read `install_url` from `data.accounts[]` and show it next to each account login. Always show `install_url`, INCLUDING for already-installed accounts — there it is the entry point for adding another org/account, so never hide or skip it just because an account is already installed. "Non-blocking" means it does not force installation or block app creation; it does NOT mean omitting the link. A from-scratch install is still handled later by the bootstrap saga when a GitHub connection is needed.
After that GitHub App surface, ask for the app name, then show the creation preview and ask for explicit approval.
Do not run `axhub apps bootstrap --execute`, create repositories, connect GitHub, install dependencies, start dev servers, or deploy until the user has approved the preview for that exact action.
Bash titles must be Korean only, such as `앱 생성 상태 확인`, `템플릿 확인`, `앱 생성 준비`, or `앱 생성 실행`.
Do not write route labels, slash commands, skill names, workflow/워크플로, preflight, raw question JSON, command mappings, raw helper JSON, raw IDs, raw emails, file paths, or English tool-title fragments in visible text.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const APP_LIFECYCLE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence:
- pause intent: <app> 앱을 잠깐 멈출 준비를 할게요.
- resume intent: <app> 앱을 다시 켤 준비를 할게요.
- fork intent: <app> 앱을 복제할 준비를 할게요.
This is an AXHub hosted app lifecycle request. Do not inspect local Next.js/dev-server processes, ports, ps/lsof output, package scripts, or local server state.
Continue in this same answer flow. Do not invoke slash commands, and do not write route-conversion or implementation explanation sentences.
Verify login/current app, find the AXHub app, explain the service impact in Korean, then ask exactly `앱 변경을 실행할까요?` with visible options `취소` and `진행` before any change.
Bash titles only: `앱 상태 확인`, `앱 찾기`, `앱 변경 실행`.
If another lookup is needed, say exactly `앱을 한 번 더 확인할게요.` Do not describe identifier lookup.
When summarizing app metadata, translate raw enum values into Korean labels only. Say `비공개`, `공개`, `개발 단계`, or `운영 단계`; never write raw enum words such as `private`, `public`, `development`, `production`, and never write mixed labels such as `비공개 (private)`.
Human-visible flow:
- Use `앱 상태 확인` for `axhub-helpers preflight --json`.
- Use `앱 찾기` for `axhub apps list --json` or the narrow AXHub app lookup needed to identify the named hosted app.
- After the user chooses `진행`, do not write any visible sentence before tool calls. Never say `User chose`, `execute suspend`, `execute resume`, or similar implementation narration.
- Use exactly one `앱 변경 실행` Bash tool call for the matching top-level `axhub apps suspend|resume|fork ... --execute --json >/dev/null` command, and do not leave raw JSON stdout visible in the tool panel.
- The first `앱 변경 실행` tool call with exit code 0 is terminal. Do not verify by re-running the mutation, and do not continue to a second app-changing command.
If an internal safety check blocks the command, do not explain internals in visible chat; retry the same approved top-level command exactly once, then say `앱 변경을 시작하지 못했어요. 다시 시도해 주세요.` if it still fails.
Do not say route labels, slash commands, skill names, preflight details, internal app/context fields, auth results, runtime words, lifecycle verbs in English, raw JSON, raw identifiers, owner names, English tool-title fragments, or parenthesized internal labels.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const DEPLOY_CREATE_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 배포 준비를 확인할게요.
Use exactly one Bash tool call before asking for approval. Bash description/title, exactly: 배포 준비 확인
Bash command: `axhub-helpers deploy-preview-summary --user-utterance "<latest user sentence>"`
This is an AXHub live deployment request, not a generic release, git release, Vercel, Netlify, Cloudflare, Fly, Render, or Railway deploy.
Before that Bash command, make sure it runs in the user-visible app folder; if an added folder is the only Vite/React app, `cd` there first, and if multiple app folders are plausible, ask which folder and stop.
After the tool call, copy the Korean preview stdout and ask for explicit approval. If stdout says `axhub 매니페스트(axhub.yaml)가 없어요.`, show the local choices React/Vite로 초기화, 다른 템플릿 선택, 취소, then stop without deploy approval. Do not read or summarize the long deploy skill body before this preview card is shown.
After explicit approval, use exactly one Bash tool call. Bash description/title, exactly: 배포 실행
Bash command: `axhub-helpers deploy-approved-run --user-utterance "<latest user sentence>"`
Copy that Korean stdout as the result. Do not invoke a skill again after approval.
Do not write route labels, slash commands, command mappings, skill names, `Invoke deploy skill`, `Read rest of SKILL`, `Read full SKILL`, `Route=axhub`, `preflight`, `deploy-prep`, raw helper JSON, raw IDs, raw account email, or English tool-title fragments in the visible answer.
Use Korean Bash tool titles only, such as `배포 준비 확인`, `배포 실행`, or `배포 상태 확인`.
On auth error, explain token/login expiry safely in Korean and ask before login.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const APIS_ROUTING_HINT: &str = r#"<axhub-routing-hint>
[axhub hook | apis]
Observed: axhub API/endpoint catalog prompt.
Suggested: use the AXHub API catalog workflow; use latest CLI `axhub catalog resources --json --limit 50`, not removed `axhub apis list` or repo API inspection.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const INSPECT_ROUTING_HINT: &str = r#"<axhub-routing-hint>
Control only; do not summarize this block to the user.
First visible sentence, exactly: 매니페스트와 설정을 확인할게요.
Use exactly one Bash tool call. Bash description/title, exactly: 매니페스트와 설정 확인
Bash command: `axhub-helpers inspect-config-summary`
After the tool call, copy the Korean stdout as the answer. Do not add a table, a second diagnosis, command names, JSON field names, internal labels, file contents, or English tool-title fragments.
Do not call Read, LS, Glob, Grep, `find`, `cat`, raw `axhub manifest validate`, raw `axhub config explain`, plugin package inspection, marketplace inspection, or hook script auditing.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

const AUTH_STATUS_ROUTING_HINT: &str = r#"<axhub-routing-hint>
첫 문장: 로그인 상태를 확인할게요.
첫 확인: Bash title `로그인 상태 확인`, 실행 `axhub-helpers auth-summary --user-utterance "<latest user sentence>"`.
답변: 확인 결과의 한국어 요약만 사용.
범위: 로그인 여부와 다시 로그인 필요 여부만 확인. 설치 상태 점검, 환경 진단, 업데이트 확인, 새 로그인, 로그아웃, 계정 상세 표시는 사용자가 따로 물을 때만.
표현: 계정 이메일, id, team/workspace/profile/scope, 정확한 만료 시각, JSON 같은 내부 값은 답변에 넣지 않음.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-routing-hint>"#;

// Approach E (Phase 2): cmd_prompt_route is preflight + audit first.
// It generally avoids keyword chains and `skills/<X>/SKILL.md` path enforcement;
// narrow high-risk/Desktop hints exist where UltraQA caught native matching
// detouring through generic/model-synthesized flows.
pub(crate) fn cmd_prompt_route() -> anyhow::Result<i32> {
    use axhub_helpers::audit::{
        append as audit_append, now_iso8601, sha256_hex, AuditDecision, AuditRecord,
    };
    use axhub_helpers::routing::{
        apis_intent_present, app_lifecycle_intent_present, apps_intent_present,
        auth_status_intent_present, axhub_keyword_present, browse_template_intent_present,
        clarify_intent_present, connectors_intent_present, data_intent_present, decide,
        deploy_create_intent_present, deploy_logs_intent_present, deploy_restore_intent_present,
        deploy_status_intent_present, deploy_trace_intent_present, deploy_verify_intent_present,
        doctor_intent_present, dynamic_table_intent_present, env_intent_present, find_marker,
        foreign_keyword_present, github_connection_intent_present, init_intent_present,
        inspect_config_intent_present, install_cli_intent_present, is_slash_invocation,
        migrate_intent_present, onboarding_intent_present, open_app_intent_present,
        publish_intent_present, quality_debug_intent_present, quality_diagnose_intent_present,
        quality_plan_intent_present, quality_review_intent_present, quality_ship_intent_present,
        quality_tdd_intent_present, resources_intent_present, routing_stats_intent_present,
        statusline_intent_present, team_intent_present, token_present, update_check_intent_present,
        MarkerStatus,
    };

    if hook_safety::is_hook_disabled("prompt-route") {
        out_json(json!({}));
        return Ok(0);
    }
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");
    // Stable per-session id from the UserPromptSubmit payload (snake_case, always
    // present per the hooks input schema). Empty when absent → the drift nudge
    // degrades to a time-only snooze. Drives the per-session re-surface.
    let session_id = payload
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("");

    // AC-12 / hook-integration: the shared routing decision, computed once here
    // from pure reads (marker walk-up + token-file stat + slash detection). Both
    // the AC-12 audit population below AND the hook action mapping consume this
    // single `routing_decision` so the two layers cannot drift (spec 006 §53-57,
    // composition-consistency). `token_present()` is a `.exists()` stat only — it
    // never triggers bootstrap (constraint: auth-read must not be circular).
    let marker = find_marker();
    let authed = token_present();
    let explicit = is_slash_invocation(prompt);
    let routing_decision = decide(prompt, marker, authed, explicit);

    let preflight = run_preflight();

    // AC-12: persist the decision label + the four decide() inputs + the two
    // keyword-driven signals (spec 006 §80/§94) so routing-stats can read & report
    // the non-axhub ignore rate. The jsonl line MUST carry the value for the AC to
    // verify — do not drop this population.
    let record = AuditRecord {
        ts: now_iso8601(),
        prompt_hash: sha256_hex(prompt),
        prompt_len: prompt.len() as u32,
        cli_version: preflight.output.cli_version.clone(),
        auth_ok: preflight.output.auth_ok,
        is_axhub_related: heuristic_axhub_keyword(prompt),
        clarify_invoked: false,
        chosen_skill: None,
        decision: Some(AuditDecision::from_routing(routing_decision, explicit)),
        marker_present: Some(marker == MarkerStatus::Present),
        authed: Some(authed),
        explicit_invocation: Some(explicit),
        axhub_keyword_present: Some(axhub_keyword_present(prompt)),
        foreign_keyword_present: Some(foreign_keyword_present(prompt)),
    };
    let _ = audit_append(record);

    // AC-11 (grace): once-per-project migration nudge for an authed user whose
    // implicit deploy request resolved to `ignore` (no `axhub.yaml` marker). The
    // `ignore→silent` action mapping itself is owned by hook-integration; this
    // layers ONLY the educational systemMessage onto the existing output. It
    // consumes the already-computed shared `routing_decision`/`authed` (no
    // parallel chain → composition-consistency) and persists best-effort, so it
    // never changes the fail-open `Ok(0)` exit (spec 006 §43, §86).
    // HANDOFF (hook-integration-complete): when you implement the ignore→silent
    // action mapping, KEEP this line and emit `grace` as systemMessage — do NOT
    // re-add a second grace path from your action map, or it double-fires.
    // `maybe_grace_message` IS the single composable seam for the grace nudge.
    let grace = axhub_helpers::grace::maybe_grace_message(routing_decision, authed, prompt);

    let is_quality_review = quality_review_intent_present(prompt);
    let is_auth_status = auth_status_intent_present(prompt);
    let mut context = if is_quality_review {
        QUALITY_REVIEW_ROUTING_HINT.to_string()
    } else if is_auth_status {
        AUTH_STATUS_ROUTING_HINT.to_string()
    } else {
        format_preflight_context(&preflight)
    };
    if dynamic_table_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(DYNAMIC_TABLE_ROUTING_HINT);
    }
    if connectors_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(CONNECTORS_ROUTING_HINT);
    }
    if data_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(DATA_ROUTING_HINT);
    }
    if resources_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(RESOURCES_ROUTING_HINT);
    }
    if github_connection_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(GITHUB_ROUTING_HINT);
    }
    if migrate_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(MIGRATE_ROUTING_HINT);
    }
    if publish_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(PUBLISH_ROUTING_HINT);
    }
    if team_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(TEAM_ROUTING_HINT);
    }
    if !is_quality_review && quality_review_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(QUALITY_REVIEW_ROUTING_HINT);
    }
    if quality_debug_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(QUALITY_DEBUG_ROUTING_HINT);
    }
    if quality_diagnose_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(QUALITY_DIAGNOSE_ROUTING_HINT);
    }
    if quality_plan_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(QUALITY_PLAN_ROUTING_HINT);
    }
    if quality_ship_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(QUALITY_SHIP_ROUTING_HINT);
    }
    if quality_tdd_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(QUALITY_TDD_ROUTING_HINT);
    }
    if apis_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(APIS_ROUTING_HINT);
    }
    if inspect_config_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(INSPECT_ROUTING_HINT);
    }
    if deploy_restore_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(DEPLOY_ROLLBACK_ROUTING_HINT);
    }
    if deploy_status_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(DEPLOY_STATUS_ROUTING_HINT);
    }
    if deploy_logs_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(DEPLOY_LOGS_ROUTING_HINT);
    }
    if deploy_trace_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(TRACE_ROUTING_HINT);
    }
    if open_app_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(OPEN_ROUTING_HINT);
    }
    if deploy_verify_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(VERIFY_ROUTING_HINT);
    }
    if routing_stats_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(ROUTING_STATS_ROUTING_HINT);
    }
    if env_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(ENV_ROUTING_HINT);
    }
    if onboarding_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(ONBOARDING_ROUTING_HINT);
    }
    if init_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(INIT_ROUTING_HINT);
    }
    if app_lifecycle_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(APP_LIFECYCLE_ROUTING_HINT);
    }
    if deploy_create_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(DEPLOY_CREATE_ROUTING_HINT);
    }
    if install_cli_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(INSTALL_CLI_ROUTING_HINT);
    }
    if update_check_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(UPDATE_ROUTING_HINT);
    }
    if doctor_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(DOCTOR_ROUTING_HINT);
    }
    if statusline_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(STATUSLINE_ROUTING_HINT);
    }
    if !is_auth_status && auth_status_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(AUTH_STATUS_ROUTING_HINT);
    }
    if clarify_intent_present(prompt) {
        context.push_str("\n\n");
        context.push_str(CLARIFY_ROUTING_HINT);
    }
    if karpathy_intent_present(prompt) && !hook_safety::is_karpathy_disabled() {
        if let Some(karpathy) = axhub_helpers::karpathy_inject::user_prompt_karpathy_inject()? {
            context.push_str("\n\n");
            context.push_str(&karpathy);
        }
    }
    // Proactive plugin version-drift nudge (UserPromptSubmit surface; D4). Prompt-
    // intent-independent: reads only the TTL cache (no network on this hot path),
    // fires at most once per latest version, and self-suppresses via the per-version
    // marker + opt-out. Fail-open — `plugin_drift_context` never errors.
    let plugin_drift_system =
        if let Some(nudge) = axhub_helpers::plugin_update::plugin_drift_nudge(session_id) {
            let system = nudge.system_message;
            context.push_str("\n\n");
            context.push_str(&nudge.additional_context);
            Some(system)
        } else {
            None
        };
    // Proactive CLI binary version-drift nudge. Turn-cap: at most one update
    // nudge per turn — plugin takes priority, and its per-version marker
    // naturally yields the slot to CLI on the next turn (no cross-turn state
    // needed). Suppressed when the prompt is already an update-check intent (the
    // reactive update-summary path owns that turn). Fail-open.
    let cli_drift_system = if plugin_drift_system.is_none() && !update_check_intent_present(prompt)
    {
        if let Some(nudge) = axhub_helpers::cli_drift::cli_drift_nudge(session_id) {
            context.push_str("\n\n");
            context.push_str(&nudge.additional_context);
            Some(nudge.system_message)
        } else {
            None
        }
    } else {
        None
    };
    let drift_system = plugin_drift_system.or(cli_drift_system);
    let intent_system = if dynamic_table_intent_present(prompt) {
        Some("이 요청은 AXHub hosted app 의 테이블 생성/컬럼/행/권한 변경 요청이에요. 로컬 앱 코드, local database, server.js, package.json, ORM, .env, SQL migration, QA 결과 파일, plugin source 를 읽지 않아요. visible chat 첫 문장은 정확히 \"테이블 변경 내용을 확인할게요.\" 로만 말해요. 그 다음 문장도 내부 경로를 설명하지 말고 로그인 상태, 현재 앱, 대상 테이블, 컬럼 타입을 확인하겠다고만 자연스럽게 말해요. Bash title 은 `로그인 상태 확인`, `테이블 상태 확인`, `테이블 변경 준비`, `테이블 변경 실행` 같은 한국어만 써요. create/drop/column/row/grant 변경은 대상 앱, 테이블, 작업, 컬럼/행 요약을 한국어로 보여주고 사용자가 명시적으로 승인하기 전에는 실행하지 않아요. visible preview 에 raw CLI command line 을 쓰지 말고, 실제 명령은 승인 후 Bash tool call 안에서만 실행해요. Claude Desktop 에서는 AskUserQuestion, Question, 질문 카드 도구를 쓰지 말고 일반 채팅으로 `이대로 만들까요? 진행 또는 취소라고 답해 주세요.` 라고 묻고 멈춰요. 다음 사용자 답변이 `진행`이면 승인된 것으로 보고 바로 `테이블 변경 준비`, `테이블 변경 실행` 순서로 이어가요. 사용자에게 route label, slash command, skill name, workflow/워크플로, preflight, command name, raw command line, raw question JSON, raw JSON, raw email, raw id, raw app slug, local file contents, repo inspection, 영어 tool title fragment, A/B 구현 분기 라벨을 쓰지 않아요. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 절대 쓰지 말고 `로그인되어 있어요`처럼 상태만 말해요.")
    } else if connectors_intent_present(prompt) {
        Some("이 요청은 AXHub 외부 데이터베이스 연결 설정 요청이에요. 로컬 앱 코드 수정, server.js/package.json 읽기, pg 패키지 설치, DATABASE_URL 코드 연결, ORM 설정으로 우회하지 않아요. visible chat 첫 문장은 정확히 \"데이터베이스 연결을 준비할게요.\" 로만 말해요. 그 다음 문장도 내부 경로를 설명하지 말고, 현재 로그인 상태와 workspace 를 확인하겠다고만 자연스럽게 말해요. 필요한 정보가 부족하면 사람에게 묻듯이 엔진, 연결 이름, workspace, host/port/database/user/SSL 같은 연결 정보가 필요하다고 짧게 안내해요. 비밀값은 채팅에 평문으로 받지 말고 로컬 credentials 파일 또는 안전한 입력 방식을 쓰도록 안내해요. 변경 실행 전에는 현재 workspace 와 기존 연결 설정을 확인하고, 생성/수정/삭제 preview 를 보여준 뒤 명시적 승인을 받아요. Bash title 은 `커넥터 상태 확인`, `커넥터 목록 확인`, `커넥터 변경 준비`, `커넥터 변경 실행` 같은 한국어만 써요. 사용자에게 route label, slash command, skill name, workflow/워크플로, local file contents, repo inspection, package install plan, DATABASE_URL app-code path, raw JSON, raw email, raw id, preflight, 영어 tool title fragment, A/B 구현 분기 라벨을 쓰지 않아요. 로그인 확인 결과에는 계정 이메일, raw user id 를 절대 쓰지 말고 `로그인되어 있어요`처럼 상태만 말해요.")
    } else if data_intent_present(prompt) {
        Some("이 요청은 AXHub 데이터 리소스 조회/설명/스니펫 요청이에요. 로컬 앱 코드, server.js, package.json, ORM, .env, QA 결과 파일, plugin source 를 읽지 않아요. visible chat 첫 문장은 정확히 \"데이터 리소스를 확인할게요.\" 로만 말해요. 이후에도 내부 경로를 설명하지 말고 로그인 상태와 연결된 데이터 리소스를 확인하겠다고만 자연스럽게 말해요. Bash title 은 `로그인 상태 확인`, `데이터 리소스 확인`, `데이터 설명`, `실데이터 확인`, `스니펫 준비` 같은 한국어만 써요. live read 는 대상, 컬럼, row limit, query shape 를 보여주고 사용자가 명시적으로 승인하기 전에는 실행하지 않아요. 리소스가 없으면 `현재 연결된 데이터 리소스를 찾지 못했어요. 먼저 데이터베이스 연결을 만들어야 해요.`처럼 말하고, raw CLI/JSON 세부값을 덧붙이지 않아요. 사용자에게 route label, slash command, skill name, workflow/워크플로, preflight, catalog 조회, catalog 비어있음, connector 목록, catalog kinds, raw JSON, raw email, raw id, account scope, raw app slug, governance/path guessing 용어, 영어 tool title fragment, A/B 구현 분기 라벨을 쓰지 않아요. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 절대 쓰지 말고 `로그인되어 있어요`처럼 상태만 말해요.")
    } else if resources_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"리소스 정리 방식을 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"리소스 현황 확인\" 으로 설정하고 `axhub-helpers resources-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 게이트웨이 리소스 정리/조직 요청이에요. 로컬 파일 정리, git 작업트리 정리, QA 산출물 정리, shim 로그 정리로 우회하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 표, ToolSearch, catalog kinds, connector/resource, 원시 명령명, JSON field name, 내부 라벨, 영어 tool-title fragment, `모호`, 변경 작업을 못 한다는 단정, 로컬 파일 내용을 덧붙이지 않아요. 삭제, 이동, 이름 변경, 태그 변경, 등록 같은 변경은 대상과 작업 preview 를 보여주고 명시적 승인 전에는 실행하지 않아요. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 절대 쓰지 말고 `로그인되어 있어요`처럼 상태만 말해요.")
    } else if github_connection_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"GitHub 연결 상태를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"GitHub 연결 상태 확인\" 으로 설정하고 `axhub-helpers github-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub hosted app 과 GitHub 저장소의 연결 상태 확인이에요. 로컬 git remote, git config, gh CLI, GitHub PR, repo source, package 파일, .git, QA 결과 파일로 우회하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 표, 원시 명령명, JSON field name, raw ID, 계정 이메일, installation ID, local git remote 증거, 파일 내용, route label, slash command, skill name, ToolSearch 설명, 영어 tool-title fragment 를 사용자에게 쓰지 않아요. 연결/해제/repo 생성/remote 추가/push 는 변경 작업이므로 대상 preview 와 명시적 승인 전에는 실행하지 않아요.")
    } else if migrate_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"가져오기 상태를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"가져오기 상태 확인\" 으로 설정하고 `axhub-helpers migrate-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 기존 앱이나 현재 프로젝트를 AXHub로 가져올 수 있는지 확인하는 요청이에요. 로컬 서버 점검, package script 분석, git release 상태, 일반 배포 조언, QA 결과 파일, 이전 배포 실패 상태로 우회하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 표, 원시 명령명, JSON field name, raw deploy status field, local server 증거, 파일 내용, route label, slash command, skill name, ToolSearch 설명, emoji, 영어 tool-title fragment 를 사용자에게 쓰지 않아요. 앱 등록, GitHub 연결, env 저장, 배포는 변경 작업이므로 대상 preview 와 명시적 승인 전에는 실행하지 않아요.")
    } else if publish_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"공개 심사 준비를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"공개 심사 준비 확인\" 으로 설정하고 `axhub-helpers publish-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 마켓플레이스 공개 심사 제출 준비 요청이에요. quality.json, state file, QA 결과 파일, repo 파일, package 파일, plugin source, 로컬 상태 점검으로 우회하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 표, 원시 명령명, JSON field name, raw review status field, route label, slash command, skill name, ToolSearch 설명, preflight, quality state, file 내용, 영어 tool-title fragment, workflow/워크플로 라벨을 사용자에게 쓰지 않아요. 공개 심사 제출은 외부 공개 변경 작업이므로 제출 사유, 대상 앱 preview, 명시적 승인 전에는 실행하지 않아요.")
    } else if team_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"팀 작업을 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"팀 작업 확인\" 으로 설정하고 `axhub-helpers team-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 워크스페이스 팀원 초대, 초대 목록, 또는 앱 접근 공유 요청이에요. Claude/OMC 멀티에이전트 작업 팀, 코드 작업 팀, 일반 협업자 모집, 파일/프로젝트 탐색으로 우회하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 표, 원시 명령명, JSON field name, raw tenant id, raw user id, 사용자가 직접 쓴 이메일 외의 raw email, route label, slash command, skill name, ToolSearch 설명, preflight, tenant/workspace 구현 용어, OMC/Claude 팀 비교, 파일 내용, 영어 tool-title fragment, workflow/워크플로 라벨을 사용자에게 쓰지 않아요. 초대 발송, 초대 취소, 앱 접근 변경은 권한 변경 작업이므로 대상자와 대상 preview, 명시적 승인 전에는 실행하지 않아요.")
    } else if is_quality_review {
        None
    } else if quality_debug_intent_present(prompt) {
        Some("visible chat 첫 문장은 정확히 \"원인을 좁혀볼게요.\" 한 문장만 말해요. 이 요청은 직접 코드/테스트 디버그 요청이에요. background quality auto-mode 나 일반 파일 탐색 답변으로 처리하지 말고 바로 전용 디버그 절차를 시작해요. `.axhub-state/quality.json` 은 직접 디버그 시작 전에 읽지 않아요. 첫 Bash tool call 은 `문제 신호 확인` 또는 `최근 실패 확인` 같은 한국어 title 만 써요. generic file listing, repo survey, local QA 결과 읽기부터 시작하지 않아요. 증상/로그 수집, 가설, 증거, 다음 probe 순서로 정리해요. 디버그 패스를 마치면 `디버그 상태 저장` title 로 `axhub-helpers state-update --debug-acknowledged` 를 실행해요. visible text 에 route label, slash command, skill name, quality auto-mode, workflow/워크플로, TodoWrite availability, preflight internals, raw JSON field, raw email, 파일 listing 설명, 영어 tool-title fragment 를 쓰지 않아요.")
    } else if quality_diagnose_intent_present(prompt) {
        Some("visible chat 첫 문장은 정확히 \"진단 루프를 준비할게요.\" 한 문장만 말해요. 이 요청은 직접 auto-diagnose loop 요청이에요. background quality auto-mode 나 일반 파일 탐색 답변으로 처리하지 말고 진단 루프 절차를 시작해요. 첫 Bash tool call 은 `진단 루프 준비` 또는 `실패 신호 확인` 같은 한국어 title 만 써요. Claude Desktop 에서는 raw AskUserQuestion JSON 이 보일 수 있으니 질문 카드 JSON 을 노출하지 말고, 선택이 필요하면 일반 채팅으로 짧은 한국어 선택지를 물은 뒤 멈춰요. visible text 에 route label, slash command, skill name, quality auto-mode, workflow/워크플로, preflight internals, raw JSON field, raw email, 영어 tool-title fragment 를 쓰지 않아요.")
    } else if quality_plan_intent_present(prompt) {
        Some("visible chat 첫 문장은 정확히 \"변경 계획을 잡아볼게요.\" 한 문장만 말해요. 이 요청은 직접 변경 계획 요청이에요. background quality auto-mode 나 일반 파일 탐색 답변으로 처리하지 말고 계획 절차를 시작해요. 첫 Bash tool call 은 `계획 범위 확인` 또는 `영향 범위 확인` 같은 한국어 title 만 써요. 요구 범위, 영향 범위, 3-5단계 계획, 검증 명령을 정리하고, 이 단계에서는 구현을 바로 시작하지 않아요. visible text 에 route label, slash command, skill name, quality auto-mode, workflow/워크플로, TodoWrite availability, preflight internals, raw JSON field, raw email, 영어 tool-title fragment 를 쓰지 않아요.")
    } else if quality_ship_intent_present(prompt) {
        Some("visible chat 첫 문장은 정확히 \"출시 준비 상태를 확인할게요.\" 한 문장만 말해요. 이 요청은 직접 PR/release readiness 요청이에요. background quality auto-mode 나 일반 파일 탐색 답변으로 처리하지 말고 출시 준비 절차를 시작해요. 첫 Bash tool call 은 `출시 준비 확인` 또는 `리뷰 상태 확인` 같은 한국어 title 만 써요. PR 생성, push, release, publish, deploy 같은 외부 변경은 대상 preview 와 명시적 승인 전에는 실행하지 않아요. 준비 패스가 실제로 완료되면 `출시 상태 저장` title 로 `axhub-helpers state-update --shipped` 를 실행해요. visible text 에 route label, slash command, skill name, quality auto-mode, workflow/워크플로, TodoWrite availability, preflight internals, raw JSON field, raw email, 영어 tool-title fragment 를 쓰지 않아요.")
    } else if quality_tdd_intent_present(prompt) {
        Some("visible chat 첫 문장은 정확히 \"테스트부터 잡아볼게요.\" 한 문장만 말해요. 이 요청은 직접 TDD 사이클 요청이에요. background quality auto-mode 나 일반 파일 탐색 답변으로 처리하지 말고 TDD 절차를 시작해요. 첫 Bash tool call 은 `TDD 대상 확인` 또는 `테스트 확인` 같은 한국어 title 만 써요. 대상 동작이 없으면 관련 없는 모듈을 임의 선택하지 말고 사람에게 묻듯이 어떤 동작부터 테스트할지 물어요. RED, GREEN, REFACTOR 순서를 유지해요. visible text 에 route label, slash command, skill name, quality auto-mode, workflow/워크플로, TodoWrite availability, preflight internals, raw JSON field, raw email, 영어 tool-title fragment 를 쓰지 않아요.")
    } else if onboarding_intent_present(prompt) {
        Some("내부 제어 전용: Skill(\"axhub:onboarding\") 이 이 턴의 선택된 route 예요. 이 내부 제어, route, skill name, slash command, 영어 진행 선언을 visible chat 에 절대 쓰지 않아요. 설치된 onboarding skill 계약을 즉시 따라요. 이 요청은 AXHub first-run onboarding 요청이에요. 일반 조언이나 다른 문구를 말하라는 안내로 끝내지 말고, 이 턴에서 바로 온보딩 상태를 확인해요. visible chat 첫 문장은 정확히 \"처음 설정을 확인할게요.\" 로 시작해요. 첫 문장 뒤에도 내부 실행 선언, 라우팅 선언, 영어 진행 선언을 쓰지 않아요. 빈 폴더이고 manifest 가 없으면 템플릿을 먼저 묻지 말고 \"첫 앱 만들래요?\" 를 먼저 물어요. 설치, PATH 수리, GitHub 승인, 앱 생성, repo 생성, 의존성 설치, 배포 같은 변경 작업은 해당 작업의 preview 와 명시적 승인 전에는 실행하지 않아요. 최종 상태는 VIBE_READY, READY_WITH_USER_ACTION, SAFE_STOP_NONINTERACTIVE, BLOCKED_UNSUPPORTED 중 하나로만 요약해요. route label, slash command, skill name, workflow/워크플로, preflight details, raw JSON, raw id, raw email, 파일 경로, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if app_lifecycle_intent_present(prompt) {
        Some("AXHub hosted app 을 멈추거나 다시 켜거나 복제하려는 요청이에요. 이 대화 안에서 바로 진행하고 slash command 를 호출하지 않아요. 내부 처리, route conversion, 라벨 설명 문장을 visible chat 에 쓰지 않아요. 로컬 Next.js/dev-server 프로세스, 포트, ps/lsof, package script, 로컬 서버 상태를 확인하지 않아요. pause 의 첫 visible chat 문장은 `<앱 이름> 앱을 잠깐 멈출 준비를 할게요.` 형태로 말하고, resume 은 `<앱 이름> 앱을 다시 켤 준비를 할게요.`, fork 는 `<앱 이름> 앱을 복제할 준비를 할게요.` 로 말해요. Bash tool title 은 `앱 상태 확인`, `앱 찾기`, `앱 변경 실행` 같은 한국어만 써요. 추가 조회가 필요하면 visible chat 은 `앱을 한 번 더 확인할게요.` 로만 말하고 식별자 조회를 설명하지 않아요. 로그인과 현재 앱을 확인하고, AXHub 앱을 찾고, 서비스 영향 설명 뒤 `앱 변경을 실행할까요?` 라고 묻고 visible option 은 `취소`, `진행`만 써요. 로그인 확인 결과에는 계정 이메일, owner 이름, raw user id 를 쓰지 않아요. 앱 metadata 의 raw enum 값은 한국어 라벨로만 번역해요. `private`, `public`, `development`, `production` 같은 raw enum 이나 `비공개 (private)` 같은 혼합 표기를 쓰지 않아요. 사용자가 `진행`을 고르기 전에는 앱 상태를 바꾸지 않아요. `진행` 뒤에는 visible chat 에 아무 문장도 쓰지 말고 바로 Bash tool call 을 실행해요. `User chose`, `execute suspend`, `execute resume` 같은 영어 구현 문장을 쓰지 않아요. JSON 을 직접 만들지 않고, schema 확인, source 탐색, fixture 탐색, helper 위치 탐색, grep, rg 같은 탐색 명령을 실행하지 않아요. 앱 변경은 Bash tool call 하나로 matching top-level `axhub apps ... --execute --json >/dev/null` 만 실행하고, raw JSON stdout 을 tool panel 에 남기지 않아요. 첫 `앱 변경 실행` 이 exit code 0 으로 끝나면 그것이 terminal success 예요. mutation 을 재검증한다는 이유로 같은 변경 명령을 다시 실행하지 않아요. 내부 안전 점검이 막으면 내부를 설명하지 말고 같은 top-level 명령을 한 번만 재시도해요. 그래도 실패하면 `앱 변경을 시작하지 못했어요. 다시 시도해 주세요.` 라고만 말해요. route label, slash command, skill name, preflight details, internal app/context fields, auth results, runtime words, lifecycle verbs in English, raw JSON, raw identifier, owner name, 계정 이메일, 영어 tool title fragment, 괄호 안 내부 라벨을 사용자에게 쓰지 않아요.")
    } else if init_intent_present(prompt) {
        Some("내부 제어 전용: Skill(\"axhub:init\") 이 이 턴의 선택된 route 예요. 이 내부 제어, route, skill name, slash command, 영어 진행 선언을 visible chat 에 절대 쓰지 않아요. 설치된 init skill 계약을 즉시 따라요. 현재 AXHub 프로젝트에서 새 앱 생성 요청이에요. 이 요청은 브레인스토밍, 일반 프로젝트 탐색, 또는 앱 아이디어 분류가 아니라 AXHub 앱 생성 절차예요. visible chat 첫 문장은 정확히 \"새 앱을 만들 수 있는 템플릿을 확인할게요.\" 로 시작하고, 이 문장 앞에는 아무 말도 붙이지 않아요. 먼저 Bash title 을 정확히 \"앱 생성 상태 확인\" 으로 설정해 `axhub-helpers init-resume route --json` 를 한 번 실행해요. 이어갈 생성 상태가 있으면 먼저 이어갈지 물어요. 이어갈 생성 상태가 없으면 Bash title 을 정확히 \"템플릿 확인\" 으로 설정해 `axhub apps templates list --json` 를 실행하고, backend template registry 가 반환한 템플릿만 사람용 라벨로 보여줘요. 명시 선택지는 최대 3개만 넣고, backend 가 반환하지 않은 템플릿을 만들지 않으며, Claude Desktop 이 자동으로 free-form 기타/Other 를 추가하므로 기타를 별도 옵션으로 또 넣지 않아요. 웹 앱/API/백엔드/CLI 도구/axhub 앱 같은 일반 앱 종류 질문을 하지 않아요. 템플릿 확인 뒤에는 GitHub App 계정 설치 상태를 read-only 로 한 번 보여주고(이미 설치된 계정이어도 install_url 은 다른 org/계정 추가용으로 항상 같이 보여줘요 — 설치됐다고 링크를 숨기거나 건너뛰지 않아요. '비차단'은 설치를 강요하지 않는다는 뜻이지 링크를 빼라는 뜻이 아니에요), 그다음 앱 이름을 묻고, 생성 preview 와 명시적 승인을 받기 전에는 `axhub apps bootstrap --execute`, repo 생성, GitHub 연결, 의존성 설치, dev server 시작, 배포를 실행하지 않아요. route label, slash command, skill name, workflow/워크플로, raw question JSON, raw helper JSON, command mapping, raw id, raw email, 파일 경로, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if apps_intent_present(prompt) {
        Some("axhub 내 앱 목록/관리 요청이에요. 현재 팀 scope 의 앱 목록을 보여줘요. 생성/수정/삭제는 별도 승인 전에는 실행하지 않아요. 사용자에게 내부 라벨 설명을 하지 말고 바로 결과 확인 문장으로 시작해요.")
    } else if browse_template_intent_present(prompt) {
        Some("axhub 템플릿 또는 마켓플레이스 탐색 요청이에요. 공개 앱/템플릿을 read-only 로 보여줘요. 내 앱 목록은 앱 목록 흐름, 새 앱 생성은 앱 생성 흐름으로 이어가요. 사용자에게 내부 라벨 설명을 하지 말고 바로 탐색 결과 확인 문장으로 시작해요.")
    } else if apis_intent_present(prompt) {
        Some("axhub API 카탈로그 요청이에요. 최신 CLI `axhub catalog resources --json --limit 50`를 실행해 connector/path/kind 목록을 보여줘요. 제거된 `axhub apis list`는 실행하지 않아요. 사용자에게 내부 라벨 설명을 하지 말고 바로 카탈로그 확인 문장으로 시작해요.")
    } else if inspect_config_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"매니페스트와 설정을 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"매니페스트와 설정 확인\" 으로 설정하고 `axhub-helpers inspect-config-summary` 를 한 번만 실행해요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 진단, 표, 원시 명령명, JSON field name, 파일 내용, 내부 라벨, 영어 tool title fragment 를 사용자에게 쓰지 않아요. raw `axhub manifest validate`, raw `axhub config explain`, Read tool, LS tool, Glob tool, Grep tool, `ls`, `find`, `cat`, `.claude-plugin/plugin.json`, marketplace.json, hooks.json 읽기는 호출하지 않아요. secret 은 복원하거나 추측하지 않아요.")
    } else if deploy_restore_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"되돌릴 수 있는 배포를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"배포 되돌리기 확인\" 으로 설정하고 `axhub-helpers rollback-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 배포 되돌리기/복구 요청이에요. rollback 인지 recover 인지, slash command, skill name, route label 을 사용자에게 설명하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 실제 되돌리기나 재배포는 외부 변경 작업이므로 사용자가 한국어 preview 를 보고 명시적으로 승인하기 전에는 실행하지 않아요. 원시 명령명, raw deploy id, raw commit hash, raw status name, commit_not_found, no-op, app id/slug, preflight, JSON field name, 로컬 파일 탐색, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if deploy_status_intent_present(prompt) {
        Some("axhub 배포 상태 요청이에요. 로그인/토큰 확인이 필요하면 그 안내를 한국어로 말해요. 사용자에게 내부 라벨 설명을 하지 말고 바로 상태 확인 문장으로 시작해요.")
    } else if deploy_trace_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"배포 기록을 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"배포 기록 확인\" 으로 설정하고 `axhub-helpers trace-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 배포 실패 원인 확인이에요. route, slash command, skill name, preflight, deploy id, raw status name, JSON field name, failure_reason, matched_patterns, build_log_errors, QA 결과 파일, plugin source, 영어 tool title fragment 를 사용자에게 쓰지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 표나 추가 조사 레이어를 덧붙이지 않아요.")
    } else if deploy_logs_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"로그를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"로그 확인\" 으로 설정하고 `axhub-helpers logs-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 앱 배포 로그 확인이에요. 로컬 파일 로그, .omc 로그, git log, 플러그인 캐시, 패키지 로그를 찾지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 진단, 원시 명령명, JSON field name, 파일 내용, 내부 라벨, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if open_app_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"앱 페이지를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"앱 페이지 확인\" 으로 설정하고 `axhub-helpers open-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 앱 페이지 열기예요. QA 결과 파일, .omc, .claude, 플러그인 캐시, git log, Chrome MCP 상태, 브라우저 확장 상태를 찾지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 추가 진단, 원시 명령명, JSON field name, 파일 내용, 내부 라벨, ToolSearch 설명, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if deploy_verify_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"배포가 실제로 열리는지 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"배포 검증\" 으로 설정하고 `axhub-helpers verify-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 배포 라이브 검증이에요. routing, preflight, stale cache id, deploy id, user email, raw status name, JSON field name, 중간 fallback 시도, 내부 라벨을 사용자에게 쓰지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요.")
    } else if routing_stats_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"라우팅 통계를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"라우팅 통계 확인\" 으로 설정하고 `axhub-helpers routing-stats --since 7d` 를 한 번만 실행해요. 이 요청은 AXHub 플러그인 라우팅 통계 확인이에요. QA 결과 파일, desktop QA 로그, repo 파일, plugin source, git history, .omc, .claude, 로컬 프로젝트 노트를 읽지 않아요. 도구가 끝나면 stdout 의 한국어 요약만 짧게 답변해요. 원시 명령명, 파일 내용, 내부 라벨, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if env_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"환경변수를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"환경변수 확인\" 으로 설정하고 `axhub-helpers env-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 앱 환경변수 확인이에요. 셸 환경변수, .env 파일, repo 파일, plugin source, git history, .omc, .claude, QA 결과 파일을 읽지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 원시 명령명, JSON field name, 내부 라벨, preflight, raw value, secret value, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if deploy_create_intent_present(prompt) {
        Some("도구를 호출하거나 스킬 내용을 요약하기 전에 visible chat 으로 정확히 \"배포 준비를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 전에 사용자에게 보이는 앱 폴더에서 실행되는지 확인해요. active root 와 추가 폴더가 다르고 추가 폴더만 Vite/React 앱이면 그 폴더로 `cd` 한 뒤 실행해요. 후보가 여러 개면 어떤 폴더를 배포할지 묻고 멈춰요. 첫 Bash tool call 의 description/title 은 정확히 \"배포 준비 확인\" 으로 설정하고 `axhub-helpers deploy-preview-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. stdout 이 `axhub 매니페스트(axhub.yaml)가 없어요.` 라고 하면 React/Vite로 초기화, 다른 템플릿 선택, 취소 선택지만 보여주고 배포 승인 질문 없이 멈춰요. 그 외에는 stdout 의 한국어 preview 를 그대로 보여주고 명시적 승인 질문을 해요. 이 preview 전에는 긴 deploy skill 본문을 읽거나 요약하지 않아요. 사용자가 승인하면 두 번째 Bash tool call 의 description/title 은 정확히 \"배포 실행\" 으로 설정하고 `axhub-helpers deploy-approved-run --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 승인 후에는 skill 을 다시 호출하거나 긴 deploy skill 본문을 읽지 않아요. 이 요청은 AXHub 라이브 배포 요청이에요. 일반 release/git release/다른 호스팅 배포로 우회하지 않아요. 실제 배포 전에는 앱, 환경, 브랜치, 커밋, 예상 시간을 보여주고 명시적 사용자 승인을 받아요. 본문에는 route label, slash command, command mapping, skill name, `Invoke deploy skill`, `Read rest of SKILL`, `Read full SKILL`, `Route=axhub`, `preflight`, `deploy-prep`, raw helper JSON, raw id, raw email, 영어 tool title fragment 를 쓰지 않아요. 승인 후 실행 단계의 Bash tool title 은 `배포 실행` 또는 `배포 상태 확인` 같은 한국어로만 써요. 토큰 만료나 인증 오류가 있으면 로그인 필요 여부를 한국어로 안전하게 설명하고 로그인은 묻기 전에는 시작하지 않아요.")
    } else if is_auth_status {
        None
    } else if install_cli_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"설치 상태를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"설치 상태 확인\" 으로 설정하고 `axhub-helpers install-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub CLI 설치 요청이에요. 이미 설치되어 있으면 설치 작업이 필요 없다고만 말하고 installer 를 실행하지 않아요. 설치되어 있지 않으면 공식 설치를 진행할 수 있다고 안내하고, 실제 설치 명령은 명시적 승인 전에는 실행하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 원시 명령명, slash command, skill name, 내부 라벨, raw JSON field, auth status field, 파일 경로, installer URL, preflight, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if update_check_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"업데이트를 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"업데이트 확인\" 으로 설정하고 `axhub-helpers update-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub CLI 업데이트 확인이에요. update apply, install, doctor, auth login/logout, plugin update, cache scan, compatibility diagnostics 로 우회하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 사용자가 명시적으로 적용을 승인하기 전에는 업데이트를 실행하지 않아요. 원시 명령명, slash command, skill name, 내부 라벨, raw JSON field, has_update, 파일 경로, installer URL, plugin update 제안, preflight, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if doctor_intent_present(prompt) {
        Some("내부 제어 전용: Skill(\"axhub:doctor\") 이 이 턴의 선택된 route 예요. 이 내부 제어, route, skill name, slash command 를 visible chat 에 절대 쓰지 않아요. 설치된 doctor skill 계약이 authoritative 예요. Skill surface 가 있으면 visible chat 으로 정확히 \"설치 상태를 확인할게요.\" 한 문장만 먼저 말하고, Bash tool call 전에 Skill(\"axhub:doctor\") 를 내부 진단 surface 로 먼저 호출해요. Skill surface 가 없을 때만 fallback 으로 첫 Bash tool call 의 description/title 을 정확히 \"설치 상태 확인\" 으로 설정하고 `axhub-helpers doctor-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub CLI/플러그인/로그인 준비 상태 확인이에요. 설치, 업데이트, 로그인, 로그아웃, 설정 변경은 사용자가 명시적으로 요청하지 않았으면 실행하지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 원시 명령명, slash command, Skill(\"axhub:doctor\"), skill name, 내부 라벨, raw JSON field, raw user email, 파일 경로, preflight, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if statusline_intent_present(prompt) {
        Some("도구를 호출하기 전에 visible chat 으로 정확히 \"상태바 설정을 확인할게요.\" 한 문장만 말해요. 첫 Bash tool call 의 description/title 은 정확히 \"상태바 설정\" 으로 설정하고 `axhub-helpers statusline-summary --user-utterance \"<방금 사용자 문장>\"` 를 한 번만 실행해요. 이 요청은 AXHub 상태바 활성화예요. 기존 다른 상태바가 있으면 덮어쓰지 않아요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변으로 사용해요. 원시 명령명, slash command, skill name, 내부 라벨, raw settings JSON, 기존 command 문자열, exit code, scope fallback 설명, statusLine/wire/settings-merge 용어, 파일 경로, 영어 tool title fragment 를 사용자에게 쓰지 않아요.")
    } else if clarify_intent_present(prompt) {
        Some("사용자가 AXHub에서 무엇을 할지 넓게 물었어요. visible chat 첫 문장은 정확히 \"어떤 걸 도와드릴까요?\" 로만 말해요. 곧바로 질문 카드 하나를 열고 header 는 \"작업 선택\", question 은 \"어떤 걸 도와드릴까요?\" 로 설정해요. 선택지는 \"환경 점검\", \"앱 배포\", \"앱과 리소스 조회\", \"문제 원인 보기\", \"처음부터 안내\" 다섯 개만 보여줘요. 질문 카드 도구가 value 를 요구하면 각 value 는 visible label 과 같은 한국어 문구로 설정해요. 영어 skill slug 나 slash-command form 을 담은 hidden value 는 넣지 말아요. 설명은 자연어로만 쓰고 괄호 안 영어/내부 라벨, slash command, skill name, command mapping, route label, raw tool name, 사용자를 탓하는 모호성 표현을 사용자에게 쓰지 않아요. 사용자가 고른 뒤에는 Claude Skill tool 이나 slash command 를 호출하지 말고 inline 으로 이어가요. \"환경 점검\" 선택 시 visible chat 은 정확히 \"설치 상태를 확인할게요.\" 한 문장으로 시작하고, 첫 Bash tool call 의 description/title 은 정확히 \"설치 상태 확인\" 으로 설정한 뒤 `axhub-helpers doctor-summary --user-utterance \"<처음 사용자가 한 넓은 문장>\"` 를 한 번만 실행해요. 도구가 끝나면 stdout 의 한국어 문장을 그대로 답변해요. \"앱 배포\" 선택 시 visible chat 은 정확히 \"배포 준비를 확인할게요.\" 로 시작하고, 사용자에게 보이는 앱 폴더에서 실행되는지 확인한 뒤 `axhub-helpers deploy-preview-summary --user-utterance \"<처음 사용자가 한 넓은 문장>\"` 만 먼저 실행해요. stdout 이 `axhub 매니페스트(axhub.yaml)가 없어요.` 라고 하면 React/Vite로 초기화, 다른 템플릿 선택, 취소 선택지만 보여주고 배포 승인 질문 없이 멈춰요.")
    } else {
        None
    };
    // `intent_system` is internal routing control for the model, not a message
    // for the user — append it to `additionalContext` (agent-facing, hidden from
    // the user, the same channel the routing hints already ride) so the raw route
    // contract never surfaces as a user-visible `UserPromptSubmit says:` block.
    // Only genuinely user-facing nudges (`grace` and plugin drift) stay on
    // `systemMessage`.
    if let Some(intent) = intent_system {
        context.push_str("\n\n");
        context.push_str(intent);
    }
    let system_message = match (grace, drift_system) {
        (Some(grace), Some(drift)) => Some(format!("{grace}\n\n{drift}")),
        (Some(grace), None) => Some(grace.to_string()),
        (None, Some(drift)) => Some(drift),
        (None, None) => None,
    };
    println!(
        "{}",
        hook_output::user_prompt_context_with_system(&context, system_message.as_deref())
    );
    Ok(0)
}

/// Single substring check for measurement only. NOT intent classification.
fn heuristic_axhub_keyword(prompt: &str) -> bool {
    prompt.to_lowercase().contains("axhub")
}

/// Explicit coding-reminder phrases only. Do not inject this unrelated SKILL
/// into ordinary axhub deploy/status/auth prompts.
fn karpathy_intent_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    [
        "작은 diff",
        "테스트 우선",
        "과신 금지",
        "evidence first",
        "keep changes small",
        "small diff",
        "tests first",
        "no overconfidence",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// Render preflight result as a tagged additionalContext block. User-facing
/// Korean systemMessage prose is intentionally not emitted here; this path is
/// agent-facing only and is locked by the Phase 26 hook template linter.
fn format_preflight_context(preflight: &PreflightRun) -> String {
    let cli_version = preflight
        .output
        .cli_version
        .clone()
        .unwrap_or_else(|| "unknown".into());
    let mut observed = Vec::new();
    let suggested = if preflight.output.cli_too_old {
        observed.push(format!("axhub CLI v{cli_version} below required range."));
        "run `axhub update` before axhub commands."
    } else if preflight.output.cli_too_new {
        observed.push(format!(
            "axhub CLI v{cli_version} above validated plugin range."
        ));
        "check for an axhub plugin update before release-sensitive commands."
    } else if !preflight.output.cli_present {
        observed.push("axhub CLI not found on PATH.".to_string());
        "install axhub CLI before axhub commands."
    } else if !preflight.output.cli_on_path {
        observed.push(format!(
            "axhub CLI v{cli_version} found on disk but not on PATH."
        ));
        if let Some(path) = preflight.output.cli_resolved_path.as_deref() {
            observed.push(format!("resolved axhub path: {path}."));
        }
        "use the resolved axhub path for this session, or run PATH repair before new shell commands."
    } else {
        observed.push(format!("axhub CLI v{cli_version} healthy."));
        "no action required."
    };
    if !preflight.output.auth_ok {
        if let Some(code) = preflight.output.auth_error_code.as_deref() {
            observed.push(format!("auth status: failed ({code})."));
        }
    } else {
        observed.push("auth status: ok.".to_string());
    }
    let observed_block = if observed.len() == 1 {
        format!("Observed: {}", observed[0])
    } else {
        format!(
            "Observed:\n{}",
            observed
                .into_iter()
                .map(|line| format!("  - {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let mut block = format!(
        "<axhub-preflight-status>\n[axhub hook | session preflight]\n{observed_block}\nSuggested: {suggested}\nSkip: AXHUB_DISABLE_HOOK=prompt-route\n</axhub-preflight-status>"
    );
    if std::env::var("AXHUB_INJECT_EXAMPLES").is_ok() {
        block.push_str(
            "\n\n<axhub-examples-context>\n[axhub hook | examples fallback]\nObserved: AXHUB_INJECT_EXAMPLES enabled.\nSuggested: consult SKILL.md frontmatter examples when matching skills.\nSkip: AXHUB_DISABLE_HOOK=prompt-route\n</axhub-examples-context>",
        );
    }
    block
}

// Approach E (Phase 4): routing-stats + cleanup-audit subcommands.
//
// Local-only audit log analytics. AXHUB_NO_AUDIT respected. Silent skip on
// disk read errors. Always Korean default + --json machine-readable.

pub(crate) const ROUTING_STATS_HELP: &str =
    "axhub-helpers routing-stats — 라우팅 audit log 통계 조회

USAGE:
  axhub-helpers routing-stats [OPTIONS]

OPTIONS:
  --since <DURATION>    조회 기간 (예: 1d, 7d, 30d, all). 기본: 7d
  --json                machine-readable JSON 출력
  --top <N>             top N axhub-related prompt hash 표시. 기본: 10
  --confused            clarify_invoked=true 인 records 만 표시 (사용자 disambiguation 발동)
  -h, --help            도움말

PRIVACY:
  prompt content 저장 X. sha256 hash + length + cli_version + auth_ok 만 기록.
  외부 전송 X. 모두 로컬 ~/.local/share/axhub-plugin/ 또는 동등 경로.
  끄려면: AXHUB_NO_AUDIT=1 환경 변수 설정.
  삭제: axhub-helpers cleanup-audit --all
";

fn parse_duration(s: &str) -> anyhow::Result<chrono::Duration> {
    if s == "all" {
        return Ok(chrono::Duration::days(36500));
    }
    if s.is_empty() {
        anyhow::bail!("duration 비어 있어요");
    }
    let last = s.chars().last().unwrap();
    let (num_str, unit) = s.split_at(s.len() - last.len_utf8());
    let num: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("duration 숫자 부분 파싱 실패: {s}"))?;
    match unit {
        "d" => Ok(chrono::Duration::days(num)),
        "h" => Ok(chrono::Duration::hours(num)),
        "m" => Ok(chrono::Duration::minutes(num)),
        _ => anyhow::bail!("duration 단위는 d/h/m 또는 'all' 만 (받은 값: {s})"),
    }
}

fn percentile(sorted: &[u32], p: f64) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

pub(crate) fn cmd_routing_stats(
    since_arg: Option<String>,
    json: bool,
    top_arg: Option<String>,
    confused: bool,
) -> anyhow::Result<i32> {
    use axhub_helpers::audit;

    let since = match since_arg {
        None => chrono::Duration::days(7),
        Some(s) => match parse_duration(&s) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("axhub-helpers routing-stats: {e}");
                return Ok(64);
            }
        },
    };
    let top: u32 = match top_arg {
        None => 10,
        Some(s) => match s.parse() {
            Ok(t) => t,
            Err(_) => {
                eprintln!("axhub-helpers routing-stats: --top 은 숫자여야 해요");
                return Ok(64);
            }
        },
    };

    // 매 호출마다 7-day rotation 자동 trigger (silent).
    let _ = audit::rotate(7);

    if std::env::var("AXHUB_NO_AUDIT").is_ok() {
        if json {
            println!(
                "{}",
                json!({
                    "audit_disabled": true,
                    "message": "AXHUB_NO_AUDIT 환경 변수가 설정되어 audit 가 비활성이에요."
                })
            );
        } else {
            println!("audit log 가 비활성이에요 (AXHUB_NO_AUDIT 환경 변수 설정).");
            println!("끄려면 변수 unset 후 다음 prompt 부터 기록해요.");
        }
        return Ok(0);
    }

    let mut records = audit::read_since(since)?;
    if confused {
        records.retain(|r| r.clarify_invoked);
    } else {
        // Clarify sentinel records are feedback events, not regular prompt-route samples.
        // Keeping them in default stats inflates auth_failed because sentinel records have
        // auth_ok=false by construction, and can depress axhub_related counts.
        records.retain(|r| !r.clarify_invoked);
    }
    if records.is_empty() {
        if json {
            println!(
                "{}",
                json!({"records": [], "total_prompts": 0, "confused_prompts": []})
            );
        } else if confused {
            println!("최근 {since:?} 동안 clarify 발동 prompt 가 없어요.");
        } else {
            println!("아직 audit 데이터가 없어요. axhub 사용하다 보면 자동 누적돼요.");
        }
        return Ok(0);
    }

    let total = records.len() as u32;
    let axhub_related = records.iter().filter(|r| r.is_axhub_related).count() as u32;
    let auth_failed = records.iter().filter(|r| !r.auth_ok).count() as u32;

    // AC-12 / spec §94: decision-type breakdown (axhub/yield/ignore/ask/explicit).
    // Lines predating the decision field (legacy) carry `decision == None`; they
    // are bucketed as "legacy" so counts never silently misattribute. ignore_rate
    // measures the non-axhub pass-through signal (spec §82) over decided records.
    let mut decision_counts: std::collections::BTreeMap<&'static str, u32> =
        std::collections::BTreeMap::new();
    for r in &records {
        let label = r.decision.map_or("legacy", |d| d.as_str());
        *decision_counts.entry(label).or_insert(0) += 1;
    }
    let ignore_count = decision_counts.get("ignore").copied().unwrap_or(0);
    let decided_total: u32 = decision_counts
        .iter()
        .filter(|(label, _)| **label != "legacy")
        .map(|(_, count)| *count)
        .sum();
    let ignore_rate = if decided_total > 0 {
        ignore_count as f64 / decided_total as f64
    } else {
        0.0
    };

    let mut lengths: Vec<u32> = records.iter().map(|r| r.prompt_len).collect();
    lengths.sort_unstable();
    let p50 = percentile(&lengths, 0.50);
    let p95 = percentile(&lengths, 0.95);

    let mut versions: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for r in &records {
        if let Some(v) = &r.cli_version {
            *versions.entry(v.clone()).or_insert(0) += 1;
        }
    }

    let mut hash_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for r in records.iter().filter(|r| r.is_axhub_related) {
        *hash_counts.entry(r.prompt_hash.clone()).or_insert(0) += 1;
    }
    let mut top_hashes: Vec<(String, u32)> = hash_counts.into_iter().collect();
    top_hashes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    top_hashes.truncate(top as usize);

    let mut confused_counts: std::collections::HashMap<(String, Option<String>), (u32, String)> =
        std::collections::HashMap::new();
    for r in records.iter().filter(|r| r.clarify_invoked) {
        let entry = confused_counts
            .entry((r.prompt_hash.clone(), r.chosen_skill.clone()))
            .or_insert((0, r.ts.clone()));
        entry.0 += 1;
        if r.ts.as_str() > entry.1.as_str() {
            entry.1 = r.ts.clone();
        }
    }
    let mut confused_rows: Vec<(String, Option<String>, u32, String)> = confused_counts
        .into_iter()
        .map(|((hash, chosen_skill), (count, latest_ts))| (hash, chosen_skill, count, latest_ts))
        .collect();
    confused_rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

    if json {
        let summary = json!({
            "total_prompts": total,
            "axhub_related": axhub_related,
            "axhub_related_rate": axhub_related as f64 / total as f64,
            "auth_failed": auth_failed,
            "decision_counts": decision_counts,
            "ignore_count": ignore_count,
            "ignore_rate": ignore_rate,
            "prompt_length_p50": p50,
            "prompt_length_p95": p95,
            "cli_versions": versions,
            "top_axhub_hashes": top_hashes.iter().map(|(h, c)| json!({"hash": h, "count": c})).collect::<Vec<_>>(),
            "confused_prompts": confused_rows.iter().map(|(hash, chosen_skill, count, latest_ts)| json!({
                "hash": hash,
                "count": count,
                "chosen_skill": chosen_skill,
                "latest_ts": latest_ts,
            })).collect::<Vec<_>>(),
        });
        println!("{}", summary);
        return Ok(0);
    }

    // Korean default output
    println!("[지난 prompt 통계]");
    println!("==========================================");
    println!("총 prompt:           {total}");
    let rate_pct = 100.0 * axhub_related as f64 / total as f64;
    println!("axhub 관련 가능성:    {axhub_related} ({rate_pct:.1}%)");
    println!("auth 실패:           {auth_failed}");
    println!("prompt 길이 p50/p95: {p50} / {p95} bytes");
    println!();
    println!("CLI 버전:");
    for (v, c) in &versions {
        println!("  {v}: {c}");
    }
    if !top_hashes.is_empty() {
        println!();
        println!("상위 axhub 관련 prompt (hash):");
        for (h, c) in &top_hashes {
            println!("  {h}: {c:>4}");
        }
    }
    println!();
    println!("결정 타입 분포 (axhub/yield/ignore/ask/explicit):");
    for (label, count) in &decision_counts {
        println!("  {label}: {count}");
    }
    if decided_total > 0 {
        println!(
            "non-axhub ignore 율: {:.1}% ({ignore_count}/{decided_total})",
            100.0 * ignore_rate
        );
    }
    println!();
    if let Some(dir) = axhub_helpers::runtime_paths::state_dir() {
        println!("audit log 위치: {}", dir.display());
    }
    println!("끄려면: AXHUB_NO_AUDIT=1");
    println!("삭제: axhub-helpers cleanup-audit --all");
    Ok(0)
}

const CLEANUP_AUDIT_HELP: &str = "axhub-helpers cleanup-audit — audit log 삭제

USAGE:
  axhub-helpers cleanup-audit          # 7일 이상 된 파일만 삭제 (rotation)
  axhub-helpers cleanup-audit --all    # 전체 삭제 (확인 prompt)
  axhub-helpers cleanup-audit --all --yes   # 확인 우회

OPTIONS:
  --all      전체 삭제 (default 는 7일 이상만)
  --yes -y   확인 prompt 우회
  -h --help  도움말
";

fn cmd_cleanup_audit(args: &[String]) -> anyhow::Result<i32> {
    use axhub_helpers::audit;

    let mut all = false;
    let mut yes = false;
    for arg in args {
        match arg.as_str() {
            "--all" => all = true,
            "--yes" | "-y" => yes = true,
            "-h" | "--help" => {
                print!("{CLEANUP_AUDIT_HELP}");
                return Ok(0);
            }
            other => {
                eprintln!("axhub-helpers cleanup-audit: 알 수 없는 flag: {other}");
                return Ok(64);
            }
        }
    }

    if all {
        if !yes {
            print!("audit log 전체 삭제할까요? (y/N): ");
            use std::io::Write;
            std::io::stdout().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("취소했어요.");
                return Ok(0);
            }
        }
        let count = audit::cleanup_all()?;
        println!("audit log {count} 파일 삭제했어요.");
    } else {
        let count = audit::rotate(7)?;
        println!("7일 이상 된 audit log {count} 파일 삭제했어요. 전체 삭제는 --all 사용해요.");
    }
    Ok(0)
}

// Phase 10 — audit-clarify subcommand: clarify SKILL fires this command after the
// user picks a final disambiguation. Adds an audit record with clarify_invoked=true
// + chosen_skill=Some(name). routing-stats --confused filters on this signal.

#[allow(dead_code)] // US3: -h 는 clap 자동 help 로 대체, 한국어 본문은 소스 보존
const AUDIT_CLARIFY_HELP: &str = "axhub-helpers audit-clarify — clarify feedback record

USAGE:
  axhub-helpers audit-clarify (--hash <prompt-hash>|--prompt <prompt>) --chosen <skill-name>

OPTIONS:
  --hash <H>       원본 prompt 의 sha256 hash (e.g. sha256:abc...)
  --prompt <P>     원본 prompt. helper 가 로컬에서 sha256 hash 로 변환해요.
  --chosen <S>     사용자가 final 선택한 skill name (또는 'null')
  -h, --help       도움말
";

pub(crate) fn cmd_audit_clarify(
    hash: Option<String>,
    prompt: Option<String>,
    chosen: Option<String>,
) -> anyhow::Result<i32> {
    use axhub_helpers::audit::{self, sha256_hex, AuditRecord};
    if hash.is_some() && prompt.is_some() {
        eprintln!("axhub-helpers audit-clarify: --hash 또는 --prompt 하나만 사용해요");
        return Ok(64);
    }
    let (prompt_hash, prompt_len) = match (hash, prompt) {
        (Some(prompt_hash), None) => (prompt_hash, 0),
        (None, Some(prompt)) => (sha256_hex(&prompt), prompt.len() as u32),
        (None, None) => {
            eprintln!("axhub-helpers audit-clarify: --hash 또는 --prompt 필요해요");
            return Ok(64);
        }
        (Some(_), Some(_)) => unreachable!(),
    };
    let chosen_skill = chosen.and_then(|s| if s == "null" { None } else { Some(s) });
    let record = AuditRecord {
        ts: audit::now_iso8601(),
        prompt_hash,
        prompt_len,
        cli_version: None,
        auth_ok: false,
        is_axhub_related: false,
        clarify_invoked: true,
        chosen_skill,
        // Clarify is a feedback sentinel, not a routing-decision sample — leave the
        // decision + routing-input fields at their `None` defaults.
        ..Default::default()
    };
    audit::append(record).ok();
    println!("audit-clarify 기록했어요.");
    Ok(0)
}

// Phase 10 — routing-dashboard subcommand: per-skill stats HTML render.

const ROUTING_DASHBOARD_HELP: &str = "axhub-helpers routing-dashboard — per-skill drift dashboard

USAGE:
  axhub-helpers routing-dashboard [--html]

OPTIONS:
  --html      inline HTML render (per-skill table + drift trend + failing prompts)
  -h, --help  도움말
";

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn cmd_routing_dashboard(args: &[String]) -> anyhow::Result<i32> {
    use axhub_helpers::audit;
    let mut html_mode = false;
    let mut help = false;
    for arg in args {
        match arg.as_str() {
            "--html" => html_mode = true,
            "-h" | "--help" => help = true,
            _ => return legacy_usage_error("routing-dashboard", "unknown option"),
        }
    }
    if help {
        print!("{ROUTING_DASHBOARD_HELP}");
        return Ok(0);
    }
    let records = audit::read_since(chrono::Duration::days(7))?;
    let operational_records: Vec<_> = records.iter().filter(|r| !r.clarify_invoked).collect();
    let total = operational_records.len();
    let axhub_related = operational_records
        .iter()
        .filter(|r| r.is_axhub_related)
        .count();
    let auth_failed = operational_records.iter().filter(|r| !r.auth_ok).count();
    let confused = records.iter().filter(|r| r.clarify_invoked).count();
    let mut chosen_counts: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    for r in records.iter().filter(|r| r.clarify_invoked) {
        if let Some(skill) = &r.chosen_skill {
            *chosen_counts.entry(skill.clone()).or_insert(0) += 1;
        }
    }
    let mut rows: Vec<(String, u32)> = chosen_counts.into_iter().collect();
    rows.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    if html_mode {
        let mut chosen_rows = String::new();
        for (skill, count) in &rows {
            chosen_rows.push_str(&format!(
                "<tr><td>{}</td><td>{count}</td><td>n/a</td><td>n/a</td></tr>",
                html_escape(skill)
            ));
        }
        if chosen_rows.is_empty() {
            chosen_rows
                .push_str("<tr><td colspan=\"4\">clarify feedback 이 아직 없어요.</td></tr>");
        }
        let mut failing_rows = String::new();
        for r in records.iter().filter(|r| r.clarify_invoked).take(25) {
            failing_rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&r.prompt_hash),
                html_escape(r.chosen_skill.as_deref().unwrap_or("null")),
                html_escape(&r.ts),
            ));
        }
        if failing_rows.is_empty() {
            failing_rows
                .push_str("<tr><td colspan=\"3\">failing prompt hash 가 아직 없어요.</td></tr>");
        }
        let html = format!(
            include_str!("../templates/dashboard.html"),
            total = total,
            axhub_related = axhub_related,
            auth_failed = auth_failed,
            confused = confused,
            chosen_rows = chosen_rows,
            failing_rows = failing_rows,
        );
        print!("{html}");
    } else {
        println!("[axhub routing dashboard — last 7d]");
        println!("total prompts: {total}");
        println!("axhub-related: {axhub_related}");
        println!("auth failed: {auth_failed}");
        println!("clarify invoked: {confused}");
        if !rows.is_empty() {
            println!("\nUser-chosen skill (clarify feedback):");
            for (skill, count) in &rows {
                println!("  {skill:<16} {count}");
            }
        }
    }
    Ok(0)
}

// Phase 7 (Component 6): SessionStart magical-moment message.
//
// Base systemMessage (onboarding, common natural-language actions, and audit
// disclosure) + current-version first-session welcome (one-shot, gated by the
// welcome marker file). Marker write is best-effort — failure surfaces the
// welcome again next session, never blocks Claude.

const WELCOME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Warm the plugin + CLI version-drift caches in the background so the user's
/// first prompt this session can already see a freshly-published release —
/// without adding any session-start latency.
///
/// This replaces the detached `nohup …-fetch-bg &` spawns the shell wrappers
/// used to do, consolidating the trigger into one cross-platform Rust spot (no
/// `.ps1`/`.sh` fork to drift). The fetch itself stays detached: session-start
/// returns immediately while a child process warms the cache. A human takes
/// more than a second to type their first message, so the sub-second fetch
/// lands before the first prompt-route turn and the turn-1 nudge still fires —
/// at zero startup cost. A short cache TTL (`FETCH_TTL_SECS`) means a restart
/// re-fetches almost immediately, eliminating the detection lag a long TTL caused.
///
/// Fully fail-open: only a channel whose cache is stale/absent (`needs_refresh`)
/// is spawned, any spawn error is swallowed, and the next session retries.
fn warm_drift_caches() {
    // Only warm inside a real Claude Code plugin session. The session-start
    // shell wrapper hard-requires CLAUDE_PLUGIN_ROOT (exported by Claude Code),
    // so its presence marks a live hook invocation. Direct `cargo test`
    // invocations of `session-start` don't set it → the test suite never spawns
    // a network fetch.
    if std::env::var_os("CLAUDE_PLUGIN_ROOT").is_none() {
        return;
    }
    // Same gate the legacy shell fetch used: never fetch in CI / non-interactive.
    if std::env::var_os("CI").is_some() || std::env::var_os("CLAUDE_NON_INTERACTIVE").is_some() {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return; // can't locate ourselves → skip warming, never block
    };
    let exe = exe.to_string_lossy().into_owned();

    if axhub_helpers::plugin_update::needs_refresh() {
        let _ = axhub_helpers::spawn::spawn_detached_with_fallback(&[
            exe.as_str(),
            "plugin-latest-fetch-bg",
        ]);
    }
    if axhub_helpers::cli_drift::needs_refresh() {
        let _ = axhub_helpers::spawn::spawn_detached_with_fallback(&[
            exe.as_str(),
            "cli-latest-fetch-bg",
        ]);
    }
}

pub(crate) fn cmd_session_start() -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("session-start") {
        out_json(json!({}));
        return Ok(0);
    }
    // Warm the version-drift caches in the background (detached, fail-open) so the
    // user's first prompt this session can see a freshly-published plugin/CLI
    // release — without adding any session-start latency. See fn doc.
    warm_drift_caches();
    write_session_start_bundle_best_effort();

    let mut lines: Vec<String> = vec![
        format!("axhub 준비됐어요 (v{}).", env!("CARGO_PKG_VERSION")),
        "- 처음이면 \"처음 설정 도와줘\"라고 말하면 설치·로그인·첫 배포까지 안내해요.".to_string(),
        "- 막히거나 안 되면 \"설치 상태 확인해줘\" 또는 \"도움말 보여줘\"라고 말해 주세요."
            .to_string(),
        "- 자주 쓰는 말: \"배포해\", \"상태 보여줘\", \"로그 보여줘\", \"앱 목록 보여줘\"."
            .to_string(),
        "- 비대화형 환경에서는 안전한 기본값으로 진행하고 위험 작업은 승인 없이는 실행하지 않아요."
            .to_string(),
        "- 외부로 전송하지 않는 감사 로그는 로컬에 일주일간 저장돼요. 끄려면 말씀해주세요."
            .to_string(),
    ];

    let marker = welcome_marker_path(WELCOME_VERSION);
    let show_welcome = marker.as_ref().map(|p| !p.exists()).unwrap_or(false);
    if show_welcome {
        lines.push(String::new());
        lines.push(format!("[axhub v{WELCOME_VERSION} 첫 세션] 환영해요."));
        lines.push(
            "- 가장 쉬운 시작: \"안녕\" 또는 \"처음 설정 도와줘\" — 설치부터 첫 배포까지 함께 가요."
                .to_string(),
        );
        lines.push("- 이미 앱이 있으면 \"배포해\" 한마디면 돼요.".to_string());
        lines.push(
            "- 헷갈리면 \"도움말 보여줘\" 또는 \"설치 상태 확인해줘\"라고 말해 주세요.".to_string(),
        );

        if let Some(path) = marker {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(
                &path,
                format!("shown: {}\n", chrono::Utc::now().to_rfc3339()),
            );
        }
    }

    let context = lines.join("\n");
    let mut output = json!({"systemMessage": context});
    // spec 006 — quality-context injection is eager axhub infra, gated on the
    // project marker. Non-axhub projects (no axhub.yaml walk-up) get zero
    // quality-context footprint; marker-error falls open auth-conditionally.
    // The base systemMessage + welcome stay ungated (helper-runtime notice, not
    // one of the three gated targets per spec §범위).
    if !hook_safety::is_megaskill_disabled() && should_run_eager_infra() {
        if let Some(megaskill) = session_start_megaskill_context() {
            output["hookSpecificOutput"] = json!({
                "hookEventName": "SessionStart",
                "additionalContext": megaskill,
            });
        }
    }
    println!("{}", output);
    let mut m = Map::new();
    m.insert("event".into(), Value::String("session_start".into()));
    emit_meta_envelope(m).ok();
    Ok(0)
}

/// spec 006 — session-start eager-infra marker gate, shared by the shell wrapper
/// (`session-eager-gate` subcommand) and the in-helper quality-context injection
/// so the two can never disagree (composition-consistency).
///
/// "Run eager infra" is defined as **exactly** the bare-NL routing outcome
/// `Axhub`: reusing the locked `routing::decide_from_flags` priority chain (no
/// keywords, no slash) ties this gate to the single routing source of truth.
/// That yields: marker Present → run; Absent → skip (zero-footprint even for an
/// authed returning user); Unknown (fs error) → auth-conditional (token-file
/// `.exists()` stat only — never spawns the CLI or token-init bootstrap).
fn should_run_eager_infra() -> bool {
    use axhub_helpers::routing::{decide_from_flags, find_marker, token_present, RoutingDecision};
    matches!(
        decide_from_flags(false, false, find_marker(), token_present(), false),
        RoutingDecision::Axhub
    )
}

/// `session-eager-gate` subcommand: the shell session-start wrapper calls this to
/// decide whether to run the (token-init / Gatekeeper warmup) eager infra. Pure
/// exit-code contract: `0` = run, `1` = skip. Never panics (fail-open: the shell
/// treats any other rc as a spawn error and falls back auth-conditionally).
pub(crate) fn cmd_session_eager_gate() -> anyhow::Result<i32> {
    Ok(if should_run_eager_infra() { 0 } else { 1 })
}

/// `route-decision` subcommand (spec 006 §57/§68): the prompt-bearing consumer of
/// the shared routing-decision function for the **deploy SKILL preflight Step 0**.
///
/// The hook (`prompt-route`) consumes `routing::decide_from_flags` in-process; the
/// SKILL preflight is bash, so it needs this subcommand as its entry into the same
/// single source of truth. Both paths therefore inherit one decision for identical
/// inputs (composition-consistency, spec §49-59).
///
/// Inputs derived here (never spawns the axhub CLI, never triggers token-init
/// bootstrap — auth is a cheap token-file `.exists()` stat, spec §102):
/// - `marker` = cwd→git-root walk-up for `axhub.yaml` ([`routing::find_marker`]),
/// - `authed` = [`routing::token_present`],
/// - keyword flags = the shared detectors over `user_utterance`,
/// - `explicit_invocation` = the model-passed `--explicit` (slash invocation, which
///   the SKILL detects from its invocation context because `commands/deploy.md`
///   forwards only `$ARGUMENTS` — the leading `/deploy` token is gone) OR a slash
///   still detectable in the utterance text. Either signal alone makes it explicit.
///
/// Always prints JSON and exits 0 (fail-open): the SKILL branches on `.decision`
/// and, if this emits nothing (binary truly missing), falls open to `axhub`.
pub(crate) fn cmd_route_decision(user_utterance: &str, explicit: bool) -> anyhow::Result<i32> {
    use axhub_helpers::routing::{
        axhub_keyword_present, decide_from_flags, find_marker, foreign_keyword_present,
        is_slash_invocation, token_present, MarkerStatus,
    };

    let marker = find_marker();
    let authed = token_present();
    let axhub_keyword = axhub_keyword_present(user_utterance);
    let foreign_keyword = foreign_keyword_present(user_utterance);
    // Either the model-passed slash signal OR a slash left in the utterance text
    // counts as explicit — rule 0 must win even if only one signal survives.
    let explicit_invocation = explicit || is_slash_invocation(user_utterance);
    let decision = decide_from_flags(
        axhub_keyword,
        foreign_keyword,
        marker,
        authed,
        explicit_invocation,
    );
    let marker_str = match marker {
        MarkerStatus::Present => "present",
        MarkerStatus::Absent => "absent",
        MarkerStatus::Unknown => "unknown",
    };
    out_json(json!({
        "decision": decision.as_str(),
        "marker": marker_str,
        "marker_present": marker == MarkerStatus::Present,
        "authed": authed,
        "axhub_keyword": axhub_keyword,
        "foreign_keyword": foreign_keyword,
        "explicit_invocation": explicit_invocation,
    }));
    Ok(0)
}

fn session_start_megaskill_context() -> Option<String> {
    let root = std::env::var_os("CLAUDE_PLUGIN_ROOT")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;
    let path = root.join("skills/using-axhub-quality/SKILL.md");
    let content = fs::read_to_string(path).ok()?;
    Some(format!(
        "<axhub-quality-auto-mode>\n[axhub hook | next-turn quality reminder]\nObserved: quality auto-mode available.\nSuggested: read `.axhub-state/quality.json` and call the matching quality SKILL when thresholds require it.\nSkip: AXHUB_DISABLE_HOOK=session-start or AXHUB_DISABLE_MEGASKILL=1\n</axhub-quality-auto-mode>\n\n{}",
        content
    ))
}

fn session_bundle_path() -> Option<PathBuf> {
    last_deploy_file()
        .map(|path| path.with_file_name("session-bundle.json"))
        .or_else(|| state_dir().map(|dir| dir.join("session-bundle.json")))
}

fn session_bundle_from_preflight(preflight: &PreflightRun) -> SessionBundle {
    let output = &preflight.output;
    SessionBundle {
        schema_version: axhub_helpers::session_bundle::SESSION_BUNDLE_SCHEMA_VERSION.to_string(),
        auth_status: AuthStatusBundle {
            ok: output.auth_ok,
            user_email: output.user_email.clone(),
            user_id: None,
            expires_at: output.expires_at.clone(),
            scopes: output.scopes.clone(),
        },
        current_app: output.current_app.clone(),
        current_env: output.current_env.clone(),
        last_deploy: output
            .last_deploy_id
            .as_ref()
            .map(|deployment_id| LastDeployBundle {
                deployment_id: deployment_id.clone(),
                status: output
                    .last_deploy_status
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                commit_sha: None,
            }),
        plugin_version: output.plugin_version.clone(),
        helper_version: env!("CARGO_PKG_VERSION").to_string(),
        written_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }
}

fn write_session_start_bundle_best_effort() {
    if std::env::var("AXHUB_SESSION_BUNDLE").as_deref() == Ok("0") {
        return;
    }
    let Some(path) = session_bundle_path() else {
        return;
    };
    let preflight = run_preflight();
    let bundle = session_bundle_from_preflight(&preflight);
    let _ = write_session_bundle(&bundle, &path);
}

pub(crate) fn cmd_list_deployments(
    app_id: Option<String>,
    limit_arg: Option<String>,
) -> anyhow::Result<i32> {
    let Some(app_id) = app_id else {
        eprintln!("--app (alias: --app-id) is required");
        return Ok(64);
    };
    let limit = match limit_arg {
        None => None,
        Some(s) => {
            let parsed = match s.parse::<usize>() {
                Ok(value) => value,
                Err(_) => {
                    eprintln!("invalid --limit: {s}");
                    return Ok(64);
                }
            };
            if !(1..=MAX_LIST_DEPLOYMENTS_LIMIT).contains(&parsed) {
                eprintln!("--limit must be between 1 and {MAX_LIST_DEPLOYMENTS_LIMIT}");
                return Ok(64);
            }
            Some(parsed)
        }
    };
    let result = run_list_deployments(ListDeploymentsArgs { app_id, limit });
    let code = result.exit_code;
    out_json(serde_json::to_value(result)?);
    Ok(code)
}

fn cmd_mark(rest: &[String]) -> anyhow::Result<i32> {
    let Some(phase_name) = rest.first() else {
        eprintln!("mark requires <phase_name>");
        return Ok(64);
    };
    if rest.len() != 1 {
        return legacy_usage_error("mark", "expected exactly one <phase_name>");
    }
    if phase_name.starts_with('-') {
        return legacy_usage_error("mark", "unknown option");
    }
    let path_env = match std::env::var("AXHUB_PHASE_MARKER_FILE") {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(0),
    };
    let path = std::path::PathBuf::from(path_env);
    if let Err(err) = append_phase_marker_to_file(&path, phase_name) {
        eprintln!("mark: {err}");
        return Ok(1);
    }
    Ok(0)
}

fn cmd_publish_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("publish-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("공개 심사 준비를 확인했어요.");

    if !output.cli_present {
        println!("- CLI를 먼저 설치해야 공개 심사 준비를 확인할 수 있어요.");
        println!("- 설치가 필요하면 \"axhub 설치해줘\"라고 말해 주세요.");
        return Ok(0);
    }
    if output.cli_too_old || output.cli_too_new {
        println!("- 현재 CLI 버전으로는 공개 심사 제출을 안전하게 진행하지 않을게요.");
        println!("- 먼저 설치 상태를 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }
    if !output.auth_ok {
        println!("- 로그인: 다시 로그인이 필요해요.");
        println!("- 제출 준비를 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        return Ok(0);
    }

    let resolve_args = vec![
        "--intent".to_string(),
        "publish".to_string(),
        "--user-utterance".to_string(),
        user_utterance.clone(),
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone())
        .or_else(|| output.current_app.clone());

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    println!("- 대상 앱: {}", redact(&app));
    if publish_note_present(&user_utterance) {
        println!("- 제출 사유: 대화에 포함된 문구로 미리보기를 만들 수 있어요.");
    } else {
        println!("- 제출 사유: 아직 필요해요.");
        println!("- 계속하려면 심사에 보낼 한 줄 사유를 알려 주세요.");
    }
    println!("- 공개 심사는 앱을 마켓플레이스 검토 대상으로 보내는 외부 변경 작업이에요.");
    println!("제출은 대상 앱과 사유 미리보기를 보여드리고, 명시적으로 승인받은 뒤에만 진행할게요.");
    Ok(0)
}

fn publish_note_present(utterance: &str) -> bool {
    let lower = utterance.to_lowercase();
    let p = lower.as_str();
    [
        "사유는",
        "사유:",
        "제출 사유",
        "note",
        "reason",
        "왜냐면",
        "설명은",
    ]
    .iter()
    .any(|needle| p.contains(needle))
}

fn cmd_rollback_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("rollback-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("되돌릴 수 있는 배포를 확인했어요.");

    if !output.cli_present {
        println!("- CLI를 먼저 설치해야 배포 기록을 확인할 수 있어요.");
        println!("- 설치가 필요하면 \"axhub 설치해줘\"라고 말해 주세요.");
        return Ok(0);
    }
    if output.cli_too_old || output.cli_too_new {
        println!("- 현재 CLI 버전으로는 되돌리기 여부를 안전하게 판단하지 않을게요.");
        println!("- 먼저 설치 상태를 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }
    if !output.auth_ok {
        println!("- 로그인: 다시 로그인이 필요해요.");
        println!("- 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        return Ok(0);
    }

    let resolve_args = vec![
        "--intent".to_string(),
        "recover".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone())
        .or_else(|| output.current_app.clone());

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let list = run_list_deployments(ListDeploymentsArgs {
        app_id: app,
        limit: Some(10),
    });
    if list.exit_code != 0 {
        println!("- 배포 기록을 확인하지 못했어요.");
        println!("- 로그인 상태나 앱 권한을 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }
    if list.deployments.is_empty() {
        println!("- 아직 배포 이력이 없어서 되돌릴 대상이 없어요.");
        println!("- 먼저 배포를 시작한 뒤 다시 확인하면 돼요.");
        return Ok(0);
    }

    let latest = &list.deployments[0];
    let successful = list
        .deployments
        .iter()
        .filter(|deploy| rollback_status_successish(&deploy.status))
        .collect::<Vec<_>>();

    if rollback_status_successish(&latest.status) {
        describe_successful_latest_for_rollback(&successful);
    } else {
        describe_unsuccessful_latest_for_rollback(&successful, latest);
    }

    Ok(0)
}

fn describe_successful_latest_for_rollback(successful: &[&DeploymentSummary]) {
    if successful.len() >= 2 {
        println!("- 현재 공개된 버전에서 한 단계 이전 성공 버전으로 되돌릴 수 있어요.");
        println!("- 이 작업은 이전 성공 버전을 새 배포로 다시 올리는 방식이에요.");
        println!("진행하려면 \"진행\"이라고 답해 주세요. 실제 변경 전에는 한 번 더 미리보기와 승인을 받을게요.");
    } else {
        println!("- 현재 공개된 성공 배포는 찾았지만, 그보다 이전 성공 배포는 없어요.");
        println!("- 되돌릴 대상이 부족해서 지금은 변경하지 않을게요.");
    }
}

fn describe_unsuccessful_latest_for_rollback(
    successful: &[&DeploymentSummary],
    latest: &DeploymentSummary,
) {
    if rollback_status_in_flight(&latest.status) {
        println!("- 방금 시도한 배포는 아직 진행 중이에요.");
        println!("- 지금 되돌리기보다 먼저 상태가 끝나는지 확인하는 편이 안전해요.");
        println!("- 계속 보려면 \"배포 상태 봐줘\"라고 말해 주세요.");
        return;
    }

    if successful.is_empty() {
        println!("- 방금 시도한 배포는 공개 버전으로 반영되지 않았어요.");
        println!("- 이전에 성공한 배포를 찾지 못해서 되돌릴 대상이 없어요.");
        println!("- 실패 원인을 본 뒤 다시 배포하는 쪽이 안전해요.");
        return;
    }

    println!("- 방금 시도한 배포는 공개 버전으로 반영되지 않았어요.");
    println!("- 현재 공개된 버전은 이미 최근 성공 버전으로 보입니다.");
    if successful.len() >= 2 {
        println!("- 그래도 한 단계 더 이전 성공 버전으로 되돌리는 선택지는 있어요.");
        println!("진행하려면 \"진행\"이라고 답해 주세요. 실제 변경 전에는 한 번 더 미리보기와 승인을 받을게요.");
    } else {
        println!("- 더 이전에 되돌릴 성공 배포는 찾지 못했어요.");
        println!("- 지금은 그대로 두는 게 안전해요.");
    }
}

fn rollback_status_successish(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "succeeded" | "success" | "live" | "deployed" | "active" | "ok"
    )
}

fn rollback_status_in_flight(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "pending" | "queued" | "building" | "deploying" | "running" | "in_progress"
    )
}

fn cmd_team_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("team-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("팀 작업을 확인했어요.");

    if !output.cli_present {
        println!("- CLI를 먼저 설치해야 팀 작업을 확인할 수 있어요.");
        println!("- 설치가 필요하면 \"axhub 설치해줘\"라고 말해 주세요.");
        return Ok(0);
    }
    if output.cli_too_old || output.cli_too_new {
        println!("- 현재 CLI 버전으로는 팀 작업을 안전하게 진행하지 않을게요.");
        println!("- 먼저 설치 상태를 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }
    if !output.auth_ok {
        println!("- 로그인: 다시 로그인이 필요해요.");
        println!("- 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        return Ok(0);
    }

    let request = team_request_kind(&user_utterance);
    let team = output
        .current_team_id
        .as_deref()
        .map(str::trim)
        .filter(|team| !team.is_empty());

    match request {
        TeamRequestKind::ListInvitations => {
            let Some(team) = team else {
                println!("- 먼저 작업할 워크스페이스를 골라야 초대 목록을 볼 수 있어요.");
                println!("- 워크스페이스 이름을 알려 주면 그 범위에서 다시 확인할게요.");
                return Ok(0);
            };
            let list = run_axhub(&[
                "invitations",
                "list",
                "--status",
                "pending",
                "--expires-within",
                "168h",
                "--tenant",
                team,
                "--json",
            ]);
            if list.exit_code != 0 {
                println!("- 초대 목록을 확인하지 못했어요.");
                if list.timed_out {
                    println!("- 조회가 오래 걸려 중단했어요.");
                } else {
                    println!("- 워크스페이스 권한이나 로그인 상태를 확인한 뒤 다시 시도해 주세요.");
                }
                return Ok(0);
            }
            let parsed = serde_json::from_str::<Value>(&list.stdout).unwrap_or(Value::Null);
            let items = response_items(&parsed)
                .map(|items| items.as_slice())
                .unwrap_or(&[]);
            println!(
                "- 현재 선택된 워크스페이스의 대기 중인 초대: {}개",
                items.len()
            );
            for item in items.iter().take(5) {
                let email = item
                    .get("email")
                    .or_else(|| item.get("invitee_email"))
                    .and_then(Value::as_str)
                    .map(redact)
                    .unwrap_or_else(|| "이메일 숨김".to_string());
                let role = item
                    .get("role")
                    .and_then(Value::as_str)
                    .map(team_role_label)
                    .unwrap_or("멤버");
                println!("- {email}: {role}");
            }
            if items.len() > 5 {
                println!("- 그 밖에 {}개가 더 있어요.", items.len() - 5);
            }
            println!("초대 취소나 재발송은 대상 미리보기와 승인 후 진행할게요.");
        }
        TeamRequestKind::AppAccess => {
            let app = output
                .current_app
                .as_deref()
                .map(str::trim)
                .filter(|app| !app.is_empty());
            println!("- 작업: 앱 접근 공유");
            if let Some(app) = app {
                println!("- 대상 앱: {}", redact(app));
            } else {
                println!("- 대상 앱: 먼저 앱을 골라야 해요.");
            }
            if let Some(email) = extract_email_like(&user_utterance) {
                println!("- 대상자: {}", redact(&email));
            } else {
                println!("- 대상자 이메일이나 사용자 식별자가 아직 필요해요.");
            }
            println!("앱 접근 변경은 권한 변경 작업이에요. 대상 앱과 대상자를 확정한 뒤 미리보기와 승인 후 진행할게요.");
        }
        TeamRequestKind::InviteMember => {
            println!("- 작업: 워크스페이스 팀원 초대");
            if team.is_some() {
                println!("- 워크스페이스: 현재 선택된 워크스페이스");
            } else {
                println!("- 워크스페이스: 먼저 작업할 워크스페이스를 골라야 해요.");
            }
            if let Some(email) = extract_email_like(&user_utterance) {
                println!("- 초대 대상: {}", redact(&email));
                println!(
                    "- 역할: {}",
                    team_role_label(team_role_from_utterance(&user_utterance))
                );
                println!("초대 메일 발송은 권한 변경 작업이에요. 이대로 보낼지 미리보기와 승인 후 진행할게요.");
            } else {
                println!("- 초대할 사람의 이메일이 아직 필요해요.");
                println!("- 역할을 따로 말하지 않으면 기본 멤버로 준비할게요.");
                println!("이메일을 알려 주면 보낼 내용 미리보기를 보여드리고, 명시적으로 승인받은 뒤에만 발송할게요.");
            }
        }
    }

    Ok(0)
}

#[derive(Clone, Copy)]
enum TeamRequestKind {
    InviteMember,
    ListInvitations,
    AppAccess,
}

fn team_request_kind(utterance: &str) -> TeamRequestKind {
    let lower = utterance.to_lowercase();
    let p = lower.as_str();
    if [
        "초대 목록",
        "초대 리스트",
        "pending invite",
        "invitation list",
        "list invitations",
    ]
    .iter()
    .any(|needle| p.contains(needle))
    {
        TeamRequestKind::ListInvitations
    } else if [
        "앱 공유",
        "공유해",
        "접근 권한",
        "access",
        "grant access",
        "share app",
    ]
    .iter()
    .any(|needle| p.contains(needle))
    {
        TeamRequestKind::AppAccess
    } else {
        TeamRequestKind::InviteMember
    }
}

fn team_role_from_utterance(utterance: &str) -> &str {
    let lower = utterance.to_lowercase();
    let p = lower.as_str();
    if ["관리자", "admin", "owner"]
        .iter()
        .any(|needle| p.contains(needle))
    {
        "admin"
    } else if ["viewer", "읽기", "read only", "readonly"]
        .iter()
        .any(|needle| p.contains(needle))
    {
        "viewer"
    } else {
        "member"
    }
}

fn team_role_label(role: &str) -> &'static str {
    match role.to_ascii_lowercase().as_str() {
        "admin" | "owner" => "관리자",
        "viewer" | "read" | "readonly" => "읽기",
        _ => "멤버",
    }
}

fn extract_email_like(utterance: &str) -> Option<String> {
    utterance
        .split(|ch: char| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    ',' | ';' | '(' | ')' | '<' | '>' | '[' | ']' | '{' | '}' | '"' | '\''
                )
        })
        .map(str::trim)
        .find(|part| {
            let part = part.trim_matches('.');
            part.contains('@')
                && part.contains('.')
                && !part.starts_with('@')
                && !part.ends_with('@')
                && !part.ends_with('.')
        })
        .map(|part| part.trim_matches('.').to_string())
}

/// Deterministic git safety guard for SDK conversion (D2 = git-guarded preview-first).
/// The expert agent writes user source; this guard owns the rollback net so the
/// LLM never has to. A checkpoint snapshots existing state — it never authors
/// user source — so it stays within the helper invariant. Fail-open: any failure
/// returns `ok:false` with no checkpoint, and the SKILL refuses to apply.
fn cmd_migrate_guard(rest: &[String]) -> anyhow::Result<i32> {
    use std::process::Command;
    let mut dir: Option<String> = None;
    let mut checkpoint = false;
    let mut allow_dirty = false;
    let mut init_ok = false;
    let mut label = "axhub SDK 변환 checkpoint".to_string();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--dir" => {
                dir = rest.get(i + 1).cloned();
                i += 2;
            }
            "--checkpoint" => {
                checkpoint = true;
                i += 1;
            }
            "--allow-dirty" => {
                allow_dirty = true;
                i += 1;
            }
            "--init-ok" => {
                init_ok = true;
                i += 1;
            }
            "--label" => {
                if let Some(value) = rest.get(i + 1) {
                    label = value.clone();
                }
                i += 2;
            }
            // --json is implied; tolerate unknown flags (fail-open).
            _ => i += 1,
        }
    }
    let dir = dir.unwrap_or_else(|| ".".to_string());
    if !std::path::Path::new(&dir).exists() {
        out_json(json!({
            "ok": false, "dir": dir, "mode": "missing_dir", "checkpoint_ref": Value::Null,
            "message": format!("디렉터리를 찾지 못했어요: {dir}"),
        }));
        return Ok(0);
    }

    let git = |args: &[&str]| -> (bool, String) {
        match Command::new("git").arg("-C").arg(&dir).args(args).output() {
            Ok(out) => (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).trim().to_string(),
            ),
            Err(_) => (false, String::new()),
        }
    };
    // Commit with a baked identity so checkpoints work even where git user.* is unset.
    let git_commit = |msg: &str| -> bool {
        git(&[
            "-c",
            "user.email=axhub@local",
            "-c",
            "user.name=axhub",
            "commit",
            "-m",
            msg,
        ])
        .0
    };

    let (inside_ok, inside) = git(&["rev-parse", "--is-inside-work-tree"]);
    let inside_git = inside_ok && inside == "true";
    let dirty = inside_git && !git(&["status", "--porcelain"]).1.is_empty();
    let reset_rollback =
        |sha: &str| format!("git -C {dir} reset --hard {sha} && git -C {dir} clean -fd");

    if !checkpoint {
        let mode = if !inside_git {
            "no_git"
        } else if dirty {
            "git_dirty"
        } else {
            "git_clean"
        };
        out_json(json!({
            "ok": true, "dir": dir, "inside_git": inside_git, "dirty": dirty, "mode": mode,
            "checkpoint_ref": Value::Null, "rollback_command": Value::Null,
            "needs_decision": mode != "git_clean",
            "message": match mode {
                "git_clean" => "git clean tree — checkpoint 가능해요",
                "git_dirty" => "변경사항을 commit 후 재시도하거나 --allow-dirty 로 WIP commit 후 진행할래요?",
                _ => "git repo 가 아니에요 — --init-ok 로 git init + checkpoint 하거나 백업 후 진행할래요?",
            },
        }));
        return Ok(0);
    }

    if inside_git && !dirty {
        let sha = git(&["rev-parse", "HEAD"]).1;
        out_json(json!({
            "ok": true, "dir": dir, "mode": "git_clean", "checkpoint_ref": sha,
            "rollback_command": reset_rollback(&sha),
            "message": "clean tree HEAD 를 checkpoint 로 잡았어요. 변환 후 rollback_command 로 되돌려요.",
        }));
        return Ok(0);
    }
    if inside_git && dirty {
        if !allow_dirty {
            out_json(json!({
                "ok": false, "dir": dir, "mode": "git_dirty", "needs_decision": true,
                "checkpoint_ref": Value::Null,
                "message": "working tree 에 변경사항이 있어요. commit 후 재시도하거나 --allow-dirty 로 WIP commit 후 진행해요.",
            }));
            return Ok(0);
        }
        if !git(&["add", "-A"]).0 || !git_commit(&format!("{label} (WIP)")) {
            out_json(json!({
                "ok": false, "dir": dir, "mode": "git_dirty", "checkpoint_ref": Value::Null,
                "message": "WIP commit 에 실패했어요. 수동 commit 후 재시도해요.",
            }));
            return Ok(0);
        }
        let sha = git(&["rev-parse", "HEAD"]).1;
        out_json(json!({
            "ok": true, "dir": dir, "mode": "git_dirty_wip", "checkpoint_ref": sha,
            // clean -fd removes only files created AFTER this checkpoint (the
            // expert's wrapper); the user's pre-existing changes are already in
            // the WIP commit, so reset --hard restores them untouched.
            "rollback_command": reset_rollback(&sha),
            "message": "기존 변경을 WIP commit 으로 저장하고 checkpoint 로 잡았어요. rollback_command 로 변환을 되돌려요.",
        }));
        return Ok(0);
    }

    // Not a git repo.
    if !init_ok {
        out_json(json!({
            "ok": false, "dir": dir, "mode": "no_git", "needs_decision": true,
            "checkpoint_ref": Value::Null,
            "message": "git repo 가 아니에요. --init-ok 로 git init + checkpoint 하거나 touched 파일 백업 후 진행해요.",
        }));
        return Ok(0);
    }
    if !git(&["init"]).0 || !git(&["add", "-A"]).0 || !git_commit(&label) {
        out_json(json!({
            "ok": false, "dir": dir, "mode": "no_git", "checkpoint_ref": Value::Null,
            "message": "git init/checkpoint 에 실패했어요. 수동 백업 후 진행해요.",
        }));
        return Ok(0);
    }
    let sha = git(&["rev-parse", "HEAD"]).1;
    out_json(json!({
        "ok": true, "dir": dir, "mode": "no_git_init", "checkpoint_ref": sha,
        "rollback_command": reset_rollback(&sha),
        "message": "git init 후 첫 commit 을 checkpoint 로 잡았어요. rollback_command 로 변환을 되돌려요.",
    }));
    Ok(0)
}

fn cmd_migrate_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("migrate-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("가져오기 상태를 확인했어요.");

    if !output.cli_present {
        println!("- CLI를 먼저 설치해야 앱 가져오기와 배포를 이어갈 수 있어요.");
        println!("- 설치가 필요하면 \"axhub 설치해줘\"라고 말해 주세요.");
        return Ok(0);
    }
    if output.cli_too_old || output.cli_too_new {
        println!("- 현재 CLI 버전으로는 가져오기 절차를 안전하게 진행하지 않을게요.");
        println!("- 먼저 설치 상태를 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }
    if !output.auth_ok {
        println!("- 로그인: 다시 로그인이 필요해요.");
        println!("- 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        return Ok(0);
    }

    let cwd = std::env::current_dir()?;
    let plan = match build_migrate_plan(&cwd) {
        Ok(plan) => plan,
        Err(err) => {
            println!("- 현재 폴더를 앱 후보로 읽지 못했어요.");
            println!("- 이유: {}", redact(&err.to_string()));
            println!("- 앱 폴더에서 다시 묻거나, 가져올 폴더를 알려 주세요.");
            return Ok(0);
        }
    };
    let has_manifest = cwd.join("axhub.yaml").is_file();

    if has_manifest {
        println!("- 이 프로젝트는 이미 AXHub 앱 설정이 있어요.");
        if let Some(app) = output
            .current_app
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            println!("- 연결된 앱: {}", redact(app));
        }
        if let Some(candidate) = plan.candidates.first() {
            println!(
                "- 감지된 앱 형태: {}",
                migrate_stack_label(&candidate.stack_hint)
            );
        }
        println!("- 새로 옮기는 작업은 필요 없고, 설정 점검이나 배포 준비로 이어갈 수 있어요.");
        println!("변경 작업은 미리보기와 승인 후 진행할게요.");
        return Ok(0);
    }

    if plan.candidates.is_empty() {
        println!("- 현재 폴더에서 바로 가져올 웹 앱 후보를 찾지 못했어요.");
        println!("- 앱 루트 폴더에서 다시 묻거나, 가져올 하위 폴더를 알려 주세요.");
        return Ok(0);
    }

    println!(
        "- 가져올 수 있는 앱 후보를 {}개 찾았어요.",
        plan.candidates.len()
    );
    let first = &plan.candidates[0];
    println!(
        "- 우선 후보: {} ({})",
        if first.path == "." {
            "현재 폴더"
        } else {
            &first.path
        },
        migrate_stack_label(&first.stack_hint)
    );
    if first.has_compose {
        println!("- 배포 방식: compose 설정을 사용할 수 있어 보여요.");
    } else if first.has_dockerfile {
        println!("- 배포 방식: Dockerfile을 사용할 수 있어 보여요.");
    } else {
        println!("- 배포 방식: 자동 감지로 시작할 수 있어 보여요.");
    }
    let env_count = plan.env_refs.len();
    if env_count > 0 {
        println!(
            "- 필요한 환경변수 이름 {}개를 찾았어요. 값은 표시하지 않았어요.",
            env_count
        );
    }
    println!("앱 등록, GitHub 연결, 배포 같은 변경은 미리보기와 승인 후 진행할게요.");
    Ok(0)
}

fn migrate_stack_label(stack_hint: &str) -> &'static str {
    match stack_hint.to_ascii_lowercase().as_str() {
        "nextjs" | "next.js" => "Next.js",
        "node" | "nodejs" => "Node.js",
        "python" => "Python 웹 앱",
        "fastapi" => "FastAPI",
        "django" => "Django",
        "flask" => "Flask",
        "go" => "Go 웹 앱",
        "rust" => "Rust 웹 앱",
        "ruby" => "Ruby 웹 앱",
        "java" => "Java 웹 앱",
        "kotlin" => "Kotlin 웹 앱",
        "docker" => "Docker 앱",
        "compose" => "Compose 앱",
        _ => "웹 앱",
    }
}

fn cmd_status_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("status-summary", rest)?;
    let resolve_args = vec![
        "--intent".to_string(),
        "status".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone());

    println!("배포 상태를 확인했어요.");

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let list = run_list_deployments(ListDeploymentsArgs {
        app_id: app.clone(),
        limit: Some(1),
    });
    if list.exit_code != 0 {
        let message = list
            .error_message_kr
            .unwrap_or_else(|| "배포 목록을 가져오지 못했어요.".to_string());
        println!("- {message}");
        println!("- 잠시 뒤 다시 확인하거나 로그인 상태를 확인해 주세요.");
        return Ok(0);
    }

    let Some(deploy) = list.deployments.first() else {
        println!("- 아직 이 앱의 배포 이력이 없어요.");
        println!("- 먼저 배포를 시작한 뒤 다시 확인하면 돼요.");
        return Ok(0);
    };

    let status = run_axhub(&["--json", "deploy", "status", &deploy.id, "--app", &app]);
    let status_json = serde_json::from_str::<Value>(&status.stdout).ok();
    let status_text = status_json
        .as_ref()
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .unwrap_or(deploy.status.as_str());
    let started_at = status_json
        .as_ref()
        .and_then(|v| v.get("started_at"))
        .and_then(Value::as_str)
        .or({
            if deploy.created_at.is_empty() {
                None
            } else {
                Some(deploy.created_at.as_str())
            }
        });
    let completed_at = status_json
        .as_ref()
        .and_then(|v| v.get("completed_at"))
        .and_then(Value::as_str);
    let failure_reason = status_json
        .as_ref()
        .and_then(|v| v.get("failure_reason"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty());

    println!(
        "- 앱 {}의 최근 배포는 {}",
        app,
        deploy_status_sentence(status_text)
    );
    if let Some(started_at) = started_at {
        println!("- 시작 시간: {}", compact_time(started_at));
    }
    if let Some(completed_at) = completed_at {
        println!("- 완료 시간: {}", compact_time(completed_at));
    }
    if !deploy.commit_sha.is_empty() {
        println!("- 커밋: {}", short_commit(&deploy.commit_sha));
    }
    if let Some(reason) = failure_reason {
        println!("- 실패 이유: {reason}");
    }
    if matches!(
        status_text,
        "queued" | "pending" | "building" | "deploying" | "running" | "in_progress"
    ) {
        println!("아직 진행 중이에요. 잠시 뒤 다시 확인하면 이어서 볼 수 있어요.");
    } else if status_text == "succeeded" {
        println!("배포는 끝난 상태예요.");
    } else {
        println!("자세한 원인은 로그나 실패 추적으로 이어서 확인할 수 있어요.");
    }
    Ok(0)
}

fn cmd_logs_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("logs-summary", rest)?;
    let resolve_args = vec![
        "--intent".to_string(),
        "logs".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone());

    println!("로그를 확인했어요.");

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let list = run_list_deployments(ListDeploymentsArgs {
        app_id: app.clone(),
        limit: Some(1),
    });
    if list.exit_code != 0 {
        let message = list
            .error_message_kr
            .unwrap_or_else(|| "배포 목록을 가져오지 못했어요.".to_string());
        println!("- {message}");
        println!("- 로그인 상태나 앱 권한을 확인한 뒤 다시 로그를 볼 수 있어요.");
        return Ok(0);
    }

    let Some(deploy) = list.deployments.first() else {
        println!("- 아직 이 앱의 배포 이력이 없어서 보여줄 로그도 없어요.");
        println!("- 먼저 배포를 시작한 뒤 다시 로그를 보면 돼요.");
        return Ok(0);
    };

    let logs = run_axhub(&[
        "--json", "deploy", "logs", &deploy.id, "--app", &app, "--limit", "50",
    ]);
    if logs.timed_out {
        println!("- 로그 조회가 오래 걸려 중단했어요.");
        println!("- 잠시 뒤 다시 묻거나, 실시간 로그가 필요하면 그렇게 말해 주세요.");
        return Ok(0);
    }
    if logs.exit_code != 0 {
        println!("- 로그를 가져오지 못했어요.");
        println!(
            "- 최근 배포 상태는 {}.",
            deploy_status_sentence(&deploy.status)
        );
        println!("- 로그인 상태나 앱 권한을 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }

    let lines = extract_log_lines(&logs.stdout, 50);
    println!(
        "- 앱 {}의 최근 배포는 {}",
        app,
        deploy_status_sentence(&deploy.status)
    );
    if !deploy.commit_sha.is_empty() {
        println!("- 커밋: {}", short_commit(&deploy.commit_sha));
    }
    if lines.is_empty() {
        println!("- 지금 가져올 수 있는 로그가 없어요.");
        println!("- 배포가 너무 빨리 끝났거나 아직 로그가 저장되지 않았을 수 있어요.");
        return Ok(0);
    }

    if let Some(error) = first_error_like_line(&lines) {
        println!("- 눈에 띄는 오류: {error}");
    }
    println!("- 최근 로그:");
    for line in lines {
        println!("  {}", redact(&line));
    }
    Ok(0)
}

fn cmd_open_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("open-summary", rest)?;
    let resolve_args = vec![
        "--intent".to_string(),
        "open".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone());

    println!("앱 페이지를 확인했어요.");

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let opened = run_axhub(&["open", &app, "--json"]);
    if opened.timed_out {
        println!("- 앱 페이지 확인이 오래 걸려 중단했어요.");
        println!("- 잠시 뒤 다시 묻거나 앱 이름을 함께 말해 주세요.");
        return Ok(0);
    }
    if opened.exit_code != 0 {
        println!("- 앱 페이지를 열지 못했어요.");
        println!("- 로그인 상태나 앱 권한을 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }

    let parsed = serde_json::from_str::<Value>(&opened.stdout).unwrap_or(Value::Null);
    let data = parsed.get("data").unwrap_or(&parsed);
    let url = data
        .get("url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    let status = data
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("opening");
    let opened_flag = data.get("opened").and_then(Value::as_bool).unwrap_or(false);

    println!("- 앱: {app}");
    if !url.is_empty() {
        println!("- URL: {}", redact(&url));
    }
    if opened_flag || status == "opening" {
        println!("- 브라우저에서 열기 요청을 보냈어요.");
    } else {
        println!("- 브라우저가 자동으로 열리지 않으면 위 URL을 열면 돼요.");
    }
    Ok(0)
}

fn cmd_env_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("env-summary", rest)?;
    let resolve_args = vec![
        "--intent".to_string(),
        "env".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone());

    println!("환경변수 목록을 확인했어요.");

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let list = run_axhub(&["env", "list", "--app", &app, "--json"]);
    if list.exit_code != 0 {
        println!("⚠️ 환경변수를 확인하지 못했어요");
        if list.timed_out {
            println!("- 조회가 5초 안에 끝나지 않았어요.");
        } else {
            println!("- 로그인 상태나 앱 권한을 확인한 뒤 다시 물어봐 주세요.");
        }
        return Ok(0);
    }

    let parsed = serde_json::from_str::<Value>(&list.stdout).unwrap_or(Value::Null);
    let items = parsed.get("items").and_then(Value::as_array).or_else(|| {
        parsed
            .get("data")
            .and_then(|data| data.get("items"))
            .and_then(Value::as_array)
    });
    let Some(items) = items else {
        println!("⚠️ 환경변수 응답을 읽지 못했어요");
        println!("- 값은 표시하지 않았어요.");
        println!("- 잠시 뒤 다시 확인해 주세요.");
        return Ok(0);
    };

    if items.is_empty() {
        println!("- 앱 {app}에는 등록된 환경변수가 없어요.");
        println!("- 추가가 필요하면 \"환경변수 추가하고 싶어\"라고 말하면 돼요.");
        return Ok(0);
    }

    println!("- 앱: {app}");
    println!("- 총 {}개가 있어요. 값은 안전하게 숨겼어요.", items.len());
    println!();
    println!("| 이름 | 단계 | 값 |");
    println!("| --- | --- | --- |");
    for item in items {
        let key = item
            .get("key")
            .or_else(|| item.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("(이름 없음)");
        let stage = item
            .get("stage")
            .or_else(|| item.get("scope"))
            .and_then(Value::as_str)
            .unwrap_or("runtime");
        let has_value = item.get("value").is_some_and(|v| !v.is_null());
        let value_label = if has_value {
            "있음(숨김)"
        } else {
            "없음"
        };
        println!(
            "| {} | {} | {} |",
            markdown_cell(&redact(key)),
            markdown_cell(&redact(stage)),
            value_label
        );
    }
    println!();
    println!(
        "값을 직접 표시하지는 않았어요. 추가/수정/삭제가 필요하면 어떤 키를 바꿀지 말해 주세요."
    );
    Ok(0)
}

fn cmd_github_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("github-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("GitHub 연결 상태를 확인했어요.");

    if !output.cli_present {
        println!("- CLI를 먼저 설치해야 GitHub 연결 상태를 확인할 수 있어요.");
        println!("- 설치가 필요하면 \"axhub 설치해줘\"라고 말해 주세요.");
        return Ok(0);
    }
    if output.cli_too_old || output.cli_too_new {
        println!("- 현재 CLI 버전으로는 GitHub 연결 상태를 안전하게 확인하지 않을게요.");
        println!("- 먼저 설치 상태를 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }
    if !output.auth_ok {
        println!("- 로그인: 다시 로그인이 필요해요.");
        println!("- 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        return Ok(0);
    }

    let resolve_args = vec![
        "--intent".to_string(),
        "github".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone())
        .or_else(|| output.current_app.clone());

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let status = run_axhub(&["apps", "git", "status", "--app", &app, "--json"]);
    if status.timed_out {
        println!("- GitHub 연결 상태 확인이 오래 걸려 중단했어요.");
        println!("- 잠시 뒤 다시 묻거나 앱 이름을 함께 말해 주세요.");
        return Ok(0);
    }
    if status.exit_code != 0 {
        println!("- GitHub 연결 상태를 확인하지 못했어요.");
        println!("- 앱 권한이나 로그인 상태를 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }

    let parsed = serde_json::from_str::<Value>(&status.stdout).unwrap_or(Value::Null);
    let data = parsed.get("data").unwrap_or(&parsed);
    let connected = data
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            data.get("repo_full_name")
                .and_then(Value::as_str)
                .is_some_and(|repo| !repo.trim().is_empty())
        });
    let repo = data
        .get("repo_full_name")
        .or_else(|| data.get("repository"))
        .or_else(|| data.get("repo"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|repo| !repo.is_empty());
    let branch = data
        .get("branch")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|branch| !branch.is_empty());
    let install_url = data
        .get("install_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty());

    if connected {
        println!("- 앱 {app}는 GitHub 저장소에 연결되어 있어요.");
        if let Some(repo) = repo {
            println!("- 저장소: {}", redact(repo));
        }
        if let Some(branch) = branch {
            println!("- 브랜치: {}", redact(branch));
        }
        println!(
            "연결 변경이나 해제가 필요하면 말해 주세요. 변경 작업은 미리보기와 승인 후 진행할게요."
        );
    } else {
        println!("- 앱 {app}는 아직 GitHub 저장소에 연결되어 있지 않아요.");
        if let Some(install_url) = install_url {
            println!("- GitHub 연결 링크: {}", redact(install_url));
        }
        println!("연결하려면 저장소와 브랜치를 정한 뒤 미리보기와 승인 후 진행할게요.");
    }

    Ok(0)
}

fn cmd_resources_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("resources-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("리소스 현황을 확인했어요.");

    if !output.cli_present {
        println!("- CLI를 먼저 설치해야 리소스를 확인할 수 있어요.");
        println!("- 설치가 필요하면 \"axhub 설치해줘\"라고 말해 주세요.");
        return Ok(0);
    }
    if output.cli_too_old || output.cli_too_new {
        println!("- 현재 CLI 버전으로는 리소스 변경을 안전하게 진행하지 않을게요.");
        println!("- 먼저 설치 상태를 확인한 뒤 다시 시도해 주세요.");
        return Ok(0);
    }
    if !output.auth_ok {
        println!("- 로그인: 다시 로그인이 필요해요.");
        println!("- 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        return Ok(0);
    }

    let resources = run_axhub(&["resources", "list", "--json"]);
    if resources.exit_code != 0 {
        println!("- 리소스 목록을 확인하지 못했어요.");
        if resources.timed_out {
            println!("- 조회가 5초 안에 끝나지 않았어요.");
        } else {
            println!("- 현재 워크스페이스 권한이나 로그인 상태를 다시 확인해 주세요.");
        }
        return Ok(0);
    }
    let connectors = run_axhub(&["connectors", "list", "--enabled-only", "--json"]);

    let parsed_resources = serde_json::from_str::<Value>(&resources.stdout).unwrap_or(Value::Null);
    let resource_items = response_items(&parsed_resources)
        .map(|items| items.as_slice())
        .unwrap_or(&[]);

    let parsed_connectors =
        serde_json::from_str::<Value>(&connectors.stdout).unwrap_or(Value::Null);
    let connector_count = response_items(&parsed_connectors)
        .map(|items| items.len())
        .unwrap_or(0);

    println!("- 데이터베이스 연결: {connector_count}개");
    println!("- 정리할 리소스: {}개", resource_items.len());

    if resource_items.is_empty() {
        println!();
        println!("지금 워크스페이스에서 정리할 리소스를 찾지 못했어요.");
        println!("- 아직 외부 데이터베이스 연결이나 리소스 등록이 없을 수 있어요.");
        println!("- 다른 워크스페이스를 쓰려는 거라면 워크스페이스 이름을 알려 주세요.");
        println!();
        println!("어떤 정리를 할까요? 목록 확인, 이름 변경, 이동, 태그 정리, 등록, 삭제 중에서 골라주세요.");
        println!("변경 작업은 대상과 작업을 정한 뒤 미리보기와 승인 후 진행할게요.");
        return Ok(0);
    }

    println!();
    println!("| 리소스 | 종류 |");
    println!("| --- | --- |");
    for item in resource_items.iter().take(8) {
        let name = item
            .get("name")
            .or_else(|| item.get("title"))
            .or_else(|| item.get("display_name"))
            .or_else(|| item.get("path"))
            .and_then(Value::as_str)
            .unwrap_or("이름 없는 리소스");
        let kind = item
            .get("kind")
            .and_then(Value::as_str)
            .map(resource_kind_label)
            .unwrap_or("리소스");
        println!("| {} | {} |", markdown_cell(&redact(name)), kind);
    }
    if resource_items.len() > 8 {
        println!();
        println!("- 그 밖에 {}개가 더 있어요.", resource_items.len() - 8);
    }
    println!();
    println!("어떤 정리를 할까요? 이름 변경, 이동, 태그 정리, 등록, 삭제 중에서 골라주세요.");
    println!("변경 작업은 대상과 작업을 정한 뒤 미리보기와 승인 후 진행할게요.");
    Ok(0)
}

fn cmd_review_scope_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("review-scope-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("리뷰 범위를 확인했어요.");
    if output.auth_ok {
        println!("- 로그인되어 있어요. 변경 범위 확인할게요.");
    } else {
        println!(
            "- 로그인 상태는 리뷰 진행을 막지 않아요. 필요한 AXHub 작업은 나중에 다시 확인할게요."
        );
    }

    let pathspecs = [
        ".",
        ":!desktop-pure-routing-results.md",
        ":!desktop-*routing-results.md",
        ":!desktop-qa-results.md",
        ":!.axhub-state/**",
        ":!.omx/**",
        ":!.omc/**",
        ":!.shim/**",
        ":!.shim-local-market/**",
        ":!.claude/**",
        ":!node_modules/**",
        ":!*test-results*",
        ":!*.log",
    ];

    let mut diff_args = vec!["diff", "HEAD", "--numstat", "--"];
    diff_args.extend(pathspecs);
    let diff = std::process::Command::new("git").args(&diff_args).output();

    let Ok(diff) = diff else {
        println!("- 변경 범위: 현재 폴더의 git 상태를 확인하지 못했어요.");
        println!("- 다음: 현재 작업 폴더가 맞는지 확인한 뒤 실제 변경 파일을 열어 리뷰하면 돼요.");
        return Ok(0);
    };

    if !diff.status.success() {
        println!("- 변경 범위: HEAD 기준 diff 를 확인하지 못했어요.");
        println!("- 다음: 현재 작업 폴더가 git 저장소인지 확인한 뒤 실제 변경 파일을 열어 리뷰하면 돼요.");
        return Ok(0);
    }

    let stdout = String::from_utf8_lossy(&diff.stdout);
    let mut files = std::collections::BTreeSet::new();
    let mut added: u64 = 0;
    let mut deleted: u64 = 0;
    for line in stdout.lines() {
        let mut parts = line.split('\t');
        let add = parts.next().unwrap_or("0");
        let del = parts.next().unwrap_or("0");
        let path = parts.next().unwrap_or("").trim();
        if path.is_empty() || review_path_excluded(path) {
            continue;
        }
        if let Ok(n) = add.parse::<u64>() {
            added += n;
        }
        if let Ok(n) = del.parse::<u64>() {
            deleted += n;
        }
        files.insert(path.to_string());
    }

    let mut untracked_args = vec!["ls-files", "--others", "--exclude-standard", "--"];
    untracked_args.extend(pathspecs);
    if let Ok(untracked) = std::process::Command::new("git")
        .args(&untracked_args)
        .output()
    {
        if untracked.status.success() {
            for path in String::from_utf8_lossy(&untracked.stdout).lines() {
                let path = path.trim();
                if !path.is_empty() && !review_path_excluded(path) && files.insert(path.to_string())
                {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        added += content.lines().count() as u64;
                    }
                }
            }
        }
    }

    let file_count = files.len();
    let line_count = added + deleted;
    println!("- 변경 범위: {file_count}개 파일, +{added}/-{deleted}줄");

    if file_count == 0 {
        println!("- 현재 HEAD 기준으로 리뷰할 변경 파일을 찾지 못했어요.");
        println!("- 다음: 특정 파일이나 브랜치를 알려주면 그 범위로 리뷰하면 돼요.");
    } else if file_count >= 100 || line_count >= 1000 {
        println!("- 변경량이 커요. 전체를 볼지 핵심 파일만 볼지 먼저 정하면 좋아요.");
    } else {
        println!(
            "- 다음: 이 범위의 실제 변경 파일을 열어 버그와 회귀 위험 중심으로 리뷰하면 돼요."
        );
    }

    Ok(0)
}

fn review_path_excluded(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized == "desktop-pure-routing-results.md"
        || (normalized.starts_with("desktop-") && normalized.ends_with("routing-results.md"))
        || normalized == "desktop-qa-results.md"
        || normalized.contains("/desktop-pure-routing-results.md")
        || normalized.contains("/desktop-") && normalized.ends_with("routing-results.md")
        || normalized.contains("/desktop-qa-results.md")
        || normalized.starts_with(".axhub-state/")
        || normalized.contains("/.axhub-state/")
        || normalized.starts_with(".omx/")
        || normalized.contains("/.omx/")
        || normalized.starts_with(".omc/")
        || normalized.contains("/.omc/")
        || normalized.starts_with(".shim/")
        || normalized.contains("/.shim/")
        || normalized.starts_with(".shim-local-market/")
        || normalized.contains("/.shim-local-market/")
        || normalized.starts_with(".claude/")
        || normalized.contains("/.claude/")
        || normalized.starts_with("node_modules/")
        || normalized.contains("/node_modules/")
        || normalized.contains("test-results")
        || normalized.ends_with(".log")
}

fn cmd_auth_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("auth-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("로그인 상태를 확인했어요.");

    if !output.cli_present {
        println!("- 로그인: CLI 설치 후 확인할 수 있어요.");
        println!("- 다시 로그인: 아직 판단하지 않을게요.");
        println!("- 다음: 설치 상태를 먼저 확인하면 돼요.");
        return Ok(0);
    }

    if output.cli_too_old || output.cli_too_new {
        println!("- 로그인: 현재 CLI 상태 때문에 안전하게 확인하지 못했어요.");
        println!("- 다시 로그인: 지금 바로 시작하지 않을게요.");
        println!("- 다음: 설치 상태를 먼저 확인하면 돼요.");
        return Ok(0);
    }

    if output.auth_ok {
        println!("- 로그인: 되어 있어요.");
        println!("- 다시 로그인: 지금은 필요 없어요.");
        println!("- 다음: 그대로 조회나 배포 작업을 진행해도 돼요.");
        return Ok(0);
    }

    match output.auth_error_code.as_deref() {
        Some("token_expired") => {
            println!("- 로그인: 만료됐어요.");
            println!("- 다시 로그인: 필요해요.");
            println!("- 다음: 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        }
        Some("not_logged_in") => {
            println!("- 로그인: 아직 되어 있지 않아요.");
            println!("- 로그인: 필요해요.");
            println!("- 다음: 계속하려면 \"로그인해줘\"라고 말해 주세요.");
        }
        Some("auth_unavailable") => {
            println!("- 로그인: 상태를 확인하지 못했어요.");
            println!("- 다시 로그인: 지금 바로 시작하지 않을게요.");
            println!("- 다음: 설치 상태를 먼저 확인하면 돼요.");
        }
        Some(_) | None => {
            println!("- 로그인: 확인이 필요해요.");
            println!("- 다시 로그인: 지금 바로 시작하지 않을게요.");
            println!("- 다음: 설치 상태를 먼저 확인하면 돼요.");
        }
    }

    Ok(0)
}

fn cmd_install_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("install-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("설치 상태를 확인했어요.");

    if output.cli_present {
        let cli_version = output.cli_version.as_deref().unwrap_or("버전 확인 필요");
        println!("- axhub CLI: 이미 설치되어 있어요. (v{cli_version})");
        if output.in_range {
            println!("- 호환성: 현재 플러그인과 함께 쓸 수 있어요.");
        } else if output.cli_too_old {
            println!("- 호환성: CLI가 오래되어 업데이트 확인이 필요해요.");
        } else if output.cli_too_new {
            println!("- 호환성: CLI가 플러그인 검증 범위보다 최신이에요.");
        } else {
            println!("- 호환성: 버전 범위를 다시 확인해야 해요.");
        }
        println!("- 설치 작업: 지금은 필요 없어요.");
        return Ok(0);
    }

    println!("- axhub CLI: 아직 설치되어 있지 않아요.");
    println!("- 설치 방식: 공식 설치 프로그램으로 설치할 수 있어요.");
    println!("- 안전 장치: 자동 설치는 명시적으로 승인받은 뒤에만 실행할게요.");
    println!("설치할까요? 진행 또는 취소라고 답해 주세요.");
    Ok(0)
}

fn cmd_update_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("update-summary", rest)?;
    let check = run_axhub(&["update", "check", "--json"]);

    println!("업데이트를 확인했어요.");

    if check.timed_out {
        println!("- 상태: 확인 시간이 초과됐어요.");
        println!("- 업데이트 적용: 시작하지 않았어요.");
        println!("- 다음: 잠시 뒤 다시 확인해 주세요.");
        return Ok(0);
    }

    if check.exit_code != 0 {
        println!("- 상태: 지금은 업데이트 정보를 확인하지 못했어요.");
        println!("- 업데이트 적용: 시작하지 않았어요.");
        println!("- 다음: 네트워크 상태를 확인한 뒤 다시 말해 주세요.");
        return Ok(0);
    }

    let parsed = serde_json::from_str::<Value>(&check.stdout).unwrap_or(Value::Null);
    if !parsed.is_object() {
        println!("- 상태: 업데이트 응답을 해석하지 못했어요.");
        println!("- 업데이트 적용: 시작하지 않았어요.");
        println!("- 다음: 잠시 뒤 다시 확인해 주세요.");
        return Ok(0);
    }

    let current = parsed
        .get("current")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("현재 버전 확인 필요");
    let latest = parsed
        .get("latest")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(current);
    let has_update = parsed
        .get("has_update")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if has_update {
        println!("- 현재 버전: {current}");
        println!("- 새 버전: {latest}");
        println!("- 업데이트: 받을 수 있어요.");
        println!("- 적용: 아직 시작하지 않았어요.");
        println!("적용하려면 \"업데이트 적용해줘\"라고 말해 주세요. 적용 전에는 먼저 미리보기와 승인을 받을게요.");
    } else {
        println!("- 현재 버전: {current}");
        println!("- 업데이트: 이미 최신이에요.");
        println!("- 적용: 지금은 필요 없어요.");
    }

    Ok(0)
}

fn cmd_doctor_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("doctor-summary", rest)?;
    let preflight = run_preflight();
    let output = &preflight.output;

    println!("설치 상태를 확인했어요.");

    if !output.cli_present {
        println!("- CLI: 아직 설치되어 있지 않아요.");
        println!("- 로그인: CLI 설치 후 확인할 수 있어요.");
        println!("- 다음: 설치하려면 \"axhub 설치해줘\"라고 말해 주세요.");
        return Ok(0);
    }

    let cli_version = output.cli_version.as_deref().unwrap_or("버전 확인 필요");
    if output.cli_too_old {
        println!("- CLI: v{cli_version}, 플러그인 기준보다 오래됐어요.");
        if !output.cli_on_path {
            println!("- PATH: 설치는 됐는데 PATH에는 아직 없어요.");
            println!("- 다음: PATH를 고치려면 \"PATH 고쳐줘\"라고 말해 주세요.");
        }
        println!("- 로그인: CLI 업데이트 뒤 다시 확인하는 편이 안전해요.");
        println!("- 다음: 업데이트하려면 \"axhub 업데이트 확인해줘\"라고 말해 주세요.");
        return Ok(0);
    }
    if output.cli_too_new {
        println!("- CLI: v{cli_version}, 현재 플러그인 검증 범위보다 최신이에요.");
        if !output.cli_on_path {
            println!("- PATH: 설치는 됐는데 PATH에는 아직 없어요.");
            println!("- 다음: PATH를 고치려면 \"PATH 고쳐줘\"라고 말해 주세요.");
        }
        println!("- 로그인: 플러그인 업데이트 뒤 다시 확인하는 편이 안전해요.");
        println!(
            "- 다음: 플러그인을 최신으로 보려면 \"axhub 플러그인 업데이트해줘\"라고 말해 주세요."
        );
        return Ok(0);
    }

    if output.in_range && !output.cli_on_path {
        println!("- CLI: v{cli_version}, 설치는 됐는데 PATH에는 아직 없어요.");
        println!("- 확인: 알려진 설치 경로에서 CLI를 찾았어요.");
        println!("- 다음: PATH를 고치려면 \"PATH 고쳐줘\"라고 말해 주세요.");
    } else if output.in_range {
        println!("- CLI: v{cli_version}, 플러그인과 호환돼요.");
    } else {
        println!("- CLI: v{cli_version}, 호환 범위를 다시 확인해야 해요.");
    }

    if output.auth_ok {
        println!("- 로그인: 되어 있어요. 지금은 다시 로그인할 필요 없어요.");
        if let Some(expires) = output.expires_human.as_deref() {
            println!("- 만료: {expires}");
        }
        if !output.scopes.is_empty() {
            println!("- 권한: {}", output.scopes.join(", "));
        }
        let profile = output.profile.as_deref().unwrap_or("default");
        println!("- 프로필: {profile}");
        println!("- 다음: 그대로 배포나 조회 작업을 진행해도 돼요.");
    } else {
        let reason = match output.auth_error_code.as_deref() {
            Some("token_expired") => "로그인이 만료됐어요.",
            Some("not_logged_in") => "아직 로그인되어 있지 않아요.",
            Some("auth_unavailable") => "로그인 상태를 확인하지 못했어요.",
            Some(_) => "로그인 확인이 필요해요.",
            None => "로그인 확인이 필요해요.",
        };
        println!("- 로그인: {reason}");
        println!("- 다음: 로그인하려면 \"로그인해줘\"라고 말해 주세요.");
    }
    Ok(0)
}

fn cmd_statusline_summary(rest: &[String]) -> anyhow::Result<i32> {
    let _user_utterance = parse_optional_user_utterance("statusline-summary", rest)?;

    let Some(_stub_path) = axhub_helpers::orphan_stub::install_and_verify() else {
        println!("상태바 설정을 확인했어요.");
        println!("- 상태바 연결 파일을 준비하지 못했어요.");
        println!("- Claude Code를 다시 연 뒤 다시 말해 주세요.");
        return Ok(0);
    };

    let outcome = match run_settings_merge(MergeOptions {
        silent: true,
        command_path_override: None,
        scope: Scope::User,
        dry_run: false,
    }) {
        Ok(outcome) => outcome,
        Err(_) => {
            println!("상태바 설정을 확인했어요.");
            println!("- 상태바 설정을 자동으로 바꾸지 못했어요.");
            println!("- Claude Code를 다시 연 뒤 다시 말해 주세요.");
            return Ok(0);
        }
    };

    match outcome {
        MergeOutcome::Created | MergeOutcome::Merged => {
            println!("상태바를 켰어요.");
            println!("- Claude Code를 다시 열면 axhub 상태가 보일 거예요.");
        }
        MergeOutcome::NoOp => {
            println!("상태바는 이미 켜져 있어요.");
            println!("- Claude Code를 다시 열면 axhub 상태가 보일 거예요.");
        }
        MergeOutcome::PreservedOther => {
            println!("상태바 설정을 확인했어요.");
            println!("- 이미 다른 상태바가 켜져 있어요.");
            println!("- axhub가 기존 상태바를 덮어쓰지 않았어요.");
            println!("- axhub 상태바로 바꾸고 싶으면 그렇게 말해 주세요.");
        }
        MergeOutcome::InvalidJson => {
            println!("상태바 설정을 확인했어요.");
            println!("- 설정 내용을 자동으로 읽지 못해 변경하지 않았어요.");
            println!("- 문법을 정리한 뒤 다시 말해 주세요.");
        }
        MergeOutcome::PartialSchema => {
            println!("상태바 설정을 확인했어요.");
            println!("- 기존 상태바 설정이 완성된 형태가 아니어서 변경하지 않았어요.");
            println!("- axhub 상태바로 정리하고 싶으면 그렇게 말해 주세요.");
        }
        MergeOutcome::PermissionDenied => {
            println!("상태바 설정을 확인했어요.");
            println!("- 상태바 설정을 쓸 권한이 없어 변경하지 못했어요.");
            println!("- 권한을 확인한 뒤 다시 말해 주세요.");
        }
    }
    Ok(0)
}

fn cmd_verify_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("verify-summary", rest)?;
    let resolve_args = vec![
        "--intent".to_string(),
        "verify".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone());

    println!("배포 검증을 완료했어요.");

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let list = run_list_deployments(ListDeploymentsArgs {
        app_id: app.clone(),
        limit: Some(1),
    });
    if list.exit_code != 0 {
        let message = list
            .error_message_kr
            .unwrap_or_else(|| "최근 배포를 확인하지 못했어요.".to_string());
        println!("⚠️ 확인이 더 필요해요");
        println!("- {message}");
        println!("- 로그인 상태나 앱 권한을 확인한 뒤 다시 검증해 주세요.");
        return Ok(0);
    }

    let Some(deploy) = list.deployments.first() else {
        println!("❌ 아직 라이브로 확인되지 않았어요");
        println!("- 앱 {app}에서 최근 배포를 찾지 못했어요.");
        println!("- 먼저 배포를 시작한 뒤 다시 확인하면 돼요.");
        return Ok(0);
    };

    let status = run_axhub(&["--json", "deploy", "status", &deploy.id, "--app", &app]);
    let status_json = serde_json::from_str::<Value>(&status.stdout).ok();
    let status_data = status_json
        .as_ref()
        .and_then(|v| v.get("data"))
        .or(status_json.as_ref());
    let status_text = status_data
        .and_then(|v| {
            v.get("state")
                .or_else(|| v.get("status"))
                .and_then(Value::as_str)
        })
        .unwrap_or(deploy.status.as_str());

    let logs = run_axhub(&[
        "--json", "deploy", "logs", &deploy.id, "--app", &app, "--limit", "50",
    ]);
    let log_lines = if logs.exit_code == 0 && !logs.timed_out {
        extract_log_lines(&logs.stdout, 50)
    } else {
        Vec::new()
    };
    let error_line = first_error_like_line(&log_lines);

    let liveish = verify_status_liveish(status_text);
    let failedish = verify_status_failedish(status_text);
    if liveish && error_line.is_none() {
        println!("✅ 라이브 확정");
        println!("- 앱 {app}는 최근 배포 기준으로 열리는 상태예요.");
        println!("- 최근 로그에서 눈에 띄는 ERROR/FATAL은 보이지 않았어요.");
        println!("- 더 자세히 보려면 \"방금 거 로그 보여줘\"라고 말하면 돼요.");
    } else if failedish {
        println!("❌ 아직 라이브로 확인되지 않았어요");
        println!("- 앱 {app}의 최근 배포 상태가 성공으로 보이지 않아요.");
        if let Some(error) = error_line {
            println!("- 첫 오류: {}", redact(&error));
        }
        println!("- 다음: \"왜 실패했어\"라고 물으면 원인을 이어서 추적할 수 있어요.");
    } else {
        println!("⚠️ 확인이 더 필요해요");
        println!("- 앱 {app}의 최근 배포를 확인했지만 바로 확정하기는 어려워요.");
        println!("- 상태 신호: {}", deploy_status_sentence(status_text));
        if logs.timed_out {
            println!("- 로그 확인이 5초 안에 끝나지 않았어요.");
        } else if logs.exit_code != 0 {
            println!("- 로그를 가져오지 못했어요.");
        } else if let Some(error) = error_line {
            println!("- 첫 오류: {}", redact(&error));
        }
        println!("- 잠시 뒤 \"다시 확인해줘\"라고 말하면 이어서 확인할 수 있어요.");
    }
    Ok(0)
}

fn cmd_trace_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("trace-summary", rest)?;
    let resolve_args = vec![
        "--intent".to_string(),
        "trace".to_string(),
        "--user-utterance".to_string(),
        user_utterance,
    ];
    let resolved = run_resolve(&resolve_args);
    let app = resolved
        .output
        .app_slug
        .clone()
        .or_else(|| resolved.output.app_id.clone());

    println!("배포 기록을 확인했어요.");

    let Some(app) = app else {
        println!("- 현재 폴더에서 연결된 앱을 찾지 못했어요.");
        println!("- 앱 이름이 들어간 폴더에서 다시 묻거나, 먼저 앱을 골라 주세요.");
        return Ok(0);
    };

    let list = run_list_deployments(ListDeploymentsArgs {
        app_id: app.clone(),
        limit: Some(5),
    });
    if list.exit_code != 0 {
        let message = list
            .error_message_kr
            .unwrap_or_else(|| "최근 배포 기록을 확인하지 못했어요.".to_string());
        println!("- {message}");
        println!("- 로그인 상태나 앱 권한을 확인한 뒤 다시 물어봐 주세요.");
        return Ok(0);
    }

    let Some(failed) = list
        .deployments
        .iter()
        .find(|deploy| verify_status_failedish(&deploy.status))
    else {
        println!("- 앱 {app}에서 최근 실패한 배포는 찾지 못했어요.");
        if let Some(latest) = list.deployments.first() {
            println!("- 최신 배포는 {}", deploy_status_sentence(&latest.status));
            if !latest.commit_sha.is_empty() {
                println!("- 최근 커밋: {}", short_commit(&latest.commit_sha));
            }
        } else {
            println!("- 아직 이 앱의 배포 이력이 없어요.");
        }
        println!("- 실패 화면이나 에러 문구가 따로 보이면 그 문장을 붙여서 다시 물어봐 주세요.");
        return Ok(0);
    };

    let status = run_axhub(&["--json", "deploy", "status", &failed.id, "--app", &app]);
    let status_json = serde_json::from_str::<Value>(&status.stdout).ok();
    let status_reason = status_json
        .as_ref()
        .and_then(trace_failure_reason_from_status);

    let logs = run_axhub(&[
        "--json", "deploy", "logs", &failed.id, "--app", &app, "--limit", "50",
    ]);
    let log_error = if logs.exit_code == 0 && !logs.timed_out {
        let lines = extract_log_lines(&logs.stdout, 50);
        first_error_like_line(&lines)
    } else {
        None
    };

    let probes = RealTraceProbes {
        app_ref: Some(app.clone()),
        warnings: std::cell::RefCell::new(Vec::new()),
    };
    let trace_report = axhub_helpers::trace_helper::trace(&failed.id, &probes).ok();
    let trace_reason = trace_report
        .as_ref()
        .and_then(|report| report.failure_reason.as_deref())
        .map(str::to_string);
    let trace_log_error = trace_report
        .as_ref()
        .and_then(|report| report.build_log_errors.first())
        .map(|line| redact(line));

    println!("- 앱 {app}의 최근 실패 배포를 확인했어요.");
    let reason = status_reason.or(trace_reason);
    if let Some(reason) = reason {
        println!("- 원인: {}", redact(&reason));
    } else if let Some(error) = log_error.or(trace_log_error) {
        println!("- 눈에 띄는 오류: {}", redact(&error));
    } else {
        println!("- 서버가 보관한 실패 메시지는 아직 확인되지 않았어요.");
        println!("- 최근 로그에서도 뚜렷한 ERROR/FATAL 신호를 찾지 못했어요.");
    }
    if logs.timed_out {
        println!("- 로그 확인이 오래 걸려 짧게 중단했어요.");
    }
    println!("- 다음: 원인을 고친 뒤 다시 배포하거나, 더 자세한 로그가 필요하면 \"로그 좀 보여줘\"라고 말해 주세요.");
    Ok(0)
}

fn trace_failure_reason_from_status(value: &Value) -> Option<String> {
    let data = value.get("data").unwrap_or(value);
    [
        "failure_reason",
        "failureReason",
        "error",
        "error_message",
        "message",
    ]
    .iter()
    .find_map(|key| {
        data.get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "null")
            .map(str::to_string)
    })
}

fn parse_optional_user_utterance(command: &str, rest: &[String]) -> anyhow::Result<String> {
    let mut user_utterance = String::new();
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--user-utterance" => {
                if index + 1 >= rest.len() {
                    return legacy_usage_error(command, "missing --user-utterance value")
                        .map(|_| String::new());
                }
                user_utterance = rest[index + 1].clone();
                index += 2;
            }
            "--json" => index += 1,
            _ => return legacy_usage_error(command, "unknown option").map(|_| String::new()),
        }
    }
    Ok(user_utterance)
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn response_items(value: &Value) -> Option<&Vec<Value>> {
    value
        .as_array()
        .or_else(|| value.get("items").and_then(Value::as_array))
        .or_else(|| value.get("resources").and_then(Value::as_array))
        .or_else(|| value.get("data").and_then(Value::as_array))
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("items"))
                .and_then(Value::as_array)
        })
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("resources"))
                .and_then(Value::as_array)
        })
}

fn resource_kind_label(kind: &str) -> &'static str {
    match kind.to_lowercase().as_str() {
        "table" | "postgresql table" | "mysql table" => "테이블",
        "view" | "postgresql view" | "mysql view" => "뷰",
        "namespace" | "folder" => "폴더",
        _ => "리소스",
    }
}

fn extract_log_lines(stdout: &str, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in stdout.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let rendered = serde_json::from_str::<Value>(line)
            .ok()
            .and_then(|value| log_message_from_value(&value))
            .unwrap_or_else(|| line.to_string());
        let rendered = rendered.trim();
        if !rendered.is_empty() {
            lines.push(rendered.to_string());
        }
    }
    let start = lines.len().saturating_sub(limit);
    lines[start..].to_vec()
}

fn log_message_from_value(value: &Value) -> Option<String> {
    value
        .get("message")
        .or_else(|| value.get("line"))
        .or_else(|| value.get("log"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("message").or_else(|| data.get("line")))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn first_error_like_line(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .find(|line| {
            let lower = line.to_lowercase();
            lower.contains("error")
                || lower.contains("fatal")
                || lower.contains("panic")
                || lower.contains("failed")
                || lower.contains("exception")
        })
        .map(|line| redact(line))
}

fn deploy_status_sentence(status: &str) -> &'static str {
    match status {
        "succeeded" => "성공했어요.",
        "failed" => "실패했어요.",
        "cancelled" => "취소됐어요.",
        "rolled_back" => "롤백됐어요.",
        "queued" | "pending" => "대기 중이에요.",
        "building" => "빌드 중이에요.",
        "deploying" | "running" | "in_progress" => "진행 중이에요.",
        _ => "확인됐어요.",
    }
}

fn verify_status_liveish(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "live" | "running" | "deployed" | "active" | "ok" | "succeeded" | "success"
    )
}

fn verify_status_failedish(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "failed" | "error" | "cancelled" | "canceled" | "stopped" | "rolled_back"
    )
}

fn compact_time(value: &str) -> String {
    value
        .strip_suffix('Z')
        .unwrap_or(value)
        .replace('T', " ")
        .split('.')
        .next()
        .unwrap_or(value)
        .to_string()
}

fn short_commit(value: &str) -> &str {
    value.get(..7).unwrap_or(value)
}

fn cmd_inspect_config_summary() -> anyhow::Result<i32> {
    let manifest = run_axhub(&["manifest", "validate", "--file", "axhub.yaml", "--json"]);
    let config = run_axhub(&["config", "explain", "--json"]);

    let manifest_json = serde_json::from_str::<Value>(&manifest.stdout).ok();
    let config_json = serde_json::from_str::<Value>(&config.stdout).ok();

    println!("매니페스트와 설정을 확인했어요.");

    if manifest.exit_code == 0 {
        let data = manifest_json.as_ref().and_then(|v| v.get("data"));
        let app_missing = data.and_then(|v| v.get("app")).is_none_or(Value::is_null);
        let ci_missing = data.and_then(|v| v.get("ci")).is_none_or(Value::is_null);
        let deploy = data.and_then(|v| v.get("deploy"));
        let port_missing = deploy
            .and_then(|v| v.get("port"))
            .is_none_or(Value::is_null);
        let commands_missing = deploy
            .and_then(|v| v.get("commands"))
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty);

        if app_missing || ci_missing || port_missing || commands_missing {
            println!("- 매니페스트 문법은 맞지만 실제 배포에 필요한 항목이 아직 비어 있어요.");
        } else {
            println!("- 매니페스트 문법과 주요 배포 설정이 괜찮아 보여요.");
        }
        if app_missing {
            println!("- 연결된 앱 정보가 아직 없어요.");
        }
        if port_missing || commands_missing {
            println!("- 배포 포트나 실행 명령이 비어 있어 컨테이너를 어떻게 띄울지 추가 설정이 필요해요.");
        }
    } else if manifest.timed_out {
        println!(
            "- 매니페스트 확인이 시간 안에 끝나지 않았어요. 잠시 뒤 다시 확인해보는 게 좋아요."
        );
    } else {
        println!(
            "- 매니페스트를 확인하지 못했어요. 현재 폴더에 axhub.yaml이 있는지 먼저 봐야 해요."
        );
    }

    if config.exit_code == 0 {
        let token_present = config_json
            .as_ref()
            .and_then(|v| v.pointer("/token/present"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if token_present {
            println!("- 로그인 정보는 확인됐어요.");
        } else {
            println!("- 로그인 정보가 없어 배포 전에 다시 로그인 확인이 필요해요.");
        }
    } else if config.timed_out {
        println!(
            "- 설정 확인이 시간 안에 끝나지 않았어요. 로그인 상태를 다시 확인하는 편이 안전해요."
        );
    } else {
        println!("- 설정 정보를 확인하지 못했어요. 로그인 상태를 먼저 확인하는 편이 안전해요.");
    }

    println!();
    println!("다음에는 앱의 포트와 실행 명령을 채우고, 로그인 상태를 다시 확인하면 돼요.");
    Ok(0)
}

fn cmd_config(rest: &[String]) -> anyhow::Result<i32> {
    let Some(action) = rest.first() else {
        eprintln!("axhub-helpers config: expected 'get' or 'set'");
        return Ok(64);
    };
    match action.as_str() {
        "get" => {
            let Some(key) = rest.get(1) else {
                eprintln!("axhub-helpers config get: expected <key>");
                return Ok(64);
            };
            let json = rest.iter().any(|a| a == "--json");
            let value = config_get(key);
            if json {
                println!("{}", render_get_json(key, value.as_deref()));
                Ok(0)
            } else {
                match value {
                    Some(v) => {
                        println!("{v}");
                        Ok(0)
                    }
                    None => Ok(1),
                }
            }
        }
        "set" => {
            let Some(key) = rest.get(1) else {
                eprintln!("axhub-helpers config set: expected <key> <value>");
                return Ok(64);
            };
            let Some(value) = rest.get(2) else {
                eprintln!("axhub-helpers config set: expected <value>");
                return Ok(64);
            };
            if let Err(err) = config_set(key, value) {
                eprintln!("axhub-helpers config set: {err}");
                // Unknown-key is a usage error (caller passed a bad CLI
                // argument); reserve exit 1 for IO/runtime failures.
                let exit_code = if err.to_string().contains("unknown config key") {
                    64
                } else {
                    1
                };
                return Ok(exit_code);
            }
            Ok(0)
        }
        other => {
            eprintln!("axhub-helpers config: unknown action \"{other}\"");
            Ok(64)
        }
    }
}

/// Real verify probes. Holds a memoized cache of the latest `deploy_id`
/// lookup outcome so that `axhub_status` and `axhub_logs_tail` do not
/// each independently spawn `axhub deploy list` (PR #149 / review #14 —
/// pre-fix verify spawned the same probe twice).
struct RealVerifyProbes {
    cached_lookup: std::cell::RefCell<Option<DeployIdLookup>>,
}

impl RealVerifyProbes {
    fn new() -> Self {
        Self {
            cached_lookup: std::cell::RefCell::new(None),
        }
    }

    /// Resolve the latest deploy_id lookup for `app_id`, memoizing the
    /// outcome. All three variants are cached — re-querying on every
    /// probe call is the bug review #14 surfaced. The richer
    /// `DeployIdLookup` (vs the prior `Option<String>`) lets review #8's
    /// fix propagate transport reasons up to the verify verdict.
    fn resolve_deploy_id(&self, axhub_bin: &str, app_id: &str) -> DeployIdLookup {
        self.resolve_deploy_id_with(app_id, |args| run_probe_with_timeout(axhub_bin, args))
    }

    /// Testable seam: same memoization behaviour as `resolve_deploy_id`
    /// but with an injected runner closure (so unit tests can count
    /// spawns / assert single-flight without spinning up a subprocess).
    fn resolve_deploy_id_with<F>(&self, app_id: &str, runner: F) -> DeployIdLookup
    where
        F: FnOnce(&[&str]) -> axhub_helpers::verify_helper::ProbeResult,
    {
        if let Some(cached) = self.cached_lookup.borrow().as_ref() {
            return cached.clone();
        }
        let resolved = latest_deploy_id_with_runner(app_id, runner);
        *self.cached_lookup.borrow_mut() = Some(resolved.clone());
        resolved
    }
}

fn run_probe_with_timeout(
    axhub_bin: &str,
    args: &[&str],
) -> axhub_helpers::verify_helper::ProbeResult {
    // PR #149 / architect note: collapse the three 5s timeout constants
    // (formerly AXHUB_PROBE_TIMEOUT here + AXHUB_TRACE_PROBE_TIMEOUT in
    // the trace path) onto the single public source in axhub_cli.
    let out = axhub_helpers::axhub_cli::run_axhub_with_timeout(
        axhub_bin,
        args,
        axhub_helpers::axhub_cli::DEFAULT_AXHUB_TIMEOUT,
    );
    axhub_helpers::verify_helper::ProbeResult {
        stdout: out.stdout,
        exit_code: out.exit_code,
        timed_out: out.timed_out,
    }
}

/// Outcome of looking up the latest `deploy_id` for an app. Distinguishes:
/// - `Found` — the canonical happy path
/// - `NoRecentDeploy` — list query succeeded but returned zero rows
/// - `TransportFailure` — list query itself failed (auth expired, CLI
///   missing, timeout, etc.). Carries the underlying exit code so the
///   verify path can surface the real cause instead of synthesising a
///   misleading "state = unknown" verdict (PR #149 / review #8).
#[derive(Debug, Clone)]
enum DeployIdLookup {
    Found(String),
    NoRecentDeploy,
    TransportFailure { reason: String },
}

/// Resolve the latest deploy_id for an app via the canonical CLI. Takes
/// a runner closure so unit tests can inject canned `ProbeResult`s +
/// count spawn invocations (PR #149 / review #14 — the memoization test
/// needs a non-process runner to assert "single spawn per cmd_verify").
/// Production callers (`RealVerifyProbes::resolve_deploy_id`) wire this
/// to `run_probe_with_timeout`.
fn latest_deploy_id_with_runner<F>(app_id: &str, runner: F) -> DeployIdLookup
where
    F: FnOnce(&[&str]) -> axhub_helpers::verify_helper::ProbeResult,
{
    let out = runner(&[
        "--json",
        "deploy",
        "list",
        "--app",
        app_id,
        "--page-size",
        "1",
    ]);
    if out.timed_out {
        return DeployIdLookup::TransportFailure {
            reason: "axhub deploy list timeout (5초)".to_string(),
        };
    }
    if out.exit_code != 0 {
        // Map known CLI exit codes to actionable reasons; otherwise echo the
        // raw exit code so verify_helper's verdict reasons aren't silently
        // collapsed to "state = unknown". These are the *spawned* `axhub deploy
        // list` exit codes — current CLI contract: 4=unauth, 5=not_found (not
        // 65/67, which are this helper's own output namespace). 127 is shell.
        let reason = match out.exit_code {
            4 => "axhub auth 만료 — axhub auth login 으로 재인증해주세요.".to_string(),
            5 => "axhub: 앱을 찾을 수 없어요 (resource not found).".to_string(),
            127 => "axhub CLI 를 찾을 수 없어요. 설치 도와줘라고 말해 주세요.".to_string(),
            code => format!("axhub deploy list exit code {code}"),
        };
        return DeployIdLookup::TransportFailure { reason };
    }
    let parsed = match serde_json::from_str::<serde_json::Value>(&out.stdout) {
        Ok(value) => value,
        Err(_) => {
            return DeployIdLookup::TransportFailure {
                reason: "axhub deploy list 응답 파싱 실패".to_string(),
            }
        }
    };
    let id = axhub_helpers::cli_envelope::rows(&parsed)
        .first()
        .and_then(|row| {
            axhub_helpers::cli_envelope::string_at_any(row, &["id", "deployment_id", "deploy_id"])
        });
    match id {
        Some(id) => DeployIdLookup::Found(id),
        None => DeployIdLookup::NoRecentDeploy,
    }
}

impl axhub_helpers::verify_helper::VerifyProbes for RealVerifyProbes {
    fn axhub_status(&self, app_id: &str) -> axhub_helpers::verify_helper::ProbeResult {
        let axhub_bin = std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string());
        match self.resolve_deploy_id(&axhub_bin, app_id) {
            DeployIdLookup::Found(deploy_id) => run_probe_with_timeout(
                &axhub_bin,
                &["--json", "deploy", "status", &deploy_id, "--app", app_id],
            ),
            DeployIdLookup::NoRecentDeploy => {
                axhub_helpers::verify_helper::ProbeResult::no_recent_deploy()
            }
            // Transport failure: synthesize a JSON body carrying the actual
            // reason via `transport_reason`. exit_code stays 0 so
            // verify_helper enters the JSON-parse branch (where it now
            // recognises `transport_reason`). `state="transport_error"`
            // ensures the verdict still degrades to NotLive — the
            // synthesised "fake-success" of pre-PR-#149 is gone.
            DeployIdLookup::TransportFailure { reason, .. } => {
                let body = serde_json::json!({
                    "state": "transport_error",
                    "last_deploy_id": null,
                    "transport_reason": reason,
                });
                axhub_helpers::verify_helper::ProbeResult {
                    stdout: body.to_string(),
                    exit_code: 0,
                    timed_out: false,
                }
            }
        }
    }

    fn axhub_logs_tail(
        &self,
        app_id: &str,
        lines: u32,
    ) -> axhub_helpers::verify_helper::ProbeResult {
        let axhub_bin = std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string());
        match self.resolve_deploy_id(&axhub_bin, app_id) {
            DeployIdLookup::Found(deploy_id) => {
                let limit = lines.to_string();
                run_probe_with_timeout(
                    &axhub_bin,
                    &[
                        "--json", "deploy", "logs", &deploy_id, "--app", app_id, "--limit", &limit,
                    ],
                )
            }
            DeployIdLookup::NoRecentDeploy | DeployIdLookup::TransportFailure { .. } => {
                // logs path has no JSON shape to carry richer reasons; the
                // status-side `transport_reason` already populates verdict
                // reasons. Return empty stdout + exit 0 so the logs branch
                // doesn't pile a second redundant "exit code" reason.
                axhub_helpers::verify_helper::ProbeResult {
                    stdout: String::new(),
                    exit_code: 0,
                    timed_out: false,
                }
            }
        }
    }
}

fn humanize_verify_korean(result: &axhub_helpers::verify_helper::VerifyResult) -> String {
    use axhub_helpers::verify_helper::Verdict;
    let mut lines: Vec<String> = Vec::new();
    let header = match result.verdict {
        Verdict::Live => "✅ 라이브 확정",
        Verdict::Suspect => "⚠️ 의심",
        Verdict::NotLive => "❌ 라이브 안 됨",
    };
    lines.push(header.to_string());
    if let Some(state) = &result.state {
        lines.push(format!("  - 상태: {state}"));
    }
    if let Some(id) = &result.last_deploy_id {
        lines.push(format!("  - 마지막 배포 ID: {id}"));
    }
    if let Some(age) = result.last_deploy_age_secs {
        lines.push(format!("  - 마지막 배포 경과: {age}초"));
    }
    if !result.errors.is_empty() {
        lines.push(format!("  - runtime 에러 {}건", result.errors.len()));
    }
    for reason in &result.reasons {
        lines.push(format!("  · {reason}"));
    }
    lines.push(match result.verdict {
        Verdict::Live => "  - 다음: \"방금 거 로그 보여줘\" / \"방금 거 상태\"".to_string(),
        Verdict::Suspect => {
            "  - 다음: \"방금 거 로그 보여줘\" / 1 분 뒤 \"다시 확인해줘\"".to_string()
        }
        Verdict::NotLive => "  - 다음: \"왜 실패했어\"".to_string(),
    });
    lines.join("\n")
}

pub(crate) fn cmd_verify(app_id: Option<String>, json_mode: bool) -> anyhow::Result<i32> {
    let Some(app_id) = app_id else {
        eprintln!("axhub-helpers verify: --app (alias: --app-id) <id> required");
        return Ok(64);
    };

    let probes = RealVerifyProbes::new();
    let result = axhub_helpers::verify_helper::run_verify(&app_id, &probes);

    if json_mode {
        out_json(serde_json::to_value(&result)?);
    } else {
        println!("{}", humanize_verify_korean(&result));
    }

    use axhub_helpers::verify_helper::Verdict;
    Ok(match result.verdict {
        Verdict::Live => 0,
        Verdict::Suspect => 0, // fail-soft: SKILL surfaces "의심" but doesn't error
        Verdict::NotLive => 64,
    })
}

/// Phase 25 PR 25.6 — `axhub-helpers doctor` health JSON.
/// Reports plugin version + helper version + deploy-events disk usage so
/// the `axhub:doctor` SKILL can decide whether to surface a size warning.
/// Cooldown is enforced via `doctor-cooldown.json` mtime so repeat doctor
/// runs within an hour stay quiet.
const DEPLOY_EVENTS_WARN_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;
const DOCTOR_COOLDOWN_SECS: u64 = 3600;

fn measure_deploy_events_size() -> (u64, u64) {
    let Some(dir) = axhub_helpers::runtime_paths::deploy_events_dir() else {
        return (0, 0);
    };
    if !dir.exists() {
        return (0, 0);
    }
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it,
        Err(_) => return (0, 0),
    };
    let mut size_bytes: u64 = 0;
    let mut count: u64 = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            size_bytes = size_bytes.saturating_add(meta.len());
            count += 1;
        }
    }
    (size_bytes, count)
}

fn cooldown_expired(now: std::time::SystemTime, last_warned_secs: u64) -> bool {
    let now_secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now_secs.saturating_sub(last_warned_secs) >= DOCTOR_COOLDOWN_SECS
}

fn read_cooldown_last_warned() -> Option<u64> {
    let path = axhub_helpers::runtime_paths::doctor_cooldown_path()?;
    let raw = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    v.get("deploy_events_size_warning")
        .and_then(|x| x.get("last_warned_secs"))
        .and_then(|x| x.as_u64())
}

fn write_cooldown_now() -> std::io::Result<()> {
    let Some(path) = axhub_helpers::runtime_paths::doctor_cooldown_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let payload = serde_json::json!({
        "deploy_events_size_warning": {
            "last_warned_secs": now_secs,
        }
    });
    std::fs::write(&path, serde_json::to_string(&payload)?)
}

pub(crate) fn cmd_doctor(json_mode: bool, no_cooldown: bool) -> anyhow::Result<i32> {
    let preflight = run_preflight();
    let output = &preflight.output;
    let (size_bytes, count) = measure_deploy_events_size();
    let last_warned = read_cooldown_last_warned();
    let cooldown_open = match last_warned {
        Some(t) if !no_cooldown => cooldown_expired(std::time::SystemTime::now(), t),
        _ => true,
    };
    let over_threshold = size_bytes > DEPLOY_EVENTS_WARN_THRESHOLD_BYTES;
    let should_warn = over_threshold && cooldown_open;

    if should_warn {
        let _ = write_cooldown_now();
    }

    let report = serde_json::json!({
        "axhub_helpers_version": env!("CARGO_PKG_VERSION"),
        "helper_version_expected": output.helper_version_expected.clone(),
        "helper_version_ok": output.helper_version_ok,
        "cli_present": output.cli_present,
        "cli_on_path": output.cli_on_path,
        "cli_state": output.cli_state.clone(),
        "cli_resolved_path": output.cli_resolved_path.clone(),
        "cli_version": output.cli_version.clone(),
        "preflight_exit_code": preflight.exit_code,
        "deploy_events_dir": axhub_helpers::runtime_paths::deploy_events_dir()
            .as_ref()
            .map(|p| p.display().to_string()),
        "deploy_events_size_bytes": size_bytes,
        "deploy_events_count": count,
        "deploy_events_threshold_bytes": DEPLOY_EVENTS_WARN_THRESHOLD_BYTES,
        "over_threshold": over_threshold,
        "should_warn": should_warn,
        "last_warned_secs": last_warned,
    });

    if json_mode {
        out_json(report);
    } else {
        println!("axhub-helpers v{}", env!("CARGO_PKG_VERSION"));
        if output.helper_version_ok {
            println!(
                "helper version: OK (expected {})",
                output
                    .helper_version_expected
                    .as_deref()
                    .unwrap_or(env!("CARGO_PKG_VERSION"))
            );
        } else {
            println!(
                "helper version: mismatch (expected {}, actual {})",
                output
                    .helper_version_expected
                    .as_deref()
                    .unwrap_or("unknown"),
                env!("CARGO_PKG_VERSION")
            );
        }
        if output.cli_present {
            let cli_version = output.cli_version.as_deref().unwrap_or("unknown");
            if output.cli_on_path {
                println!("axhub CLI: v{cli_version} on PATH");
            } else {
                println!("axhub CLI: v{cli_version} installed, but not on PATH");
                println!("next: PATH 고쳐줘");
            }
        } else {
            println!("axhub CLI: missing");
        }
        println!("deploy-events: {count} files, {size_bytes} bytes");
        if over_threshold {
            if should_warn {
                println!(
                    "⚠️ deploy-events 디렉토리가 {} MB 를 넘었어요. cleanup 필요. (cooldown 1 시간 활성)",
                    DEPLOY_EVENTS_WARN_THRESHOLD_BYTES / (1024 * 1024)
                );
            } else {
                println!(
                    "(deploy-events {} MB 초과 하지만 cooldown 활성 — 다음 알림은 1 시간 후)",
                    DEPLOY_EVENTS_WARN_THRESHOLD_BYTES / (1024 * 1024)
                );
            }
        }
    }
    Ok(0)
}

pub(crate) fn cmd_repair_path(json_mode: bool, dir: Option<String>) -> anyhow::Result<i32> {
    let report = axhub_helpers::repair_path::repair_path(dir.map(PathBuf::from));
    if json_mode {
        out_json(serde_json::to_value(&report)?);
    } else if report.disabled {
        println!("PATH 자동 수리는 꺼져 있어요.");
        println!("다시 켜려면 AXHUB_DISABLE_PATH_REPAIR 값을 비워 주세요.");
    } else if report.already_present {
        println!("PATH는 이미 axhub 설치 경로를 포함해요.");
    } else if report.repaired {
        println!("PATH 수리를 적용했어요.");
        if let Some(rc) = report.shell_rc.as_ref() {
            println!("- 설정 파일: {}", rc.display());
        }
        println!("- 다음: 새 터미널을 열거나 shell 설정을 다시 불러와 주세요.");
    } else {
        println!("PATH 수리를 완료하지 못했어요.");
        println!("- 이유: {}", report.message);
        if let Some(error) = report.error.as_deref() {
            println!("- 상세: {error}");
        }
    }
    Ok(0)
}

struct RealTraceProbes {
    app_ref: Option<String>,
    warnings: std::cell::RefCell<Vec<String>>,
}

fn axhub_stdout_with_timeout(axhub_bin: &str, args: &[&str]) -> Result<String, &'static str> {
    // Single source of truth for the 5s helper-probe budget — see
    // run_probe_with_timeout for the rationale.
    let out = axhub_helpers::axhub_cli::run_axhub_with_timeout(
        axhub_bin,
        args,
        axhub_helpers::axhub_cli::DEFAULT_AXHUB_TIMEOUT,
    );
    if out.timed_out {
        return Err("timeout");
    }
    if out.exit_code == 127 && out.stdout.is_empty() {
        return Err("spawn");
    }
    Ok(out.stdout)
}

impl axhub_helpers::trace_helper::TraceProbes for RealTraceProbes {
    fn axhub_build_log(&self, _deploy_id: &str, tail: u32) -> String {
        let Some(app_ref) = self.app_ref.as_deref() else {
            self.warnings.borrow_mut().push(
                "runtime_log_probe_skipped: --app required for current deploy logs".to_string(),
            );
            return String::new();
        };
        let axhub_bin = std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string());
        let tail = tail.to_string();
        // R3γ: 현행 `axhub deploy logs` 는 app-level 런타임 로그 NDJSON 을 반환해요
        // (build-log 엔드포인트 부재 — F3). `--source`/deploy-id 는 CLI 가 무시하므로
        // 보내지 않고, NDJSON 각 라인의 `message` 만 unwrap 해서 plain 텍스트로 넘겨요.
        let stdout = match axhub_stdout_with_timeout(
            &axhub_bin,
            &[
                "--json", "deploy", "logs", "--app", app_ref, "--limit", &tail,
            ],
        ) {
            Ok(stdout) => stdout,
            Err("timeout") => {
                self.warnings
                    .borrow_mut()
                    .push("runtime_log_probe_timeout: axhub deploy logs exceeded 5s".to_string());
                return String::new();
            }
            Err(_) => {
                self.warnings
                    .borrow_mut()
                    .push("runtime_log_probe_failed: axhub CLI unavailable".to_string());
                return String::new();
            }
        };

        let mut messages: Vec<String> = Vec::new();
        let mut parse_failed = false;
        let mut message_field_missing = false;
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                // valid JSON 인데 string `message` 가 없으면 (스키마 drift) silent
                // drop 하지 않고 별도 신호를 남겨요 (CR #7).
                Ok(value) => match value.get("message").and_then(|m| m.as_str()) {
                    // 한 message 안의 embedded newline 은 공백으로 펴서 "1 message =
                    // 1 라인" 을 유지 — multi-line stack-trace 가 display 5-라인 budget
                    // 을 통째로 먹지 않게 해요 (CR P3).
                    Some(msg) => messages.push(msg.replace('\n', " ")),
                    None => message_field_missing = true,
                },
                Err(_) => parse_failed = true,
            }
        }
        if messages.is_empty() {
            // 하나도 못 뽑았으면 원인별로 **단일** warning 만 — parse_warning +
            // unavailable 을 동시에 내보내 모순 신호를 주지 않게 해요 (CR P3).
            let warn = if parse_failed {
                "runtime_log_unparseable: 로그가 NDJSON 형식이 아니에요 (빌드 단계 실패 가능)"
            } else if message_field_missing {
                "runtime_log_schema_mismatch: 로그에 message 필드가 없어요 (CLI 스키마 변경 가능)"
            } else {
                "runtime_log_unavailable: 런타임 로그가 비어 있어요 (빌드 단계 실패 가능)"
            };
            self.warnings.borrow_mut().push(warn.to_string());
            return String::new();
        }
        // 일부 message 는 뽑혔지만 noise 가 섞인 경우의 부분 경고.
        if parse_failed {
            self.warnings
                .borrow_mut()
                .push("runtime_log_parse_warning: 일부 로그 라인 NDJSON 파싱 실패".to_string());
        }
        if message_field_missing {
            self.warnings
                .borrow_mut()
                .push("runtime_log_schema_warning: 일부 라인에 message 필드 없음".to_string());
        }
        messages.join("\n")
    }

    fn trace_warnings(&self) -> Vec<String> {
        self.warnings.borrow().clone()
    }

    fn recent_routing_context(&self) -> Option<axhub_helpers::trace_helper::RoutingContext> {
        use axhub_helpers::audit;
        let records = audit::read_since(chrono::Duration::seconds(3600)).ok()?;
        let last = records.last()?;
        Some(axhub_helpers::trace_helper::RoutingContext {
            last_routing_audit_ts: last.ts.clone(),
            last_prompt_hash_prefix: last
                .prompt_hash
                .strip_prefix("sha256:")
                .unwrap_or(&last.prompt_hash)
                .chars()
                .take(12)
                .collect(),
            is_axhub_related_recent: last.is_axhub_related,
        })
    }
}

fn humanize_trace_korean(report: &axhub_helpers::trace_helper::TraceReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("📍 deploy_id: {}", report.deploy_id));
    lines.push(format!("  - 마지막 phase: {}", report.last_phase));
    if let Some(reason) = &report.failure_reason {
        lines.push(format!("  - 실패 사유: {reason}"));
    }
    if !report.phase_durations.is_empty() {
        lines.push("  - phase 별 소요:".to_string());
        for phase in &report.phase_durations {
            let dur = phase
                .duration_ms
                .map(|ms| format!("{ms}ms"))
                .unwrap_or_else(|| "?".to_string());
            lines.push(format!(
                "    · step {} {} → {}",
                phase.step, phase.phase, dur
            ));
        }
    }
    if !report.build_log_errors.is_empty() {
        lines.push(format!(
            "  - build_log 마지막 {} 라인:",
            report.build_log_errors.len()
        ));
        for err in &report.build_log_errors {
            lines.push(format!("    > {err}"));
        }
    }
    if !report.matched_patterns.is_empty() {
        lines.push(format!(
            "  - 매칭 패턴: {}",
            report.matched_patterns.join(", ")
        ));
        lines.push(
            "  - 다음: skills/trace/references/error-patterns.md 의 매칭 entry 참고".to_string(),
        );
    } else if !report.build_log_errors.is_empty() {
        lines.push("  - 자동 매칭 실패. 위 raw 에러 라인 직접 검색해주세요.".to_string());
    }
    if !report.warnings.is_empty() {
        lines.push("  - ⚠️ evidence 경고:".to_string());
        for warn in &report.warnings {
            lines.push(format!("    · {warn}"));
        }
    }
    if let Some(rc) = &report.routing_context {
        lines.push(format!(
            "  - 최근 routing audit: {} (axhub_related={})",
            rc.last_routing_audit_ts, rc.is_axhub_related_recent
        ));
    }
    lines.join("\n")
}

pub(crate) fn cmd_trace(
    deploy_id: Option<String>,
    app_ref: Option<String>,
    json_mode: bool,
) -> anyhow::Result<i32> {
    let Some(deploy_id) = deploy_id else {
        eprintln!("axhub-helpers trace: --deploy-id <id> required");
        return Ok(64);
    };

    let probes = RealTraceProbes {
        app_ref,
        warnings: std::cell::RefCell::new(Vec::new()),
    };
    let report = axhub_helpers::trace_helper::trace(&deploy_id, &probes)?;

    if json_mode {
        out_json(serde_json::to_value(&report)?);
    } else {
        println!("{}", humanize_trace_korean(&report));
    }
    Ok(0)
}

/// Background auth refresh runs from `hooks/session-start.sh` as `nohup …
/// &`. The default 5s probe timeout is too aggressive for the OAuth round
/// trip, so use a dedicated longer bound. Cap is still tight enough that
/// an unreachable refresh server can never accumulate orphan children.
const AUTH_REFRESH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

fn cmd_auth_refresh_bg() -> anyhow::Result<i32> {
    if std::env::var("AXHUB_AUTH_BG_REFRESH").as_deref() == Ok("0") {
        return Ok(0);
    }

    // Single-flight gate: concurrent SessionStart hooks must not pile up
    // parallel `axhub auth refresh` invocations that would race the token
    // store. fslock::try_lock returns false when the lock is held — in
    // that case we exit 0 silently (fail-open hook contract) so the other
    // refresh runs uncontested. PR #149 / review #7.
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let lock_dir = PathBuf::from(&home).join(".config/axhub-plugin");
    let _ = fs::create_dir_all(&lock_dir);
    let lock_path = lock_dir.join("auth-refresh.lock");
    let mut lock = match fslock::LockFile::open(&lock_path) {
        Ok(lock) => lock,
        // Lock-file open failure is itself not a hook regression: skip
        // refresh, exit 0. Don't write a sentinel — preserve whatever
        // the previous successful refresh recorded.
        Err(_) => return Ok(0),
    };
    match lock.try_lock_with_pid() {
        Ok(true) => {}
        // Held by another invocation — peer is already refreshing.
        Ok(false) => return Ok(0),
        Err(_) => return Ok(0),
    }

    let axhub_bin = axhub_helpers::axhub_cli::axhub_bin_from_env();

    // Probe whether the binary is invokable. A 5s bound matches the default
    // helper probe budget — a slow `axhub --version` is itself a signal that
    // something is wrong, no point waiting longer.
    let probe = axhub_helpers::axhub_cli::run_axhub_with_timeout(
        &axhub_bin,
        &["--version"],
        axhub_helpers::axhub_cli::DEFAULT_AXHUB_TIMEOUT,
    );
    if probe.exit_code == 127 {
        // axhub CLI missing — write a fail sentinel and exit cleanly so the
        // hook never blocks session-start on a stale install.
        let _ = write_refresh_sentinel(false, "axhub_cli_missing");
        return Ok(0);
    }
    if probe.timed_out {
        let _ = write_refresh_sentinel(false, "probe_timeout");
        return Ok(0);
    }

    let result = axhub_helpers::axhub_cli::run_axhub_with_timeout(
        &axhub_bin,
        &["--json", "auth", "refresh", "--no-browser"],
        AUTH_REFRESH_TIMEOUT,
    );
    let status_label = if result.timed_out {
        "refresh_timeout"
    } else if result.exit_code == 0 {
        "ok"
    } else {
        "fail"
    };
    let success = result.exit_code == 0 && !result.timed_out;
    let _ = write_refresh_sentinel(success, status_label);
    // fslock LockFile drops here, releasing the lock on scope exit.
    Ok(if success { 0 } else { 1 })
}

fn write_refresh_sentinel(success: bool, status: &str) -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join(".config/axhub-plugin");
    fs::create_dir_all(&dir)?;
    let path = dir.join("auth-refresh-status.json");
    let body = format!(
        "{{\"success\":{success},\"status\":\"{status}\",\"ts\":\"{}\"}}\n",
        Utc::now().to_rfc3339()
    );
    fs::write(&path, body)?;
    Ok(())
}

fn deploy_prep_output_with_quality(
    result: &axhub_helpers::deploy_prep::DeployPrepResult,
    quality: &QualityCheckResult,
    exit_code: i32,
) -> anyhow::Result<Value> {
    let mut value = serde_json::to_value(result)?;
    if let Value::Object(ref mut obj) = value {
        obj.insert("quality_gate".to_string(), serde_json::to_value(quality)?);
        obj.insert("exit_code".to_string(), json!(exit_code));
    }
    Ok(value)
}

fn cmd_deploy_prep(rest: &[String]) -> anyhow::Result<i32> {
    if let Err(message) = validate_deploy_prep_args(rest) {
        return legacy_usage_error("deploy-prep", message);
    }
    if std::env::var("AXHUB_DEPLOY_PREP").as_deref() == Ok("0") {
        // Backwards-compat fallback signal: SKILL detects exit 0 + no JSON
        // payload and routes to the legacy 3x resolve / 2x preflight cascade.
        return Ok(0);
    }
    let result = run_deploy_prep(rest);
    let quality = validate_deploy_prep_quality(&result);
    let exit_code = if quality.passed { result.exit_code } else { 64 };
    if !quality.passed {
        eprintln!(
            "quality gate failed (non-interactive): {:?}",
            quality.violations
        );
        eprintln!("axhub-error-sub-key: {}", QualityCheckResult::SUB_KEY);
    }
    println!(
        "{}",
        serde_json::to_string(&deploy_prep_output_with_quality(
            &result, &quality, exit_code
        )?)?
    );
    Ok(exit_code)
}

fn cmd_deploy_preview_summary(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("deploy-preview-summary", rest)?;
    if local_deploy_manifest_missing()? {
        print_missing_manifest_choices();
        return Ok(0);
    }
    let mut prep_args = vec!["--intent".to_string(), "deploy".to_string()];
    if !user_utterance.trim().is_empty() {
        prep_args.push("--user-utterance".to_string());
        prep_args.push(user_utterance);
    }
    let result = run_deploy_prep(&prep_args);
    let quality = validate_deploy_prep_quality(&result);

    if !result.preflight.auth_ok {
        println!("axhub 로그인이 필요해요.");
        println!("- 지금은 배포 준비를 끝낼 수 없어요.");
        println!("- 로그인부터 다시 확인할까요?");
        return Ok(65);
    }
    if result.preflight.cli_too_old {
        let version = result.preflight.cli_version.as_deref().unwrap_or("unknown");
        println!("axhub CLI 버전이 낮아서 배포 전에 업데이트가 필요해요.");
        println!("- 현재 버전: {version}");
        println!("- 먼저 업데이트 확인을 진행할까요?");
        return Ok(64);
    }
    if !quality.passed {
        println!("배포 전에 품질 확인에서 막힌 항목이 있어요.");
        for violation in quality.violations.iter().take(4) {
            println!("- {}", quality_violation_label(violation));
        }
        println!("- 이 상태에서는 바로 배포하지 않는 게 안전해요.");
        return Ok(64);
    }
    if let Some(in_flight) = result.in_flight_deploy.as_ref() {
        println!("이미 진행 중인 배포가 있어요.");
        if !in_flight.commit_sha.is_empty() {
            println!("- 커밋: {}", short_commit(&in_flight.commit_sha));
        }
        println!("- 이 배포를 계속 볼지, 새 배포를 시작할지 확인이 필요해요.");
        return Ok(0);
    }

    let app = result
        .resolve
        .app_slug
        .as_deref()
        .or(result.resolve.candidate_slug.as_deref())
        .unwrap_or("확인 필요");
    let branch = result.resolve.branch.as_deref().unwrap_or("확인 필요");
    let commit = result.resolve.commit_sha.as_deref().unwrap_or("확인 필요");
    let message = result
        .resolve
        .commit_message
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("커밋 메시지 없음");
    let eta = deploy_eta_human(result.resolve.eta_sec);
    let env = result
        .resolve
        .profile
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("production");

    if result.resolve.app_id.is_none() {
        println!("처음 배포라 앱 등록 준비가 먼저 필요해요.");
        println!("- 앱 후보: {app}");
        println!("- 브랜치: {branch}");
        println!("- 커밋: {} — \"{}\"", short_commit(commit), message);
        println!("- 계속 진행하기 전에 앱 등록 미리보기를 확인할게요.");
        return Ok(0);
    }

    println!("다음을 실행할게요:");
    println!("- 앱: {app}");
    println!("- 환경: {env}");
    println!("- 브랜치: {branch}");
    println!("- 커밋: {} — \"{}\"", short_commit(commit), message);
    println!("- 예상: {eta}");
    println!();
    println!("진행할까요?");
    Ok(0)
}

fn local_deploy_manifest_missing() -> anyhow::Result<bool> {
    let cwd = std::env::current_dir()?;
    if cwd.join("axhub.yaml").is_file() || cwd.join("apphub.yaml").is_file() {
        return Ok(false);
    }
    Ok(true)
}

fn print_missing_manifest_choices() {
    println!("axhub 매니페스트(axhub.yaml)가 없어요.");
    println!("- React/Vite로 초기화");
    println!("- 다른 템플릿 선택");
    println!("- 취소");
    println!("원격 앱 등록이나 배포는 아직 시작하지 않았어요.");
}

fn cmd_deploy_approved_run(rest: &[String]) -> anyhow::Result<i32> {
    let user_utterance = parse_optional_user_utterance("deploy-approved-run", rest)?;
    if local_deploy_manifest_missing()? {
        println!("axhub 매니페스트(axhub.yaml)가 없어서 배포를 시작하지 않았어요.");
        println!("- React/Vite로 초기화");
        println!("- 다른 템플릿 선택");
        println!("- 취소");
        println!("원격 앱 등록이나 배포는 아직 시작하지 않았어요.");
        return Ok(0);
    }
    let mut prep_args = vec!["--intent".to_string(), "deploy".to_string()];
    if !user_utterance.trim().is_empty() {
        prep_args.push("--user-utterance".to_string());
        prep_args.push(user_utterance);
    }
    let result = run_deploy_prep(&prep_args);
    let quality = validate_deploy_prep_quality(&result);

    if !result.preflight.auth_ok {
        println!("axhub 로그인이 필요해요.");
        println!("- 지금은 배포를 시작하지 않았어요.");
        println!("- 다시 로그인한 뒤 배포를 이어가면 돼요.");
        return Ok(0);
    }
    if result.preflight.cli_too_old {
        let version = result.preflight.cli_version.as_deref().unwrap_or("unknown");
        println!("axhub CLI 버전이 낮아서 배포를 시작하지 않았어요.");
        println!("- 현재 버전: {version}");
        println!("- 업데이트 확인을 먼저 진행해 주세요.");
        return Ok(0);
    }
    if !quality.passed {
        println!("배포 전에 품질 확인에서 막힌 항목이 있어요.");
        for violation in quality.violations.iter().take(4) {
            println!("- {}", quality_violation_label(violation));
        }
        println!("- 이 상태에서는 바로 배포하지 않는 게 안전해요.");
        return Ok(0);
    }

    let app_label = result
        .resolve
        .app_slug
        .as_deref()
        .or(result.resolve.candidate_slug.as_deref())
        .unwrap_or("확인 필요");
    let Some(app_id) = result.resolve.app_id.as_deref() else {
        println!("처음 배포라 앱 등록 준비가 먼저 필요해요.");
        println!("- 앱 후보: {app_label}");
        println!("- 앱을 만든 뒤 다시 배포를 이어갈게요.");
        return Ok(0);
    };
    let Some(commit) = result.resolve.commit_sha.as_deref() else {
        println!("배포할 git 커밋을 확인하지 못했어요.");
        println!("- 변경사항을 저장한 뒤 다시 배포해 주세요.");
        return Ok(0);
    };

    if let Some(in_flight) = result.in_flight_deploy.as_ref() {
        println!("이미 진행 중인 배포가 있어요. 그 배포를 계속 확인할게요.");
        return watch_deploy_until_terminal(app_id, app_label, &in_flight.id, Some(commit));
    }

    let mut create_args = vec![
        "deploy".to_string(),
        "create".to_string(),
        "--app".to_string(),
        app_id.to_string(),
        "--commit".to_string(),
        commit.to_string(),
        "--execute".to_string(),
        "--json".to_string(),
    ];
    if let Some(profile) = result
        .resolve
        .profile
        .as_deref()
        .filter(|value| !value.trim().is_empty() && *value != "default")
    {
        create_args.push("--profile".to_string());
        create_args.push(profile.to_string());
    }

    println!("배포를 시작했어요. 완료될 때까지 확인할게요.");
    let create = run_axhub_long(&create_args, std::time::Duration::from_secs(180));
    if create.timed_out {
        println!("- 배포 시작 요청이 오래 걸리고 있어요.");
        println!("- 잠시 뒤 배포 상태를 다시 확인해 주세요.");
        return Ok(0);
    }
    if create.exit_code == 64 && create.stderr.contains("deployment_in_progress") {
        let refresh = run_deploy_prep(&[
            "--intent".to_string(),
            "deploy".to_string(),
            "--refresh-in-flight".to_string(),
        ]);
        if let Some(in_flight) = refresh.in_flight_deploy.as_ref() {
            println!("- 이미 진행 중인 배포를 찾았어요. 그 배포를 확인할게요.");
            return watch_deploy_until_terminal(app_id, app_label, &in_flight.id, Some(commit));
        }
    }
    if create.exit_code != 0 {
        println!("배포 시작에 실패했어요.");
        if let Some(reason) = concise_cli_failure_reason(&create.stderr, &create.stdout) {
            println!("- 이유: {reason}");
        } else {
            println!("- 로그인, 앱 설정, 진행 중인 배포 상태를 확인해 주세요.");
        }
        return Ok(0);
    }

    let deploy_id = deploy_id_from_create_stdout(&create.stdout);
    let Some(deploy_id) = deploy_id else {
        println!("배포 시작 요청은 보냈지만 결과 확인이 완전하지 않아요.");
        if let Some(reason) = concise_cli_failure_reason(&create.stderr, &create.stdout) {
            println!("- 이유: {reason}");
        } else {
            println!("- 잠시 뒤 배포 상태를 다시 확인해 주세요.");
        }
        return Ok(0);
    };
    watch_deploy_until_terminal(app_id, app_label, &deploy_id, Some(commit))
}

fn run_axhub_long(
    args: &[String],
    timeout: std::time::Duration,
) -> axhub_helpers::axhub_cli::CliOutput {
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let axhub_bin = axhub_helpers::axhub_cli::axhub_bin_from_env();
    axhub_helpers::axhub_cli::run_axhub_with_timeout(&axhub_bin, &refs, timeout)
}

fn watch_deploy_until_terminal(
    app_id: &str,
    app_label: &str,
    deploy_id: &str,
    commit: Option<&str>,
) -> anyhow::Result<i32> {
    let args = vec![
        "deploy".to_string(),
        "status".to_string(),
        deploy_id.to_string(),
        "--app".to_string(),
        app_id.to_string(),
        "--watch".to_string(),
        "--watch-timeout".to_string(),
        "9m".to_string(),
        "--json".to_string(),
    ];
    let status = run_axhub_long(&args, std::time::Duration::from_secs(570));
    if status.timed_out {
        println!("- 배포가 아직 진행 중이에요.");
        println!("- 잠시 뒤 상태를 다시 확인해 주세요.");
        return Ok(0);
    }
    if status.exit_code != 0 {
        println!("- 배포 상태 확인이 중간에 끊겼어요.");
        if let Some(reason) = concise_cli_failure_reason(&status.stderr, &status.stdout) {
            println!("- 이유: {reason}");
        }
        return Ok(0);
    }

    let status_values = parse_json_values(&status.stdout);
    let state = status_values
        .iter()
        .rev()
        .find_map(deploy_state_from_value)
        .unwrap_or_else(|| "unknown".to_string());
    let url = status_values.iter().rev().find_map(|value| {
        deploy_data_candidates(value).find_map(|candidate| {
            axhub_helpers::cli_envelope::string_at_any(
                candidate,
                &["url", "app_url", "public_url", "deployment_url"],
            )
        })
    });
    let failure_reason = status_values
        .iter()
        .rev()
        .find_map(deploy_failure_reason_from_value)
        .or_else(|| concise_cli_failure_reason(&status.stderr, &status.stdout));

    if matches!(state.as_str(), "succeeded" | "success" | "ready" | "live") {
        println!("- 배포가 완료됐어요.");
        println!("- 앱: {app_label}");
        if let Some(commit) = commit {
            println!("- 커밋: {}", short_commit(commit));
        }
        if let Some(url) = url {
            println!("- URL: {url}");
        }
        return Ok(0);
    }

    println!("- 배포가 끝났지만 성공 상태는 아니에요.");
    println!("- 상태: {}", deploy_status_sentence(&state));
    if let Some(reason) = failure_reason.filter(|value| !value.trim().is_empty()) {
        println!("- 이유: {reason}");
    }
    Ok(0)
}

fn parse_json_values(raw: &str) -> Vec<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut values = Vec::new();
    let stream = serde_json::Deserializer::from_str(trimmed).into_iter::<Value>();
    for value in stream {
        match value {
            Ok(value) => values.push(value),
            Err(_) => {
                values.clear();
                break;
            }
        }
    }
    if !values.is_empty() {
        return values;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return vec![value];
    }
    trimmed
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn deploy_id_from_create_stdout(stdout: &str) -> Option<String> {
    parse_json_values(stdout)
        .iter()
        .rev()
        .find_map(deploy_id_from_value)
}

fn deploy_id_from_value(value: &Value) -> Option<String> {
    deploy_data_candidates(value)
        .find_map(|candidate| {
            axhub_helpers::cli_envelope::string_at_any(
                candidate,
                &["id", "deployment_id", "deploy_id"],
            )
        })
        .or_else(|| {
            axhub_helpers::cli_envelope::string_at_any(value, &["deployment_id", "deploy_id"])
        })
}

fn deploy_data_candidates(value: &Value) -> impl Iterator<Item = &Value> {
    let data = axhub_helpers::cli_envelope::unwrap_data(value);
    [
        Some(data),
        data.get("deployment"),
        data.get("deploy"),
        value.get("deployment"),
        value.get("deploy"),
    ]
    .into_iter()
    .flatten()
}

fn deploy_state_from_value(value: &Value) -> Option<String> {
    deploy_data_candidates(value).find_map(|candidate| {
        axhub_helpers::cli_envelope::string_at_any(
            candidate,
            &["status", "state", "deployment_status"],
        )
    })
}

fn deploy_failure_reason_from_value(value: &Value) -> Option<String> {
    deploy_data_candidates(value)
        .filter_map(|candidate| {
            candidate
                .get("failure_reason")
                .or_else(|| candidate.get("error"))
                .or_else(|| candidate.get("message"))
        })
        .chain(
            [
                value.get("failure_reason"),
                value.get("error"),
                value.get("message"),
            ]
            .into_iter()
            .flatten(),
        )
        .find_map(human_deploy_failure_reason)
}

fn human_deploy_failure_reason(value: &Value) -> Option<String> {
    let raw = match value {
        Value::String(value) => value.clone(),
        Value::Object(_) => {
            let code = axhub_helpers::cli_envelope::string_at_any(value, &["code", "subcode"]);
            let message = axhub_helpers::cli_envelope::string_at_any(value, &["message", "hint"]);
            [code, message]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
        }
        _ => return None,
    };
    human_deploy_failure_text(&raw)
}

fn concise_cli_failure_reason(stderr: &str, stdout: &str) -> Option<String> {
    let combined = [stderr, stdout].join("\n");
    if let Some(reason) = human_deploy_failure_text(&combined) {
        return Some(reason);
    }
    let lower = combined.to_lowercase();
    if lower.contains("unauth") || lower.contains("login") || lower.contains("token") {
        Some("로그인이 필요하거나 토큰이 만료됐어요.".to_string())
    } else if lower.contains("deployment_in_progress") || lower.contains("in progress") {
        Some("이미 진행 중인 배포가 있어요.".to_string())
    } else if lower.contains("not found") || lower.contains("resource") {
        Some("앱이나 배포 대상을 찾지 못했어요.".to_string())
    } else {
        combined
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('{'))
            .map(|line| line.chars().take(160).collect())
    }
}

fn human_deploy_failure_text(raw: &str) -> Option<String> {
    let lower = raw.to_lowercase();
    if lower.contains("commit_not_found") || lower.contains("커밋을 찾을 수 없") {
        Some(
            "커밋을 원격 저장소에서 찾을 수 없어요. 로컬 커밋을 원격에 올린 뒤 다시 배포해 주세요."
                .to_string(),
        )
    } else if lower.contains("deployment_in_progress") || lower.contains("in progress") {
        Some("이미 진행 중인 배포가 있어요.".to_string())
    } else if lower.contains("unauth") || lower.contains("login") || lower.contains("token") {
        Some("로그인이 필요하거나 토큰이 만료됐어요.".to_string())
    } else {
        None
    }
}

fn quality_violation_label(
    violation: &axhub_helpers::quality_gate::QualityViolation,
) -> &'static str {
    use axhub_helpers::quality_gate::QualityViolation;
    match violation {
        QualityViolation::MissingCliVersion => "CLI 버전을 확인하지 못했어요.",
        QualityViolation::BootstrapPlanWithAppId => "앱 등록 상태가 서로 맞지 않아요.",
        QualityViolation::ExitCodeMismatch { .. } => "배포 준비 상태 계산이 서로 맞지 않아요.",
        QualityViolation::InvalidProfile { .. } => "프로필 정보가 서로 맞지 않아요.",
        QualityViolation::AuthMismatch => "로그인 상태가 서로 맞지 않아요.",
    }
}

fn deploy_eta_human(seconds: u64) -> String {
    if seconds == 0 {
        "곧 완료".to_string()
    } else if seconds < 60 {
        format!("약 {seconds}초")
    } else {
        let minutes = seconds.div_ceil(60);
        format!("약 {minutes}분")
    }
}

fn cmd_emit_deploy_complete(rest: &[String]) -> anyhow::Result<i32> {
    if rest.len() > 2 {
        return legacy_usage_error("emit-deploy-complete", "expected at most two arguments");
    }
    let exit_code: i32 = match rest.first() {
        None => 0,
        Some(raw) => match raw.parse() {
            Ok(code) if code >= 0 => code,
            _ => {
                return legacy_usage_error(
                    "emit-deploy-complete",
                    "exit_code must be a non-negative integer",
                )
            }
        },
    };
    let default_class = "axhub deploy create".to_string();
    let command_class = rest.get(1).unwrap_or(&default_class);
    if command_class.starts_with('-') {
        return legacy_usage_error("emit-deploy-complete", "unknown option");
    }
    if let Err(err) = emit_deploy_complete(exit_code, command_class) {
        eprintln!("emit-deploy-complete: {err}");
        return Ok(1);
    }
    Ok(0)
}

// ---------------------------------------------------------------------------
// settings-merge subcommand
// ---------------------------------------------------------------------------

struct SettingsMergeArgs {
    dry_run: bool,
    scope: Scope,
    json: bool,
    silent: bool,
    command_path_override: Option<PathBuf>,
    migrate: bool,
    yes: bool,
}

fn parse_settings_merge_args(
    cli: &cli::args::SettingsMergeCliArgs,
) -> anyhow::Result<SettingsMergeArgs> {
    let scope = match cli.scope.as_deref() {
        None | Some("auto") => Scope::Auto,
        Some("user") => Scope::User,
        Some("project") => Scope::Project,
        Some(other) => {
            anyhow::bail!("--scope 값이 잘못됐어요: {other} (user|project|auto 만 가능)")
        }
    };
    if cli.apply && cli.dry_run {
        anyhow::bail!("--apply 와 --dry-run 은 같이 사용할 수 없어요");
    }
    if cli.migrate && cli.apply {
        anyhow::bail!("--migrate 와 --apply 는 같이 사용할 수 없어요");
    }
    Ok(SettingsMergeArgs {
        // --migrate and --apply are mutually exclusive, so `!apply` is always
        // true in migrate mode — use explicit dry_run flag instead.
        dry_run: if cli.migrate { cli.dry_run } else { !cli.apply },
        scope,
        json: cli.json,
        silent: cli.silent,
        command_path_override: cli.command_path.clone().map(PathBuf::from),
        migrate: cli.migrate,
        yes: cli.yes,
    })
}

// ---------------------------------------------------------------------------
// autowire-statusline subcommand
// ---------------------------------------------------------------------------

pub(crate) fn cmd_autowire_statusline(
    arg_scope: Option<&str>,
    silent: bool,
    is_child: bool,
    command_path: Option<PathBuf>,
) -> anyhow::Result<i32> {
    // sh/ps1-absorption Phase 2.2 (T6): `auto` scope keyword + dispatch mode.
    // flag 파싱은 clap(cli::args::AutowireCliArgs)이 담당하고, 여기선 scope 값
    // 검증(한국어 에러 보존) + auto 해석만 해요. `--child` worker 모드 보존.
    let (scope, auto_scope): (Option<Scope>, bool) = match arg_scope {
        Some("user") => (Some(Scope::User), false),
        Some("project") => (Some(Scope::Project), false),
        Some("auto") => (None, true),
        Some(other) => {
            eprintln!(
                "axhub-helpers autowire-statusline: --scope 는 user|project|auto 만 가능해요 (받은 값: {other})"
            );
            return Ok(64);
        }
        None => (None, false),
    };
    let scope = match (scope, auto_scope) {
        (Some(s), _) => s,
        (None, true) => match detect_scope_from_env() {
            Some(s) => s,
            None => {
                // Ambiguous scope (CLAUDE_PLUGIN_ROOT 가 user/project 어느 plugins dir 도
                // 아니면) — fail-closed exit 0. shell wrapper 의 step 3 동작 보존.
                // Reviewer Issue 2 (PR #114): observability log 가 --silent
                // 모드에서도 남아야 silent skip 진단이 가능해요.
                hook_safety::append_hook_error(
                    "session-start-autowire",
                    &"scope auto: CLAUDE_PLUGIN_ROOT 또는 git rev-parse cwd 가 user/project plugins prefix 와 매칭 안 됨 — merge 건너뜀",
                );
                if !silent {
                    eprintln!(
                        "axhub-helpers autowire-statusline: --scope auto 가 scope 감지 실패 — 종료 (fail-closed)"
                    );
                }
                return Ok(0);
            }
        },
        (None, false) => {
            eprintln!("axhub-helpers autowire-statusline: --scope user|project|auto 가 필요해요");
            return Ok(64);
        }
    };
    let code = autowire_statusline(AutowireArgs {
        scope,
        command_path_override: command_path,
        silent,
        is_dispatcher: !is_child,
    });
    Ok(code)
}

/// Detect statusLine scope from `CLAUDE_PLUGIN_ROOT` prefix.
///
/// Mirrors `hooks/session-start-autowire.{sh,ps1}` step 3 logic so the shell
/// wrappers can drop manual scope detection and rely on `--scope auto`.
///
/// Returns:
///   - `Some(Scope::User)`     when `CLAUDE_PLUGIN_ROOT` starts with `$HOME/.claude/plugins/`
///   - `Some(Scope::Project)`  when it starts with `<repo>/.claude/plugins/`
///   - `None`                  when ambiguous (fail-closed in caller)
fn detect_scope_from_env() -> Option<Scope> {
    let root = std::env::var("CLAUDE_PLUGIN_ROOT").ok()?;
    if let Ok(home) = std::env::var("HOME") {
        let user_prefix = format!("{home}/.claude/plugins/");
        if root.starts_with(&user_prefix) {
            return Some(Scope::User);
        }
    }
    // Windows USERPROFILE fallback for `$HOME` shape mirroring the ps1 wrapper.
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let user_prefix = format!("{userprofile}\\.claude\\plugins\\");
        if root.starts_with(&user_prefix) {
            return Some(Scope::User);
        }
    }
    // Project scope: `git -C <cwd> rev-parse --show-toplevel` then check whether
    // CLAUDE_PLUGIN_ROOT starts with `<repo>/.claude/plugins/`. cwd-sensitive —
    // dispatcher 가 child spawn 전 호출하므로 SessionStart hook 의 cwd 가
    // 사용자의 repo 일 때 정확하게 동작해요.
    let cwd = std::env::current_dir().ok()?;
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&cwd)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let repo = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if repo.is_empty() {
        return None;
    }
    let project_prefix_unix = format!("{repo}/.claude/plugins/");
    if root.starts_with(&project_prefix_unix) {
        return Some(Scope::Project);
    }
    // Windows path separator variant
    let repo_win = repo.replace('/', "\\");
    let project_prefix_win = format!("{repo_win}\\.claude\\plugins\\");
    if root.starts_with(&project_prefix_win) {
        return Some(Scope::Project);
    }
    None
}

// ---------------------------------------------------------------------------
// orphan-stub subcommand
// ---------------------------------------------------------------------------

fn cmd_orphan_stub(args: &[String]) -> anyhow::Result<i32> {
    let mut install = false;
    let mut verify = false;
    for arg in args {
        match arg.as_str() {
            "--install" => install = true,
            "--verify" => verify = true,
            "-h" | "--help" => {
                println!(
                    "axhub-helpers orphan-stub — orphan stub 설치 및 검증\n\n\
                     USAGE:\n  axhub-helpers orphan-stub --install [--verify]\n  \
                     axhub-helpers orphan-stub --verify\n\n\
                     OPTIONS:\n  --install   orphan stub 설치 (없으면 생성, 있으면 덮어쓰기)\n  \
                     --verify    stub 존재 + 실행 권한 확인\n  \
                     -h, --help  도움말\n\n\
                     Stub 경로: $XDG_STATE_HOME/axhub-plugin/orphan-stub-statusline.{{sh,ps1}}"
                );
                return Ok(0);
            }
            other => {
                eprintln!("axhub-helpers orphan-stub: 알 수 없는 flag: {other}");
                return Ok(64);
            }
        }
    }
    if !install && !verify {
        eprintln!("axhub-helpers orphan-stub: --install 또는 --verify 가 필요해요");
        return Ok(64);
    }
    if install {
        match axhub_helpers::orphan_stub::install() {
            Ok(path) => {
                if !axhub_helpers::orphan_stub::verify(&path) {
                    eprintln!(
                        "axhub-helpers orphan-stub: 설치 후 verify 실패 ({})",
                        path.display()
                    );
                    return Ok(1);
                }
                println!("{}", path.display());
                if !axhub_helpers::autowire::is_non_interactive() {
                    eprintln!("axhub: orphan stub 설치됐어요 → {}", path.display());
                }
            }
            Err(e) => {
                eprintln!("axhub-helpers orphan-stub: 설치 실패 — {e}");
                return Ok(1);
            }
        }
    }
    if verify && !install {
        // verify-only (no install)
        let Some(paths) = axhub_helpers::orphan_stub::StubPaths::resolve() else {
            eprintln!("axhub-helpers orphan-stub: state_dir() 확인 불가");
            return Ok(1);
        };
        let path = if cfg!(target_os = "windows") {
            &paths.ps1
        } else {
            &paths.sh
        };
        if !axhub_helpers::orphan_stub::verify(path) {
            eprintln!(
                "axhub-helpers orphan-stub: verify 실패 — 없거나 실행 권한 없어요 ({})",
                path.display()
            );
            return Ok(1);
        }
    }
    Ok(0)
}

pub(crate) fn cmd_settings_merge(cli: cli::args::SettingsMergeCliArgs) -> anyhow::Result<i32> {
    let parsed = match parse_settings_merge_args(&cli) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("axhub-helpers settings-merge: {e}");
            return Ok(64);
        }
    };

    // --migrate mode: detect and rewrite stale ${CLAUDE_PLUGIN_ROOT} literals.
    if parsed.migrate {
        return cmd_settings_merge_migrate(parsed);
    }

    // Ensure orphan stub is ready before --apply.
    // For --dry-run: only check presence, do NOT install.
    let command_path_override = if parsed.command_path_override.is_some() {
        // Explicit override — use as-is (deprecation warning emitted inside default_command_path).
        parsed.command_path_override
    } else if !parsed.dry_run {
        // --apply: install + verify stub (fail-open on error).
        match axhub_helpers::orphan_stub::install_and_verify() {
            Some(p) => Some(p),
            None => {
                eprintln!("axhub: orphan stub install/verify 실패했어요. merge 는 계속 진행해요.");
                None
            }
        }
    } else {
        // --dry-run: check stub presence only, do not install.
        if let Some(p) = axhub_helpers::orphan_stub::stub_path() {
            if !axhub_helpers::orphan_stub::verify(&p) {
                eprintln!("axhub: orphan stub 이 없어요. --apply 실행 시 자동 설치돼요.");
            }
        }
        None
    };

    let opts = MergeOptions {
        silent: parsed.silent,
        command_path_override,
        scope: parsed.scope,
        dry_run: parsed.dry_run,
    };
    let outcome = match run_settings_merge(opts) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("axhub-helpers settings-merge: {e}");
            return Ok(1);
        }
    };
    if parsed.json {
        println!("{}", serde_json::to_string(&outcome)?);
    }
    let exit_code = match &outcome {
        MergeOutcome::NoOp => 0,
        MergeOutcome::Created => 2,
        MergeOutcome::Merged => 3,
        MergeOutcome::PreservedOther => 4,
        MergeOutcome::InvalidJson => 5,
        MergeOutcome::PartialSchema => 6,
        MergeOutcome::PermissionDenied => 7,
    };
    Ok(exit_code)
}

fn cmd_settings_merge_migrate(parsed: SettingsMergeArgs) -> anyhow::Result<i32> {
    use std::io::IsTerminal as _;

    let dry_run = parsed.dry_run;

    // When not --yes and not --dry-run: run a detection pass, then prompt on TTY.
    if !parsed.yes && !dry_run {
        let detect = match migrate_stale_command_path(&parsed.scope, true) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("axhub-helpers settings-merge --migrate: {e}");
                return Ok(1);
            }
        };
        let stale_labels: Vec<&str> = detect
            .iter()
            .filter(|(_, o)| *o == MigrateOutcome::DryRun)
            .map(|(label, _)| label.as_str())
            .collect();

        if stale_labels.is_empty() {
            eprintln!("axhub: stale statusLine.command 항목이 없어요. 이미 최신 상태예요.");
            return Ok(0);
        }

        eprintln!("axhub: 아래 scope 의 settings.json 에서 stale statusLine.command 감지했어요:");
        for label in &stale_labels {
            eprintln!("  - {label}");
        }

        if !std::io::stdin().is_terminal() {
            eprintln!(
                "axhub: TTY 가 없어요. --yes flag 를 추가해 자동 적용하거나 직접 수정해주세요."
            );
            return Ok(0);
        }

        eprint!("axhub: orphan stub path 로 교체할까요? [y/N]: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("axhub: migrate 를 취소했어요.");
            return Ok(0);
        }
    }

    let results = match migrate_stale_command_path(&parsed.scope, dry_run) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("axhub-helpers settings-merge --migrate: {e}");
            return Ok(1);
        }
    };

    if parsed.json {
        let map: serde_json::Map<String, Value> = results
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap_or(Value::Null)))
            .collect();
        println!("{}", serde_json::to_string(&map)?);
    }

    let migrated = results.iter().any(|(_, o)| *o == MigrateOutcome::Migrated);
    let dry_detected = results.iter().any(|(_, o)| *o == MigrateOutcome::DryRun);
    let warned = results
        .iter()
        .any(|(_, o)| *o == MigrateOutcome::WarnGitTracked);

    // 0 = no-op/no stale, 2 = migrated (or would migrate), 3 = git-tracked warn
    Ok(if migrated || dry_detected {
        2
    } else if warned {
        3
    } else {
        0
    })
}

/// `diagnose` subcommand dispatch. v0.8.0 wires `hitl` only — strategy runner
/// (`run`) and probe sweep ship in follow-up PRs per the PR #113 honest
/// tradeoff section. Unknown subcommands return exit 64 (EX_USAGE) so the
/// shell sees a stable error code instead of a panic.
/// `diagnose hitl --session <loop_id> --prompts <prompts.json> [--output <captured.json>]`
///
/// Drives the user through the prompt list via `StdioRunner`, applies
/// `redact_for_handoff` to every capture, writes the result to a 0o600 file
/// at `--output` (default: `<state_dir>/loops/<session>/captured.json`).
///
/// Exit codes:
///   0 — completed (some prompts may have timed out, see `timed_out` field)
///   64 — usage error (missing flags / invalid args)
///   65 — environment error (TTY missing, state dir unresolvable)
///   1 — operational error (spec parse failure, write failure, runner abort)
pub(crate) fn cmd_diagnose_hitl(
    session: Option<String>,
    prompts: Option<String>,
    output: Option<String>,
) -> anyhow::Result<i32> {
    use axhub_helpers::diagnose::hitl::{run_from_files, StdioRunner};

    let prompts = prompts.map(PathBuf::from);
    let output = output.map(PathBuf::from);
    let Some(session) = session else {
        eprintln!("axhub-helpers diagnose hitl: --session <loop_id> required");
        return Ok(64);
    };
    let Some(prompts) = prompts else {
        eprintln!("axhub-helpers diagnose hitl: --prompts <prompts.json> required");
        return Ok(64);
    };
    if !io::stdin().is_terminal() {
        eprintln!("axhub-helpers diagnose hitl: TTY unavailable");
        return Ok(65);
    }
    let output_path = match output {
        Some(p) => p,
        None => {
            let Some(state) = state_dir() else {
                eprintln!("axhub-helpers diagnose hitl: cannot resolve state directory");
                return Ok(65);
            };
            state.join("loops").join(&session).join("captured.json")
        }
    };
    let mut runner = StdioRunner::new();
    match run_from_files(&prompts, &output_path, &mut runner) {
        Ok(result) => {
            // Echo a minimal completion summary to stdout so the orchestrator
            // can read it deterministically. captured.json holds the full
            // (already-redacted) detail.
            let summary = json!({
                "session": session,
                "captured": result.captures.len(),
                "timed_out": result.timed_out,
                "truncated": result.truncated,
                "output_path": output_path.display().to_string(),
            });
            println!("{summary}");
            Ok(0)
        }
        Err(e) => {
            eprintln!("axhub-helpers diagnose hitl: {e}");
            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// US-014: `RealVerifyProbes` must spawn `axhub deploy list` **at
    /// most once** per `cmd_verify`. The pre-PR-#149 RealVerifyProbes
    /// invoked `latest_deploy_id_for_app` twice (status + logs probe);
    /// the new memoized `cached_lookup` ensures one spawn covers both
    /// downstream probes for the lifetime of the probes struct.
    #[test]
    fn real_verify_probes_memoizes_deploy_id_lookup() {
        let probes = RealVerifyProbes::new();
        let count = AtomicUsize::new(0);
        let runner = |_args: &[&str]| {
            count.fetch_add(1, Ordering::SeqCst);
            axhub_helpers::verify_helper::ProbeResult {
                stdout: r#"{"items":[{"id":"dep-mem-1"}]}"#.to_string(),
                exit_code: 0,
                timed_out: false,
            }
        };
        let first = probes.resolve_deploy_id_with("paydrop", runner);
        let second = probes.resolve_deploy_id_with("paydrop", runner);
        assert!(
            matches!(first, DeployIdLookup::Found(ref id) if id == "dep-mem-1"),
            "first resolve must surface Found(dep-mem-1): {first:?}"
        );
        assert!(
            matches!(second, DeployIdLookup::Found(ref id) if id == "dep-mem-1"),
            "second resolve must hit the cache and surface the same id: {second:?}"
        );
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "axhub deploy list must be spawned exactly once across both probes"
        );
    }

    /// US-014 corollary: even failing lookups (NoRecentDeploy /
    /// TransportFailure) must be cached. Otherwise a transient auth
    /// error gets re-issued on every probe within the same verify run.
    #[test]
    fn real_verify_probes_memoizes_transport_failure() {
        let probes = RealVerifyProbes::new();
        let count = AtomicUsize::new(0);
        let runner = |_args: &[&str]| {
            count.fetch_add(1, Ordering::SeqCst);
            axhub_helpers::verify_helper::ProbeResult {
                stdout: String::new(),
                exit_code: 65,
                timed_out: false,
            }
        };
        let _ = probes.resolve_deploy_id_with("paydrop", runner);
        let _ = probes.resolve_deploy_id_with("paydrop", runner);
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "transport-failure lookups must also be cached"
        );
    }

    /// US-007: fslock single-flight contract that
    /// `cmd_auth_refresh_bg` depends on. The fslock primitive itself is
    /// upstream-tested; this test validates our specific use shape
    /// (try_lock_with_pid returning false when held; drop releases).
    #[cfg(unix)]
    #[test]
    fn auth_refresh_lock_is_single_flight() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("auth-refresh.lock");

        let mut first = fslock::LockFile::open(&lock_path).expect("open lock");
        assert!(
            first.try_lock_with_pid().expect("try_lock #1"),
            "first acquire must succeed"
        );

        let mut second = fslock::LockFile::open(&lock_path).expect("open lock");
        assert!(
            !second.try_lock_with_pid().expect("try_lock #2"),
            "second acquire must observe contention while first holds the lock"
        );

        drop(first);

        let mut third = fslock::LockFile::open(&lock_path).expect("open lock");
        assert!(
            third.try_lock_with_pid().expect("try_lock #3"),
            "lock must be acquirable after the first holder releases (drop)"
        );
    }

    /// INPUT-contract repair (I9 twin): `latest_deploy_id_with_runner` parses
    /// the spawned `axhub deploy list` exit code. The current CLI emits 4=unauth
    /// / 5=not_found (not 65/67), so the auth/not-found verdict reasons must key
    /// off 4/5 — otherwise a real auth failure degrades to the generic "exit
    /// code N" reason.
    #[test]
    fn latest_deploy_id_maps_current_cli_auth_and_not_found_exits() {
        let auth = latest_deploy_id_with_runner("paydrop", |_args| {
            axhub_helpers::verify_helper::ProbeResult {
                stdout: String::new(),
                exit_code: 4,
                timed_out: false,
            }
        });
        match auth {
            DeployIdLookup::TransportFailure { reason } => {
                assert!(
                    reason.contains("만료"),
                    "exit 4 must map to auth reason: {reason}"
                );
            }
            other => panic!("exit 4 must be TransportFailure, got {other:?}"),
        }

        let missing = latest_deploy_id_with_runner("paydrop", |_args| {
            axhub_helpers::verify_helper::ProbeResult {
                stdout: String::new(),
                exit_code: 5,
                timed_out: false,
            }
        });
        match missing {
            DeployIdLookup::TransportFailure { reason } => {
                assert!(
                    reason.contains("찾을 수 없"),
                    "exit 5 must map to not-found reason: {reason}"
                );
            }
            other => panic!("exit 5 must be TransportFailure, got {other:?}"),
        }
    }

    /// PR 25.7 / INPUT-contract repair: only genuine *server-side* deploy
    /// failures suggest the trace nl-trigger. Client-side pre-attempt gates
    /// (2 clap usage / 4 auth / 11 dry-run preview / 64 usage) never reached
    /// the server, so the "왜 실패했어" nudge would mislead — they return None.
    #[test]
    fn verify_trace_suggestion_fires_on_real_deploy_failure_exits() {
        // Success path: confirm nudge, never the failure trace.
        assert!(verify_trace_suggestion("axhub deploy create paydrop", 0)
            .is_some_and(|m| m.contains("확인해")));

        // Genuine server-side deploy failures — every one must suggest the trace.
        for exit in [1, 5, 7, 8, 9, 10, 12, 13] {
            assert!(
                verify_trace_suggestion("axhub deploy create paydrop", exit)
                    .is_some_and(|m| m.contains("왜 실패했어")),
                "exit {exit} on deploy create must suggest the failure trace"
            );
        }

        // Client-side pre-attempt gates are NOT trace-worthy deploy failures.
        for exit in [2, 4, 11, 64] {
            assert!(
                verify_trace_suggestion("axhub deploy create paydrop", exit).is_none(),
                "exit {exit} is a client-side gate and must NOT suggest the trace"
            );
        }

        // Non-deploy-create commands are out of scope for this nudge.
        assert!(verify_trace_suggestion("axhub apps list", 9).is_none());

        // recover success keeps its own confirm nudge.
        assert!(verify_trace_suggestion("axhub recover paydrop", 0)
            .is_some_and(|m| m.contains("확인해")));
    }
}
