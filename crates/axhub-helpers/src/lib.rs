pub mod atomic_jsonl;
pub mod autowire;
pub mod observability;
pub mod orphan_stub;

/// Process-wide mutex for tests that mutate process environment variables.
///
/// Tests in different modules share the same process env, so a single lock
/// prevents cross-module env-var races. Unit tests that call `std::env::set_var`
/// / `remove_var` MUST hold this lock for the duration of the mutation.
#[cfg(test)]
pub static PROCESS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub mod audit;
pub mod audit_ledger;
pub mod bootstrap;
pub mod diagnose;
pub mod catalog;
pub mod commit_gate;
pub mod config;
pub mod consent;
pub mod deploy_prep;
pub mod event_log;
pub mod hook_output;
pub mod hook_safety;
pub mod humanize;
pub mod karpathy_inject;
pub mod keychain;
pub mod keychain_windows;
pub mod list_deployments;
pub mod messages;
pub mod preflight;
pub mod quality_gate;
pub mod quality_state;
pub mod recovery_scan;
pub mod redact;
pub mod resolve;
pub mod runtime_paths;
pub mod session_bundle;
pub mod settings_merge;
pub mod spawn;
pub mod statusline;
pub mod tdd_inject;
pub mod telemetry;
pub mod test_classifier;
pub mod trace_helper;
pub mod verify_helper;
