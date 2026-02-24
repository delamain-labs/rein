use crate::ast::{Span, WithinBlock};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `within(cost: $0.05, latency: 2s) { step ... }`.
    pub(super) fn parse_within_block(&mut self) -> Result<WithinBlock, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Within)?;
        self.expect(&TokenKind::LParen)?;

        let mut cost = None;
        let mut latency = None;

        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RParen {
                self.advance();
                break;
            }
            let (field, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            match field.as_str() {
                "cost" => {
                    let (amount, _sym, _span) = self.expect_currency()?;
                    cost = Some(amount);
                }
                "latency" => {
                    latency = Some(self.parse_duration_token()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected constraint '{other}' in within()"),
                        self.current_span(),
                    ));
                }
            }
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }

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
        self.advance();

        Ok(WithinBlock {
            cost,
            latency,
            steps,
            span: Span::new(start, end),
        })
    }

    /// Parse a duration-like token: `2s`, `500ms`, a number followed by ident.
    pub(super) fn parse_duration_token(&mut self) -> Result<String, ParseError> {
        if let TokenKind::StringLiteral(s) = self.peek().clone() {
            self.advance();
            return Ok(s);
        }
        // Try number + unit (e.g. `2 s` or just an ident like `2s`)
        if let TokenKind::Number(n) = self.peek().clone() {
            self.advance();
            // Next might be an ident unit
            if let TokenKind::Ident(_) = self.peek() {
                let (unit, _) = self.expect_ident()?;
                return Ok(format!("{n}{unit}"));
            }
            return Ok(n);
        }
        let (text, _) = self.expect_ident()?;
        Ok(text)
    }
}
