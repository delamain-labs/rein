use crate::ast::{
    AutoResolveBlock, AutoResolveCondition, CompareOp, ExecutionMode, ParallelBlock, RouteArm,
    RouteBlock, RoutePattern, RouteRule, Span, Stage, StepDef, TypeExpr, WhenComparison,
    WithinBlock, WorkflowDef,
};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

#[derive(Default)]
struct WorkflowBody {
    trigger: Option<String>,
    stages: Vec<Stage>,
    steps: Vec<StepDef>,
    route_blocks: Vec<RouteBlock>,
    parallel_blocks: Vec<ParallelBlock>,
    auto_resolve: Option<AutoResolveBlock>,
    within_blocks: Vec<WithinBlock>,
}

impl Parser {
    pub(super) fn parse_workflow(&mut self) -> Result<WorkflowDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Workflow)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut body = WorkflowBody::default();
        self.parse_workflow_body(&name, &mut body)?;

        let end = self.current_span().end;
        self.advance(); // consume RBrace

        let trigger = body.trigger.ok_or_else(|| {
            ParseError::new(
                format!("workflow '{name}' is missing required field 'trigger'"),
                Span::new(start, end),
            )
        })?;

        if body.stages.is_empty()
            && body.steps.is_empty()
            && body.route_blocks.is_empty()
            && body.parallel_blocks.is_empty()
            && body.within_blocks.is_empty()
        {
            return Err(ParseError::new(
                format!(
                    "workflow '{name}' must have at least one stage, step, or route block"
                ),
                Span::new(start, end),
            ));
        }

        Ok(WorkflowDef {
            name,
            trigger,
            stages: body.stages,
            steps: body.steps,
            route_blocks: body.route_blocks,
            parallel_blocks: body.parallel_blocks,
            auto_resolve: body.auto_resolve,
            within_blocks: body.within_blocks,
            mode: ExecutionMode::Sequential,
            span: Span::new(start, end),
        })
    }

    fn parse_workflow_body(
        &mut self,
        name: &str,
        body: &mut WorkflowBody,
    ) -> Result<(), ParseError> {
        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => return Ok(()),
                TokenKind::Trigger => {
                    if body.trigger.is_some() {
                        return Err(ParseError::new(
                            format!("duplicate field 'trigger' in workflow '{name}'"),
                            self.current_span(),
                        ));
                    }
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    body.trigger = Some(self.parse_trigger_expr()?);
                }
                TokenKind::Stages => {
                    if !body.stages.is_empty() {
                        return Err(ParseError::new(
                            format!("duplicate field 'stages' in workflow '{name}'"),
                            self.current_span(),
                        ));
                    }
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    body.stages = self.parse_stage_list()?;
                }
                TokenKind::Step => body.steps.push(self.parse_step()?),
                TokenKind::Within => body.within_blocks.push(self.parse_within_block()?),
                TokenKind::Route => body.route_blocks.push(self.parse_route_block()?),
                TokenKind::Parallel => body.parallel_blocks.push(self.parse_parallel_block()?),
                TokenKind::Auto => {
                    if body.auto_resolve.is_some() {
                        return Err(ParseError::new(
                            format!("duplicate 'auto resolve when' in workflow '{name}'"),
                            self.current_span(),
                        ));
                    }
                    body.auto_resolve = Some(self.parse_auto_resolve()?);
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

    /// Parse `auto resolve when { condition, condition, ... }`.
    fn parse_auto_resolve(&mut self) -> Result<AutoResolveBlock, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Auto)?;
        self.expect(&TokenKind::Resolve)?;
        self.expect(&TokenKind::When)?;
        self.expect(&TokenKind::LBrace)?;

        let mut conditions = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBrace {
                break;
            }
            conditions.push(self.parse_auto_resolve_condition()?);
            // Optional comma separator
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(AutoResolveBlock {
            conditions,
            span: Span::new(start, end),
        })
    }

    /// Parse a single auto-resolve condition.
    fn parse_auto_resolve_condition(&mut self) -> Result<AutoResolveCondition, ParseError> {
        let (field, _) = self.expect_ident()?;

        // Check if next is `is one of [...]`
        if *self.peek() == TokenKind::Is {
            self.advance(); // is
            let type_expr = self.parse_one_of()?;
            match type_expr {
                TypeExpr::OneOf { variants, .. } => {
                    Ok(AutoResolveCondition::IsOneOf { field, variants })
                }
                _ => Err(ParseError::new(
                    "expected 'one of [...]' after 'is'",
                    self.current_span(),
                )),
            }
        } else {
            let op = match self.peek() {
                TokenKind::Lt => CompareOp::Lt,
                TokenKind::Gt => CompareOp::Gt,
                TokenKind::LtEq => CompareOp::LtEq,
                TokenKind::GtEq => CompareOp::GtEq,
                other => {
                    return Err(ParseError::new(
                        format!("expected comparison operator or 'is', got {other}"),
                        self.current_span(),
                    ));
                }
            };
            self.advance();
            let value = self.parse_when_value()?;
            Ok(AutoResolveCondition::Comparison(WhenComparison {
                field,
                op,
                value,
            }))
        }
    }

    /// Parse a `parallel { step a {...} step b {...} }` block.
    fn parse_parallel_block(&mut self) -> Result<ParallelBlock, ParseError> {
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

        Ok(ParallelBlock {
            steps,
            span: Span::new(start, end),
        })
    }

    /// Parse a `route on <field_path> { pattern -> step name { ... }, ... }` block.
    fn parse_route_block(&mut self) -> Result<RouteBlock, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Route)?;
        self.expect(&TokenKind::On)?;

        // Parse dot-separated field path (keywords allowed as segments)
        let (first, _) = self.expect_ident()?;
        let mut path = first;
        while *self.peek() == TokenKind::Dot {
            self.advance(); // .
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
            let pattern = if *self.peek() == TokenKind::Underscore {
                self.advance();
                RoutePattern::Wildcard
            } else {
                let (val, _) = self.expect_ident()?;
                RoutePattern::Value(val)
            };
            self.expect(&TokenKind::Arrow)?;
            let step = self.parse_step()?;
            let arm_end = self.current_span().start;
            arms.push(RouteArm {
                pattern,
                step,
                span: Span::new(arm_start, arm_end),
            });
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(RouteBlock {
            field_path: path,
            arms,
            span: Span::new(start, end),
        })
    }

    /// Parse a trigger expression: either a string literal or a sequence of
    /// identifiers (e.g., `new ticket in zendesk`).
    pub(super) fn parse_trigger_expr(&mut self) -> Result<String, ParseError> {
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
