use crate::ast::{FleetDef, ScalingConfig, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `fleet <name> { agents: [...], policy: ..., budget: ..., scaling { min: N, max: N } }`.
    pub(super) fn parse_fleet(&mut self) -> Result<FleetDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Fleet)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut agents = Vec::new();
        let mut policy = None;
        let mut budget = None;
        let mut scaling = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Agents => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    agents = self.parse_ident_list()?;
                }
                TokenKind::Policy => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    let (val, _) = self.expect_ident()?;
                    policy = Some(val);
                }
                TokenKind::Budget => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    let (amount, _sym, _span) = self.expect_currency()?;
                    budget = Some(amount);
                    // Optional "/day" or "per day" suffix — skip it
                    if *self.peek() == TokenKind::Slash || *self.peek() == TokenKind::Per {
                        self.advance();
                        let _ = self.expect_ident(); // "day", etc.
                    }
                }
                TokenKind::Scaling => {
                    self.advance();
                    scaling = Some(self.parse_scaling_config()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in fleet block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in fleet '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
        let end = self.current_span().end;
        self.advance();

        Ok(FleetDef {
            name,
            agents,
            policy,
            budget,
            scaling,
            span: Span::new(start, end),
        })
    }

    fn parse_scaling_config(&mut self) -> Result<ScalingConfig, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::LBrace)?;

        let mut min = None;
        let mut max = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Min => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    min = Some(self.parse_u32("min")?);
                }
                TokenKind::Max => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    max = Some(self.parse_u32("max")?);
                }
                TokenKind::Comma => {
                    self.advance();
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in scaling config: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
        let end = self.current_span().end;
        self.advance();

        let min = min.ok_or_else(|| {
            ParseError::new("scaling config missing 'min'", Span::new(start, end))
        })?;
        let max = max.ok_or_else(|| {
            ParseError::new("scaling config missing 'max'", Span::new(start, end))
        })?;

        Ok(ScalingConfig {
            min,
            max,
            span: Span::new(start, end),
        })
    }
}
