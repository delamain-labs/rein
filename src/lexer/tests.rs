use super::*;

fn kinds(tokens: &[Token]) -> Vec<&TokenKind> {
    tokens.iter().map(|t| &t.kind).collect()
}

fn lex_ok(src: &str) -> Vec<Token> {
    tokenize(src).expect("lex should succeed")
}

fn non_eof(tokens: Vec<Token>) -> Vec<Token> {
    tokens
        .into_iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .collect()
}

// ── parse_cents unit tests ────────────────────────────────────────────────

#[test]
fn parse_cents_whole_number() {
    assert_eq!(parse_cents("50").unwrap(), 5000);
}

#[test]
fn parse_cents_fractional() {
    assert_eq!(parse_cents("0.03").unwrap(), 3);
}

#[test]
fn parse_cents_one_decimal_place() {
    assert_eq!(parse_cents("1.5").unwrap(), 150);
}

#[test]
fn parse_cents_truncates_sub_cent() {
    assert_eq!(parse_cents("0.005").unwrap(), 0);
}

#[test]
fn parse_cents_dollar_fifty() {
    assert_eq!(parse_cents("0.50").unwrap(), 50);
}

// ── Happy-path tests ──────────────────────────────────────────────────────

#[test]
fn tokenize_agent_header() {
    let tokens = non_eof(lex_ok("agent foo {"));
    assert_eq!(
        kinds(&tokens),
        vec![
            &TokenKind::Agent,
            &TokenKind::Ident("foo".into()),
            &TokenKind::LBrace
        ]
    );
}

#[test]
fn tokenize_dollar_amount() {
    let tokens = non_eof(lex_ok("$0.03"));
    assert_eq!(
        kinds(&tokens),
        vec![&TokenKind::Currency {
            symbol: '$',
            amount: 3
        }]
    );
}

#[test]
fn tokenize_dollar_integer() {
    let tokens = non_eof(lex_ok("$50"));
    assert_eq!(
        kinds(&tokens),
        vec![&TokenKind::Currency {
            symbol: '$',
            amount: 5000
        }]
    );
}

#[test]
fn tokenize_dotted_capability() {
    let tokens = non_eof(lex_ok("zendesk.read_ticket"));
    assert_eq!(
        kinds(&tokens),
        vec![
            &TokenKind::Ident("zendesk".into()),
            &TokenKind::Dot,
            &TokenKind::Ident("read_ticket".into()),
        ]
    );
}

#[test]
fn tokenize_up_to_constraint() {
    let tokens = non_eof(lex_ok("up to $50"));
    assert_eq!(
        kinds(&tokens),
        vec![
            &TokenKind::Up,
            &TokenKind::To,
            &TokenKind::Currency {
                symbol: '$',
                amount: 5000
            }
        ]
    );
}

#[test]
fn tokenize_line_comment() {
    let tokens = non_eof(lex_ok("// this is a comment"));
    assert_eq!(kinds(&tokens), vec![&TokenKind::Comment]);
}

#[test]
fn tokenize_block_comment() {
    let tokens = non_eof(lex_ok("/* block comment */"));
    assert_eq!(kinds(&tokens), vec![&TokenKind::Comment]);
}

#[test]
fn tokenize_all_keywords() {
    let src = "agent can cannot model budget per up to";
    let tokens = non_eof(lex_ok(src));
    assert_eq!(
        kinds(&tokens),
        vec![
            &TokenKind::Agent,
            &TokenKind::Can,
            &TokenKind::Cannot,
            &TokenKind::Model,
            &TokenKind::Budget,
            &TokenKind::Per,
            &TokenKind::Up,
            &TokenKind::To,
        ]
    );
}

#[test]
fn tokenize_symbols() {
    let tokens = non_eof(lex_ok("{ } [ ] : ."));
    assert_eq!(
        kinds(&tokens),
        vec![
            &TokenKind::LBrace,
            &TokenKind::RBrace,
            &TokenKind::LBracket,
            &TokenKind::RBracket,
            &TokenKind::Colon,
            &TokenKind::Dot,
        ]
    );
}

#[test]
fn tokenize_full_agent_snippet() {
    let src = r#"
agent support_triage {
model: anthropic
can [
    zendesk.read_ticket
]
budget: $0.03 per ticket
}"#;
    let tokens = non_eof(lex_ok(src));
    // Spot-check key tokens exist
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Agent));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Model));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Can));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Budget));
    assert!(tokens.iter().any(|t| t.kind
        == TokenKind::Currency {
            symbol: '$',
            amount: 3
        }));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Per));
}

#[test]
fn span_is_correct_for_ident() {
    let src = "agent";
    let tokens = lex_ok(src);
    let agent_tok = tokens.iter().find(|t| t.kind == TokenKind::Agent).unwrap();
    assert_eq!(agent_tok.span, Span::new(0, 5));
}

#[test]
fn eof_is_always_last() {
    let tokens = lex_ok("agent foo {}");
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

// ── String literal tests ──────────────────────────────────────────────────

#[test]
fn tokenize_string_literal_simple() {
    let tokens = non_eof(lex_ok(r#""anthropic""#));
    assert_eq!(
        kinds(&tokens),
        vec![&TokenKind::StringLiteral("anthropic".into())]
    );
}

#[test]
fn tokenize_string_literal_with_slash() {
    let tokens = non_eof(lex_ok(r#""anthropic/claude-3-sonnet""#));
    assert_eq!(
        kinds(&tokens),
        vec![&TokenKind::StringLiteral(
            "anthropic/claude-3-sonnet".into()
        )]
    );
}

#[test]
fn tokenize_empty_string_literal() {
    let tokens = non_eof(lex_ok(r#""""#));
    assert_eq!(kinds(&tokens), vec![&TokenKind::StringLiteral("".into())]);
}

#[test]
fn tokenize_string_literal_span() {
    let src = r#""hello""#;
    let tokens = lex_ok(src);
    let tok = tokens
        .iter()
        .find(|t| matches!(t.kind, TokenKind::StringLiteral(_)))
        .unwrap();
    // span covers the full `"hello"` including quotes
    assert_eq!(tok.span, Span::new(0, 7));
}

#[test]
fn error_unterminated_string_literal() {
    let err = tokenize(r#""not closed"#).unwrap_err();
    assert!(err.message.contains("unterminated"), "got: {}", err.message);
}

// ── Error-path tests ──────────────────────────────────────────────────────

#[test]
fn error_on_invalid_char() {
    let result = tokenize("agent @ foo");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains('@'));
}

#[test]
fn error_on_unterminated_block_comment() {
    let result = tokenize("/* never closed");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("unterminated"));
}

#[test]
fn error_span_points_to_bad_char() {
    let src = "foo @";
    let result = tokenize(src);
    let err = result.unwrap_err();
    // '@' is at byte offset 4
    assert_eq!(err.span.start, 4);
}

// ── Dollar / number error tests ───────────────────────────────────────────

#[test]
fn error_bare_dollar_eof() {
    let err = tokenize("$").unwrap_err();
    assert!(
        err.message.contains("expected a number"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_bare_dollar_space() {
    let err = tokenize("$ ").unwrap_err();
    assert!(
        err.message.contains("expected a number"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_dollar_alpha() {
    let err = tokenize("$abc").unwrap_err();
    assert!(err.message.contains("found 'a'"), "got: {}", err.message);
}

#[test]
fn error_dollar_leading_dot() {
    // '$.' — dot is not a digit so we should get the "expected a number" error
    let err = tokenize("$.5").unwrap_err();
    assert!(
        err.message.contains("expected a number"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_dollar_trailing_dot() {
    let err = tokenize("$1.").unwrap_err();
    assert!(
        err.message.contains("expected digit after decimal"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_dollar_multiple_dots() {
    let err = tokenize("$1.2.3").unwrap_err();
    assert!(
        err.message.contains("too many decimal points"),
        "got: {}",
        err.message
    );
}

// ── Display impl tests ────────────────────────────────────────────────────

#[test]
fn display_symbols() {
    assert_eq!(TokenKind::LBrace.to_string(), "{");
    assert_eq!(TokenKind::RBrace.to_string(), "}");
    assert_eq!(TokenKind::LBracket.to_string(), "[");
    assert_eq!(TokenKind::RBracket.to_string(), "]");
    assert_eq!(TokenKind::Colon.to_string(), ":");
    assert_eq!(TokenKind::Dot.to_string(), ".");
}

#[test]
fn display_keywords() {
    assert_eq!(TokenKind::Agent.to_string(), "agent");
    assert_eq!(TokenKind::Can.to_string(), "can");
    assert_eq!(TokenKind::Cannot.to_string(), "cannot");
    assert_eq!(TokenKind::Model.to_string(), "model");
    assert_eq!(TokenKind::Budget.to_string(), "budget");
    assert_eq!(TokenKind::Per.to_string(), "per");
    assert_eq!(TokenKind::Up.to_string(), "up");
    assert_eq!(TokenKind::To.to_string(), "to");
}

#[test]
fn display_ident() {
    assert_eq!(TokenKind::Ident("foo".into()).to_string(), "foo");
    assert_eq!(
        TokenKind::Ident("read_ticket".into()).to_string(),
        "read_ticket"
    );
}

#[test]
fn display_dollar_cents_only() {
    assert_eq!(
        TokenKind::Currency {
            symbol: '$',
            amount: 3
        }
        .to_string(),
        "$0.03"
    );
}

#[test]
fn display_dollar_whole() {
    assert_eq!(
        TokenKind::Currency {
            symbol: '$',
            amount: 5000
        }
        .to_string(),
        "$50.00"
    );
}

#[test]
fn display_dollar_mixed() {
    assert_eq!(
        TokenKind::Currency {
            symbol: '$',
            amount: 503
        }
        .to_string(),
        "$5.03"
    );
}

#[test]
fn tokenize_euro() {
    let tokens = non_eof(lex_ok("€10.50"));
    assert_eq!(
        tokens[0].kind,
        TokenKind::Currency {
            symbol: '€',
            amount: 1050
        }
    );
}

#[test]
fn tokenize_pound() {
    let tokens = non_eof(lex_ok("£5.00"));
    assert_eq!(
        tokens[0].kind,
        TokenKind::Currency {
            symbol: '£',
            amount: 500
        }
    );
}

#[test]
fn tokenize_yen() {
    let tokens = non_eof(lex_ok("¥100"));
    assert_eq!(
        tokens[0].kind,
        TokenKind::Currency {
            symbol: '¥',
            amount: 10000
        }
    );
}

#[test]
fn display_euro() {
    assert_eq!(
        TokenKind::Currency {
            symbol: '€',
            amount: 1050
        }
        .to_string(),
        "€10.50"
    );
}

#[test]
fn display_string_literal() {
    assert_eq!(
        TokenKind::StringLiteral("anthropic".into()).to_string(),
        "\"anthropic\""
    );
    assert_eq!(TokenKind::StringLiteral("".into()).to_string(), "\"\"");
}

#[test]
fn display_comment() {
    assert_eq!(TokenKind::Comment.to_string(), "comment");
}

#[test]
fn display_eof() {
    assert_eq!(TokenKind::Eof.to_string(), "end of file");
}

// ── Hash (#) line comment tests ──────────────────────────────────────────

#[test]
fn tokenize_hash_comment() {
    let tokens = non_eof(lex_ok("# this is a comment"));
    assert_eq!(kinds(&tokens), vec![&TokenKind::Comment]);
}

#[test]
fn hash_comment_inline_after_code() {
    let tokens = non_eof(lex_ok("agent foo # a comment"));
    assert_eq!(
        kinds(&tokens),
        vec![
            &TokenKind::Agent,
            &TokenKind::Ident("foo".into()),
            &TokenKind::Comment
        ]
    );
}

#[test]
fn hash_comment_mixed_with_slash_comments() {
    let src = "# hash comment\n// slash comment\n/* block */";
    let tokens = non_eof(lex_ok(src));
    assert_eq!(
        kinds(&tokens),
        vec![
            &TokenKind::Comment,
            &TokenKind::Comment,
            &TokenKind::Comment
        ]
    );
}

#[test]
fn hash_comment_empty() {
    let tokens = non_eof(lex_ok("#"));
    assert_eq!(kinds(&tokens), vec![&TokenKind::Comment]);
}

#[test]
fn hash_comment_at_start_of_file_with_code() {
    let src = "# Rein config\nagent foo { model: openai }";
    let tokens = non_eof(lex_ok(src));
    assert_eq!(tokens[0].kind, TokenKind::Comment);
    assert_eq!(tokens[1].kind, TokenKind::Agent);
}

#[test]
fn defaults_keyword() {
    let src = "defaults { model: openai }";
    let tokens = non_eof(lex_ok(src));
    assert_eq!(tokens[0].kind, TokenKind::Defaults);
}

#[test]
fn guardrails_keyword() {
    let src = "guardrails { }";
    let tokens = non_eof(lex_ok(src));
    assert_eq!(tokens[0].kind, TokenKind::Guardrails);
}

#[test]
fn tool_and_endpoint_keywords() {
    let src = "tool zendesk { endpoint: \"https://api.zendesk.com\" }";
    let tokens = non_eof(lex_ok(src));
    assert_eq!(tokens[0].kind, TokenKind::Tool);
    assert_eq!(tokens[2].kind, TokenKind::LBrace);
    assert_eq!(tokens[3].kind, TokenKind::Endpoint);
    assert_eq!(tokens[4].kind, TokenKind::Colon);
}
