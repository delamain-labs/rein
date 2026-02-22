use super::*;

#[test]
fn valid_file_no_diagnostics() {
    let text = r#"agent test { model: openai }"#;
    let diags = compute_diagnostics(text);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_produces_diagnostic() {
    let text = r#"agent { }"#;
    let diags = compute_diagnostics(text);
    assert!(!diags.is_empty());
    assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
}

#[test]
fn offset_to_line_col_first_line() {
    let text = "agent test { model: openai }";
    let (line, col) = offset_to_line_col(text, 6);
    assert_eq!(line, 0);
    assert_eq!(col, 6);
}

#[test]
fn offset_to_line_col_second_line() {
    let text = "line one\nline two";
    let (line, col) = offset_to_line_col(text, 10);
    assert_eq!(line, 1);
    assert_eq!(col, 1);
}

#[test]
fn word_at_position_extracts_keyword() {
    let text = "agent test { model: openai }";
    let word = word_at_position(text, Position::new(0, 2));
    assert_eq!(word, "agent");
}

#[test]
fn word_at_position_extracts_ident() {
    let text = "agent my_agent { model: openai }";
    let word = word_at_position(text, Position::new(0, 8));
    assert_eq!(word, "my_agent");
}

#[test]
fn keyword_docs_returns_agent() {
    let docs = keyword_docs("agent");
    assert!(docs.is_some());
    assert!(docs.unwrap().contains("agent"));
}

#[test]
fn keyword_docs_returns_none_for_unknown() {
    assert!(keyword_docs("foobar").is_none());
}

#[test]
fn completions_include_agent() {
    let has_agent = KEYWORD_COMPLETIONS.iter().any(|(k, _)| *k == "agent");
    assert!(has_agent);
}

#[test]
fn completions_include_workflow() {
    let has_wf = KEYWORD_COMPLETIONS.iter().any(|(k, _)| *k == "workflow");
    assert!(has_wf);
}
