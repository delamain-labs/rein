use crate::ast::{
    AgentDef, Budget, Capability, Constraint, DefaultsDef, ExecutionMode, GuardrailRule,
    GuardrailSection, GuardrailsDef, ProviderDef, ReinFile, RouteRule, Span, Stage, ToolDef,
    ImportDef, TypeDef, TypeExpr, TypeField, ValueExpr, WorkflowDef,

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

    /// Peek at a token at `offset` positions ahead (skipping comments).
    fn peek_at(&self, offset: usize) -> Option<&TokenKind> {
        let mut pos = self.pos;
        let mut seen = 0;
        while pos < self.tokens.len() {
            if self.tokens[pos].kind == TokenKind::Comment {
                pos += 1;
                continue;
            }
            if seen == offset {
                return Some(&self.tokens[pos].kind);
            }
            seen += 1;
            pos += 1;
        }
        None
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
        let mut imports = Vec::new();
        let mut defaults: Option<DefaultsDef> = None;
        let mut providers = Vec::new();
        let mut tools = Vec::new();
        let mut agents = Vec::new();
        let mut workflows = Vec::new();
        let mut types = Vec::new();
        while self.peek() != &TokenKind::Eof {
            match self.peek() {
                TokenKind::Import => imports.push(self.parse_import()?),
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
                TokenKind::Type => types.push(self.parse_type_def()?),
                other => {
                    return Err(ParseError::new(
                        format!(
                            "expected top-level declaration (defaults, provider, tool, agent, workflow, type), got {other}"
                        ),
                        self.current_span(),
                    ));
                }
            }
            self.skip_comments();
        }
        Ok(ReinFile {
            imports,
            defaults,
            providers,
            tools,
            agents,
            workflows,
            types,
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
        let mut route_blocks: Vec<crate::ast::RouteBlock> = Vec::new();
        let mut parallel_blocks: Vec<crate::ast::ParallelBlock> = Vec::new();
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

                    if stages.is_empty() && steps.is_empty() && route_blocks.is_empty() && parallel_blocks.is_empty() {
                        return Err(ParseError::new(
                            format!("workflow '{name}' must have at least one stage, step, or route block"),
                            Span::new(start, end),
                        ));
                    }

                    return Ok(WorkflowDef {
                        name,
                        trigger,
                        stages,
                        steps,
                        route_blocks,
                        parallel_blocks,
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
                    route_blocks.push(self.parse_route_block()?);
                }
                TokenKind::Parallel => {
                    parallel_blocks.push(self.parse_parallel_block()?);
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
        let mut output_constraints: Vec<(String, TypeExpr)> = Vec::new();
        let mut seen_agent = false;
        let mut seen_goal = false;

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
                        output_constraints,
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
                TokenKind::Ident(ref field_name)
                    if self.peek_at(1).is_some_and(|t| *t == TokenKind::Colon) =>
                {
                    let field_name = field_name.clone();
                    self.advance(); // consume ident
                    self.expect(&TokenKind::Colon)?;
                    if *self.peek() == TokenKind::One {
                        let type_expr = self.parse_one_of()?;
                        output_constraints.push((field_name, type_expr));
                    } else {
                        return Err(ParseError::new(
                            format!("unexpected field '{field_name}' in step '{name}'"),
                            self.current_span(),
                        ));
                    }
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

    /// Parse a `parallel { step a {...} step b {...} }` block.
    fn parse_parallel_block(&mut self) -> Result<crate::ast::ParallelBlock, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Parallel)?;
        self.expect(&TokenKind::LBrace)?;

        let mut steps = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBrace {
                break;
            }
            steps.push(self.parse_step()?);
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        if steps.is_empty() {
            return Err(ParseError::new(
                "parallel block must contain at least one step",
                Span::new(start, end),
            ));
        }

        Ok(crate::ast::ParallelBlock {
            steps,
            span: Span::new(start, end),
        })
    }

    /// Parse a `route on <field_path> { pattern -> step name { ... }, ... }` block.
    fn parse_route_block(&mut self) -> Result<crate::ast::RouteBlock, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Route)?;
        self.expect(&TokenKind::On)?;

        // Parse dot-separated field path (keywords allowed as segments)
        let (first, _) = self.expect_ident()?;
        let mut path = first;
        while *self.peek() == TokenKind::Dot {
            self.advance(); // .
            // Allow keywords as path segments
            let segment = self.expect_ident_or_keyword()?;
            path.push('.');
            path.push_str(&segment);
        }

        self.expect(&TokenKind::LBrace)?;

        let mut arms = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBrace {
                break;
            }
            let arm_start = self.current_span().start;
            let pattern = match self.peek().clone() {
                TokenKind::Underscore => {
                    self.advance();
                    crate::ast::RoutePattern::Wildcard
                }
                TokenKind::Ident(val) => {
                    let val = val.clone();
                    self.advance();
                    crate::ast::RoutePattern::Value(val)
                }
                other => {
                    return Err(ParseError::new(
                        format!("expected pattern value or '_', got {other}"),
                        self.current_span(),
                    ));
                }
            };
            self.expect(&TokenKind::Arrow)?;
            let step = self.parse_step()?;
            let arm_end = self.current_span().start;
            arms.push(crate::ast::RouteArm {
                pattern,
                step,
                span: Span::new(arm_start, arm_end),
            });
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(crate::ast::RouteBlock {
            field_path: path,
            arms,
            span: Span::new(start, end),
        })
    }

    /// Parse an import declaration.
    ///
    /// Supports:
    /// - `import { name1, name2 } from "./path.rein"`
    /// - `import all from "./dir/"`
    /// - `import from @scope/name`
    fn parse_import(&mut self) -> Result<ImportDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Import)?;

        match self.peek().clone() {
            // import { name, ... } from "path"
            TokenKind::LBrace => {
                self.advance(); // {
                let mut names = Vec::new();
                loop {
                    self.skip_comments();
                    if *self.peek() == TokenKind::RBrace {
                        break;
                    }
                    let (name, _) = self.expect_ident()?;
                    names.push(name);
                    self.skip_comments();
                    if *self.peek() == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                self.expect(&TokenKind::From)?;
                let source = self.expect_string()?;
                let end = self.current_span().start;
                Ok(ImportDef::Named {
                    names,
                    source,
                    span: Span::new(start, end),
                })
            }
            // import all from "path"
            TokenKind::All => {
                self.advance(); // all
                self.expect(&TokenKind::From)?;
                let source = self.expect_string()?;
                let end = self.current_span().start;
                Ok(ImportDef::Glob {
                    source,
                    span: Span::new(start, end),
                })
            }
            // import from @scope/name
            TokenKind::From => {
                self.advance(); // from
                self.expect(&TokenKind::At)?;
                let (scope, _) = self.expect_ident()?;
                // Expect '/' separator — we'll use Ident since / isn't a token
                // Actually, we need a slash. Let me handle this differently.
                // For @scope/name, we'll read the rest as ident after consuming
                // what we can. Since '/' isn't a token, we'll expect the name
                // to be separated by checking for Dot or another convention.
                self.expect(&TokenKind::Slash)?;
                let (name, _) = self.expect_ident()?;
                let end = self.current_span().start;
                Ok(ImportDef::Registry {
                    scope,
                    name,
                    span: Span::new(start, end),
                })
            }
            other => Err(ParseError::new(
                format!("expected '{{', 'all', or 'from' after 'import', got {other}"),
                self.current_span(),
            )),
        }
    }

    /// Expect an identifier or keyword token, returning the text.
    /// Useful in contexts where keywords are valid (e.g. field paths).
    fn expect_ident_or_keyword(&mut self) -> Result<String, ParseError> {
        let text = self.peek().to_string();
        match self.peek() {
            TokenKind::Ident(_)
            | TokenKind::Agent
            | TokenKind::Model
            | TokenKind::Type
            | TokenKind::Step
            | TokenKind::Goal
            | TokenKind::Tool
            | TokenKind::Route
            | TokenKind::On
            | TokenKind::One
            | TokenKind::Of
            | TokenKind::All
            | TokenKind::From
            | TokenKind::Import => {
                self.advance();
                Ok(text)
            }
            other => Err(ParseError::new(
                format!("expected identifier, got {other}"),
                self.current_span(),
            )),
        }
    }

    /// Expect a string literal and return its value.
    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            other => Err(ParseError::new(
                format!("expected string literal, got {other}"),
                self.current_span(),
            )),
        }
    }

    /// Parse a `type Name { field: type_expr, ... }` definition.
    fn parse_type_def(&mut self) -> Result<TypeDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Type)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBrace {
                break;
            }
            let field_start = self.current_span().start;
            let (field_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let type_expr = self.parse_type_expr()?;
            let field_end = self.current_span().start;
            fields.push(TypeField {
                name: field_name,
                type_expr,
                span: Span::new(field_start, field_end),
            });
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(TypeDef {
            name,
            fields,
            span: Span::new(start, end),
        })
    }

    /// Parse a type expression: named type, array, one of, or range.
    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        self.skip_comments();
        match self.peek().clone() {
            TokenKind::One => self.parse_one_of(),
            TokenKind::Number(n) => {
                let min = n.clone();
                self.advance();
                self.expect(&TokenKind::DotDot)?;
                match self.peek().clone() {
                    TokenKind::Number(max) => {
                        let max = max.clone();
                        self.advance();
                        Ok(TypeExpr::Range { min, max })
                    }
                    other => Err(ParseError::new(
                        format!("expected number after '..', got {other}"),
                        self.current_span(),
                    )),
                }
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                // Check for array syntax: Type[]
                let array = if *self.peek() == TokenKind::LBracket
                    && self.peek_at(1).is_some_and(|t| *t == TokenKind::RBracket)
                {
                    self.advance(); // [
                    self.advance(); // ]
                    true
                } else {
                    false
                };
                Ok(TypeExpr::Named { name, array })
            }
            other => Err(ParseError::new(
                format!("expected type expression, got {other}"),
                self.current_span(),
            )),
        }
    }

    /// Parse `one of [a, b, c]` type expression.
    fn parse_one_of(&mut self) -> Result<TypeExpr, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::One)?;
        self.expect(&TokenKind::Of)?;
        self.expect(&TokenKind::LBracket)?;

        let mut variants = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBracket {
                break;
            }
            let (variant, _) = self.expect_ident()?;
            variants.push(variant);
            self.skip_comments();
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBracket)?;

        if variants.is_empty() {
            return Err(ParseError::new(
                "one of requires at least one variant",
                Span::new(start, end),
            ));
        }

        Ok(TypeExpr::OneOf {
            variants,
            span: Span::new(start, end),
        })
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
