use crate::ast::{
    AgentDef, Budget, Capability, Constraint, ExecutionMode, ReinFile, RouteRule, Span, Stage,
    WorkflowDef,
};
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
                format!("expected {}, got {}", expected, tok.kind),
                tok.span,
            ))
        }
    }

    /// Parse a value expression: a string literal, identifier, or function
    /// call like `env("VAR_NAME")`.
    ///
    /// Returns a `ValueExpr` representing the parsed value.
    fn parse_value_expr(&mut self) -> Result<crate::ast::ValueExpr, ParseError> {
        self.skip_comments();
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Ident(name) if name == "env" => {
                let start = tok.span.start;
                self.advance(); // consume 'env'
                self.expect(&TokenKind::LParen)?;
                self.skip_comments();
                let arg_tok = self.current().clone();
                let var_name = match &arg_tok.kind {
                    TokenKind::StringLiteral(s) => {
                        let s = s.clone();
                        self.advance();
                        s
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!(
                                "env() expects a string argument, got {}",
                                arg_tok.kind
                            ),
                            arg_tok.span,
                        ));
                    }
                };
                let end_span = self.expect(&TokenKind::RParen)?;
                Ok(crate::ast::ValueExpr::EnvRef {
                    var_name,
                    span: crate::ast::Span::new(start, end_span.end),
                })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(crate::ast::ValueExpr::Literal(name))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(crate::ast::ValueExpr::Literal(s))
            }
            _ => Err(ParseError::new(
                format!("expected value (identifier, string, or env()), got {}", tok.kind),
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
                format!("expected identifier, got {}", tok.kind),
                tok.span,
            )),
        }
    }

    // ── Grammar rules ────────────────────────────────────────────────────────

    pub fn parse_file(&mut self) -> Result<ReinFile, ParseError> {
        self.skip_comments();
        let mut agents = Vec::new();
        let mut workflows = Vec::new();
        while self.peek() != &TokenKind::Eof {
            match self.peek() {
                TokenKind::Agent => agents.push(self.parse_agent()?),
                TokenKind::Workflow => workflows.push(self.parse_workflow()?),
                other => {
                    return Err(ParseError::new(
                        format!("expected 'agent' or 'workflow', got {other}"),
                        self.current_span(),
                    ));
                }
            }
            self.skip_comments();
        }
        Ok(ReinFile { agents, workflows })
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

        let mut model: Option<crate::ast::ValueExpr> = None;
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
                            format!("duplicate field 'model' in agent '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_model = true;
                    self.advance(); // consume `model`
                    self.expect(&TokenKind::Colon)?;
                    let value = self.parse_value_expr()?;
                    model = Some(value);
                }
                TokenKind::Can => {
                    if seen_can {
                        return Err(ParseError::new(
                            format!("duplicate field 'can' in agent '{name}'"),
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
                            format!("duplicate field 'cannot' in agent '{name}'"),
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
                            format!("duplicate field 'budget' in agent '{name}'"),
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
                        format!("unexpected token in agent body: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_workflow(&mut self) -> Result<WorkflowDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Workflow)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut trigger: Option<String> = None;
        let mut stages: Vec<Stage> = Vec::new();
        let mut seen_trigger = false;
        let mut seen_stages = false;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();

                    let trigger = trigger.ok_or_else(|| {
                        ParseError::new(
                            format!("workflow '{name}' is missing required field 'trigger'"),
                            Span::new(start, end),
                        )
                    })?;

                    if stages.is_empty() {
                        return Err(ParseError::new(
                            format!("workflow '{name}' must have at least one stage"),
                            Span::new(start, end),
                        ));
                    }

                    return Ok(WorkflowDef {
                        name,
                        trigger,
                        stages,
                        mode: ExecutionMode::Sequential,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Trigger => {
                    if seen_trigger {
                        return Err(ParseError::new(
                            format!("duplicate field 'trigger' in workflow '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_trigger = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    let (value, _) = self.expect_ident()?;
                    trigger = Some(value);
                }
                TokenKind::Stages => {
                    if seen_stages {
                        return Err(ParseError::new(
                            format!("duplicate field 'stages' in workflow '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_stages = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    stages = self.parse_stage_list()?;
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file: expected `}`",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in workflow body: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_stage_list(&mut self) -> Result<Vec<Stage>, ParseError> {
        self.expect(&TokenKind::LBracket)?;
        let mut stages = Vec::new();
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
                _ => {
                    let stage_start = self.current_span().start;
                    let (agent_name, _) = self.expect_ident()?;
                    let end = self.last_consumed_end;

                    stages.push(Stage {
                        name: agent_name.clone(),
                        agent: agent_name,
                        route: RouteRule::Next,
                        span: Span::new(stage_start, end),
                    });

                    // Optional comma separator
                    if self.peek() == &TokenKind::Comma {
                        self.advance();
                    }
                }
            }
        }
        Ok(stages)
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
                format!("expected dollar amount, got {}", tok.kind),
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
mod tests;
