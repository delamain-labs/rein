use crate::ast::Span;

/// All token variants produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Agent,
    Can,
    Cannot,
    Model,
    Budget,
    Per,
    Up,
    To,
    // Symbols
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Dot,
    // Literals
    Ident(String),
    StringLiteral(String),
    Dollar(u64),
    // Trivia
    Comment,
    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Agent => write!(f, "agent"),
            TokenKind::Can => write!(f, "can"),
            TokenKind::Cannot => write!(f, "cannot"),
            TokenKind::Model => write!(f, "model"),
            TokenKind::Budget => write!(f, "budget"),
            TokenKind::Per => write!(f, "per"),
            TokenKind::Up => write!(f, "up"),
            TokenKind::To => write!(f, "to"),
            TokenKind::Ident(s) => write!(f, "{s}"),
            TokenKind::Dollar(n) => write!(f, "${}.{:02}", n / 100, n % 100),
            TokenKind::StringLiteral(s) => write!(f, "\"{s}\""),
            TokenKind::Comment => write!(f, "comment"),
            TokenKind::Eof => write!(f, "end of file"),
        }
    }
}

/// A token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            span: Span::new(start, end),
        }
    }
}

/// Lexer error.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

/// Tokenize a `.rein` source string into a `Vec<Token>`.
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(source);
    lexer.run()
}

/// Convert a dollar string (e.g. "0.03", "50") to integer cents without
/// using f64, avoiding floating-point precision issues.
fn parse_cents(s: &str) -> Result<u64, ()> {
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
        let word = std::str::from_utf8(&self.src[start..end]).unwrap();
        let kind = match word {
            "agent" => TokenKind::Agent,
            "can" => TokenKind::Can,
            "cannot" => TokenKind::Cannot,
            "model" => TokenKind::Model,
            "budget" => TokenKind::Budget,
            "per" => TokenKind::Per,
            "up" => TokenKind::Up,
            "to" => TokenKind::To,
            _ => TokenKind::Ident(word.to_string()),
        };
        Token::new(kind, start, end)
    }

    fn read_dollar(&mut self, start: usize) -> Result<Token, LexError> {
        // already consumed '$' at start; pos is now one past '$'

        // The character immediately after '$' must be an ASCII digit.
        match self.peek() {
            Some(b'0'..=b'9') => {}
            Some(ch) => {
                return Err(LexError {
                    message: format!(
                        "invalid dollar amount: expected a number after '$', found '{}'",
                        ch as char
                    ),
                    span: Span::new(start, self.pos + 1),
                });
            }
            None => {
                return Err(LexError {
                    message:
                        "invalid dollar amount: expected a number after '$', found end of input"
                            .to_string(),
                    span: Span::new(start, self.pos),
                });
            }
        }

        let num_start = self.pos;

        // Consume the integer part.
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance();
        }

        // Optional decimal part.
        if self.peek() == Some(b'.') {
            self.advance(); // consume '.'

            // A digit must immediately follow the decimal point.
            match self.peek() {
                Some(b'0'..=b'9') => {}
                Some(ch) => {
                    return Err(LexError {
                        message: format!(
                            "invalid dollar amount: expected digit after decimal point, found '{}'",
                            ch as char
                        ),
                        span: Span::new(start, self.pos + 1),
                    });
                }
                None => {
                    return Err(LexError {
                        message: "invalid dollar amount: expected digit after decimal point, found end of input"
                            .to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
            }

            // Consume the fractional digits.
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }

            // A second decimal point is never valid.
            if self.peek() == Some(b'.') {
                return Err(LexError {
                    message: "invalid dollar amount: too many decimal points".to_string(),
                    span: Span::new(start, self.pos + 1),
                });
            }
        }

        let num_str = std::str::from_utf8(&self.src[num_start..self.pos]).unwrap();
        let cents = parse_cents(num_str).map_err(|()| LexError {
            message: format!("invalid dollar amount: '${num_str}'"),
            span: Span::new(start, self.pos),
        })?;
        Ok(Token::new(TokenKind::Dollar(cents), start, self.pos))
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
                Some(b':') => tokens.push(Token::new(TokenKind::Colon, start, self.pos)),
                Some(b'.') => tokens.push(Token::new(TokenKind::Dot, start, self.pos)),
                Some(b'"') => tokens.push(self.read_string(start)?),
                Some(b'$') => tokens.push(self.read_dollar(start)?),
                Some(b'/') if self.peek() == Some(b'/') => {
                    self.advance(); // second '/'
                    tokens.push(self.skip_line_comment(start));
                }
                Some(b'/') if self.peek() == Some(b'*') => {
                    self.advance(); // '*'
                    tokens.push(self.skip_block_comment(start)?);
                }
                Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' => {
                    tokens.push(self.read_ident(start));
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

#[cfg(test)]
mod tests;
