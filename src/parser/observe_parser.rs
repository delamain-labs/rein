use crate::ast::{ObserveDef, Span, WhenExpr};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `observe <name> { trace: ..., metrics: [...], alert when { ... }, export: ... }`.
    pub(super) fn parse_observe(&mut self) -> Result<ObserveDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Observe)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut trace = None;
        let mut metrics = Vec::new();
        let mut alert_when: Option<WhenExpr> = None;
        let mut export = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Trace => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    trace = Some(self.parse_text_until_newline_boundary()?);
                }
                TokenKind::Metrics => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    metrics = self.parse_ident_list()?;
                }
                TokenKind::Alert => {
                    self.advance();
                    self.expect(&TokenKind::When)?;
                    self.expect(&TokenKind::LBrace)?;
                    alert_when = Some(self.parse_when_expr()?);
                    self.expect(&TokenKind::RBrace)?;
                }
                TokenKind::Export => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    let (val, _) = self.expect_ident()?;
                    export = Some(val);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in observe block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in observe '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
        let end = self.current_span().end;
        self.advance(); // consume RBrace

        Ok(ObserveDef {
            name,
            trace,
            metrics,
            alert_when,
            export,
            span: Span::new(start, end),
        })
    }

    /// Parse a `[ident, ident, ...]` list.
    pub(super) fn parse_ident_list(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&TokenKind::LBracket)?;
        let mut items = Vec::new();
        loop {
            self.skip_comments();
            if *self.peek() == TokenKind::RBracket {
                self.advance();
                break;
            }
            let (name, _) = self.expect_ident()?;
            items.push(name);
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }
        Ok(items)
    }

    /// Consume identifiers until we hit a token that can't be part of a
    /// free-form text value (like `}`, `[`, keywords that start new fields).
    fn parse_text_until_newline_boundary(&mut self) -> Result<String, ParseError> {
        self.skip_comments();
        // Try string literal first
        if let TokenKind::StringLiteral(s) = self.peek().clone() {
            self.advance();
            return Ok(s);
        }
        let mut parts = Vec::new();
        while matches!(
            self.peek(),
            TokenKind::Ident(_) | TokenKind::All | TokenKind::Step | TokenKind::Stages
        ) {
            let text = self.peek().to_string();
            parts.push(text);
            self.advance();
        }
        if parts.is_empty() {
            return Err(ParseError::new("expected value", self.current_span()));
        }
        Ok(parts.join(" "))
    }
}
