pub mod atomic_jsonl;
pub mod autowire;
pub mod axhub_cli;
pub mod cli_drift;
pub mod cli_envelope;
pub mod observability;
pub mod orphan_stub;

/// Process-wide mutex for ANY code that mutates process environment variables.
///
/// All env-mutating sites (production probes, hooks, recurrence env overrides,
/// preflight wall-budget overrides, headless guards) MUST acquire this lock.
/// Single source of truth so concurrent set_var/remove_var across threads/modules
/// never tears the process env block. Tests share the same lock so cargo's
/// parallel test runner stays correct.
pub static PROCESS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub mod audit;
pub mod audit_ledger;
pub mod bootstrap;
pub mod catalog;
pub mod commit_gate;
pub mod config;
pub mod deploy_prep;
pub mod diagnose;
pub mod event_log;
pub mod grace;
pub mod hook_output;
pub mod hook_safety;
pub mod humanize;
pub mod init_resume;
pub mod karpathy_inject;
pub mod keychain;
pub mod keychain_windows;
pub mod list_deployments;
pub mod messages;
pub mod migrate_plan;
pub mod onboarding_detect;
pub mod plugin_update;
pub mod preflight;
pub mod quality_gate;
pub mod quality_state;
pub mod recovery_scan;
pub mod redact;
pub mod repair_path;
pub mod resolve;
pub mod routing;
pub mod runtime_paths;
pub mod scaffold;
pub mod session_bundle;
pub mod settings_merge;
pub mod snippet;
pub mod spawn;
pub mod statusline;
pub mod sync;
pub mod tdd_inject;
pub mod telemetry;
pub mod test_classifier;
pub mod trace_helper;
pub mod verify_helper;
