use crate::ast::{ImportDef, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse an import declaration.
    ///
    /// Supports:
    /// - `import { name1, name2 } from "./path.rein"`
    /// - `import all from "./dir/"`
    /// - `import from @scope/name`
    pub(super) fn parse_import(&mut self) -> Result<ImportDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Import)?;

        match self.peek().clone() {
            // import { name, ... } from "path"
            TokenKind::LBrace => {
                self.advance(); // {
                let mut names = Vec::new();
                loop {
                    self.skip_comments();
                    if *self.peek() == TokenKind::RBrace {
                        break;
                    }
                    let (name, _) = self.expect_ident()?;
                    names.push(name);
                    self.skip_comments();
                    if *self.peek() == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                self.expect(&TokenKind::From)?;
                let source = self.expect_string()?;
                let end = self.current_span().start;
                Ok(ImportDef::Named {
                    names,
                    source,
                    span: Span::new(start, end),
                })
            }
            // import all from "path"
            TokenKind::All => {
                self.advance(); // all
                self.expect(&TokenKind::From)?;
                let source = self.expect_string()?;
                let end = self.current_span().start;
                Ok(ImportDef::Glob {
                    source,
                    span: Span::new(start, end),
                })
            }
            // import from @scope/name
            TokenKind::From => {
                self.advance(); // from
                self.expect(&TokenKind::At)?;
                let (scope, _) = self.expect_ident()?;
                self.expect(&TokenKind::Slash)?;
                let (name, _) = self.expect_ident()?;
                let end = self.current_span().start;
                Ok(ImportDef::Registry {
                    scope,
                    name,
                    span: Span::new(start, end),
                })
            }
            other => Err(ParseError::new(
                format!("expected '{{', 'all', or 'from' after 'import', got {other}"),
                self.current_span(),
            )),
        }
    }
}
