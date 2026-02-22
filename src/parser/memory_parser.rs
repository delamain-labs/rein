use crate::ast::{MemoryDef, MemoryTier, MemoryTierDef, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `memory [name] { working { ... } session { ... } knowledge { ... } }`.
    pub(super) fn parse_memory(&mut self) -> Result<MemoryDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Memory)?;

        let name = if matches!(self.peek(), TokenKind::Ident(_)) {
            let (n, _) = self.expect_ident()?;
            Some(n)
        } else {
            None
        };

        self.expect(&TokenKind::LBrace)?;

        let mut tiers = Vec::new();

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Working => {
                    tiers.push(self.parse_memory_tier(MemoryTier::Working)?);
                }
                TokenKind::Session => {
                    tiers.push(self.parse_memory_tier(MemoryTier::Session)?);
                }
                TokenKind::Knowledge => {
                    tiers.push(self.parse_memory_tier(MemoryTier::Knowledge)?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in memory block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("expected memory tier (working/session/knowledge), got {other}"),
                        self.current_span(),
                    ));
                }
            }
        }

        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(MemoryDef {
            name,
            tiers,
            span: Span::new(start, end),
        })
    }

    /// Parse a memory tier block: `working { ttl: "30m", max_entries: 100 }`.
    fn parse_memory_tier(&mut self, tier: MemoryTier) -> Result<MemoryTierDef, ParseError> {
        let start = self.current_span().start;
        self.advance(); // consume tier keyword

        self.expect(&TokenKind::LBrace)?;

        let mut ttl = None;
        let mut max_entries = None;
        let mut backend = None;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Ident(ref field) => {
                    let field = field.clone();
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    match field.as_str() {
                        "ttl" => {
                            ttl = Some(self.expect_string()?);
                        }
                        "max_entries" => {
                            match self.peek().clone() {
                                TokenKind::Number(n) => {
                                    let val = n.parse::<u64>().map_err(|_| {
                                        ParseError::new(
                                            "max_entries must be a positive integer",
                                            self.current_span(),
                                        )
                                    })?;
                                    self.advance();
                                    max_entries = Some(val);
                                }
                                other => {
                                    return Err(ParseError::new(
                                        format!("expected number for max_entries, got {other}"),
                                        self.current_span(),
                                    ));
                                }
                            }
                        }
                        "backend" => {
                            backend = Some(self.expect_string()?);
                        }
                        _ => {
                            return Err(ParseError::new(
                                format!("unknown memory tier field: {field}"),
                                self.current_span(),
                            ));
                        }
                    }
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in memory tier",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in memory tier: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }

        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(MemoryTierDef {
            tier,
            ttl,
            max_entries,
            backend,
            span: Span::new(start, end),
        })
    }
}
