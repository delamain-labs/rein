use crate::ast::{EscalateDef, Span, TypeExpr};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `for each: <collection>`.
    pub(super) fn parse_step_for_each(
        &mut self,
        name: &str,
        for_each: &mut Option<String>,
    ) -> Result<(), ParseError> {
        if for_each.is_some() {
            return Err(ParseError::new(
                format!("duplicate 'for each' in step '{name}'"),
                self.current_span(),
            ));
        }
        self.expect(&TokenKind::For)?;
        self.expect(&TokenKind::Each)?;
        self.expect(&TokenKind::Colon)?;
        let (collection, _) = self.expect_ident()?;
        *for_each = Some(collection);
        Ok(())
    }

    /// Parse `escalate to human via channel("destination")`.
    pub(super) fn parse_step_escalate(
        &mut self,
        name: &str,
        escalate: &mut Option<EscalateDef>,
    ) -> Result<(), ParseError> {
        if escalate.is_some() {
            return Err(ParseError::new(
                format!("duplicate 'escalate' in step '{name}'"),
                self.current_span(),
            ));
        }
        let start = self.current_span().start;
        self.expect(&TokenKind::Escalate)?;
        self.expect(&TokenKind::To)?;
        let (target, _) = self.expect_ident()?;
        self.expect(&TokenKind::Via)?;
        let (channel, _) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let destination = self.expect_string()?;
        let end_span = self.expect(&TokenKind::RParen)?;
        *escalate = Some(EscalateDef {
            target,
            channel,
            destination,
            span: Span::new(start, end_span.end),
        });
        Ok(())
    }

    /// Parse `output: <name>: <Type>`.
    pub(super) fn parse_step_typed_output(
        &mut self,
        _name: &str,
        typed_outputs: &mut Vec<(String, TypeExpr)>,
    ) -> Result<(), ParseError> {
        self.expect(&TokenKind::Output)?;
        self.expect(&TokenKind::Colon)?;
        let (field_name, _) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let type_expr = self.parse_type_expr_inline()?;
        typed_outputs.push((field_name, type_expr));
        Ok(())
    }

    /// Parse an inline type expression like `Product[]` or `String`.
    pub(super) fn parse_type_expr_inline(&mut self) -> Result<TypeExpr, ParseError> {
        let (type_name, _) = self.expect_ident()?;
        // Check for array suffix `[]`
        let array = if *self.peek() == TokenKind::LBracket {
            self.advance();
            self.expect(&TokenKind::RBracket)?;
            true
        } else {
            false
        };
        Ok(TypeExpr::Named {
            name: type_name,
            array,
        })
    }
}
