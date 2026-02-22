use crate::ast::{
    BackoffStrategy, CompareOp, FailureAction, RetryPolicy, WhenComparison, WhenExpr, WhenValue,
};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `on failure: retry N strategy then action`.
    pub(super) fn parse_retry_policy(&mut self) -> Result<RetryPolicy, ParseError> {
        self.expect(&TokenKind::On)?;
        self.expect(&TokenKind::Failure)?;
        self.expect(&TokenKind::Colon)?;
        self.expect(&TokenKind::Retry)?;

        let max_retries = self.parse_retry_count()?;
        let backoff = self.parse_backoff_strategy()?;

        self.expect(&TokenKind::Then)?;

        let then = self.parse_failure_action()?;

        Ok(RetryPolicy {
            max_retries,
            backoff,
            then,
        })
    }

    fn parse_retry_count(&mut self) -> Result<u32, ParseError> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                let val = n.parse::<u32>().map_err(|_| {
                    ParseError::new(format!("invalid retry count: {n}"), self.current_span())
                })?;
                self.advance();
                Ok(val)
            }
            other => Err(ParseError::new(
                format!("expected retry count (number), got {other}"),
                self.current_span(),
            )),
        }
    }

    fn parse_backoff_strategy(&mut self) -> Result<BackoffStrategy, ParseError> {
        match self.peek() {
            TokenKind::Exponential => {
                self.advance();
                Ok(BackoffStrategy::Exponential)
            }
            TokenKind::Linear => {
                self.advance();
                Ok(BackoffStrategy::Linear)
            }
            TokenKind::Fixed => {
                self.advance();
                Ok(BackoffStrategy::Fixed)
            }
            other => Err(ParseError::new(
                format!("expected backoff strategy (exponential, linear, fixed), got {other}"),
                self.current_span(),
            )),
        }
    }

    fn parse_failure_action(&mut self) -> Result<FailureAction, ParseError> {
        match self.peek() {
            TokenKind::Escalate => {
                self.advance();
                Ok(FailureAction::Escalate)
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(FailureAction::Step(name))
            }
            other => Err(ParseError::new(
                format!("expected failure action (escalate or step name), got {other}"),
                self.current_span(),
            )),
        }
    }

    /// Parse a `when` expression with proper precedence: `and` binds tighter than `or`.
    ///
    /// Grammar:
    ///   `or_expr`  = `and_expr` (`or` `and_expr`)*
    ///   `and_expr` = comparison (`and` comparison)*
    pub(super) fn parse_when_expr(&mut self) -> Result<WhenExpr, ParseError> {
        self.parse_when_or_expr()
    }

    fn parse_when_or_expr(&mut self) -> Result<WhenExpr, ParseError> {
        let first = self.parse_when_and_expr()?;
        let mut parts = vec![first];

        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::Or {
                self.advance();
                parts.push(self.parse_when_and_expr()?);
            } else {
                break;
            }
        }

        if parts.len() == 1 {
            Ok(parts.into_iter().next().expect("parts vec must have at least one element"))
        } else {
            Ok(WhenExpr::Or(parts))
        }
    }

    fn parse_when_and_expr(&mut self) -> Result<WhenExpr, ParseError> {
        let first = self.parse_when_comparison()?;
        let mut parts = vec![WhenExpr::Comparison(first)];

        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::And {
                self.advance();
                let next = self.parse_when_comparison()?;
                parts.push(WhenExpr::Comparison(next));
            } else {
                break;
            }
        }

        if parts.len() == 1 {
            Ok(parts.into_iter().next().expect("parts vec must have at least one element"))
        } else {
            Ok(WhenExpr::And(parts))
        }
    }

    /// Parse a single comparison: `field op value`.
    fn parse_when_comparison(&mut self) -> Result<WhenComparison, ParseError> {
        let (field, _) = self.expect_ident()?;

        let op = match self.peek() {
            TokenKind::Lt => CompareOp::Lt,
            TokenKind::Gt => CompareOp::Gt,
            TokenKind::LtEq => CompareOp::LtEq,
            TokenKind::GtEq => CompareOp::GtEq,
            TokenKind::EqEq => CompareOp::Eq,
            TokenKind::BangEq => CompareOp::NotEq,
            other => {
                return Err(ParseError::new(
                    format!("expected comparison operator (<, >, <=, >=, ==, !=), got {other}"),
                    self.current_span(),
                ));
            }
        };
        self.advance();

        let value = self.parse_when_value()?;

        Ok(WhenComparison { field, op, value })
    }

    /// Parse a when value: number, percent, currency, or ident.
    pub(super) fn parse_when_value(&mut self) -> Result<WhenValue, ParseError> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                let n = n.clone();
                self.advance();
                // Check for trailing %
                if *self.peek() == TokenKind::Percent {
                    self.advance();
                    Ok(WhenValue::Percent(n))
                } else {
                    Ok(WhenValue::Number(n))
                }
            }
            TokenKind::Currency { symbol, amount } => {
                self.advance();
                Ok(WhenValue::Currency { symbol, amount })
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(WhenValue::String(s))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(WhenValue::Ident(name))
            }
            other => Err(ParseError::new(
                format!(
                    "expected value (number, percentage, currency, or identifier), got {other}"
                ),
                self.current_span(),
            )),
        }
    }
}
