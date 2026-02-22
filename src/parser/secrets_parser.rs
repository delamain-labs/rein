use crate::ast::{SecretBinding, SecretSource, SecretsDef, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `secrets { key: vault("path"), other: env("VAR") }`.
    pub(super) fn parse_secrets(&mut self) -> Result<SecretsDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Secrets)?;
        self.expect(&TokenKind::LBrace)?;

        let mut bindings = Vec::new();

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => break,
                TokenKind::Ident(_) => {
                    bindings.push(self.parse_secret_binding()?);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file in secrets block",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in secrets block: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }

        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(SecretsDef {
            bindings,
            span: Span::new(start, end),
        })
    }

    /// Parse `key: vault("path")` or `key: env("VAR")`.
    fn parse_secret_binding(&mut self) -> Result<SecretBinding, ParseError> {
        let start = self.current_span().start;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;

        let source = match self.peek().clone() {
            TokenKind::Vault => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let path = self.expect_string()?;
                self.expect(&TokenKind::RParen)?;
                SecretSource::Vault { path }
            }
            TokenKind::Ident(ref s) if s == "env" => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let var = self.expect_string()?;
                self.expect(&TokenKind::RParen)?;
                SecretSource::Env { var }
            }
            other => {
                return Err(ParseError::new(
                    format!("expected 'vault(...)' or 'env(...)' for secret source, got {other}"),
                    self.current_span(),
                ));
            }
        };

        let end = self.last_consumed_end;
        Ok(SecretBinding {
            name,
            source,
            span: Span::new(start, end),
        })
    }
}
