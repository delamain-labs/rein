use crate::ast::{ConsensusStrategy};
use crate::parser::parse;

#[test]
fn consensus_majority() {
    let f = parse(
        r#"
        consensus refund_review {
            agents: [reviewer_a, reviewer_b, reviewer_c]
            strategy: majority
            require: 2 of 3 agree
        }
    "#,
    )
    .unwrap();
    let c = &f.consensus_blocks[0];
    assert_eq!(c.name, "refund_review");
    assert_eq!(c.agents, vec!["reviewer_a", "reviewer_b", "reviewer_c"]);
    assert_eq!(c.strategy, ConsensusStrategy::Majority);
    let req = c.require.as_ref().unwrap();
    assert_eq!(req.required, 2);
    assert_eq!(req.total, 3);
}

#[test]
fn consensus_unanimous_no_require() {
    let f = parse(
        r#"
        consensus approval {
            agents: [a, b]
            strategy: unanimous
        }
    "#,
    )
    .unwrap();
    let c = &f.consensus_blocks[0];
    assert_eq!(c.strategy, ConsensusStrategy::Unanimous);
    assert!(c.require.is_none());
}
