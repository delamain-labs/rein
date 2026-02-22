use super::*;
use crate::ast::{ConsensusDef, ConsensusStrategy, Span};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn make_def(agents: u32, strategy: ConsensusStrategy) -> ConsensusDef {
    ConsensusDef {
        name: "test".to_string(),
        agents: (0..agents).map(|i| format!("agent_{i}")).collect(),
        strategy,
        require: None,
        span: span(),
    }
}

#[test]
fn unanimous_requires_all() {
    let def = make_def(3, ConsensusStrategy::Unanimous);
    let eval = ConsensusEvaluator::from_def(&def);
    assert_eq!(eval.required_count(), 3);

    let votes = vec![Vote::Agree, Vote::Agree, Vote::Agree];
    assert_eq!(
        eval.evaluate(&votes),
        ConsensusOutcome::Reached {
            agreed: 3,
            total: 3
        }
    );
}

#[test]
fn unanimous_fails_with_one_disagree() {
    let def = make_def(3, ConsensusStrategy::Unanimous);
    let eval = ConsensusEvaluator::from_def(&def);

    let votes = vec![
        Vote::Agree,
        Vote::Agree,
        Vote::Disagree {
            reason: "nope".into(),
        },
    ];
    assert_eq!(
        eval.evaluate(&votes),
        ConsensusOutcome::Failed {
            agreed: 2,
            required: 3,
            total: 3
        }
    );
}

#[test]
fn majority_requires_over_half() {
    let def = make_def(3, ConsensusStrategy::Majority);
    let eval = ConsensusEvaluator::from_def(&def);
    assert_eq!(eval.required_count(), 2);

    let votes = vec![
        Vote::Agree,
        Vote::Agree,
        Vote::Disagree {
            reason: "no".into(),
        },
    ];
    assert_eq!(
        eval.evaluate(&votes),
        ConsensusOutcome::Reached {
            agreed: 2,
            total: 3
        }
    );
}

#[test]
fn majority_fails_below_threshold() {
    let def = make_def(5, ConsensusStrategy::Majority);
    let eval = ConsensusEvaluator::from_def(&def);
    assert_eq!(eval.required_count(), 3);

    let votes = vec![
        Vote::Agree,
        Vote::Agree,
        Vote::Disagree { reason: "a".into() },
        Vote::Disagree { reason: "b".into() },
        Vote::Disagree { reason: "c".into() },
    ];
    assert_eq!(
        eval.evaluate(&votes),
        ConsensusOutcome::Failed {
            agreed: 2,
            required: 3,
            total: 5
        }
    );
}

#[test]
fn explicit_requirement_overrides_strategy() {
    let mut def = make_def(5, ConsensusStrategy::Unanimous);
    def.require = Some(crate::ast::ConsensusRequirement {
        required: 2,
        total: 5,
    });
    let eval = ConsensusEvaluator::from_def(&def);
    assert_eq!(eval.required_count(), 2);
}
