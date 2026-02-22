use crate::parser::parse;

#[test]
fn step_for_each() {
    let f = parse(
        r#"
        agent copywriter { model: "gpt-4o" }
        workflow w {
            trigger: event
            step rewrite {
                agent: copywriter
                for each: underperformers
            }
        }
    "#,
    )
    .unwrap();
    let step = &f.workflows[0].steps[0];
    assert_eq!(step.for_each.as_deref(), Some("underperformers"));
}

#[test]
fn step_typed_output() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step x {
                agent: a
                output: items: Product[]
            }
        }
    "#,
    )
    .unwrap();
    let step = &f.workflows[0].steps[0];
    assert_eq!(step.typed_outputs.len(), 1);
    assert_eq!(step.typed_outputs[0].0, "items");
    assert!(matches!(
        &step.typed_outputs[0].1,
        crate::ast::TypeExpr::Named { name, array: true } if name == "Product"
    ));
}

#[test]
fn step_typed_input() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step y {
                agent: a
                input: items
            }
        }
    "#,
    )
    .unwrap();
    let step = &f.workflows[0].steps[0];
    assert_eq!(step.typed_input.as_deref(), Some("items"));
}

#[test]
fn step_escalate_to_human() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step review {
                agent: a
                escalate to human via slack("refunds")
            }
        }
    "#,
    )
    .unwrap();
    let step = &f.workflows[0].steps[0];
    let esc = step.escalate.as_ref().unwrap();
    assert_eq!(esc.target, "human");
    assert_eq!(esc.channel, "slack");
    assert_eq!(esc.destination, "refunds");
}
