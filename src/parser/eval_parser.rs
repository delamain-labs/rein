use crate::ast::{CompareOp, EvalAssertion, EvalDef, EvalFailureAction, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `eval [name] { dataset: ..., assert ..., on failure: ... }`.
    pub(super) fn parse_eval(&mut self) -> Result<EvalDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Eval)?;

        // Optional name
        let name = if matches!(self.peek(), TokenKind::Ident(_)) {
            let (n, _) = self.expect_ident()?;
            Some(n)
        } else {
            None
        };

        self.expect(&TokenKind::LBrace)?;

        let mut dataset = None;
        let mut assertions = Vec::new();
        let mut on_failure = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Dataset => {
                    if dataset.is_some() {
                        return Err(ParseError::new(
                            "duplicate 'dataset' in eval block",
                            self.current_span(),
                        ));
                    }
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    dataset = Some(self.parse_eval_path_or_string()?);
                }
                TokenKind::Assert => {
                    assertions.push(self.parse_eval_assertion()?);
                }
                TokenKind::On => {
                    if on_failure.is_some() {
                        return Err(ParseError::new(
                            "duplicate 'on failure' in eval block",
                            self.current_span(),
                        ));
                    }
                    on_failure = Some(self.parse_eval_on_failure()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in eval block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in eval block: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }

        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        let dataset = dataset.ok_or_else(|| {
            ParseError::new(
                "eval block is missing required 'dataset' field",
                Span::new(start, end),
            )
        })?;

        Ok(EvalDef {
            name,
            dataset,
            assertions,
            on_failure,
            span: Span::new(start, end),
        })
    }

    /// Parse a dataset path: either a string literal or dot-slash path tokens.
    fn parse_eval_path_or_string(&mut self) -> Result<String, ParseError> {
        if let TokenKind::StringLiteral(s) = self.peek().clone() {
            self.advance();
            return Ok(s);
        }
        // Parse as a sequence of path-like tokens: ./evals/data.yaml
        let mut path = String::new();
        if *self.peek() == TokenKind::Dot {
            path.push('.');
            self.advance();
        }
        if *self.peek() == TokenKind::Slash {
            path.push('/');
            self.advance();
        }
        loop {
            match self.peek().clone() {
                TokenKind::Ident(s) => {
                    path.push_str(&s);
                    self.advance();
                }
                TokenKind::Dot => {
                    path.push('.');
                    self.advance();
                }
                TokenKind::Slash => {
                    path.push('/');
                    self.advance();
                }
                _ => break,
            }
        }
        if path.is_empty() {
            return Err(ParseError::new(
                "expected dataset path or string",
                self.current_span(),
            ));
        }
        Ok(path)
    }

    /// Parse `assert <metric> <op> <value>`.
    fn parse_eval_assertion(&mut self) -> Result<EvalAssertion, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Assert)?;
        let (metric, _) = self.expect_ident()?;
        let op = match self.peek() {
            TokenKind::Gt => CompareOp::Gt,
            TokenKind::Lt => CompareOp::Lt,
            TokenKind::GtEq => CompareOp::GtEq,
            TokenKind::LtEq => CompareOp::LtEq,
            TokenKind::EqEq => CompareOp::Eq,
            TokenKind::BangEq => CompareOp::NotEq,
            other => {
                return Err(ParseError::new(
                    format!("expected comparison operator in assert, got {other}"),
                    self.current_span(),
                ));
            }
        };
        self.advance();

        // Parse value: number optionally followed by %
        let value = self.parse_eval_value()?;
        let end = self.last_consumed_end;

        Ok(EvalAssertion {
            metric,
            op,
            value,
            span: Span::new(start, end),
        })
    }

    /// Parse a value in an assertion (e.g. `90%`, `0.95`, `100`).
    fn parse_eval_value(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.advance();
                if *self.peek() == TokenKind::Percent {
                    self.advance();
                    Ok(format!("{n}%"))
                } else {
                    Ok(n)
                }
            }
            other => Err(ParseError::new(
                format!("expected number in assertion, got {other}"),
                self.current_span(),
            )),
        }
    }

    /// Parse `on failure: block deploy` or `on failure: escalate`.
    fn parse_eval_on_failure(&mut self) -> Result<EvalFailureAction, ParseError> {
        self.expect(&TokenKind::On)?;
        self.expect(&TokenKind::Failure)?;
        self.expect(&TokenKind::Colon)?;
        match self.peek().clone() {
            TokenKind::Block => {
                self.advance();
                let (target, _) = self.expect_ident()?;
                Ok(EvalFailureAction::Block { target })
            }
            TokenKind::Escalate => {
                self.advance();
                Ok(EvalFailureAction::Escalate)
            }
            other => Err(ParseError::new(
                format!("expected 'block' or 'escalate' after 'on failure:', got {other}"),
                self.current_span(),
            )),
        }
    }
}
