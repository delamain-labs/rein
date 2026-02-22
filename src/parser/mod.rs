use crate::ast::{ReinFile, Span, ValueExpr};
use crate::lexer::{Token, TokenKind, tokenize};

mod agent_parser;
mod channel_parser;
mod circuit_breaker_parser;
mod common_parser;
mod condition_parser;
mod eval_parser;
mod fleet_parser;
mod import_parser;
mod memory_parser;
mod observe_parser;
mod pipe_parser;
mod policy_parser;
mod schedule_parser;
mod secrets_parser;
mod step_parser;
mod type_parser;
mod within_parser;
mod workflow_parser;

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
                // Optional default: env("VAR", "default")
                let default = if *self.peek() == TokenKind::Comma {
                    self.advance();
                    self.skip_comments();
                    let default_tok = self.current().clone();
                    match &default_tok.kind {
                        TokenKind::StringLiteral(s) => {
                            let s = s.clone();
                            self.advance();
                            Some(s)
                        }
                        _ => {
                            return Err(ParseError::new(
                                format!(
                                    "env() default must be a string literal, got {}",
                                    default_tok.kind
                                ),
                                default_tok.span,
                            ));
                        }
                    }
                } else {
                    None
                };
                let end_span = self.expect(&TokenKind::RParen)?;
                Ok(ValueExpr::EnvRef {
                    var_name,
                    default,
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
        match &tok.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok((name, tok.span))
            }
            // Contextual keywords that can appear as identifiers
            TokenKind::Failure
            | TokenKind::Retry
            | TokenKind::Then
            | TokenKind::Exponential
            | TokenKind::Linear
            | TokenKind::Fixed
            | TokenKind::Escalate
            | TokenKind::One
            | TokenKind::Of
            | TokenKind::On
            | TokenKind::All
            | TokenKind::From
            | TokenKind::When
            | TokenKind::Route
            | TokenKind::Parallel
            | TokenKind::Auto
            | TokenKind::Resolve
            | TokenKind::Is
            | TokenKind::Policy
            | TokenKind::Tier
            | TokenKind::Fallback
            | TokenKind::Where
            | TokenKind::Sort
            | TokenKind::By
            | TokenKind::Take
            | TokenKind::Skip
            | TokenKind::Select
            | TokenKind::Unique
            | TokenKind::Asc
            | TokenKind::Desc
            | TokenKind::Observe
            | TokenKind::Fleet
            | TokenKind::Channel
            | TokenKind::Trace
            | TokenKind::Metrics
            | TokenKind::Alert
            | TokenKind::Export
            | TokenKind::Agents
            | TokenKind::Scaling
            | TokenKind::Min
            | TokenKind::Max
            | TokenKind::Retention
            | TokenKind::Send
            | TokenKind::To
            | TokenKind::Within
            | TokenKind::CircuitBreaker
            | TokenKind::Promote => {
                let name = tok.kind.to_string();
                self.advance();
                Ok((name, tok.span))
            }
            _ => Err(ParseError::new(
                format!("expected identifier, got {}", tok.kind),
                tok.span,
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

    // ── Grammar rules ────────────────────────────────────────────────────────

    fn parse_file(&mut self) -> Result<ReinFile, ParseError> {
        self.skip_comments();
        let mut imports = Vec::new();
        let mut defaults = None;
        let mut providers = Vec::new();
        let mut tools = Vec::new();
        let mut archetypes = Vec::new();
        let mut agents = Vec::new();
        let mut workflows = Vec::new();
        let mut types = Vec::new();
        let mut policies = Vec::new();
        let mut observes = Vec::new();
        let mut fleets = Vec::new();
        let mut channels = Vec::new();
        let mut circuit_breakers = Vec::new();
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
                TokenKind::Archetype => archetypes.push(self.parse_archetype()?),
                TokenKind::Agent => agents.push(self.parse_agent()?),
                TokenKind::Workflow => workflows.push(self.parse_workflow()?),
                TokenKind::Type => types.push(self.parse_type_def()?),
                TokenKind::Policy => policies.push(self.parse_policy()?),
                TokenKind::Observe => observes.push(self.parse_observe()?),
                TokenKind::Fleet => fleets.push(self.parse_fleet()?),
                TokenKind::Channel => channels.push(self.parse_channel()?),
                TokenKind::CircuitBreaker => {
                    circuit_breakers.push(self.parse_circuit_breaker()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!(
                            "expected top-level declaration (defaults, provider, tool, archetype, agent, workflow, type, policy, observe, fleet, channel), got {other}"
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
            archetypes,
            agents,
            workflows,
            types,
            policies,
            observes,
            fleets,
            channels,
            circuit_breakers,
        })
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod eval_parser_tests;
#[cfg(test)]
mod memory_parser_tests;
#[cfg(test)]
mod schedule_parser_tests;
#[cfg(test)]
mod step_ext_tests;
#[cfg(test)]
mod secrets_parser_tests;
