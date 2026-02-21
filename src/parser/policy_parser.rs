use crate::ast::{PolicyDef, PolicyTier, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse a `policy { tier ... tier ... }` block.
    pub(super) fn parse_policy(&mut self) -> Result<PolicyDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Policy)?;
        self.expect(&TokenKind::LBrace)?;

        let mut tiers = Vec::new();

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(PolicyDef {
                        tiers,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Tier => {
                    tiers.push(self.parse_policy_tier()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in policy block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("expected 'tier' in policy block, got {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    /// Parse `tier <name> { promote when <expr> }`.
    fn parse_policy_tier(&mut self) -> Result<PolicyTier, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Tier)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut promote_when = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(PolicyTier {
                        name,
                        promote_when,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Promote => {
                    if promote_when.is_some() {
                        return Err(ParseError::new(
                            format!("duplicate 'promote when' in tier '{name}'"),
                            self.current_span(),
                        ));
                    }
                    self.advance();
                    self.expect(&TokenKind::When)?;
                    promote_when = Some(self.parse_when_expr()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in tier block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("expected 'promote' or '}}' in tier '{name}', got {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }
}
