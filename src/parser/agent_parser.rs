use crate::ast::{
    AgentDef, ArchetypeDef, Budget, Capability, GuardrailRule, GuardrailSection, GuardrailsDef,
    Span, ValueExpr,
};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    pub(super) fn parse_archetype(&mut self) -> Result<ArchetypeDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Archetype)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let (model, can, cannot, budget, guardrails) = self.parse_agent_body(&name)?;

        let end = self.current_span().end;
        self.advance(); // consume RBrace

        Ok(ArchetypeDef {
            name,
            model,
            can,
            cannot,
            budget,
            guardrails,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_agent(&mut self) -> Result<AgentDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Agent)?;
        let (name, _) = self.expect_ident()?;

        // Optional `from <archetype>` clause
        let from = if *self.peek() == TokenKind::From {
            self.advance();
            let (archetype_name, _) = self.expect_ident()?;
            Some(archetype_name)
        } else {
            None
        };

        self.expect(&TokenKind::LBrace)?;

        let (model, can, cannot, budget, guardrails) = self.parse_agent_body(&name)?;

        let end = self.current_span().end;
        self.advance(); // consume RBrace

        Ok(AgentDef {
            name,
            from,
            model,
            can,
            cannot,
            budget,
            guardrails,
            span: Span::new(start, end),
        })
    }

    /// Parse the body fields shared by both `agent` and `archetype` blocks.
    /// Expects the opening `{` has already been consumed.
    /// Returns when `}` is encountered (but does NOT consume it).
    #[allow(clippy::type_complexity)]
    fn parse_agent_body(
        &mut self,
        name: &str,
    ) -> Result<
        (
            Option<ValueExpr>,
            Vec<Capability>,
            Vec<Capability>,
            Option<Budget>,
            Option<GuardrailsDef>,
        ),
        ParseError,
    > {
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
                    return Ok((model, can, cannot, budget, guardrails));
                }
                TokenKind::Model => {
                    if seen_model {
                        return Err(ParseError::new(
                            format!("duplicate field 'model' in '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_model = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    model = Some(self.parse_value_expr()?);
                }
                TokenKind::Can => {
                    if seen_can {
                        return Err(ParseError::new(
                            format!("duplicate field 'can' in '{name}'"),
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
                            format!("duplicate field 'cannot' in '{name}'"),
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
                            format!("duplicate field 'budget' in '{name}'"),
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
                            format!("duplicate field 'guardrails' in '{name}'"),
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
                        format!("unexpected token in body of '{name}': {other}"),
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
}
