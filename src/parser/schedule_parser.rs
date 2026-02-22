use crate::ast::{ScheduleDef, ScheduleExpr, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `schedule: daily at 2am` or `schedule: every 6 hours`.
    pub(super) fn parse_schedule(&mut self) -> Result<ScheduleDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Schedule)?;
        self.expect(&TokenKind::Colon)?;

        let expr = match self.peek().clone() {
            TokenKind::Daily => {
                self.advance();
                // `at` lexes as Ident("at"), not TokenKind::At (which is @)
                match self.peek().clone() {
                    TokenKind::Ident(ref s) if s == "at" => self.advance(),
                    TokenKind::At => self.advance(),
                    _ => {
                        return Err(ParseError::new(
                            format!("expected 'at' after 'daily', got {}", self.peek()),
                            self.current_span(),
                        ));
                    }
                };
                // Parse time: could be "2am", "14:30", etc. — grab as string or ident
                let time = match self.peek().clone() {
                    TokenKind::StringLiteral(s) | TokenKind::Ident(s) => {
                        self.advance();
                        s
                    }
                    TokenKind::Number(n) => {
                        self.advance();
                        // May be followed by "am"/"pm" ident
                        if let TokenKind::Ident(suffix) = self.peek().clone() {
                            if suffix == "am" || suffix == "pm" {
                                self.advance();
                                format!("{n}{suffix}")
                            } else {
                                n
                            }
                        } else {
                            n
                        }
                    }
                    other => {
                        return Err(ParseError::new(
                            format!("expected time after 'daily at', got {other}"),
                            self.current_span(),
                        ));
                    }
                };
                ScheduleExpr::DailyAt { time }
            }
            TokenKind::Every => {
                self.advance();
                let num = match self.peek().clone() {
                    TokenKind::Number(n) => {
                        self.advance();
                        n.parse::<u64>().map_err(|_| {
                            ParseError::new("expected integer after 'every'", self.current_span())
                        })?
                    }
                    other => {
                        return Err(ParseError::new(
                            format!("expected number after 'every', got {other}"),
                            self.current_span(),
                        ));
                    }
                };
                match self.peek().clone() {
                    TokenKind::Hours => {
                        self.advance();
                        ScheduleExpr::EveryNHours { hours: num }
                    }
                    TokenKind::Ident(ref s) if s == "minutes" => {
                        self.advance();
                        ScheduleExpr::EveryNMinutes { minutes: num }
                    }
                    other => {
                        return Err(ParseError::new(
                            format!("expected 'hours' or 'minutes' after number, got {other}"),
                            self.current_span(),
                        ));
                    }
                }
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                ScheduleExpr::Cron { expr: s }
            }
            other => {
                return Err(ParseError::new(
                    format!("expected schedule expression (daily/every/cron string), got {other}"),
                    self.current_span(),
                ));
            }
        };

        let end = self.last_consumed_end;
        Ok(ScheduleDef {
            expr,
            span: Span::new(start, end),
        })
    }
}
