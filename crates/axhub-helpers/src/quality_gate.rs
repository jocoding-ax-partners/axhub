// Phase 26 PR 26.3 — push-time quality gate.
//
// Plan reference: .plan/matrix-absorption/phases/phase-26-tier-s-quick-wins.md
// PR 26.3. Sits between `run_deploy_prep` and the actual `axhub deploy create`
// invocation; rejects (or warns) when the composed preflight/resolve/bootstrap
// state is internally inconsistent.
//
// Plan v2 §10.2 #4 — exit code reuses the existing sysexits `64` (validation)
// with the `64:validation.quality_gate_failed` sub-key so classify-exit's
// Korean empathy catalog can dispatch the right surface. v1's reach for a new
// `EXIT_QUALITY_GATE = 66` was retracted (66 is already `scope.downgrade_*` +
// 2 update/profile sub-keys).
//
// Non-interactive callers exit with `64` and the sub-key marker printed on
// stderr ("axhub-error-sub-key: 64:validation.quality_gate_failed"); the
// fixture under `tests/fixtures/ask-defaults/registry.json` records that the
// canonical safe_default is "abort" so a CI/headless deploy never silently
// proceeds with corrupt prep state.
//
// This cumulative branch wires the validator into `axhub-helpers deploy-prep`.
// The pure validator stays testable here; the binary layer prints the sub-key
// and exits 64 in headless/non-interactive paths.

use serde::{Deserialize, Serialize};

use crate::deploy_prep::DeployPrepResult;

/// Symbolic violation codes — every concrete issue the gate can detect.
/// The variants double as the JSON-ifiable shape that the caller surfaces
/// to the user via systemMessage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QualityViolation {
    /// preflight could not detect an `axhub` CLI version. Without that we
    /// cannot reason about CLI compatibility or run the deploy safely.
    MissingCliVersion,
    /// `bootstrap_plan` is `Some` (first-deploy path) but `resolve` already
    /// produced an `app_id`. The two paths are mutually exclusive — landing
    /// here means the composer or upstream `resolve` is lying.
    BootstrapPlanWithAppId,
    /// The stored `exit_code` disagrees with what we recompute from
    /// (preflight, resolve, bootstrap). Indicates the composer drifted or
    /// somebody hand-edited the struct between compose and push.
    ExitCodeMismatch { recomputed: i32, observed: i32 },
    /// preflight and resolve disagree on which `profile` the deploy targets.
    /// Pushing under a misaligned profile is how prod-vs-staging swaps
    /// historically slipped through.
    InvalidProfile { preflight: String, resolve: String },
    /// preflight says auth failed (`auth_ok=false`) but `exit_code` is still
    /// `EXIT_OK`. Some upstream branch is letting through an unauthenticated
    /// push.
    AuthMismatch,
}

/// Aggregate outcome — `passed=true` ↔ `violations.is_empty()`. Wrapping
/// both into a struct keeps the callsite ergonomic when we later want to
/// attach hints, severity, or quick-fix suggestions without reshuffling
/// existing call sites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityCheckResult {
    pub passed: bool,
    pub violations: Vec<QualityViolation>,
}

impl QualityCheckResult {
    pub fn ok() -> Self {
        Self {
            passed: true,
            violations: Vec::new(),
        }
    }

    pub fn fail(violations: Vec<QualityViolation>) -> Self {
        Self {
            passed: violations.is_empty(),
            violations,
        }
    }

    /// Canonical sub-key emitted on stderr so classify-exit + trace skill
    /// can pick the right Korean empathy entry from `catalog.json`.
    pub const SUB_KEY: &'static str = "64:validation.quality_gate_failed";
}

/// Run every gate check against a composed `DeployPrepResult`. Pure: no I/O,
/// no env reads, no interactive prompts. The deploy_prep integration layer
/// is the one that prints `Self::SUB_KEY` to stderr and calls
/// `std::process::exit(EXIT_VALIDATION)` when interactive consent is absent.
pub fn validate_deploy_prep_quality(result: &DeployPrepResult) -> QualityCheckResult {
    let mut violations = Vec::new();

    if result.preflight.cli_version.is_none() {
        violations.push(QualityViolation::MissingCliVersion);
    }

    if result.bootstrap_plan.is_some() && result.resolve.app_id.is_some() {
        violations.push(QualityViolation::BootstrapPlanWithAppId);
    }

    let recomputed = recompute_exit_code(result);
    if recomputed != result.exit_code {
        violations.push(QualityViolation::ExitCodeMismatch {
            recomputed,
            observed: result.exit_code,
        });
    }

    if let (Some(pre), Some(res)) = (
        result.preflight.profile.as_ref(),
        result.resolve.profile.as_ref(),
    ) {
        if pre != res {
            violations.push(QualityViolation::InvalidProfile {
                preflight: pre.clone(),
                resolve: res.clone(),
            });
        }
    }

    if !result.preflight.auth_ok && result.exit_code == EXIT_OK {
        violations.push(QualityViolation::AuthMismatch);
    }

    QualityCheckResult {
        passed: violations.is_empty(),
        violations,
    }
}

// `deploy_prep::merge_exit_code` is private — we mirror its decision table
// here so the gate is independently auditable. Any drift between the two
// functions is exactly what `ExitCodeMismatch` exists to catch.
const EXIT_OK: i32 = 0;
const EXIT_VALIDATION: i32 = 64;
const EXIT_AUTH: i32 = 65;
const EXIT_NOT_FOUND: i32 = 67;

fn recompute_exit_code(result: &DeployPrepResult) -> i32 {
    // preflight failure dominates (auth_ok → 65, in_range/cli_present → 64).
    if !result.preflight.cli_present {
        return EXIT_VALIDATION;
    }
    if !result.preflight.auth_ok {
        return EXIT_AUTH;
    }
    if !result.preflight.in_range {
        return EXIT_VALIDATION;
    }
    // resolve failure surfaces via the resolve.error field. Mirror resolve's
    // own exit-code taxonomy (resolve.rs): the ambiguous-match case (more than
    // one matched_apps) is EXIT_USAGE (64); every other resolve error
    // (app_not_found / no_candidate_slug / apps_list_parse_error) is
    // NOT_FOUND (67). A genuine first deploy has no app yet, so resolve returns
    // 67 and `merge_exit_code` surfaces it verbatim before the bootstrap branch.
    // The earlier blanket `resolve.error.is_some() → 64` collapsed that 67 into
    // 64, so ExitCodeMismatch mis-fired on every first deploy and blocked the
    // happy-path bootstrap.
    if result.resolve.error.is_some() {
        if result.resolve.matched_apps.len() > 1 {
            return EXIT_VALIDATION; // ambiguous match → resolve's EXIT_USAGE (64)
        }
        return EXIT_NOT_FOUND;
    }
    // bootstrap_plan with no resolve error = first-deploy path → 67 (NOT_FOUND)
    // so the recover/bootstrap skill takes over.
    if result.bootstrap_plan.is_some() {
        return EXIT_NOT_FOUND;
    }
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preflight::PreflightOutput;
    use crate::resolve::ResolveOutput;

    fn happy_preflight() -> PreflightOutput {
        PreflightOutput {
            cli_version: Some("0.17.3".to_string()),
            in_range: true,
            cli_too_old: false,
            cli_too_new: false,
            cli_present: true,
            cli_state: "ok".to_string(),
            cli_on_path: true,
            cli_resolved_path: None,
            auth_ok: true,
            auth_error_code: None,
            scopes: vec!["deploy".to_string()],
            profile: Some("prod".to_string()),
            endpoint: Some("https://axhub-api.jocodingax.ai".to_string()),
            user_email: Some("test@example.com".to_string()),
            expires_at: None,
            expires_human: None,
            current_app: Some("paydrop".to_string()),
            current_team_id: Some("acme".to_string()),
            current_env: Some("prod".to_string()),
            last_deploy_id: None,
            last_deploy_status: None,
            helper_version_expected: Some("0.5.6".to_string()),
            helper_version_ok: true,
            plugin_version: "0.5.6".to_string(),
        }
    }

    fn happy_resolve() -> ResolveOutput {
        ResolveOutput {
            profile: Some("prod".to_string()),
            endpoint: Some("https://axhub-api.jocodingax.ai".to_string()),
            app_id: Some("42".to_string()),
            app_slug: Some("paydrop".to_string()),
            candidate_slug: None,
            matched_apps: vec![],
            branch: Some("main".to_string()),
            commit_sha: Some("abc123".to_string()),
            commit_message: Some("ship".to_string()),
            git_repo: true,
            git_has_commit: true,
            git_init_needed: false,
            eta_sec: 60,
            error: None,
            github_repo_url: None,
        }
    }

    fn happy_result() -> DeployPrepResult {
        DeployPrepResult {
            preflight: happy_preflight(),
            resolve: happy_resolve(),
            bootstrap_plan: None,
            exit_code: EXIT_OK,
            preflight_exit_code: EXIT_OK,
            in_flight_deploy: None,
            github_connected: false,
        }
    }

    #[test]
    fn happy_path_passes_with_no_violations() {
        let result = happy_result();
        let gate = validate_deploy_prep_quality(&result);
        assert!(gate.passed);
        assert!(gate.violations.is_empty());
    }

    #[test]
    fn missing_cli_version_flagged() {
        let mut result = happy_result();
        result.preflight.cli_version = None;
        let gate = validate_deploy_prep_quality(&result);
        assert!(!gate.passed);
        assert!(gate
            .violations
            .iter()
            .any(|v| matches!(v, QualityViolation::MissingCliVersion)));
    }

    #[test]
    fn bootstrap_plan_with_app_id_flagged() {
        // bootstrap_plan + app_id = corrupted composer output. Real
        // `derive_bootstrap_plan` would never produce this, but the gate
        // exists precisely to catch that drift.
        let mut result = happy_result();
        result.bootstrap_plan = Some(crate::deploy_prep::BootstrapPlan {
            is_first_deploy: true,
            required_steps: vec!["template".to_string()],
        });
        // app_id stays Some(42). exit_code stays EXIT_OK, but with bootstrap
        // present the recompute path returns EXIT_NOT_FOUND, so we also see
        // an ExitCodeMismatch alongside BootstrapPlanWithAppId.
        let gate = validate_deploy_prep_quality(&result);
        assert!(!gate.passed);
        assert!(gate
            .violations
            .iter()
            .any(|v| matches!(v, QualityViolation::BootstrapPlanWithAppId)));
    }

    #[test]
    fn exit_code_mismatch_flagged_when_observed_diverges_from_recompute() {
        let mut result = happy_result();
        result.exit_code = 64; // observed
        let gate = validate_deploy_prep_quality(&result);
        let mismatch = gate
            .violations
            .iter()
            .find_map(|v| match v {
                QualityViolation::ExitCodeMismatch {
                    recomputed,
                    observed,
                } => Some((*recomputed, *observed)),
                _ => None,
            })
            .expect("ExitCodeMismatch missing");
        assert_eq!(mismatch, (EXIT_OK, 64));
    }

    #[test]
    fn first_deploy_no_app_recomputes_not_found_without_mismatch() {
        // Genuine first deploy: resolve finds no app yet (app_not_found, empty
        // matched_apps) so it returns NOT_FOUND (67), and derive_bootstrap_plan
        // produces a first-deploy plan. The composer's merged exit_code is 67.
        // recompute_exit_code MUST also yield 67 — the pre-fix blanket
        // `resolve.error.is_some() → 64` made it 64, tripping a spurious
        // ExitCodeMismatch that blocked every first deploy.
        let mut result = happy_result();
        result.resolve.app_id = None;
        result.resolve.matched_apps = vec![];
        result.resolve.error = Some("app_not_found".to_string());
        result.bootstrap_plan = Some(crate::deploy_prep::BootstrapPlan {
            is_first_deploy: true,
            required_steps: vec!["template".to_string(), "apps_create".to_string()],
        });
        result.exit_code = EXIT_NOT_FOUND;

        let gate = validate_deploy_prep_quality(&result);
        assert!(
            !gate
                .violations
                .iter()
                .any(|v| matches!(v, QualityViolation::ExitCodeMismatch { .. })),
            "first deploy must not trip ExitCodeMismatch: {:?}",
            gate.violations
        );
    }

    #[test]
    fn ambiguous_match_recomputes_usage_without_mismatch() {
        // Ambiguous match: app_id stays None but matched_apps has >1 entry, so
        // resolve returns EXIT_USAGE (64), NOT 67. recompute must keep that 64
        // distinct from the first-deploy NOT_FOUND path so the gate stays quiet
        // on a legitimately ambiguous resolve.
        let mut result = happy_result();
        result.resolve.app_id = None;
        result.resolve.matched_apps = vec![
            crate::resolve::AppMatch {
                id: "1".into(),
                slug: "dup".into(),
            },
            crate::resolve::AppMatch {
                id: "2".into(),
                slug: "dup".into(),
            },
        ];
        result.resolve.error = Some("app_ambiguous".to_string());
        result.bootstrap_plan = Some(crate::deploy_prep::BootstrapPlan {
            is_first_deploy: true,
            required_steps: vec!["template".to_string()],
        });
        result.exit_code = 64;

        let gate = validate_deploy_prep_quality(&result);
        assert!(
            !gate
                .violations
                .iter()
                .any(|v| matches!(v, QualityViolation::ExitCodeMismatch { .. })),
            "ambiguous match (64) must not trip ExitCodeMismatch: {:?}",
            gate.violations
        );
    }

    #[test]
    fn invalid_profile_flagged_when_preflight_and_resolve_disagree() {
        let mut result = happy_result();
        result.preflight.profile = Some("prod".to_string());
        result.resolve.profile = Some("staging".to_string());
        let gate = validate_deploy_prep_quality(&result);
        let bad = gate
            .violations
            .iter()
            .find_map(|v| match v {
                QualityViolation::InvalidProfile { preflight, resolve } => {
                    Some((preflight.clone(), resolve.clone()))
                }
                _ => None,
            })
            .expect("InvalidProfile missing");
        assert_eq!(bad, ("prod".to_string(), "staging".to_string()));
    }

    #[test]
    fn invalid_profile_skipped_when_either_side_is_none() {
        // We only enforce profile equality when BOTH sides claim a profile.
        // Either side being absent is "preflight cleared the field" — that's
        // not a mismatch, that's missing data.
        let mut result = happy_result();
        result.resolve.profile = None;
        let gate = validate_deploy_prep_quality(&result);
        assert!(!gate
            .violations
            .iter()
            .any(|v| matches!(v, QualityViolation::InvalidProfile { .. })));
    }

    #[test]
    fn auth_mismatch_flagged_when_unauthenticated_but_exit_ok() {
        let mut result = happy_result();
        result.preflight.auth_ok = false;
        // exit_code = 0 means the composer let an unauthenticated push
        // through somehow. recompute_exit_code catches the same issue and
        // returns 65, so we get both AuthMismatch and ExitCodeMismatch.
        let gate = validate_deploy_prep_quality(&result);
        assert!(gate
            .violations
            .iter()
            .any(|v| matches!(v, QualityViolation::AuthMismatch)));
    }

    #[test]
    fn auth_mismatch_silent_when_exit_already_reflects_auth_failure() {
        let mut result = happy_result();
        result.preflight.auth_ok = false;
        result.exit_code = EXIT_AUTH;
        let gate = validate_deploy_prep_quality(&result);
        assert!(!gate
            .violations
            .iter()
            .any(|v| matches!(v, QualityViolation::AuthMismatch)));
    }

    #[test]
    fn sub_key_constant_matches_catalog_namespace() {
        assert_eq!(
            QualityCheckResult::SUB_KEY,
            "64:validation.quality_gate_failed"
        );
    }
}
