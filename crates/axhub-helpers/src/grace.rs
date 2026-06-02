//! Once-per-project migration **grace warning** (spec 006 §43, AC 11).
//!
//! When an axhub-authed user types an *implicit* deploy request ("배포해") in a
//! repo with **no** `axhub.yaml` marker, the shared routing decision is
//! [`RoutingDecision::Ignore`] (rule `e`) and axhub yields the repo. A user who
//! previously leaned on that implicit nudge would otherwise lose it silently —
//! a backward-compat regression surface (spec §86). So the prompt-route hook
//! emits a **one-time** `systemMessage` that explains the new contract and names
//! the two explicit paths (`/init` to drop a marker, or say "axhub 배포").
//!
//! Bounded to a single exposure **per project** (spec §43 "이중 노출 정책":
//! grace educates *once*; the deploy preflight disambiguation blocks *every*
//! time — so after the first prompt only the preflight remains). The once-only
//! guarantee is a state-file marker, mirroring
//! [`crate::runtime_paths::welcome_marker_path`] (the SessionStart welcome
//! "megaskill" pattern the spec footnote points at).
//!
//! ## What this is NOT — a coarse nudge gate, not routing
//!
//! [`is_deploy_intent`] is a **coarse educational gate**, never a routing input.
//! The routing decision is already [`RoutingDecision::Ignore`] before this module
//! runs (see [`should_show_grace`]); this only decides whether to *educate* on
//! that decision. The asymmetry is deliberately exploited:
//!
//! - **under-match** (a deploy phrase we don't list) → the one-time nudge is
//!   skipped, but the deploy preflight still disambiguates → harmless.
//! - **over-match** (a non-deploy prompt we wrongly match) → a spurious nudge →
//!   the actual harm.
//!
//! So the phrase set is kept small and conservative. It is intentionally **not**
//! the SKILL-description trigger-phrase set (that one drives Claude's skill
//! selection and is `lint:keywords` baseline-locked; this one drives nothing and
//! has no baseline coupling). Keeping them separate is what lets this exist
//! without resurrecting the deleted routing keyword chain.

use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::routing::RoutingDecision;
use crate::runtime_paths;

/// The once-per-project migration nudge, user-facing 해요체 (`lint:tone`).
/// Names both explicit recovery paths from spec §43: `/init` (drop an
/// `axhub.yaml` marker) or the literal `"axhub"` keyword.
pub const GRACE_MESSAGE: &str = "이 프로젝트엔 axhub.yaml 이 없어서 \"배포해\" 같은 \
무명시 요청으로는 axhub 자동 배포를 더 안 해요. axhub 로 배포하려면 `/init` 으로 \
axhub.yaml 을 만들거나 \"axhub 배포\" 처럼 명시해 주세요. (이 안내는 프로젝트당 한 번만 나와요.)";

/// Korean deploy-intent stems (substring match). Korean is agglutinative, so a
/// stem like "배포" covers "배포해 / 배포해줘 / 배포하자" without word boundaries.
/// Conservative core verbs only — see the module doc on why the set is small.
const DEPLOY_INTENT_KO_STEMS: &[&str] = &["배포", "출시", "런칭", "내보내"];

/// English deploy-intent words (whole-word match, so "ship" does not fire inside
/// "relationship" nor "launch" inside "relaunch" — over-match is the real harm).
const DEPLOY_INTENT_EN_WORDS: &[&str] = &["deploy", "ship", "launch", "release", "rollout"];

/// Defensive walk-up cap, matching `routing::MAX_WALK_UP_DEPTH`.
const MAX_WALK_UP_DEPTH: usize = 64;

/// Pure predicate: should the migration grace nudge be shown for this prompt?
///
/// Fires iff the shared decision is [`RoutingDecision::Ignore`] **and** the user
/// is axhub-authed **and** the prompt reads as a deploy request. No I/O — the
/// once-per-project persistence is a separate concern ([`try_consume_once`]).
///
/// Note the decision already encodes the marker/modality conditions the AC
/// names: `decide_from_flags` only yields `Ignore` for a bare prompt (no
/// `axhub`/foreign keyword, no slash), and for an *authed* user a marker *error*
/// falls open to `Axhub` — so `Ignore && authed` implies a **confirmed-absent**
/// marker plus an implicit prompt. The only fresh signal is deploy-intent, hence
/// this gate consumes the shared decision rather than re-deriving marker state
/// (composition-consistency: one decision, no parallel chain).
#[must_use]
pub fn should_show_grace(decision: RoutingDecision, authed: bool, prompt: &str) -> bool {
    decision == RoutingDecision::Ignore && authed && is_deploy_intent(prompt)
}

/// Coarse deploy-intent detector for the grace gate (see module doc — NOT
/// routing, under-match acceptable).
#[must_use]
pub fn is_deploy_intent(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    DEPLOY_INTENT_KO_STEMS
        .iter()
        .any(|stem| lower.contains(stem))
        || DEPLOY_INTENT_EN_WORDS
            .iter()
            .any(|word| contains_ascii_word(&lower, word))
}

/// Whole-word ASCII match. `word` must be lowercase ASCII; `haystack` must
/// already be lowercased by the caller. A hit must be bounded on both sides by a
/// non-ASCII-alphanumeric byte (or a string edge). Mirrors the bounded-match
/// contract of `routing::contains_word` (duplicated locally so this module never
/// has to edit the locked shared routing source).
fn contains_ascii_word(haystack: &str, word: &str) -> bool {
    let w = word.as_bytes();
    let hay = haystack.as_bytes();
    if w.is_empty() || hay.len() < w.len() {
        return false;
    }
    let mut i = 0;
    while i + w.len() <= hay.len() {
        if &hay[i..i + w.len()] == w {
            let before_ok = i == 0 || !hay[i - 1].is_ascii_alphanumeric();
            let after = i + w.len();
            let after_ok = after == hay.len() || !hay[after].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// The project root used to key the once-per-project marker: the nearest
/// ancestor of `start` containing a `.git` directory, falling back to `start`
/// itself for non-git trees. Keying by git-root (not cwd) means prompts from any
/// subdirectory of the project share the single once-flag.
#[must_use]
pub fn find_project_root_from(start: &Path) -> PathBuf {
    for dir in start.ancestors().take(MAX_WALK_UP_DEPTH) {
        if dir.join(".git").try_exists().unwrap_or(false) {
            return dir.to_path_buf();
        }
    }
    start.to_path_buf()
}

/// [`find_project_root_from`] anchored at the current working directory.
/// Returns `None` if the cwd cannot be read.
#[must_use]
pub fn find_project_root() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|cwd| find_project_root_from(&cwd))
}

/// Per-project marker filename, e.g. `.grace-<16hex>-shown`. The hash of the
/// project-root path keeps the name short and filesystem-safe regardless of how
/// deep or unusual the path is.
fn grace_marker_filename(project_root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!(".grace-{hex}-shown")
}

/// Full path to the once-per-project grace marker under the helper state dir.
/// `None` if no state dir can be resolved.
#[must_use]
pub fn grace_marker_path(project_root: &Path) -> Option<PathBuf> {
    runtime_paths::state_dir().map(|dir| dir.join(grace_marker_filename(project_root)))
}

/// Atomically claim the one-time grace slot for `project_root` **inside an
/// explicit state dir**. Returns `true` exactly once (the call that creates the
/// marker). `create_new` makes "first writer wins" atomic, so a concurrent
/// second hook still sees `AlreadyExists` → `false`. Any failure (cannot create
/// the dir, write error, already claimed) returns `false`, so the nudge shows at
/// most once and **never repeats**. Best-effort — callers must not let this
/// affect the hook exit code. Split out from [`try_consume_once`] so tests can
/// drive it with a tempdir and no process-global env mutation.
#[must_use]
pub fn try_consume_once_in(state_dir: &Path, project_root: &Path) -> bool {
    if fs::create_dir_all(state_dir).is_err() {
        return false;
    }
    let path = state_dir.join(grace_marker_filename(project_root));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .is_ok()
}

/// [`try_consume_once_in`] against the resolved helper state dir. `false` if no
/// state dir is available (then the nudge is skipped rather than risk repeating).
#[must_use]
pub fn try_consume_once(project_root: &Path) -> bool {
    match runtime_paths::state_dir() {
        Some(dir) => try_consume_once_in(&dir, project_root),
        None => false,
    }
}

/// I/O orchestrator consumed by the prompt-route hook: returns [`GRACE_MESSAGE`]
/// exactly once per project when [`should_show_grace`] holds, else `None`. The
/// side-effecting [`try_consume_once`] runs **only after** the pure predicate
/// passes, so a non-deploy / unauthed / non-`Ignore` prompt never writes a marker
/// (it would otherwise burn the once-slot without ever showing the nudge).
#[must_use]
pub fn maybe_grace_message(
    decision: RoutingDecision,
    authed: bool,
    prompt: &str,
) -> Option<&'static str> {
    if !should_show_grace(decision, authed, prompt) {
        return None;
    }
    let root = find_project_root()?;
    if try_consume_once(&root) {
        Some(GRACE_MESSAGE)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_intent_matches_core_verbs() {
        // Korean stems (the canonical AC prompt + variants).
        assert!(is_deploy_intent("배포해"));
        assert!(is_deploy_intent("지금 배포해줘"));
        assert!(is_deploy_intent("이거 출시하자"));
        assert!(is_deploy_intent("프로덕션에 내보내자"));
        // English whole words.
        assert!(is_deploy_intent("deploy this"));
        assert!(is_deploy_intent("ship it now"));
        assert!(is_deploy_intent("please LAUNCH"));
    }

    #[test]
    fn deploy_intent_rejects_non_deploy_and_substrings() {
        assert!(!is_deploy_intent("안녕하세요"));
        assert!(!is_deploy_intent("오늘 날씨 어때"));
        // English whole-word boundary: no fire inside larger words.
        assert!(!is_deploy_intent("relationship 분석")); // contains "ship"
        assert!(!is_deploy_intent("relaunch later")); // contains "launch"
        assert!(!is_deploy_intent("just a normal prompt"));
    }

    #[test]
    fn should_show_grace_requires_all_three_conditions() {
        // The firing row: Ignore + authed + deploy-intent.
        assert!(should_show_grace(RoutingDecision::Ignore, true, "배포해"));

        // Each condition is load-bearing — drop exactly one, no nudge.
        assert!(
            !should_show_grace(RoutingDecision::Ignore, false, "배포해"),
            "unauthed must not nudge (zero-footprint for non-axhub users)"
        );
        assert!(
            !should_show_grace(RoutingDecision::Ignore, true, "안녕"),
            "non-deploy bare NL must not nudge"
        );
        for decision in [
            RoutingDecision::Axhub,
            RoutingDecision::Yield,
            RoutingDecision::Ask,
        ] {
            assert!(
                !should_show_grace(decision, true, "배포해"),
                "only the Ignore decision nudges (got {decision:?})"
            );
        }
    }

    #[test]
    fn project_root_is_nearest_git_ancestor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir(root.join(".git")).expect("mkdir .git");
        let nested = root.join("pkg").join("src");
        std::fs::create_dir_all(&nested).expect("mkdir nested");

        // canonicalize both sides: macOS /var → /private/var symlink.
        assert_eq!(
            find_project_root_from(&nested).canonicalize().unwrap(),
            root.canonicalize().unwrap(),
            "any subdir resolves to the same git root → shared once-flag"
        );
    }

    #[test]
    fn project_root_falls_back_to_start_without_git() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let start = tmp.path().join("a").join("b");
        std::fs::create_dir_all(&start).expect("mkdir");
        assert_eq!(find_project_root_from(&start), start);
    }

    #[test]
    fn consume_once_is_true_then_false_for_same_project() {
        let state = tempfile::tempdir().expect("state dir");
        let project = tempfile::tempdir().expect("project dir");

        assert!(
            try_consume_once_in(state.path(), project.path()),
            "first claim must succeed"
        );
        assert!(
            !try_consume_once_in(state.path(), project.path()),
            "second claim for the same project must NOT re-fire (once-per-project)"
        );
    }

    #[test]
    fn consume_once_is_independent_across_projects() {
        let state = tempfile::tempdir().expect("state dir");
        let project_a = tempfile::tempdir().expect("project a");
        let project_b = tempfile::tempdir().expect("project b");

        assert!(try_consume_once_in(state.path(), project_a.path()));
        // A different project root has its own once-slot.
        assert!(
            try_consume_once_in(state.path(), project_b.path()),
            "a distinct project must get its own grace slot"
        );
    }

    #[test]
    fn marker_filename_is_stable_and_path_keyed() {
        let a = std::path::Path::new("/home/u/proj-a");
        let b = std::path::Path::new("/home/u/proj-b");
        assert_eq!(grace_marker_filename(a), grace_marker_filename(a));
        assert_ne!(grace_marker_filename(a), grace_marker_filename(b));
        assert!(grace_marker_filename(a).starts_with(".grace-"));
        assert!(grace_marker_filename(a).ends_with("-shown"));
    }

    #[test]
    fn grace_message_is_haeyo_and_names_recovery_paths() {
        // lint:tone 금지 토큰 전부 부재 (해요체). `lint:tone` 은 .md 만 스캔하므로
        // (.rs 미포함) 이 단위 테스트가 grace 메시지 톤의 유일한 가드 — 6개 전부 검사.
        for banned in ["합니다", "입니다", "시겠어요", "드립니다", "당신", "아이고"] {
            assert!(
                !GRACE_MESSAGE.contains(banned),
                "grace message must stay 해요체 (found {banned})"
            );
        }
        // Both explicit recovery paths from spec §43 are named.
        assert!(GRACE_MESSAGE.contains("/init"));
        assert!(GRACE_MESSAGE.contains("axhub 배포"));
    }
}
