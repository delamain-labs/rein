use crate::ast::{Span, TypeDef, TypeExpr, TypeField};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse a `type Name { field: type_expr, ... }` definition.
    pub(super) fn parse_type_def(&mut self) -> Result<TypeDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Type)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBrace {
                break;
            }
            let field_start = self.current_span().start;
            let (field_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let type_expr = self.parse_type_expr()?;
            let field_end = self.current_span().start;
            fields.push(TypeField {
                name: field_name,
                type_expr,
                span: Span::new(field_start, field_end),
            });
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(TypeDef {
            name,
            fields,
            span: Span::new(start, end),
        })
    }

    /// Parse a type expression: named type, array, one of, or range.
    pub(super) fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        self.skip_comments();
        match self.peek().clone() {
            TokenKind::One => self.parse_one_of(),
            TokenKind::Number(n) => {
                let min = n.clone();
                self.advance();
                self.expect(&TokenKind::DotDot)?;
                match self.peek().clone() {
                    TokenKind::Number(max) => {
                        let max = max.clone();
                        self.advance();
                        Ok(TypeExpr::Range { min, max })
                    }
                    other => Err(ParseError::new(
                        format!("expected number after '..', got {other}"),
                        self.current_span(),
                    )),
                }
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                // Check for array syntax: Type[]
                let array = if *self.peek() == TokenKind::LBracket
                    && self.peek_at(1).is_some_and(|t| *t == TokenKind::RBracket)
                {
                    self.advance(); // [
                    self.advance(); // ]
                    true
                } else {
                    false
                };
                Ok(TypeExpr::Named { name, array })
            }
            other => Err(ParseError::new(
                format!("expected type expression, got {other}"),
                self.current_span(),
            )),
        }
    }

    /// Parse `one of [a, b, c]` type expression.
    pub(super) fn parse_one_of(&mut self) -> Result<TypeExpr, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::One)?;
        self.expect(&TokenKind::Of)?;
        self.expect(&TokenKind::LBracket)?;

        let mut variants = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBracket {
                break;
            }
            let (variant, _) = self.expect_ident()?;
            variants.push(variant);
            self.skip_comments();
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }
        let end = self.current_span().end;
        self.expect(&TokenKind::RBracket)?;

        if variants.is_empty() {
            return Err(ParseError::new(
                "one of requires at least one variant",
                Span::new(start, end),
            ));
        }

        Ok(TypeExpr::OneOf {
            variants,
            span: Span::new(start, end),
        })
    }
}
