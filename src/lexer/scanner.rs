use crate::ast::Span;

use super::{LexError, Token, TokenKind};

/// Tokenize a `.rein` source string into a `Vec<Token>`.
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(source);
    lexer.run()
}

/// Convert a dollar string (e.g. "0.03", "50") to integer cents without
/// using f64, avoiding floating-point precision issues.
pub(crate) fn parse_cents(s: &str) -> Result<u64, ()> {
    let mut parts = s.splitn(2, '.');
    let whole_str = parts.next().unwrap_or("0");
    let frac_str = parts.next().unwrap_or("");

    let whole: u64 = whole_str.parse().map_err(|_| ())?;

    // Normalise fractional part to exactly 2 digits (truncate beyond cent).
    let cents_str = match frac_str.len() {
        0 => "00".to_string(),
        1 => format!("{frac_str}0"),
        _ => frac_str[..2].to_string(),
    };
    let cents: u64 = cents_str.parse().map_err(|_| ())?;

    Ok(whole * 100 + cents)
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            src: source.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.src.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.advance();
        }
    }

    fn read_ident(&mut self, start: usize) -> Token {
        while matches!(
            self.peek(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
        ) {
            self.advance();
        }
        let end = self.pos;
        let word = std::str::from_utf8(&self.src[start..end]).expect("input should be valid UTF-8");
        // "key" is context-sensitive; lexed as Ident, matched in parser
        let kind = if word == "key" {
            TokenKind::Ident("key".to_string())
        } else {
            TokenKind::from_word(word).unwrap_or_else(|| TokenKind::Ident(word.to_string()))
        };
        Token::new(kind, start, end)
    }

    fn read_number(&mut self, start: usize) -> Token {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance();
        }
        // Optional decimal part
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            self.advance(); // consume '.'
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }
        }
        let end = self.pos;
        let text = std::str::from_utf8(&self.src[start..end])
            .expect("input should be valid UTF-8")
            .to_string();
        Token::new(TokenKind::Number(text), start, end)
    }

    fn read_currency(&mut self, symbol: char, start: usize) -> Result<Token, LexError> {
        // Already consumed currency symbol at start; pos is now past it.
        match self.peek() {
            Some(b'0'..=b'9') => {}
            Some(ch) => {
                return Err(LexError {
                    message: format!(
                        "invalid currency amount: expected a number after '{symbol}', found '{}'",
                        ch as char
                    ),
                    span: Span::new(start, self.pos + 1),
                });
            }
            None => {
                return Err(LexError {
                    message: format!(
                        "invalid currency amount: expected a number after '{symbol}', found end of input"
                    ),
                    span: Span::new(start, self.pos),
                });
            }
        }

        let num_start = self.pos;

        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance();
        }

        if self.peek() == Some(b'.') {
            self.advance();

            match self.peek() {
                Some(b'0'..=b'9') => {}
                Some(ch) => {
                    return Err(LexError {
                        message: format!(
                            "invalid currency amount: expected digit after decimal point, found '{}'",
                            ch as char
                        ),
                        span: Span::new(start, self.pos + 1),
                    });
                }
                None => {
                    return Err(LexError {
                        message: "invalid currency amount: expected digit after decimal point, found end of input"
                            .to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
            }

            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }

            if self.peek() == Some(b'.') {
                return Err(LexError {
                    message: "invalid currency amount: too many decimal points".to_string(),
                    span: Span::new(start, self.pos + 1),
                });
            }
        }

        let num_str = std::str::from_utf8(&self.src[num_start..self.pos])
            .expect("numeric literal should be valid UTF-8");
        let cents = parse_cents(num_str).map_err(|()| LexError {
            message: format!("invalid currency amount: '{symbol}{num_str}'"),
            span: Span::new(start, self.pos),
        })?;
        Ok(Token::new(
            TokenKind::Currency {
                symbol,
                amount: cents,
            },
            start,
            self.pos,
        ))
    }

    fn read_string(&mut self, start: usize) -> Result<Token, LexError> {
        // Opening '"' already consumed; collect content until closing '"'.
        let mut value = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(LexError {
                        message: "unterminated string literal".to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
                Some(b'"') => break,
                Some(ch) => value.push(ch as char),
            }
        }
        Ok(Token::new(TokenKind::StringLiteral(value), start, self.pos))
    }

    fn skip_line_comment(&mut self, start: usize) -> Token {
        while !matches!(self.peek(), Some(b'\n') | None) {
            self.advance();
        }
        Token::new(TokenKind::Comment, start, self.pos)
    }

    fn skip_block_comment(&mut self, start: usize) -> Result<Token, LexError> {
        // already past '/*'
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        message: "unterminated block comment".to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
                Some(b'*') if self.peek_at(1) == Some(b'/') => {
                    self.advance(); // '*'
                    self.advance(); // '/'
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
        Ok(Token::new(TokenKind::Comment, start, self.pos))
    }

    /// Lex a comparison or equality operator starting with `<`, `>`, `=`, or `!`.
    fn read_comparison(&mut self, ch: u8, start: usize) -> Option<Token> {
        match ch {
            b'<' if self.peek() == Some(b'=') => {
                self.advance();
                Some(Token::new(TokenKind::LtEq, start, self.pos))
            }
            b'<' => Some(Token::new(TokenKind::Lt, start, self.pos)),
            b'>' if self.peek() == Some(b'=') => {
                self.advance();
                Some(Token::new(TokenKind::GtEq, start, self.pos))
            }
            b'>' => Some(Token::new(TokenKind::Gt, start, self.pos)),
            b'=' if self.peek() == Some(b'=') => {
                self.advance();
                Some(Token::new(TokenKind::EqEq, start, self.pos))
            }
            b'!' if self.peek() == Some(b'=') => {
                self.advance();
                Some(Token::new(TokenKind::BangEq, start, self.pos))
            }
            _ => None,
        }
    }

    fn run(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            let start = self.pos;
            match self.advance() {
                None => {
                    tokens.push(Token::new(TokenKind::Eof, start, start));
                    break;
                }
                Some(b'{') => tokens.push(Token::new(TokenKind::LBrace, start, self.pos)),
                Some(b'}') => tokens.push(Token::new(TokenKind::RBrace, start, self.pos)),
                Some(b'[') => tokens.push(Token::new(TokenKind::LBracket, start, self.pos)),
                Some(b']') => tokens.push(Token::new(TokenKind::RBracket, start, self.pos)),
                Some(b'(') => tokens.push(Token::new(TokenKind::LParen, start, self.pos)),
                Some(b')') => tokens.push(Token::new(TokenKind::RParen, start, self.pos)),
                Some(b':') => tokens.push(Token::new(TokenKind::Colon, start, self.pos)),
                Some(b'.') => {
                    if self.peek() == Some(b'.') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::DotDot, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Dot, start, self.pos));
                    }
                }
                Some(b',') => tokens.push(Token::new(TokenKind::Comma, start, self.pos)),
                Some(b'"') => tokens.push(self.read_string(start)?),
                Some(b'$') => tokens.push(self.read_currency('$', start)?),
                Some(b'#') => {
                    tokens.push(self.skip_line_comment(start));
                }
                Some(b'/') if self.peek() == Some(b'/') => {
                    self.advance(); // second '/'
                    tokens.push(self.skip_line_comment(start));
                }
                Some(b'/') if self.peek() == Some(b'*') => {
                    self.advance(); // '*'
                    tokens.push(self.skip_block_comment(start)?);
                }
                Some(b'/') => tokens.push(Token::new(TokenKind::Slash, start, self.pos)),
                Some(b'@') => tokens.push(Token::new(TokenKind::At, start, self.pos)),
                Some(b'-') if self.peek() == Some(b'>') => {
                    self.advance(); // consume '>'
                    tokens.push(Token::new(TokenKind::Arrow, start, self.pos));
                }
                Some(ch @ (b'<' | b'>' | b'=' | b'!')) => {
                    if let Some(tok) = self.read_comparison(ch, start) {
                        tokens.push(tok);
                    } else {
                        return Err(LexError {
                            message: format!("unexpected character: '{}'", ch as char),
                            span: Span::new(start, self.pos),
                        });
                    }
                }
                Some(b'%') => tokens.push(Token::new(TokenKind::Percent, start, self.pos)),
                Some(b'|') => tokens.push(Token::new(TokenKind::Pipe, start, self.pos)),
                Some(ch) if ch.is_ascii_digit() => {
                    tokens.push(self.read_number(start));
                }
                Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' => {
                    tokens.push(self.read_ident(start));
                }
                // Multi-byte currency symbols: £ (C2 A3), ¥ (C2 A5), € (E2 82 AC)
                Some(0xC2) if matches!(self.peek(), Some(0xA3 | 0xA5)) => {
                    let sym = if self.src[self.pos] == 0xA3 {
                        '£'
                    } else {
                        '¥'
                    };
                    self.advance(); // consume second byte
                    tokens.push(self.read_currency(sym, start)?);
                }
                Some(0xE2)
                    if self.pos + 1 < self.src.len()
                        && self.src[self.pos] == 0x82
                        && self.src[self.pos + 1] == 0xAC =>
                {
                    self.advance(); // consume 0x82
                    self.advance(); // consume 0xAC
                    tokens.push(self.read_currency('€', start)?);
                }
                Some(ch) => {
                    return Err(LexError {
                        message: format!("unexpected character: '{}'", ch as char),
                        span: Span::new(start, self.pos),
                    });
                }
            }
        }
        Ok(tokens)
    }
}
