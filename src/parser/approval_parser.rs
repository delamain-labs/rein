use crate::ast::{ApprovalDef, ApprovalKind, CollaborationMode, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `approve: human via channel("dest") timeout 4h`
    /// or `collaborate: human via channel("dest") { mode: edit }`
    pub(super) fn parse_approval(&mut self) -> Result<ApprovalDef, ParseError> {
        let start = self.current_span().start;

        let kind = match self.peek() {
            TokenKind::Approve => {
                self.advance();
                ApprovalKind::Approve
            }
            TokenKind::Collaborate => {
                self.advance();
                ApprovalKind::Collaborate
            }
            _ => {
                return Err(ParseError::new(
                    "expected 'approve' or 'collaborate'",
                    self.current_span(),
                ));
            }
        };

        self.expect(&TokenKind::Colon)?;
        // Expect "human"
        let (target, _) = self.expect_ident()?;
        if target != "human" {
            return Err(ParseError::new(
                format!("expected 'human' after '{kind:?}:', got '{target}'"),
                self.current_span(),
            ));
        }

        self.expect(&TokenKind::Via)?;
        let (channel, _) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let destination = self.expect_string()?;
        let end = self.expect(&TokenKind::RParen)?;

        let mut timeout = None;
        let mut mode = None;

        // Optional timeout (inline form: `timeout 4h`)
        if *self.peek() == TokenKind::Timeout {
            self.advance();
            let t = self.expect_string()?;
            timeout = Some(t);
        }

        // Optional body block for collaborate
        if *self.peek() == TokenKind::LBrace {
            self.advance();
            while *self.peek() != TokenKind::RBrace {
                match self.peek() {
                    TokenKind::Mode => {
                        self.advance();
                        self.expect(&TokenKind::Colon)?;
                        mode = Some(self.parse_collaboration_mode()?);
                    }
                    TokenKind::Timeout => {
                        self.advance();
                        self.expect(&TokenKind::Colon)?;
                        let t = self.expect_string()?;
                        timeout = Some(t);
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("unexpected token in approval block: {}", self.peek()),
                            self.current_span(),
                        ));
                    }
                }
            }
            self.advance(); // consume RBrace
        }

        Ok(ApprovalDef {
            kind,
            channel,
            destination,
            timeout,
            mode,
            span: Span::new(start, end.end),
        })
    }

    fn parse_collaboration_mode(&mut self) -> Result<CollaborationMode, ParseError> {
        match self.peek() {
            TokenKind::Edit => {
                self.advance();
                Ok(CollaborationMode::Edit)
            }
            TokenKind::Suggest => {
                self.advance();
                Ok(CollaborationMode::Suggest)
            }
            TokenKind::Review => {
                self.advance();
                Ok(CollaborationMode::Review)
            }
            _ => Err(ParseError::new(
                format!(
                    "expected collaboration mode (edit, suggest, review), got {}",
                    self.peek()
                ),
                self.current_span(),
            )),
        }
    }
}
