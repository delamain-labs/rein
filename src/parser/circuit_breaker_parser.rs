use crate::ast::{CircuitBreakerDef, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `circuit_breaker <name> { open after: N failures in M min, half_open after: N min }`.
    pub(super) fn parse_circuit_breaker(&mut self) -> Result<CircuitBreakerDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::CircuitBreaker)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut failure_threshold = None;
        let mut window_minutes = None;
        let mut half_open_after = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                // `open after: N failures in M min`
                TokenKind::Ident(ref kw) if kw == "open" => {
                    self.advance();
                    self.expect_ident_matching("after")?;
                    self.expect(&TokenKind::Colon)?;
                    failure_threshold = Some(self.parse_u32("failure threshold")?);
                    self.expect_ident_matching("failures")?;
                    self.expect_ident_matching("in")?;
                    window_minutes = Some(self.parse_u32("window minutes")?);
                    self.expect_ident_matching("min")?;
                }
                // `half_open after: N min`
                TokenKind::Ident(ref kw) if kw == "half_open" => {
                    self.advance();
                    self.expect_ident_matching("after")?;
                    self.expect(&TokenKind::Colon)?;
                    half_open_after = Some(self.parse_u32("half_open minutes")?);
                    self.expect_ident_matching("min")?;
                }
                TokenKind::Comma => {
                    self.advance();
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in circuit_breaker '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
        let end = self.current_span().end;
        self.advance();

        let failure_threshold = failure_threshold.ok_or_else(|| {
            ParseError::new(
                format!("circuit_breaker '{name}' missing 'open after'"),
                Span::new(start, end),
            )
        })?;
        let window_minutes = window_minutes.ok_or_else(|| {
            ParseError::new(
                format!("circuit_breaker '{name}' missing window"),
                Span::new(start, end),
            )
        })?;
        let half_open_after_minutes = half_open_after.ok_or_else(|| {
            ParseError::new(
                format!("circuit_breaker '{name}' missing 'half_open after'"),
                Span::new(start, end),
            )
        })?;

        Ok(CircuitBreakerDef {
            name,
            failure_threshold,
            window_minutes,
            half_open_after_minutes,
            span: Span::new(start, end),
        })
    }

    /// Expect the current token to be an Ident matching a specific string.
    pub(super) fn expect_ident_matching(&mut self, expected: &str) -> Result<(), ParseError> {
        let (name, _) = self.expect_ident()?;
        if name != expected {
            return Err(ParseError::new(
                format!("expected '{expected}', got '{name}'"),
                Span::new(self.last_consumed_end - name.len(), self.last_consumed_end),
            ));
        }
        Ok(())
    }
}
