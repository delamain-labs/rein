use crate::ast::{
    BackoffStrategy, CompareOp, FailureAction, RetryPolicy, Span, StepDef, TypeExpr, ValueExpr,
    WhenComparison, WhenExpr, WhenValue,
};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

/// Accumulator for step block fields during parsing.
#[derive(Default)]
struct StepFields {
    agent: Option<String>,
    goal: Option<String>,
    output_constraints: Vec<(String, TypeExpr)>,
    when: Option<WhenExpr>,
    on_failure: Option<RetryPolicy>,
    fallback: Option<Box<StepDef>>,
    seen_agent: bool,
    seen_goal: bool,
}

impl Parser {
    /// Parse a step: either block form `step <name> { ... }` or inline
    /// shorthand `step <name>: <agent> goal "<text>"`.
    pub(super) fn parse_step(&mut self) -> Result<StepDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Step)?;
        let (name, _) = self.expect_ident()?;

        // Inline shorthand: `step name: agent goal "text"`
        if *self.peek() == TokenKind::Colon {
            return self.parse_inline_step(name, start);
        }

        self.expect(&TokenKind::LBrace)?;
        self.parse_step_block_body(name, start)
    }

    /// Parse the body of a block-form step (after the opening `{`).
    fn parse_step_block_body(
        &mut self,
        name: String,
        start: usize,
    ) -> Result<StepDef, ParseError> {
        let mut f = StepFields::default();

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    let agent = f.agent.ok_or_else(|| {
                        ParseError::new(
                            format!("step '{name}' is missing required field 'agent'"),
                            Span::new(start, end),
                        )
                    })?;
                    return Ok(StepDef {
                        name,
                        agent,
                        goal: f.goal,
                        output_constraints: f.output_constraints,
                        when: f.when,
                        on_failure: f.on_failure,
                        fallback: f.fallback,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Agent => {
                    self.parse_step_agent_field(&name, &mut f.agent, &mut f.seen_agent)?;
                }
                TokenKind::Goal => {
                    self.parse_step_goal_field(&name, &mut f.goal, &mut f.seen_goal)?;
                }
                TokenKind::On => {
                    self.parse_step_on_failure(&name, &mut f.on_failure)?;
                }
                TokenKind::When => {
                    self.parse_step_when(&name, &mut f.when)?;
                }
                TokenKind::Fallback => {
                    self.parse_step_fallback(&name, &mut f.fallback)?;
                }
                _ => {
                    self.parse_step_field_or_error(&name, &mut f.output_constraints)?;
                }
            }
        }
    }

    fn parse_step_on_failure(
        &mut self,
        name: &str,
        on_failure: &mut Option<RetryPolicy>,
    ) -> Result<(), ParseError> {
        if on_failure.is_some() {
            return Err(ParseError::new(
                format!("duplicate 'on failure' in step '{name}'"),
                self.current_span(),
            ));
        }
        *on_failure = Some(self.parse_retry_policy()?);
        Ok(())
    }

    fn parse_step_when(
        &mut self,
        name: &str,
        when: &mut Option<WhenExpr>,
    ) -> Result<(), ParseError> {
        if when.is_some() {
            return Err(ParseError::new(
                format!("duplicate field 'when' in step '{name}'"),
                self.current_span(),
            ));
        }
        self.advance();
        self.expect(&TokenKind::Colon)?;
        *when = Some(self.parse_when_expr()?);
        Ok(())
    }

    fn parse_step_fallback(
        &mut self,
        name: &str,
        fallback: &mut Option<Box<StepDef>>,
    ) -> Result<(), ParseError> {
        if fallback.is_some() {
            return Err(ParseError::new(
                format!("duplicate 'fallback' in step '{name}'"),
                self.current_span(),
            ));
        }
        self.advance();
        *fallback = Some(Box::new(self.parse_step()?));
        Ok(())
    }

    fn parse_step_field_or_error(
        &mut self,
        name: &str,
        output_constraints: &mut Vec<(String, TypeExpr)>,
    ) -> Result<(), ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(ref field_name)
                if self.peek_at(1).is_some_and(|t| *t == TokenKind::Colon) =>
            {
                let field_name = field_name.clone();
                self.advance();
                self.expect(&TokenKind::Colon)?;
                if *self.peek() == TokenKind::One {
                    let type_expr = self.parse_one_of()?;
                    output_constraints.push((field_name, type_expr));
                    Ok(())
                } else {
                    Err(ParseError::new(
                        format!("unexpected field '{field_name}' in step '{name}'"),
                        self.current_span(),
                    ))
                }
            }
            TokenKind::Eof => Err(ParseError::new(
                "unexpected end of file: expected `}`",
                self.current_span(),
            )),
            other => Err(ParseError::new(
                format!("unexpected token in step '{name}': {other}"),
                self.current_span(),
            )),
        }
    }

    /// Parse inline step shorthand: `step <name>: <agent> goal "<text>"`.
    /// The `step` keyword and name have already been consumed.
    fn parse_inline_step(&mut self, name: String, start: usize) -> Result<StepDef, ParseError> {
        self.expect(&TokenKind::Colon)?;
        let (agent, _) = self.expect_ident()?;

        let goal = if *self.peek() == TokenKind::Goal {
            self.advance();
            let value = self.parse_value_expr()?;
            Some(match value {
                ValueExpr::Literal(s) => s,
                ValueExpr::EnvRef { span, .. } => {
                    return Err(ParseError::new(
                        "goal must be a string literal, not env()",
                        span,
                    ));
                }
            })
        } else {
            None
        };

        let end = self.last_consumed_end;
        Ok(StepDef {
            name,
            agent,
            goal,
            output_constraints: Vec::new(),
            when: None,
            on_failure: None,
            fallback: None,
            span: Span::new(start, end),
        })
    }

    fn parse_step_agent_field(
        &mut self,
        step_name: &str,
        agent: &mut Option<String>,
        seen: &mut bool,
    ) -> Result<(), ParseError> {
        if *seen {
            return Err(ParseError::new(
                format!("duplicate field 'agent' in step '{step_name}'"),
                self.current_span(),
            ));
        }
        *seen = true;
        self.advance();
        self.expect(&TokenKind::Colon)?;
        let (value, _) = self.expect_ident()?;
        *agent = Some(value);
        Ok(())
    }

    fn parse_step_goal_field(
        &mut self,
        step_name: &str,
        goal: &mut Option<String>,
        seen: &mut bool,
    ) -> Result<(), ParseError> {
        if *seen {
            return Err(ParseError::new(
                format!("duplicate field 'goal' in step '{step_name}'"),
                self.current_span(),
            ));
        }
        *seen = true;
        self.advance();
        self.expect(&TokenKind::Colon)?;
        let value = self.parse_value_expr()?;
        *goal = Some(match value {
            ValueExpr::Literal(s) => s,
            ValueExpr::EnvRef { span, .. } => {
                return Err(ParseError::new(
                    "goal must be a string literal, not env()",
                    span,
                ));
            }
        });
        Ok(())
    }

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
            Ok(parts.into_iter().next().unwrap())
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
            Ok(parts.into_iter().next().unwrap())
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
            other => {
                return Err(ParseError::new(
                    format!("expected comparison operator (<, >, <=, >=), got {other}"),
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
