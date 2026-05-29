use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use axhub_helpers::autowire::{autowire_statusline, AutowireArgs};
use axhub_helpers::bootstrap::{cmd_bootstrap_dependency_plan, run_bootstrap};
use axhub_helpers::catalog::classify;
use axhub_helpers::config::{config_get, config_set, render_get_json};
use axhub_helpers::consent::{
    format_preauth_deny_hint, mint_token, parse_axhub_command, validate_binding_schema,
    verify_or_claim_token, verify_token, write_private_file_no_follow, ConsentBinding,
};
use axhub_helpers::deploy_prep::run_deploy_prep;
use axhub_helpers::hook_safety;
use axhub_helpers::keychain::{parse_keyring_value, read_keychain_token};
use axhub_helpers::list_deployments::{run_list_deployments, ListDeploymentsArgs};
use axhub_helpers::preflight::{run_preflight, PreflightRun};
use axhub_helpers::quality_gate::{validate_deploy_prep_quality, QualityCheckResult};
use axhub_helpers::redact::redact;
use axhub_helpers::resolve::run_resolve;
use axhub_helpers::runtime_paths::{last_deploy_file, state_dir, token_file, welcome_marker_path};
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
use axhub_helpers::{commit_gate, hook_output, quality_state};
use chrono::Utc;
use serde_json::{json, Map, Value};

mod cli;

pub(crate) const HOOK_SCHEMA_VERSION: &str = "v0";
pub(crate) const USAGE: &str = "axhub-helpers - axhub plugin adapter binary (Rust)\n\nUsage:\n  axhub-helpers <subcommand> [args]\n\nSubcommands:\n  session-start\n  preauth-check\n  prompt-route\n  consent-mint [--validate-only]\n  consent-verify\n  resolve\n  preflight\n  classify-exit\n  redact\n  statusline\n  path <token-file|last-deploy-file|state-dir>\n  token-init [--json]\n  token-import [--json]\n  token-gate\n  post-install --target-name <N> --bin-dir <D> --link-path <P> [--repo-root <R>]\n  list-deployments\n  bootstrap [--json] [--dry-run|--plan-only|--auto-chain|--record <event>|dependency-plan]\n  routing-stats [--since <D>] [--json] [--top <N>] [--confused]\n  cleanup-audit [--all] [--yes]\n  audit-clarify (--hash <H>|--prompt <P>) --chosen <S>\n  routing-dashboard [--html]\n  mark <phase_name>\n  emit-deploy-complete [<exit_code> [<command_class>]]\n  deploy-prep --intent <name> [--user-utterance <s>] [--refresh-in-flight] [--json]\n  config get <key> [--json]\n  config set <key> <value>\n  sync [--target <target>|auto] [--out <dir>] [--json] [--no-detail] [--allow-identity-change]\n  snippet --mode A|B --language <lang> --target <target> --connector <name> --path <path> --sql <sql> --allowed-columns <csv>\n  auth-refresh-bg\n  verify --app-id <id> [--json]\n  trace --deploy-id <id> [--app <app>] [--json]\n  doctor [--json] [--no-cooldown]\n  settings-merge --apply|--dry-run [--scope user|project|auto] [--json]\n  autowire-statusline --scope user|project [--silent] [--command-path <p>] [--child]\n  orphan-stub --install [--verify] | --verify\n  diagnose hitl --session <loop_id> --prompts <prompts.json> [--output <captured.json>]\n  version [--quiet]\n  help";

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
            let run = run_preflight();
            println!("{}", serde_json::to_string(&run.output)?);
            Ok(run.exit_code)
        }
        "resolve" => {
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
        // consent-mint: US2 typed (cli::Commands::ConsentMint) — legacy arm 제거.
        "consent-verify" => cmd_consent_verify(),
        "state-show" => cmd_state_show(&rest),
        "state-update" => cmd_state_update(&rest),
        "commit-gate" => cmd_commit_gate(),
        "test-classifier" => cmd_test_classifier(),
        "tdd-inject" => cmd_tdd_inject(),
        "karpathy-inject" => cmd_karpathy_inject(),
        "consent" => cmd_quality_consent(&rest),
        "preauth-check" => cmd_preauth_check(),
        "prompt-route" => cmd_prompt_route(),
        "session-start" => cmd_session_start(),
        "mark" => cmd_mark(&rest),
        "emit-deploy-complete" => cmd_emit_deploy_complete(&rest),
        "deploy-prep" => cmd_deploy_prep(&rest),
        "config" => cmd_config(&rest),
        "sync" => run_sync(&rest),
        "snippet" => run_snippet(&rest),
        "auth-refresh-bg" => cmd_auth_refresh_bg(),
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
    axhub_helpers::consent::set_private_dir_mode(&parent).ok();
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
    if command.starts_with("axhub deploy create") && (64..=68).contains(&exit_code) {
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

fn parse_binding(raw: &str) -> anyhow::Result<ConsentBinding> {
    Ok(serde_json::from_str(raw)?)
}

fn consent_mint_json_stdin_help() -> &'static str {
    r#"PowerShell example: $binding | ConvertTo-Json -Compress | & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" consent-mint
Temp-file fallback: Get-Content -Raw "$Path" | & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" consent-mint"#
}

fn parse_consent_mint_binding(raw: &str) -> Result<ConsentBinding, i32> {
    let binding_json = raw.trim().trim_start_matches('\u{feff}').trim();
    if binding_json.is_empty() {
        eprintln!(
            "axhub-helpers consent-mint: empty stdin; no JSON binding was provided.\n{}",
            consent_mint_json_stdin_help()
        );
        return Err(65);
    }
    serde_json::from_str(binding_json).map_err(|err| {
        eprintln!(
            "axhub-helpers consent-mint: invalid JSON; consent-mint expects one JSON object binding on stdin.\nError: {err}\n{}",
            consent_mint_json_stdin_help()
        );
        65
    })
}

pub(crate) fn cmd_consent_mint(validate_only: bool) -> anyhow::Result<i32> {
    let raw = read_stdin()?;
    let b = match parse_consent_mint_binding(&raw) {
        Ok(binding) => binding,
        Err(code) => return Ok(code),
    };
    validate_binding_schema(&b)?;
    if validate_only {
        out_json(json!({"valid": true, "action": b.action}));
        return Ok(0);
    }
    let result = mint_token(b, 60)?;
    out_json(serde_json::to_value(result)?);
    Ok(0)
}
fn cmd_consent_verify() -> anyhow::Result<i32> {
    let b = parse_binding(&read_stdin()?)?;
    let result = verify_token(b);
    out_json(serde_json::to_value(&result)?);
    Ok(if result.valid { 0 } else { 65 })
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

pub(crate) fn cmd_preauth_check() -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("preauth-check") {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        return Ok(0);
    }
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if let Some(sid) = payload.get("session_id").and_then(Value::as_str) {
        if std::env::var("CLAUDE_SESSION_ID").is_err() {
            std::env::set_var("CLAUDE_SESSION_ID", sid);
        }
    }
    if payload.get("tool_name").and_then(Value::as_str) != Some("Bash") {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        return Ok(0);
    }
    let cmd = payload
        .pointer("/tool_input/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    let parsed = parse_axhub_command(cmd);
    if !parsed.is_destructive {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        return Ok(0);
    }
    let deny_hint = format_preauth_deny_hint(parsed.action.as_deref(), parsed.app_id.as_deref());
    let is_identity = parsed.action.as_deref() == Some("auth_login");
    let binding = ConsentBinding {
        tool_call_id: format!(
            "{}:{}",
            payload
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            payload
                .get("tool_call_id")
                .and_then(Value::as_str)
                .unwrap_or("")
        ),
        action: parsed.action.unwrap_or_default(),
        app_id: parsed
            .app_id
            .unwrap_or_else(|| if is_identity { "_".into() } else { "".into() }),
        profile: parsed.profile.unwrap_or_else(|| {
            if is_identity {
                std::env::var("AXHUB_PROFILE").unwrap_or_else(|_| "default".into())
            } else {
                "".into()
            }
        }),
        branch: parsed
            .branch
            .unwrap_or_else(|| if is_identity { "_".into() } else { "".into() }),
        commit_sha: parsed.commit_sha.unwrap_or_else(|| {
            if is_identity {
                "_".into()
            } else {
                "".into()
            }
        }),
        context: parsed.context,
        synthesized_by_helper: false,
    };
    let result = verify_or_claim_token(binding);
    if result.valid {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        Ok(0)
    } else {
        out_json(json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny"
            },
            "systemMessage": deny_hint
        }));
        Ok(0)
    }
}

const MAX_LIST_DEPLOYMENTS_LIMIT: usize = 100;

// Approach E (Phase 2): cmd_prompt_route is preflight + audit only.
// No keyword chain, no skill enforcement, no `skills/<X>/SKILL.md` paths in
// additionalContext. Claude Code matches skills via SKILL.md frontmatter
// description natively (Phase 1 codegen merged main.rs phrases into descriptions).
pub(crate) fn cmd_prompt_route() -> anyhow::Result<i32> {
    use axhub_helpers::audit::{append as audit_append, now_iso8601, sha256_hex, AuditRecord};

    if hook_safety::is_hook_disabled("prompt-route") {
        out_json(json!({}));
        return Ok(0);
    }
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");

    let preflight = run_preflight();

    let record = AuditRecord {
        ts: now_iso8601(),
        prompt_hash: sha256_hex(prompt),
        prompt_len: prompt.len() as u32,
        cli_version: preflight.output.cli_version.clone(),
        auth_ok: preflight.output.auth_ok,
        is_axhub_related: heuristic_axhub_keyword(prompt),
        clarify_invoked: false,
        chosen_skill: None,
    };
    let _ = audit_append(record);

    let mut context = format_preflight_context(&preflight);
    if !hook_safety::is_karpathy_disabled() {
        if let Some(karpathy) = axhub_helpers::karpathy_inject::user_prompt_karpathy_inject()? {
            context.push_str("\n\n");
            context.push_str(&karpathy);
        }
    }
    println!("{}", hook_output::user_prompt_context(&context));
    Ok(0)
}

/// Single substring check for measurement only. NOT intent classification.
fn heuristic_axhub_keyword(prompt: &str) -> bool {
    prompt.to_lowercase().contains("axhub")
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
    let html_mode = args.iter().any(|a| a == "--html");
    if args.iter().any(|a| a == "-h" || a == "--help") {
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
// Base systemMessage (always 3 lines) + current-version first-session welcome
// (6 extra lines, one-shot, gated by welcome marker file). Marker write is
// best-effort — failure surfaces the welcome again next session, never blocks Claude.

const WELCOME_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn cmd_session_start() -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("session-start") {
        out_json(json!({}));
        return Ok(0);
    }
    write_session_start_bundle_best_effort();

    let mut lines: Vec<String> = vec![
        format!(
            "axhub helper Rust runtime 활성 (v{}).",
            env!("CARGO_PKG_VERSION")
        ),
        "막히면 /axhub:help 로 명령 메뉴를, /axhub:clarify 로 모호한 의도 확인을 부탁해요."
            .to_string(),
        "라우팅 통계는 axhub-helpers routing-stats 로 봐요.".to_string(),
        "audit log 로컬 7일 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1. 삭제: axhub-helpers cleanup-audit --all"
            .to_string(),
    ];

    let marker = welcome_marker_path(WELCOME_VERSION);
    let show_welcome = marker.as_ref().map(|p| !p.exists()).unwrap_or(false);
    if show_welcome {
        lines.push(String::new());
        lines.push(format!(
            "[axhub v{WELCOME_VERSION} 첫 세션] 라우팅 똑똑해졌어요."
        ));
        lines.push(
            "- Rust 키워드 체인 ~600줄 폐기. Claude 가 SKILL.md description 으로 직접 매칭해요."
                .to_string(),
        );
        lines.push("- 메타 질문 (\"왜 ~ 키워드 매칭이야?\") 자동 처리해요.".to_string());
        lines.push(
            "- routing audit log 7일 로컬 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1.".to_string(),
        );
        lines.push("- 변경점 보기: /axhub:whatsnew".to_string());

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
    if !hook_safety::is_megaskill_disabled() {
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
        // Map known exit codes to actionable reasons; otherwise echo the raw
        // exit code so verify_helper's verdict reasons aren't silently
        // collapsed to "state = unknown".
        let reason = match out.exit_code {
            65 => "axhub auth 만료 — axhub auth login 으로 재인증해주세요.".to_string(),
            67 => "axhub: 앱을 찾을 수 없어요 (resource not found).".to_string(),
            127 => "axhub CLI 를 찾을 수 없어요 (axhub:setup 으로 재설치).".to_string(),
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
    fn axhub_build_log(&self, deploy_id: &str, tail: u32) -> String {
        let Some(app_ref) = self.app_ref.as_deref() else {
            self.warnings.borrow_mut().push(
                "build_log_probe_skipped: --app required for current deploy logs".to_string(),
            );
            return String::new();
        };
        let axhub_bin = std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string());
        let tail = tail.to_string();
        match axhub_stdout_with_timeout(
            &axhub_bin,
            &[
                "--json", "deploy", "logs", deploy_id, "--app", app_ref, "--source", "build",
                "--limit", &tail,
            ],
        ) {
            Ok(stdout) => stdout,
            Err("timeout") => {
                self.warnings
                    .borrow_mut()
                    .push("build_log_probe_timeout: axhub deploy logs exceeded 5s".to_string());
                String::new()
            }
            Err(_) => {
                self.warnings
                    .borrow_mut()
                    .push("build_log_probe_failed: axhub CLI unavailable".to_string());
                String::new()
            }
        }
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

fn cmd_emit_deploy_complete(rest: &[String]) -> anyhow::Result<i32> {
    let exit_code: i32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let default_class = "axhub deploy create".to_string();
    let command_class = rest.get(1).unwrap_or(&default_class);
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
}
