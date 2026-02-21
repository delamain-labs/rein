use crate::ast::{
    AgentDef, Budget, Capability, Constraint, DefaultsDef, ExecutionMode, GuardrailRule,
    GuardrailSection, GuardrailsDef, ProviderDef, ReinFile, RouteRule, Span, Stage, ToolDef,
    ValueExpr, WorkflowDef,
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
    fn parse_value_expr(&mut self) -> Result<ValueExpr, ParseError> {
        self.skip_comments();
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Ident(name) if name == "env" => {
                let start = tok.span.start;
                self.advance();
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
                            format!("env() expects a string argument, got {}", arg_tok.kind),
                            arg_tok.span,
                        ));
                    }
                };
                let end_span = self.expect(&TokenKind::RParen)?;
                Ok(ValueExpr::EnvRef {
                    var_name,
                    span: Span::new(start, end_span.end),
                })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(ValueExpr::Literal(name))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(ValueExpr::Literal(s))
            }
            _ => Err(ParseError::new(
                format!(
                    "expected value (identifier, string, or env()), got {}",
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
                format!("expected identifier, got {}", tok.kind),
                tok.span,
            )),
        }
    }

    // ── Grammar rules ────────────────────────────────────────────────────────

    pub fn parse_file(&mut self) -> Result<ReinFile, ParseError> {
        self.skip_comments();
        let mut defaults: Option<DefaultsDef> = None;
        let mut providers = Vec::new();
        let mut tools = Vec::new();
        let mut agents = Vec::new();
        let mut workflows = Vec::new();
        while self.peek() != &TokenKind::Eof {
            match self.peek() {
                TokenKind::Defaults => {
                    if defaults.is_some() {
                        return Err(ParseError::new(
                            "duplicate 'defaults' block (only one allowed per file)",
                            self.current_span(),
                        ));
                    }
                    defaults = Some(self.parse_defaults()?);
                }
                TokenKind::Provider => providers.push(self.parse_provider()?),
                TokenKind::Tool => tools.push(self.parse_tool()?),
                TokenKind::Agent => agents.push(self.parse_agent()?),
                TokenKind::Workflow => workflows.push(self.parse_workflow()?),
                other => {
                    return Err(ParseError::new(
                        format!(
                            "expected 'defaults', 'provider', 'tool', 'agent', or 'workflow', got {other}"
                        ),
                        self.current_span(),
                    ));
                }
            }
            self.skip_comments();
        }
        Ok(ReinFile {
            defaults,
            providers,
            tools,
            agents,
            workflows,
        })
    }

    fn parse_defaults(&mut self) -> Result<DefaultsDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Defaults)?;
        self.expect(&TokenKind::LBrace)?;

        let mut model: Option<ValueExpr> = None;
        let mut budget: Option<Budget> = None;
        let (mut seen_model, mut seen_budget) = (false, false);

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(DefaultsDef {
                        model,
                        budget,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Model => {
                    if seen_model {
                        return Err(ParseError::new(
                            "duplicate field 'model' in defaults block",
                            self.current_span(),
                        ));
                    }
                    seen_model = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    model = Some(self.parse_value_expr()?);
                }
                TokenKind::Budget => {
                    if seen_budget {
                        return Err(ParseError::new(
                            "duplicate field 'budget' in defaults block",
                            self.current_span(),
                        ));
                    }
                    seen_budget = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    budget = Some(self.parse_budget()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected field in defaults block: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_provider(&mut self) -> Result<ProviderDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Provider)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut model: Option<ValueExpr> = None;
        let mut key: Option<ValueExpr> = None;
        let mut seen_model = false;
        let mut seen_key = false;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(ProviderDef {
                        name,
                        model,
                        key,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Model => {
                    if seen_model {
                        return Err(ParseError::new(
                            format!("duplicate field 'model' in provider '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_model = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    model = Some(self.parse_value_expr()?);
                }
                TokenKind::Key => {
                    if seen_key {
                        return Err(ParseError::new(
                            format!("duplicate field 'key' in provider '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_key = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    key = Some(self.parse_value_expr()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected field in provider '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_tool(&mut self) -> Result<ToolDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Tool)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut endpoint: Option<ValueExpr> = None;
        let mut provider: Option<ValueExpr> = None;
        let mut key: Option<ValueExpr> = None;
        let mut seen_endpoint = false;
        let mut seen_provider = false;
        let mut seen_key = false;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(ToolDef {
                        name,
                        endpoint,
                        provider,
                        key,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Endpoint => {
                    if seen_endpoint {
                        return Err(ParseError::new(
                            format!("duplicate field 'endpoint' in tool '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_endpoint = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    endpoint = Some(self.parse_value_expr()?);
                }
                TokenKind::Provider => {
                    if seen_provider {
                        return Err(ParseError::new(
                            format!("duplicate field 'provider' in tool '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_provider = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    provider = Some(self.parse_value_expr()?);
                }
                TokenKind::Key => {
                    if seen_key {
                        return Err(ParseError::new(
                            format!("duplicate field 'key' in tool '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_key = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    key = Some(self.parse_value_expr()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected field in tool '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_agent(&mut self) -> Result<AgentDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Agent)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut model: Option<ValueExpr> = None;
        let mut can: Vec<Capability> = Vec::new();
        let mut cannot: Vec<Capability> = Vec::new();
        let mut budget: Option<Budget> = None;
        let mut guardrails: Option<GuardrailsDef> = None;

        let (mut seen_model, mut seen_can, mut seen_cannot) = (false, false, false);
        let (mut seen_budget, mut seen_guardrails) = (false, false);

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
                        guardrails,
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
                    self.advance();
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
                    self.advance();
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
                    self.advance();
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
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    budget = Some(self.parse_budget()?);
                }
                TokenKind::Guardrails => {
                    if seen_guardrails {
                        return Err(ParseError::new(
                            format!("duplicate field 'guardrails' in agent '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_guardrails = true;
                    guardrails = Some(self.parse_guardrails()?);
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

    fn parse_guardrails(&mut self) -> Result<GuardrailsDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Guardrails)?;
        self.expect(&TokenKind::LBrace)?;

        let mut sections = Vec::new();
        let mut seen_names: Vec<String> = Vec::new();

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(GuardrailsDef {
                        sections,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Ident(section_name) => {
                    if seen_names.contains(&section_name) {
                        return Err(ParseError::new(
                            format!("duplicate guardrail section '{section_name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_names.push(section_name.clone());
                    sections.push(self.parse_guardrail_section()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in guardrails block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("expected section name in guardrails, got {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_guardrail_section(&mut self) -> Result<GuardrailSection, ParseError> {
        let start = self.current_span().start;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut rules = Vec::new();

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(GuardrailSection {
                        name,
                        rules,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Ident(key) => {
                    let rule_start = self.current_span().start;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    let (value, _) = self.expect_ident()?;
                    let rule_end = self.current_span().start;
                    rules.push(GuardrailRule {
                        key,
                        value,
                        span: Span::new(rule_start, rule_end),
                    });
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in guardrail section",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("expected rule or '}}' in guardrail section, got {other}"),
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
        let mut steps: Vec<crate::ast::StepDef> = Vec::new();
        let mut route_ons: Vec<crate::ast::RouteOnDef> = Vec::new();
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

                    if stages.is_empty() && steps.is_empty() && route_ons.is_empty() {
                        return Err(ParseError::new(
                            format!(
                                "workflow '{name}' must have at least one stage, step, or route on"
                            ),
                            Span::new(start, end),
                        ));
                    }

                    return Ok(WorkflowDef {
                        name,
                        trigger,
                        stages,
                        steps,
                        route_ons,
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
                    trigger = Some(self.parse_trigger_expr()?);
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
                TokenKind::Step => {
                    steps.push(self.parse_step()?);
                }
                TokenKind::Route => {
                    route_ons.push(self.parse_route_on()?);
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

    /// Parse a `step <name> { agent: <ident> goal: <text> }` block.
    fn parse_step(&mut self) -> Result<crate::ast::StepDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Step)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut agent: Option<String> = None;
        let mut goal: Option<String> = None;
        let mut when: Option<crate::ast::WhenExpr> = None;
        let mut seen_agent = false;
        let mut seen_goal = false;
        let mut seen_when = false;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();

                    let agent = agent.ok_or_else(|| {
                        ParseError::new(
                            format!("step '{name}' is missing required field 'agent'"),
                            Span::new(start, end),
                        )
                    })?;

                    return Ok(crate::ast::StepDef {
                        name,
                        agent,
                        goal,
                        when,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Agent => {
                    if seen_agent {
                        return Err(ParseError::new(
                            format!("duplicate field 'agent' in step '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_agent = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    let (value, _) = self.expect_ident()?;
                    agent = Some(value);
                }
                TokenKind::Goal => {
                    if seen_goal {
                        return Err(ParseError::new(
                            format!("duplicate field 'goal' in step '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_goal = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    // Goal is a string literal or plain text value
                    let value = self.parse_value_expr()?;
                    goal = Some(match value {
                        ValueExpr::Literal(s) => s,
                        ValueExpr::EnvRef { span, .. } => {
                            return Err(ParseError::new(
                                "goal must be a string literal, not env()",
                                span,
                            ));
                        }
                    });
                }
                TokenKind::When => {
                    if seen_when {
                        return Err(ParseError::new(
                            format!("duplicate field 'when' in step '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_when = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    when = Some(self.parse_when_expr()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file: expected `}`",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in step '{name}': {other}"),
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
            self.advance();
            self.expect(&TokenKind::To)?;
            let (amount, symbol, _) = self.expect_currency()?;
            let currency = match symbol {
                '€' => "EUR",
                '£' => "GBP",
                '¥' => "JPY",
                _ => "USD",
            };
            Some(Constraint::MonetaryCap {
                amount,
                currency: currency.to_string(),
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

    fn expect_currency(&mut self) -> Result<(u64, char, Span), ParseError> {
        self.skip_comments();
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::Currency { amount, symbol } => {
                self.advance();
                Ok((amount, symbol, tok.span))
            }
            _ => Err(ParseError::new(
                format!("expected currency amount, got {}", tok.kind),
                tok.span,
            )),
        }
    }

    fn parse_budget(&mut self) -> Result<Budget, ParseError> {
        let start = self.current_span().start;
        let (amount, symbol, _) = self.expect_currency()?;
        self.expect(&TokenKind::Per)?;
        let (unit, _) = self.expect_ident()?;
        let end = self.last_consumed_end;
        let currency = match symbol {
            '€' => "EUR",
            '£' => "GBP",
            '¥' => "JPY",
            _ => "USD",
        };
        Ok(Budget {
            amount,
            currency: currency.to_string(),
            unit,
            span: Span::new(start, end),
        })
    }
    /// Parse a trigger expression: either a string literal or a sequence of
    /// identifiers (e.g., `new ticket in zendesk`).
    /// Parse a `when` expression: `field < 70% or field > $50`.
    fn parse_when_expr(&mut self) -> Result<crate::ast::WhenExpr, ParseError> {
        let start = self.current_span().start;
        let mut conditions = Vec::new();

        // First comparison (no leading logic op)
        let comp = self.parse_comparison()?;
        conditions.push((None, comp));

        // Subsequent comparisons joined by `and`/`or`
        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::And => {
                    self.advance();
                    let comp = self.parse_comparison()?;
                    conditions.push((Some(crate::ast::LogicOp::And), comp));
                }
                TokenKind::Or => {
                    self.advance();
                    let comp = self.parse_comparison()?;
                    conditions.push((Some(crate::ast::LogicOp::Or), comp));
                }
                _ => break,
            }
        }

        let end = self.last_consumed_end;
        Ok(crate::ast::WhenExpr {
            conditions,
            span: Span::new(start, end),
        })
    }

    /// Parse a single comparison: `field < threshold` or `field > threshold`.
    fn parse_comparison(&mut self) -> Result<crate::ast::Comparison, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        // Field: dotted identifier (e.g. `confidence` or `step.refund`)
        let mut field_parts = Vec::new();
        let (first, _) = self.expect_ident()?;
        field_parts.push(first);
        while self.peek() == &TokenKind::Dot {
            self.advance();
            let (part, _) = self.expect_ident()?;
            field_parts.push(part);
        }
        let field = field_parts.join(".");

        // Operator
        self.skip_comments();
        let op = match self.peek().clone() {
            TokenKind::LessThan => {
                self.advance();
                crate::ast::CompareOp::LessThan
            }
            TokenKind::GreaterThan => {
                self.advance();
                crate::ast::CompareOp::GreaterThan
            }
            other => {
                return Err(ParseError::new(
                    format!("expected '<' or '>' in when expression, got {other}"),
                    self.current_span(),
                ));
            }
        };

        // Threshold: percentage or currency
        self.skip_comments();
        let tok = self.current().clone();
        let threshold = match &tok.kind {
            TokenKind::Percent(value) => {
                let value = *value;
                self.advance();
                crate::ast::ThresholdValue::Percent { value }
            }
            TokenKind::Currency { amount, symbol } => {
                let amount = *amount;
                let currency = match symbol {
                    '€' => "EUR",
                    '£' => "GBP",
                    '¥' => "JPY",
                    _ => "USD",
                }
                .to_string();
                self.advance();
                crate::ast::ThresholdValue::Currency { amount, currency }
            }
            other => {
                return Err(ParseError::new(
                    format!(
                        "expected percentage or currency amount in when expression, got {other}"
                    ),
                    tok.span,
                ));
            }
        };

        let end = self.last_consumed_end;
        Ok(crate::ast::Comparison {
            field,
            op,
            threshold,
            span: Span::new(start, end),
        })
    }

    fn parse_route_on(&mut self) -> Result<crate::ast::RouteOnDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Route)?;
        self.expect(&TokenKind::On)?;

        // Parse dotted expression (e.g., `classify.category` or `step.output`)
        let mut expr_parts = Vec::new();
        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::Ident(word) => {
                    expr_parts.push(word);
                    self.advance();
                    if self.peek() == &TokenKind::Dot {
                        expr_parts.push(".".to_string());
                        self.advance();
                    }
                }
                // Allow keywords as expression parts (e.g., `step.output`)
                ref tok if self.peek() != &TokenKind::LBrace && self.peek() != &TokenKind::Eof => {
                    let word = tok.to_string();
                    if self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Dot)
                        || !expr_parts.is_empty()
                    {
                        expr_parts.push(word);
                        self.advance();
                        if self.peek() == &TokenKind::Dot {
                            expr_parts.push(".".to_string());
                            self.advance();
                        }
                    } else {
                        return Err(ParseError::new(
                            format!("expected expression or '{{' in route on, got {tok}"),
                            self.current_span(),
                        ));
                    }
                }
                _ => break,
            }
        }

        if expr_parts.is_empty() {
            return Err(ParseError::new(
                "route on requires an expression",
                self.current_span(),
            ));
        }

        let expr = expr_parts.join("");
        self.expect(&TokenKind::LBrace)?;

        let mut branches = Vec::new();
        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(crate::ast::RouteOnDef {
                        expr,
                        branches,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Ident(_) | TokenKind::Underscore => {
                    branches.push(self.parse_route_branch()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in route on block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("expected branch or '}}' in route on, got {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    fn parse_route_branch(&mut self) -> Result<crate::ast::RouteBranch, ParseError> {
        let start = self.current_span().start;

        let pattern = match self.peek().clone() {
            TokenKind::Underscore => {
                self.advance();
                "_".to_string()
            }
            TokenKind::Ident(name) => {
                self.advance();
                name
            }
            other => {
                return Err(ParseError::new(
                    format!("expected branch pattern, got {other}"),
                    self.current_span(),
                ));
            }
        };

        self.expect(&TokenKind::Arrow)?;

        // Target: idents and keywords, but stop if the NEXT word-like token
        // is followed by Arrow (that means it's the start of the next branch)
        let mut target_parts = Vec::new();
        loop {
            self.skip_comments();
            if let Some(word) = self.current_word() {
                // Lookahead: if the token after this is Arrow,
                // this word is the next branch's pattern, not our target
                if !target_parts.is_empty() && self.is_next_arrow() {
                    break;
                }
                target_parts.push(word);
                self.advance();
            } else {
                break;
            }
        }

        if target_parts.is_empty() {
            return Err(ParseError::new(
                format!("expected target after '->' in branch '{pattern}'"),
                self.current_span(),
            ));
        }

        let end = self.last_consumed_end;
        Ok(crate::ast::RouteBranch {
            pattern,
            target: target_parts.join(" "),
            span: Span::new(start, end),
        })
    }

    /// If the current token is a word-like token (identifier or keyword),
    /// return its string representation. Used in contexts where keywords
    /// can appear as plain words (e.g. route branch targets).
    fn current_word(&self) -> Option<String> {
        match &self.current().kind {
            TokenKind::Ident(s) => Some(s.clone()),
            TokenKind::Agent => Some("agent".to_string()),
            TokenKind::Can => Some("can".to_string()),
            TokenKind::Cannot => Some("cannot".to_string()),
            TokenKind::Model => Some("model".to_string()),
            TokenKind::Budget => Some("budget".to_string()),
            TokenKind::Per => Some("per".to_string()),
            TokenKind::Up => Some("up".to_string()),
            TokenKind::To => Some("to".to_string()),
            TokenKind::Workflow => Some("workflow".to_string()),
            TokenKind::Trigger => Some("trigger".to_string()),
            TokenKind::Stages => Some("stages".to_string()),
            TokenKind::Provider => Some("provider".to_string()),
            TokenKind::Key => Some("key".to_string()),
            TokenKind::Step => Some("step".to_string()),
            TokenKind::Goal => Some("goal".to_string()),
            TokenKind::Tool => Some("tool".to_string()),
            TokenKind::Endpoint => Some("endpoint".to_string()),
            TokenKind::Guardrails => Some("guardrails".to_string()),
            TokenKind::Defaults => Some("defaults".to_string()),
            TokenKind::Route => Some("route".to_string()),
            TokenKind::On => Some("on".to_string()),
            TokenKind::When => Some("when".to_string()),
            TokenKind::And => Some("and".to_string()),
            TokenKind::Or => Some("or".to_string()),
            _ => None,
        }
    }

    /// Peek two tokens ahead: is the token after the current one an Arrow?
    fn is_next_arrow(&self) -> bool {
        if self.pos + 1 < self.tokens.len() {
            self.tokens[self.pos + 1].kind == TokenKind::Arrow
        } else {
            false
        }
    }

    fn parse_trigger_expr(&mut self) -> Result<String, ParseError> {
        self.skip_comments();
        match self.peek().clone() {
            TokenKind::StringLiteral(s) => {
                self.advance();
                Ok(s)
            }
            TokenKind::Ident(_) => {
                let mut parts = Vec::new();
                while let TokenKind::Ident(word) = self.peek().clone() {
                    parts.push(word);
                    self.advance();
                }
                if parts.is_empty() {
                    return Err(ParseError::new(
                        "expected trigger expression",
                        self.current_span(),
                    ));
                }
                Ok(parts.join(" "))
            }
            _ => Err(ParseError::new(
                format!("expected trigger expression, got {}", self.peek()),
                self.current_span(),
            )),
        }
    }
}

#[cfg(test)]
mod tests;
