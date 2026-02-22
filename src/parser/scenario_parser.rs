use crate::ast::{ScenarioDef, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `scenario <name> { given { ... } expect { ... } }`
    pub(super) fn parse_scenario(&mut self) -> Result<ScenarioDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Scenario)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut given = Vec::new();
        let mut expect = Vec::new();

        while *self.peek() != TokenKind::RBrace {
            self.skip_comments();
            match self.peek() {
                TokenKind::Given => {
                    self.advance();
                    self.expect(&TokenKind::LBrace)?;
                    given = self.parse_key_value_pairs()?;
                    self.expect(&TokenKind::RBrace)?;
                }
                TokenKind::Expect => {
                    self.advance();
                    self.expect(&TokenKind::LBrace)?;
                    expect = self.parse_key_value_pairs()?;
                    self.expect(&TokenKind::RBrace)?;
                }
                _ => {
                    return Err(ParseError::new(
                        format!("expected 'given' or 'expect', got {}", self.peek()),
                        self.current_span(),
                    ));
                }
            }
        }

        let end = self.current_span().end;
        self.advance(); // consume RBrace

        Ok(ScenarioDef {
            name,
            given,
            expect,
            span: Span::new(start, end),
        })
    }

    /// Parse key-value pairs like `key: "value"` or `key: value`.
    fn parse_key_value_pairs(&mut self) -> Result<Vec<(String, String)>, ParseError> {
        let mut pairs = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            self.skip_comments();
            let (key, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let value = if let TokenKind::StringLiteral(s) = self.peek().clone() {
                self.advance();
                s
            } else {
                let (v, _) = self.expect_ident()?;
                v
            };
            pairs.push((key, value));
        }
        Ok(pairs)
    }
}
