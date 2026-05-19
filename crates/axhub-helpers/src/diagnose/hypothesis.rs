//! Phase 2R hypothesis generator — plan v6 §4.3.
//!
//! Generates 3-5 ranked, falsifiable hypotheses for a given failure signal.
//! Each hypothesis carries an If-X-then-Y prediction so wrong fixes can be
//! ruled out cheaply. v0.8.0 ships catalog + template generators; LLM-augmented
//! source ships in v0.8.1 once cost policy is settled.
//!
//! Plan v6 §1.2 — `cause` is internal jargon, `user_facing_explanation` is the
//! vibe-coder-friendly translation. UX layer presents the latter; logs keep
//! the former for engineers.

use serde::{Deserialize, Serialize};

use super::loop_builder::LoopStrategy;
use super::signal::Signal;

/// One falsifiable hypothesis. Order in a `Vec<Hypothesis>` is the rank.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Stable ID for audit ledger correlation.
    pub id: String,
    /// Engineer-facing cause statement (jargon OK).
    pub cause: String,
    /// Vibe-coder-facing 1-line explanation (no jargon). Required.
    pub user_facing_explanation: String,
    /// If-X-then-Y prediction. Sentence must finish "...will make the error
    /// disappear" or "...will reproduce the failure faster."
    pub prediction: String,
    /// Falsifier — what would prove this hypothesis WRONG.
    pub falsifier: String,
    /// Confidence 0.0–1.0 from the generator. Caller may override via
    /// user re-rank.
    pub confidence: f32,
    /// Source: catalog / template / llm.
    pub source: HypothesisSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisSource {
    Catalog,
    Template,
    Llm,
}

/// Input context the generator uses to rank hypotheses.
#[derive(Debug, Clone)]
pub struct HypothesisContext<'a> {
    pub strategy: LoopStrategy,
    pub signal: &'a Signal,
    /// First N stderr / log lines after the failure surfaced. MUST be redacted
    /// before being passed in.
    pub head_evidence: Option<&'a str>,
}

/// Generate up to 5 hypotheses. v0.8.0 skeleton emits 1 template-based
/// placeholder so the orchestrator has at least one falsifiable candidate
/// before the catalog + LLM sources land.
pub fn generate(ctx: &HypothesisContext<'_>) -> Vec<Hypothesis> {
    let mut out = Vec::new();

    // Always emit at least one template-based generic hypothesis so Phase 2R
    // never hits NoHypothesis in v0.8.0 — orchestrator can then fall through
    // to HITL extra capture if user rejects all candidates.
    out.push(Hypothesis {
        id: format!("H-{}-template-1", ctx.strategy.as_str()),
        cause: format!(
            "tool `{}` failed at strategy `{}` with elapsed_ms={}",
            ctx.strategy.as_str(),
            ctx.signal.strategy,
            ctx.signal.elapsed_ms
        ),
        user_facing_explanation: match ctx.strategy {
            LoopStrategy::AxhubDeploy => "axhub deploy 명령이 중간에 멈췄어요. 이전 상태가 남아 있을 수 있어요.".into(),
            LoopStrategy::Test => "테스트가 실패했어요. 직전 변경 또는 의존성 차이가 원인일 수 있어요.".into(),
        },
        prediction: format!(
            "이 가설이 맞다면, `{}` 의 직전 상태를 정리한 뒤 재실행하면 에러가 사라져요.",
            ctx.strategy.as_str()
        ),
        falsifier: "정리 후 재실행에도 같은 에러가 나오면 이 가설은 기각이에요.".into(),
        confidence: 0.4,
        source: HypothesisSource::Template,
    });

    out
}

/// Re-rank by user feedback. Moves `selected_idx` to front and decays others.
pub fn rerank(mut hypotheses: Vec<Hypothesis>, selected_idx: usize) -> Vec<Hypothesis> {
    if selected_idx == 0 || selected_idx >= hypotheses.len() {
        return hypotheses;
    }
    let picked = hypotheses.remove(selected_idx);
    hypotheses.insert(0, picked);
    hypotheses
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn red_signal() -> Signal {
        Signal::red(
            Duration::from_millis(1500),
            "cli-replay",
            Some("EACCES".into()),
            Some(13),
        )
    }

    #[test]
    fn generate_emits_at_least_one_hypothesis_per_strategy() {
        for strat in [LoopStrategy::AxhubDeploy, LoopStrategy::Test] {
            let sig = red_signal();
            let ctx = HypothesisContext {
                strategy: strat,
                signal: &sig,
                head_evidence: None,
            };
            let hs = generate(&ctx);
            assert!(
                !hs.is_empty(),
                "every strategy must yield ≥1 hypothesis (got 0 for {strat:?})"
            );
            for h in &hs {
                assert!(!h.cause.is_empty());
                assert!(
                    !h.user_facing_explanation.is_empty(),
                    "user_facing_explanation must always be present (plan v6 §1.2 critical UX gap)"
                );
                assert!(!h.prediction.is_empty(), "If-X-then-Y prediction required");
                assert!(!h.falsifier.is_empty(), "falsifier required");
                assert!(h.confidence >= 0.0 && h.confidence <= 1.0);
            }
        }
    }

    #[test]
    fn template_hypothesis_uses_korean_haeyo_register() {
        let sig = red_signal();
        let ctx = HypothesisContext {
            strategy: LoopStrategy::Test,
            signal: &sig,
            head_evidence: None,
        };
        let hs = generate(&ctx);
        let h = &hs[0];
        // Plan v6 §12 — UX text must be 해요체. Probe the suffix.
        assert!(
            h.user_facing_explanation.contains("요")
                || h.user_facing_explanation.contains("에요"),
            "expected 해요체 ending in user_facing_explanation: {}",
            h.user_facing_explanation
        );
    }

    #[test]
    fn rerank_moves_selected_to_front() {
        let sig = red_signal();
        let ctx = HypothesisContext {
            strategy: LoopStrategy::Test,
            signal: &sig,
            head_evidence: None,
        };
        let mut hs = generate(&ctx);
        // Stuff in two additional placeholder hypotheses to exercise the move.
        hs.push(Hypothesis {
            id: "H-extra-1".into(),
            cause: "c1".into(),
            user_facing_explanation: "u1요".into(),
            prediction: "p1".into(),
            falsifier: "f1".into(),
            confidence: 0.2,
            source: HypothesisSource::Template,
        });
        hs.push(Hypothesis {
            id: "H-extra-2".into(),
            cause: "c2".into(),
            user_facing_explanation: "u2요".into(),
            prediction: "p2".into(),
            falsifier: "f2".into(),
            confidence: 0.3,
            source: HypothesisSource::Template,
        });
        let reranked = rerank(hs.clone(), 2);
        assert_eq!(reranked[0].id, "H-extra-2");
        assert_eq!(reranked.len(), hs.len());
    }

    #[test]
    fn rerank_zero_is_noop() {
        let sig = red_signal();
        let ctx = HypothesisContext {
            strategy: LoopStrategy::Test,
            signal: &sig,
            head_evidence: None,
        };
        let hs = generate(&ctx);
        let same = rerank(hs.clone(), 0);
        assert_eq!(same, hs);
    }

    #[test]
    fn serde_roundtrip() {
        let h = Hypothesis {
            id: "H-r1".into(),
            cause: "c".into(),
            user_facing_explanation: "u요".into(),
            prediction: "p".into(),
            falsifier: "f".into(),
            confidence: 0.7,
            source: HypothesisSource::Catalog,
        };
        let s = serde_json::to_string(&h).unwrap();
        let back: Hypothesis = serde_json::from_str(&s).unwrap();
        assert_eq!(h, back);
    }
}
