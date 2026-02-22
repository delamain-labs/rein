use crate::parser::parse;

#[test]
fn scenario_basic() {
    let f = parse(
        r#"
        scenario angry_customer {
            given {
                sentiment: "angry"
                category: "billing"
            }
            expect {
                action: "refund"
                channel: "zendesk"
            }
        }
    "#,
    )
    .unwrap();
    let s = &f.scenarios[0];
    assert_eq!(s.name, "angry_customer");
    assert_eq!(s.given.len(), 2);
    assert_eq!(s.given[0], ("sentiment".to_string(), "angry".to_string()));
    assert_eq!(s.expect.len(), 2);
    assert_eq!(s.expect[0], ("action".to_string(), "refund".to_string()));
}

#[test]
fn scenario_ident_values() {
    let f = parse(
        r#"
        scenario quick_reply {
            given {
                priority: low
            }
            expect {
                action: auto_respond
            }
        }
    "#,
    )
    .unwrap();
    let s = &f.scenarios[0];
    assert_eq!(s.given[0], ("priority".to_string(), "low".to_string()));
    assert_eq!(
        s.expect[0],
        ("action".to_string(), "auto_respond".to_string())
    );
}
