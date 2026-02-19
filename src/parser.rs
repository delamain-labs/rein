use crate::ast::{AgentDef, Budget, Capability, Constraint, ReinFile, Span};
use crate::lexer::{Token, TokenKind, tokenize};

/// Parse error with source location and message.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (at {}..{})",
            self.message, self.span.start, self.span.end
        )
    }
}

/// Parse a `.rein` source string into a `ReinFile`.
pub fn parse(source: &str) -> Result<ReinFile, ParseError> {
    let tokens = tokenize(source).map_err(|e| ParseError::new(e.message, e.span))?;
    let mut parser = Parser::new(tokens);
    parser.parse_file()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Byte offset of the end of the most recently consumed (non-comment) token.
    last_consumed_end: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            last_consumed_end: 0,
        }
    }

    /// Current token (never out of bounds — last token is always Eof).
    fn current(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn current_span(&self) -> Span {
        self.current().span.clone()
    }

    /// Peek at current kind without consuming.
    fn peek(&self) -> &TokenKind {
        &self.current().kind
    }

    /// Advance past comment tokens, returning the next meaningful token.
    fn skip_comments(&mut self) {
        while self.current().kind == TokenKind::Comment {
            self.pos += 1;
        }
    }

    /// Advance one token, skipping comments.
    fn advance(&mut self) {
        if self.current().kind != TokenKind::Eof {
            self.last_consumed_end = self.current().span.end;
            self.pos += 1;
        }
        self.skip_comments();
    }

    /// Expect a specific token kind, advance, return its span.
    fn expect(&mut self, expected: &TokenKind) -> Result<Span, ParseError> {
        self.skip_comments();
        let tok = self.current().clone();
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(expected) {
            self.advance();
            Ok(tok.span)
        } else {
            Err(ParseError::new(
                format!("expected {:?}, got {:?}", expected, tok.kind),
                tok.span,
            ))
        }
    }

    /// Consume an `Ident` or `StringLiteral` token as a model name and return its value.
    fn expect_model_value(&mut self) -> Result<String, ParseError> {
        self.skip_comments();
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(ParseError::new(
                format!(
                    "expected model name (identifier or string literal), got {:?}",
                    tok.kind
                ),
                tok.span,
            )),
        }
    }

    /// Consume an `Ident` token and return its string value + span.
    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        self.skip_comments();
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                Ok((name, tok.span))
            }
            _ => Err(ParseError::new(
                format!("expected identifier, got {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    // ── Grammar rules ────────────────────────────────────────────────────────

    pub fn parse_file(&mut self) -> Result<ReinFile, ParseError> {
        self.skip_comments();
        let mut agents = Vec::new();
        while self.peek() != &TokenKind::Eof {
            agents.push(self.parse_agent()?);
            self.skip_comments();
        }
        Ok(ReinFile { agents })
    }

    fn parse_agent(&mut self) -> Result<AgentDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        // `agent`
        self.expect(&TokenKind::Agent)?;

        // agent name
        let (name, _) = self.expect_ident()?;

        // `{`
        self.expect(&TokenKind::LBrace)?;

        let mut model: Option<String> = None;
        let mut can: Vec<Capability> = Vec::new();
        let mut cannot: Vec<Capability> = Vec::new();
        let mut budget: Option<Budget> = None;

        let mut seen_model = false;
        let mut seen_can = false;
        let mut seen_cannot = false;
        let mut seen_budget = false;

        // Parse fields until `}`
        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(AgentDef {
                        name,
                        model,
                        can,
                        cannot,
                        budget,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Model => {
                    if seen_model {
                        return Err(ParseError::new(
                            format!("duplicate field 'model' in agent '{}'", name),
                            self.current_span(),
                        ));
                    }
                    seen_model = true;
                    self.advance(); // consume `model`
                    self.expect(&TokenKind::Colon)?;
                    // model value: bare ident or quoted string literal
                    let value = self.expect_model_value()?;
                    model = Some(value);
                }
                TokenKind::Can => {
                    if seen_can {
                        return Err(ParseError::new(
                            format!("duplicate field 'can' in agent '{}'", name),
                            self.current_span(),
                        ));
                    }
                    seen_can = true;
                    self.advance(); // consume `can`
                    can = self.parse_capability_list()?;
                }
                TokenKind::Cannot => {
                    if seen_cannot {
                        return Err(ParseError::new(
                            format!("duplicate field 'cannot' in agent '{}'", name),
                            self.current_span(),
                        ));
                    }
                    seen_cannot = true;
                    self.advance(); // consume `cannot`
                    cannot = self.parse_capability_list()?;
                }
                TokenKind::Budget => {
                    if seen_budget {
                        return Err(ParseError::new(
                            format!("duplicate field 'budget' in agent '{}'", name),
                            self.current_span(),
                        ));
                    }
                    seen_budget = true;
                    self.advance(); // consume `budget`
                    self.expect(&TokenKind::Colon)?;
                    budget = Some(self.parse_budget()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file: expected `}`",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in agent body: {:?}", other),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_capability_list(&mut self) -> Result<Vec<Capability>, ParseError> {
        self.expect(&TokenKind::LBracket)?;
        let mut caps = Vec::new();
        loop {
            self.skip_comments();
            match self.peek() {
                TokenKind::RBracket => {
                    self.advance();
                    break;
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file: expected `]`",
                        self.current_span(),
                    ));
                }
                _ => caps.push(self.parse_capability()?),
            }
        }
        Ok(caps)
    }

    fn parse_capability(&mut self) -> Result<Capability, ParseError> {
        let start = self.current_span().start;
        let (namespace, _) = self.expect_ident()?;
        self.expect(&TokenKind::Dot)?;
        let (action, _) = self.expect_ident()?;

        // optional `up to $<amount>`
        let constraint = if self.peek() == &TokenKind::Up {
            self.advance(); // consume `up`
            self.expect(&TokenKind::To)?;
            let (amount, _) = self.expect_dollar()?;
            Some(Constraint::MonetaryCap {
                amount,
                currency: "USD".to_string(),
            })
        } else {
            None
        };

        let end = self.last_consumed_end;
        Ok(Capability {
            namespace,
            action,
            constraint,
            span: Span::new(start, end),
        })
    }

    fn expect_dollar(&mut self) -> Result<(u64, Span), ParseError> {
        self.skip_comments();
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::Dollar(amount) => {
                self.advance();
                Ok((amount, tok.span))
            }
            _ => Err(ParseError::new(
                format!("expected dollar amount, got {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    fn parse_budget(&mut self) -> Result<Budget, ParseError> {
        let start = self.current_span().start;
        let (amount, _) = self.expect_dollar()?;
        self.expect(&TokenKind::Per)?;
        let (unit, _) = self.expect_ident()?;
        let end = self.last_consumed_end;
        Ok(Budget {
            amount,
            currency: "USD".to_string(),
            unit,
            span: Span::new(start, end),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Constraint;

    fn parse_ok(src: &str) -> ReinFile {
        parse(src).expect("expected parse to succeed")
    }

    fn parse_err(src: &str) -> ParseError {
        parse(src).expect_err("expected parse to fail")
    }

    // ── String literal model values ───────────────────────────────────────────

    #[test]
    fn parse_model_as_string_literal() {
        let f = parse_ok(r#"agent foo { model: "anthropic/claude-3-sonnet" }"#);
        assert_eq!(
            f.agents[0].model.as_deref(),
            Some("anthropic/claude-3-sonnet")
        );
    }

    #[test]
    fn parse_model_string_literal_with_dashes() {
        let f = parse_ok(r#"agent foo { model: "gpt-4o" }"#);
        assert_eq!(f.agents[0].model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn parse_model_ident_still_works() {
        // Bare identifier must continue to work alongside string literals.
        let f = parse_ok("agent foo { model: anthropic }");
        assert_eq!(f.agents[0].model.as_deref(), Some("anthropic"));
    }

    #[test]
    fn error_model_invalid_value() {
        // A dollar amount is neither an ident nor a string — must error.
        let err = parse_err("agent foo { model: $5 }");
        assert!(err.message.contains("model name"), "got: {}", err.message);
    }

    // ── Minimal agent ─────────────────────────────────────────────────────────

    #[test]
    fn parse_minimal_agent() {
        let f = parse_ok("agent foo { model: anthropic }");
        assert_eq!(f.agents.len(), 1);
        let a = &f.agents[0];
        assert_eq!(a.name, "foo");
        assert_eq!(a.model.as_deref(), Some("anthropic"));
        assert!(a.can.is_empty());
        assert!(a.cannot.is_empty());
        assert!(a.budget.is_none());
    }

    #[test]
    fn parse_agent_no_model() {
        let f = parse_ok("agent bot { }");
        assert_eq!(f.agents[0].model, None);
    }

    // ── Capabilities ──────────────────────────────────────────────────────────

    #[test]
    fn parse_can_list() {
        let src = r#"
agent foo {
    can [
        zendesk.read_ticket
        zendesk.reply_ticket
    ]
}"#;
        let f = parse_ok(src);
        let a = &f.agents[0];
        assert_eq!(a.can.len(), 2);
        assert_eq!(a.can[0].namespace, "zendesk");
        assert_eq!(a.can[0].action, "read_ticket");
        assert_eq!(a.can[1].action, "reply_ticket");
    }

    #[test]
    fn parse_cannot_list() {
        let src = "agent foo { cannot [ zendesk.delete_ticket ] }";
        let f = parse_ok(src);
        assert_eq!(f.agents[0].cannot[0].action, "delete_ticket");
    }

    #[test]
    fn parse_up_to_constraint() {
        let src = "agent foo { can [ zendesk.refund up to $50 ] }";
        let f = parse_ok(src);
        let cap = &f.agents[0].can[0];
        assert_eq!(cap.action, "refund");
        match &cap.constraint {
            Some(Constraint::MonetaryCap { amount, currency }) => {
                assert_eq!(*amount, 5000u64);
                assert_eq!(currency, "USD");
            }
            None => panic!("expected MonetaryCap constraint"),
        }
    }

    // ── Budget ────────────────────────────────────────────────────────────────

    #[test]
    fn parse_budget() {
        let src = "agent foo { budget: $0.03 per ticket }";
        let f = parse_ok(src);
        let b = f.agents[0].budget.as_ref().unwrap();
        assert_eq!(b.amount, 3u64);
        assert_eq!(b.currency, "USD");
        assert_eq!(b.unit, "ticket");
    }

    // ── Full agent ────────────────────────────────────────────────────────────

    #[test]
    fn parse_full_agent() {
        let src = r#"
agent support_triage {
    model: anthropic

    can [
        zendesk.read_ticket
        zendesk.reply_ticket
        zendesk.refund up to $50
    ]

    cannot [
        zendesk.delete_ticket
        zendesk.admin
    ]

    budget: $0.03 per ticket
}"#;
        let f = parse_ok(src);
        let a = &f.agents[0];
        assert_eq!(a.name, "support_triage");
        assert_eq!(a.model.as_deref(), Some("anthropic"));
        assert_eq!(a.can.len(), 3);
        assert_eq!(a.cannot.len(), 2);
        assert!(a.budget.is_some());
        // constraint on refund
        assert!(a.can[2].constraint.is_some());
    }

    // ── Multiple agents ───────────────────────────────────────────────────────

    #[test]
    fn parse_multiple_agents() {
        let src = r#"
agent alpha { model: openai }
agent beta  { model: anthropic }
"#;
        let f = parse_ok(src);
        assert_eq!(f.agents.len(), 2);
        assert_eq!(f.agents[0].name, "alpha");
        assert_eq!(f.agents[1].name, "beta");
    }

    // ── Comments ──────────────────────────────────────────────────────────────

    #[test]
    fn parse_with_comments() {
        let src = r#"
// top-level comment
agent foo {
    // model comment
    model: anthropic /* inline */
}
"#;
        let f = parse_ok(src);
        assert_eq!(f.agents[0].model.as_deref(), Some("anthropic"));
    }

    // ── Span accuracy ─────────────────────────────────────────────────────────

    #[test]
    fn capability_span_simple() {
        let src = "agent foo { can [ zendesk.read_ticket ] }";
        let f = parse_ok(src);
        let cap = &f.agents[0].can[0];
        let text = &src[cap.span.start..cap.span.end];
        assert_eq!(text, "zendesk.read_ticket");
    }

    #[test]
    fn capability_span_with_constraint() {
        let src = "agent foo { can [ zendesk.refund up to $50 ] }";
        let f = parse_ok(src);
        let cap = &f.agents[0].can[0];
        let text = &src[cap.span.start..cap.span.end];
        assert_eq!(text, "zendesk.refund up to $50");
    }

    // ── Error paths ───────────────────────────────────────────────────────────

    #[test]
    fn error_missing_agent_name() {
        let err = parse_err("agent { }");
        assert!(err.message.contains("identifier"), "got: {}", err.message);
    }

    #[test]
    fn error_missing_lbrace() {
        let err = parse_err("agent foo }");
        assert!(
            err.message.contains("LBrace") || err.message.contains('{'),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_missing_rbrace() {
        let err = parse_err("agent foo {");
        assert!(
            err.message.contains("end of file") || err.message.contains('}'),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_can_without_bracket() {
        let err = parse_err("agent foo { can zendesk.read_ticket }");
        assert!(
            err.message.contains("LBracket") || err.message.contains('['),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_duplicate_model() {
        let err = parse_err("agent foo { model: a model: b }");
        assert!(
            err.message.contains("duplicate field 'model'"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_duplicate_can() {
        let err = parse_err("agent foo { can [ zendesk.read ] can [ zendesk.write ] }");
        assert!(
            err.message.contains("duplicate field 'can'"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_duplicate_cannot() {
        let err = parse_err("agent foo { cannot [ zendesk.read ] cannot [ zendesk.write ] }");
        assert!(
            err.message.contains("duplicate field 'cannot'"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_duplicate_budget() {
        let err = parse_err("agent foo { budget: $0.03 per ticket budget: $0.05 per ticket }");
        assert!(
            err.message.contains("duplicate field 'budget'"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_budget_missing_dollar() {
        let err = parse_err("agent foo { budget: notadollar per ticket }");
        assert!(
            err.message.to_lowercase().contains("dollar") || err.message.contains('$'),
            "got: {}",
            err.message
        );
    }
}
