use crate::ast::{
    EscalateDef, PipeExpr, RetryPolicy, SendTarget, Span, StepDef, TypeExpr, ValueExpr, WhenExpr,
};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

/// Accumulator for step block fields during parsing.
#[derive(Default)]
struct StepFields {
    agent: Option<String>,
    goal: Option<String>,
    input: Option<PipeExpr>,
    send_to: Option<SendTarget>,
    output_constraints: Vec<(String, TypeExpr)>,
    depends_on: Vec<String>,
    when: Option<WhenExpr>,
    on_failure: Option<RetryPolicy>,
    fallback: Option<Box<StepDef>>,
    for_each: Option<String>,
    typed_input: Option<String>,
    typed_outputs: Vec<(String, TypeExpr)>,
    escalate: Option<EscalateDef>,
    approval: Option<crate::ast::ApprovalDef>,
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
    fn parse_step_block_body(&mut self, name: String, start: usize) -> Result<StepDef, ParseError> {
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
                        input: f.input,
                        send_to: f.send_to,
                        output_constraints: f.output_constraints,
                        depends_on: f.depends_on,
                        when: f.when,
                        on_failure: f.on_failure,
                        fallback: f.fallback,
                        for_each: f.for_each,
                        typed_input: f.typed_input,
                        typed_outputs: f.typed_outputs,
                        escalate: f.escalate,
                        approval: f.approval,
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
                TokenKind::Send => {
                    self.parse_step_send_to(&name, &mut f.send_to)?;
                }
                TokenKind::Fallback => {
                    self.parse_step_fallback(&name, &mut f.fallback)?;
                }
                TokenKind::For => {
                    self.parse_step_for_each(&name, &mut f.for_each)?;
                }
                TokenKind::Escalate => {
                    self.parse_step_escalate(&name, &mut f.escalate)?;
                }
                TokenKind::Input => {
                    self.advance(); // consume `input`
                    self.expect(&TokenKind::Colon)?;
                    self.parse_step_input_field(&name, &mut f.input, &mut f.typed_input)?;
                }
                TokenKind::Output => {
                    self.parse_step_typed_output(&name, &mut f.typed_outputs)?;
                }
                TokenKind::Approve | TokenKind::Collaborate => {
                    if f.approval.is_some() {
                        return Err(ParseError::new(
                            format!("duplicate approval in step '{name}'"),
                            self.current_span(),
                        ));
                    }
                    f.approval = Some(self.parse_approval()?);
                }
                _ => {
                    self.parse_step_field_or_error(&name, &mut f)?;
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

    fn parse_step_send_to(
        &mut self,
        name: &str,
        send_to: &mut Option<SendTarget>,
    ) -> Result<(), ParseError> {
        if send_to.is_some() {
            return Err(ParseError::new(
                format!("duplicate 'send to' in step '{name}'"),
                self.current_span(),
            ));
        }
        let start = self.current_span().start;
        self.advance(); // consume `send`
        self.expect(&TokenKind::To)?;
        self.expect(&TokenKind::Colon)?;

        // Parse target: could be `slack(#channel)` or a string literal
        let target = if let TokenKind::StringLiteral(s) = self.peek().clone() {
            self.advance();
            s
        } else {
            let (name_part, _) = self.expect_ident()?;
            if *self.peek() == TokenKind::LParen {
                self.advance();
                let mut inner = String::new();
                // Consume everything until RParen
                while *self.peek() != TokenKind::RParen {
                    inner.push_str(&self.peek().to_string());
                    self.advance();
                }
                self.advance(); // RParen
                format!("{name_part}({inner})")
            } else {
                name_part
            }
        };

        // Optional message field
        let message = if let TokenKind::Ident(ref kw) = self.peek().clone() {
            if kw == "message" {
                self.advance();
                self.expect(&TokenKind::Colon)?;
                if let TokenKind::StringLiteral(s) = self.peek().clone() {
                    self.advance();
                    Some(s)
                } else {
                    return Err(ParseError::new(
                        "message must be a string literal",
                        self.current_span(),
                    ));
                }
            } else {
                None
            }
        } else {
            None
        };

        let end = self.last_consumed_end;
        *send_to = Some(SendTarget {
            target,
            message,
            span: Span::new(start, end),
        });
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
        fields: &mut StepFields,
    ) -> Result<(), ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(ref field_name)
                if self.peek_at(1).is_some_and(|t| *t == TokenKind::Colon) =>
            {
                let field_name = field_name.clone();
                if field_name == "input" {
                    if fields.input.is_some() {
                        return Err(ParseError::new(
                            format!("duplicate field 'input' in step '{name}'"),
                            self.current_span(),
                        ));
                    }
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    fields.input = Some(self.parse_pipe_expr()?);
                    return Ok(());
                }
                if field_name == "depends_on" {
                    if !fields.depends_on.is_empty() {
                        return Err(ParseError::new(
                            format!("duplicate field 'depends_on' in step '{name}'"),
                            self.current_span(),
                        ));
                    }
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    fields.depends_on = self.parse_depends_on_value()?;
                    return Ok(());
                }
                self.advance();
                self.expect(&TokenKind::Colon)?;
                if *self.peek() == TokenKind::One {
                    let type_expr = self.parse_one_of()?;
                    fields.output_constraints.push((field_name, type_expr));
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
            input: None,
            send_to: None,
            output_constraints: Vec::new(),
            depends_on: Vec::new(),
            when: None,
            on_failure: None,
            fallback: None,
            for_each: None,
            typed_input: None,
            typed_outputs: Vec::new(),
            escalate: None,
            approval: None,
            span: Span::new(start, end),
        })
    }

    /// Parse `depends_on: step_name` or `depends_on: [step_a, step_b]`.
    fn parse_depends_on_value(&mut self) -> Result<Vec<String>, ParseError> {
        if *self.peek() == TokenKind::LBracket {
            self.parse_ident_list()
        } else {
            let (name, _) = self.expect_ident()?;
            Ok(vec![name])
        }
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
}
