use crate::ast::{CompareOp, PipeExpr, PipeTransform, SortDirection, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse a pipe expression: `source | transform | transform ...`
    ///
    /// Called when we've already peeked and confirmed a pipe chain
    /// (identifier followed by `|`).
    pub(super) fn parse_pipe_expr(&mut self) -> Result<PipeExpr, ParseError> {
        let start = self.current_span().start;
        let (source, _) = self.expect_ident()?;

        let mut transforms = Vec::new();
        while *self.peek() == TokenKind::Pipe {
            self.advance(); // consume `|`
            transforms.push(self.parse_pipe_transform()?);
        }

        let end = self.last_consumed_end;
        Ok(PipeExpr {
            source,
            transforms,
            span: Span::new(start, end),
        })
    }

    fn parse_pipe_transform(&mut self) -> Result<PipeTransform, ParseError> {
        match self.peek().clone() {
            TokenKind::Where => self.parse_where_transform(),
            TokenKind::Sort => self.parse_sort_transform(),
            TokenKind::Take => self.parse_take_skip_transform(true),
            TokenKind::Skip => self.parse_take_skip_transform(false),
            TokenKind::Select => self.parse_select_transform(),
            TokenKind::Unique => self.parse_unique_transform(),
            other => Err(ParseError::new(
                format!(
                    "expected pipe transform (where, sort, take, skip, select, unique), got {other}"
                ),
                self.current_span(),
            )),
        }
    }

    /// `where field op value`
    fn parse_where_transform(&mut self) -> Result<PipeTransform, ParseError> {
        self.advance(); // consume `where`
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
                    format!("expected comparison operator in where clause, got {other}"),
                    self.current_span(),
                ));
            }
        };
        self.advance();
        let value = self.parse_when_value()?;
        Ok(PipeTransform::Where { field, op, value })
    }

    /// `sort by field [asc|desc]`
    fn parse_sort_transform(&mut self) -> Result<PipeTransform, ParseError> {
        self.advance(); // consume `sort`
        self.expect(&TokenKind::By)?;
        let (field, _) = self.expect_ident()?;
        let direction = match self.peek() {
            TokenKind::Asc => {
                self.advance();
                SortDirection::Asc
            }
            TokenKind::Desc => {
                self.advance();
                SortDirection::Desc
            }
            _ => SortDirection::Asc, // default
        };
        Ok(PipeTransform::SortBy { field, direction })
    }

    /// `take N` or `skip N`
    fn parse_take_skip_transform(&mut self, is_take: bool) -> Result<PipeTransform, ParseError> {
        self.advance(); // consume `take`/`skip`
        let count = self.parse_u32("count")?;
        if is_take {
            Ok(PipeTransform::Take { count })
        } else {
            Ok(PipeTransform::Skip { count })
        }
    }

    /// `select field1, field2, ...`
    fn parse_select_transform(&mut self) -> Result<PipeTransform, ParseError> {
        self.advance(); // consume `select`
        let mut fields = Vec::new();
        let (first, _) = self.expect_ident()?;
        fields.push(first);
        while *self.peek() == TokenKind::Comma {
            self.advance();
            let (f, _) = self.expect_ident()?;
            fields.push(f);
        }
        Ok(PipeTransform::Select { fields })
    }

    /// `unique` or `unique field`
    fn parse_unique_transform(&mut self) -> Result<PipeTransform, ParseError> {
        self.advance(); // consume `unique`
        // Optional field — only consume if next is an ident (not pipe, rbrace, etc.)
        let field = if matches!(self.peek(), TokenKind::Ident(_)) {
            let (f, _) = self.expect_ident()?;
            Some(f)
        } else {
            None
        };
        Ok(PipeTransform::Unique { field })
    }

    /// Parse a u32 value from a Number token.
    pub(super) fn parse_u32(&mut self, label: &str) -> Result<u32, ParseError> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                let val = n.parse::<u32>().map_err(|_| {
                    ParseError::new(format!("invalid {label}: {n}"), self.current_span())
                })?;
                self.advance();
                Ok(val)
            }
            other => Err(ParseError::new(
                format!("expected {label} (number), got {other}"),
                self.current_span(),
            )),
        }
    }
}
