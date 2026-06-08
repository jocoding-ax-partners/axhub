use axhub_helpers::bootstrap::BootstrapState;

fn state_is_terminal_stop(state: BootstrapState) -> bool {
    matches!(
        state,
        BootstrapState::SubdomainCollision
            | BootstrapState::AlreadyDeployed
            | BootstrapState::BackendContractMissingDefaults
            | BootstrapState::IdempotencyUnavailable
    )
}

#[test]
fn matrix_pending_variants_user_decision_and_terminal_stop() {
    let cases: Vec<(BootstrapState, bool, bool)> = vec![
        (BootstrapState::TemplateRequired, true, false),
        (BootstrapState::ConflictExistingFiles, true, false),
        (BootstrapState::GitInitRequired, true, false),
        (BootstrapState::FirstCommitRequired, true, false),
        (BootstrapState::SubdomainCollision, true, true),
        (BootstrapState::AlreadyDeployed, true, true),
        (BootstrapState::AppsCreatePending, true, false),
        (BootstrapState::DeployCreatePending, true, false),
        (BootstrapState::BackendContractMissingDefaults, true, true),
        (BootstrapState::IdempotencyUnavailable, true, true),
        (BootstrapState::AppRegistered, false, false),
        (BootstrapState::Deploying, false, false),
        (BootstrapState::Deployed, false, false),
        (BootstrapState::DependencyInstallRequired, true, false),
    ];

    for (state, expect_user_decision, expect_terminal) in cases {
        assert_eq!(
            state.is_user_decision(),
            expect_user_decision,
            "is_user_decision({}) expected {}",
            state.as_str(),
            expect_user_decision
        );
        assert_eq!(
            state_is_terminal_stop(state),
            expect_terminal,
            "state_is_terminal_stop({}) expected {}",
            state.as_str(),
            expect_terminal
        );
    }
}

#[test]
fn dependency_install_required_serializes_snake_case() {
    assert_eq!(
        BootstrapState::DependencyInstallRequired.as_str(),
        "dependency_install_required"
    );
    let raw = serde_json::to_string(&BootstrapState::DependencyInstallRequired).unwrap();
    assert_eq!(raw, "\"dependency_install_required\"");
}
