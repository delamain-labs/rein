use crate::ast::{CompareOp, EvalFailureAction};
use crate::parser::parse;

#[test]
fn eval_block_with_dataset_and_assertion() {
    let f = parse(
        r#"
        eval {
            dataset: "./evals/data.yaml"
            assert accuracy > 90%
        }
    "#,
    )
    .unwrap();
    assert_eq!(f.evals.len(), 1);
    let eval = &f.evals[0];
    assert_eq!(eval.dataset, "./evals/data.yaml");
    assert_eq!(eval.assertions.len(), 1);
    assert_eq!(eval.assertions[0].metric, "accuracy");
    assert_eq!(eval.assertions[0].op, CompareOp::Gt);
    assert_eq!(eval.assertions[0].value, "90%");
    assert!(eval.on_failure.is_none());
}

#[test]
fn eval_block_with_name_and_failure_action() {
    let f = parse(
        r#"
        eval quality_check {
            dataset: "./evals/data.yaml"
            assert accuracy > 90%
            on failure: block deploy
        }
    "#,
    )
    .unwrap();
    let eval = &f.evals[0];
    assert_eq!(eval.name.as_deref(), Some("quality_check"));
    assert!(matches!(
        eval.on_failure,
        Some(EvalFailureAction::Block { ref target }) if target == "deploy"
    ));
}

#[test]
fn eval_block_multiple_assertions() {
    let f = parse(
        r#"
        eval {
            dataset: "./evals/test.yaml"
            assert accuracy > 90%
            assert latency < 500
            on failure: escalate
        }
    "#,
    )
    .unwrap();
    let eval = &f.evals[0];
    assert_eq!(eval.assertions.len(), 2);
    assert_eq!(eval.assertions[1].metric, "latency");
    assert_eq!(eval.assertions[1].op, CompareOp::Lt);
    assert_eq!(eval.assertions[1].value, "500");
    assert!(matches!(eval.on_failure, Some(EvalFailureAction::Escalate)));
}

#[test]
fn eval_block_path_without_quotes() {
    let f = parse(
        r#"
        eval {
            dataset: ./evals/data.yaml
            assert accuracy > 95%
        }
    "#,
    )
    .unwrap();
    assert_eq!(f.evals[0].dataset, "./evals/data.yaml");
}
