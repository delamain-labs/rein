use crate::ast::{
    Budget, Capability, Constraint, DefaultsDef, ProviderDef, Span, ToolDef, ValueExpr,
};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    pub(super) fn parse_defaults(&mut self) -> Result<DefaultsDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Defaults)?;
        self.expect(&TokenKind::LBrace)?;

        let mut model: Option<ValueExpr> = None;
        let mut budget: Option<Budget> = None;
        let (mut seen_model, mut seen_budget) = (false, false);

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(DefaultsDef {
                        model,
                        budget,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Model => {
                    if seen_model {
                        return Err(ParseError::new(
                            "duplicate field 'model' in defaults block",
                            self.current_span(),
                        ));
                    }
                    seen_model = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    model = Some(self.parse_value_expr()?);
                }
                TokenKind::Budget => {
                    if seen_budget {
                        return Err(ParseError::new(
                            "duplicate field 'budget' in defaults block",
                            self.current_span(),
                        ));
                    }
                    seen_budget = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    budget = Some(self.parse_budget()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected field in defaults block: {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    pub(super) fn parse_provider(&mut self) -> Result<ProviderDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Provider)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut model: Option<ValueExpr> = None;
        let mut key: Option<ValueExpr> = None;
        let mut seen_model = false;
        let mut seen_key = false;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(ProviderDef {
                        name,
                        model,
                        key,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Model => {
                    if seen_model {
                        return Err(ParseError::new(
                            format!("duplicate field 'model' in provider '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_model = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    model = Some(self.parse_value_expr()?);
                }
                TokenKind::Key => {
                    if seen_key {
                        return Err(ParseError::new(
                            format!("duplicate field 'key' in provider '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_key = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    key = Some(self.parse_value_expr()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected field in provider '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    pub(super) fn parse_tool(&mut self) -> Result<ToolDef, ParseError> {
        self.skip_comments();
        let start = self.current_span().start;

        self.expect(&TokenKind::Tool)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut endpoint: Option<ValueExpr> = None;
        let mut provider: Option<ValueExpr> = None;
        let mut key: Option<ValueExpr> = None;
        let mut seen_endpoint = false;
        let mut seen_provider = false;
        let mut seen_key = false;

        loop {
            self.skip_comments();
            match self.peek().clone() {
                TokenKind::RBrace => {
                    let end = self.current_span().end;
                    self.advance();
                    return Ok(ToolDef {
                        name,
                        endpoint,
                        provider,
                        key,
                        span: Span::new(start, end),
                    });
                }
                TokenKind::Endpoint => {
                    if seen_endpoint {
                        return Err(ParseError::new(
                            format!("duplicate field 'endpoint' in tool '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_endpoint = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    endpoint = Some(self.parse_value_expr()?);
                }
                TokenKind::Provider => {
                    if seen_provider {
                        return Err(ParseError::new(
                            format!("duplicate field 'provider' in tool '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_provider = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    provider = Some(self.parse_value_expr()?);
                }
                TokenKind::Key => {
                    if seen_key {
                        return Err(ParseError::new(
                            format!("duplicate field 'key' in tool '{name}'"),
                            self.current_span(),
                        ));
                    }
                    seen_key = true;
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    key = Some(self.parse_value_expr()?);
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected field in tool '{name}': {other}"),
                        self.current_span(),
                    ));
                }
            }
        }
    }

    pub(super) fn parse_capability_list(&mut self) -> Result<Vec<Capability>, ParseError> {
        self.expect(&TokenKind::LBracket)?;
        let mut caps = Vec::new();
        loop {
            self.skip_comments();
            match self.peek() {
                TokenKind::RBracket => {
                    self.advance();
                    break;
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        "unexpected end of file: expected `]`",
                        self.current_span(),
                    ));
                }
                _ => caps.push(self.parse_capability()?),
            }
        }
        Ok(caps)
    }

    fn parse_capability(&mut self) -> Result<Capability, ParseError> {
        let start = self.current_span().start;
        let (namespace, _) = self.expect_ident()?;
        self.expect(&TokenKind::Dot)?;
        let (action, _) = self.expect_ident()?;

        // optional `up to $<amount>`
        let constraint = if self.peek() == &TokenKind::Up {
            self.advance();
            self.expect(&TokenKind::To)?;
            let (amount, symbol, _) = self.expect_currency()?;
            let currency = match symbol {
                '€' => "EUR",
                '£' => "GBP",
                '¥' => "JPY",
                _ => "USD",
            };
            Some(Constraint::MonetaryCap {
                amount,
                currency: currency.to_string(),
            })
        } else {
            None
        };

        let end = self.last_consumed_end;
        Ok(Capability {
            namespace,
            action,
            constraint,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_budget(&mut self) -> Result<Budget, ParseError> {
        let start = self.current_span().start;
        let (amount, symbol, _) = self.expect_currency()?;
        self.expect(&TokenKind::Per)?;
        let (unit, _) = self.expect_ident()?;
        let end = self.last_consumed_end;
        let currency = match symbol {
            '€' => "EUR",
            '£' => "GBP",
            '¥' => "JPY",
            _ => "USD",
        };
        Ok(Budget {
            amount,
            currency: currency.to_string(),
            unit,
            span: Span::new(start, end),
        })
    }
}
