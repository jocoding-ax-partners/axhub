//! clap CLI layer — Phase 2 scaffold (specs/001-helpers-clap-refactor).
//!
//! 전환기(transition) 설계: 단일 `Passthrough` external-subcommand variant 이
//! 모든 subcommand 를 main.rs 의 legacy `match` dispatcher(`crate::legacy_dispatch`)
//! 로 라우팅해요. 덕분에 명령별 typed 이관이 wave 로 진행되는 동안 동작이
//! byte-identical 로 유지돼요. `version`/`help`/no-args 는 clap 이전에 pre-intercept
//! 해서 정확한 legacy 출력을 보존해요(FR-004 + FR-006 전환기). passthrough·USAGE
//! pre-intercept 는 Polish(T036-T037)에서 제거해요.

use clap::Parser;

mod args;

#[derive(Parser)]
#[command(
    name = "axhub-helpers",
    bin_name = "axhub-helpers",
    disable_help_flag = true,
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
    PreauthCheck,
    CommitGate,
    TddInject,
    TestClassifier,

    // ── US1 — flag-bearing hook 명령 (typed args) ──
    ClassifyExit(args::ClassifyExitArgs),
    StateUpdate(args::StateUpdateArgs),
    AutowireStatusline(args::AutowireCliArgs),

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
    // 제외(=Normal, parse error→64): `state-update`·`autowire-statusline` 는 legacy 가
    // 잘못된 flag 에 **exit 64** 를 반환하므로(parity guard, FR-001) parse 실패도 64 여야
    // 해요. autowire 의 hook fail-open 은 SessionStart wrapper 의 nohup detach 가
    // 담당(helper exit 흡수)하지 직접 호출 exit code 가 아니에요. `token-gate` 도 Normal
    // (SKILL gate, exit 0/65 의미 보존).
    match token {
        Some(
            "session-start" | "prompt-route" | "preauth-check" | "commit-gate"
            | "tdd-inject" | "test-classifier" | "classify-exit" | "karpathy-inject",
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
fn handle_parse_error(e: clap::Error, hook_class: HookClass) -> i32 {
    use clap::error::ErrorKind;
    match e.kind() {
        ErrorKind::DisplayHelp => {
            print!("{e}");
            0
        }
        ErrorKind::DisplayVersion => 0,
        _ => {
            if hook_class == HookClass::FailOpenHook {
                0
            } else {
                let _ = e.print();
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
        Commands::PreauthCheck => run_result(crate::cmd_preauth_check()),
        Commands::CommitGate => run_result(crate::cmd_commit_gate()),
        Commands::TddInject => run_result(crate::cmd_tdd_inject()),
        Commands::TestClassifier => run_result(crate::cmd_test_classifier()),
        Commands::ClassifyExit(a) => {
            run_result(crate::cmd_classify_exit(a.exit_code, &a.stdout))
        }
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

        Commands::Passthrough(tokens) => {
            let Some((cmd, rest)) = tokens.split_first() else {
                eprintln!("{}", crate::USAGE);
                return 64;
            };
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

    let hook_class = classify(argv.get(1).map(String::as_str));
    match Cli::try_parse_from(std::env::args_os()) {
        Ok(cli) => dispatch(cli.command),
        Err(e) => handle_parse_error(e, hook_class),
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
            "preauth-check",
            "commit-gate",
            "tdd-inject",
            "test-classifier",
            "classify-exit",
            "karpathy-inject",
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
            "autowire-statusline",
            "token-gate",
            "deploy-prep",
            "sync",
            "doctor",
            "bogus-subcommand",
        ] {
            assert_eq!(classify(Some(t)), HookClass::Normal, "{t} must classify Normal");
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
