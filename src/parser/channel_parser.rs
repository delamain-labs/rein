use crate::ast::{ChannelDef, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `channel <name> { type: ..., retention: ... }`.
    pub(super) fn parse_channel(&mut self) -> Result<ChannelDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Channel)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut message_type = None;
        let mut retention = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Type => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    let (type_name, _) = self.expect_ident()?;
                    // Handle array suffix `[]`
                    let type_str = if *self.peek() == TokenKind::LBracket {
                        self.advance();
                        self.expect(&TokenKind::RBracket)?;
                        format!("{type_name}[]")
                    } else {
                        type_name
                    };
                    message_type = Some(type_str);
                }
                TokenKind::Retention => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    retention = Some(self.parse_duration_text()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in channel block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in channel '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
        let end = self.current_span().end;
        self.advance();

        Ok(ChannelDef {
            name,
            message_type,
            retention,
            span: Span::new(start, end),
        })
    }

    /// Parse a duration-like text: `7 days`, `24 hours`, etc.
    fn parse_duration_text(&mut self) -> Result<String, ParseError> {
        // Try string literal first
        if let TokenKind::StringLiteral(s) = self.peek().clone() {
            self.advance();
            return Ok(s);
        }
        if let TokenKind::Number(n) = self.peek().clone() {
            self.advance();
            let (unit, _) = self.expect_ident()?;
            Ok(format!("{n} {unit}"))
        } else {
            let (text, _) = self.expect_ident()?;
            Ok(text)
        }
    }
}
