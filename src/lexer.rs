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
    Dollar(u64),
    // Trivia
    Comment,
    Eof,
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
        1 => format!("{}0", frac_str),
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
        while matches!(
            self.peek(),
            Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            self.advance();
        }
    }

    fn read_ident(&mut self, start: usize) -> Token {
        while matches!(
            self.peek(),
            Some(b'a'..=b'z') | Some(b'A'..=b'Z') | Some(b'0'..=b'9') | Some(b'_')
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
        let cents = parse_cents(num_str).map_err(|_| LexError {
            message: format!("invalid dollar amount: '${}'", num_str),
            span: Span::new(start, self.pos),
        })?;
        Ok(Token::new(TokenKind::Dollar(cents), start, self.pos))
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
mod tests {
    use super::*;

    fn kinds(tokens: &[Token]) -> Vec<&TokenKind> {
        tokens.iter().map(|t| &t.kind).collect()
    }

    fn lex_ok(src: &str) -> Vec<Token> {
        tokenize(src).expect("lex should succeed")
    }

    fn non_eof(tokens: Vec<Token>) -> Vec<Token> {
        tokens
            .into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .collect()
    }

    // ── parse_cents unit tests ────────────────────────────────────────────────

    #[test]
    fn parse_cents_whole_number() {
        assert_eq!(parse_cents("50").unwrap(), 5000);
    }

    #[test]
    fn parse_cents_fractional() {
        assert_eq!(parse_cents("0.03").unwrap(), 3);
    }

    #[test]
    fn parse_cents_one_decimal_place() {
        assert_eq!(parse_cents("1.5").unwrap(), 150);
    }

    #[test]
    fn parse_cents_truncates_sub_cent() {
        assert_eq!(parse_cents("0.005").unwrap(), 0);
    }

    #[test]
    fn parse_cents_dollar_fifty() {
        assert_eq!(parse_cents("0.50").unwrap(), 50);
    }

    // ── Happy-path tests ──────────────────────────────────────────────────────

    #[test]
    fn tokenize_agent_header() {
        let tokens = non_eof(lex_ok("agent foo {"));
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Agent,
                &TokenKind::Ident("foo".into()),
                &TokenKind::LBrace
            ]
        );
    }

    #[test]
    fn tokenize_dollar_amount() {
        let tokens = non_eof(lex_ok("$0.03"));
        assert_eq!(kinds(&tokens), vec![&TokenKind::Dollar(3)]);
    }

    #[test]
    fn tokenize_dollar_integer() {
        let tokens = non_eof(lex_ok("$50"));
        assert_eq!(kinds(&tokens), vec![&TokenKind::Dollar(5000)]);
    }

    #[test]
    fn tokenize_dotted_capability() {
        let tokens = non_eof(lex_ok("zendesk.read_ticket"));
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Ident("zendesk".into()),
                &TokenKind::Dot,
                &TokenKind::Ident("read_ticket".into()),
            ]
        );
    }

    #[test]
    fn tokenize_up_to_constraint() {
        let tokens = non_eof(lex_ok("up to $50"));
        assert_eq!(
            kinds(&tokens),
            vec![&TokenKind::Up, &TokenKind::To, &TokenKind::Dollar(5000)]
        );
    }

    #[test]
    fn tokenize_line_comment() {
        let tokens = non_eof(lex_ok("// this is a comment"));
        assert_eq!(kinds(&tokens), vec![&TokenKind::Comment]);
    }

    #[test]
    fn tokenize_block_comment() {
        let tokens = non_eof(lex_ok("/* block comment */"));
        assert_eq!(kinds(&tokens), vec![&TokenKind::Comment]);
    }

    #[test]
    fn tokenize_all_keywords() {
        let src = "agent can cannot model budget per up to";
        let tokens = non_eof(lex_ok(src));
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Agent,
                &TokenKind::Can,
                &TokenKind::Cannot,
                &TokenKind::Model,
                &TokenKind::Budget,
                &TokenKind::Per,
                &TokenKind::Up,
                &TokenKind::To,
            ]
        );
    }

    #[test]
    fn tokenize_symbols() {
        let tokens = non_eof(lex_ok("{ } [ ] : ."));
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::LBrace,
                &TokenKind::RBrace,
                &TokenKind::LBracket,
                &TokenKind::RBracket,
                &TokenKind::Colon,
                &TokenKind::Dot,
            ]
        );
    }

    #[test]
    fn tokenize_full_agent_snippet() {
        let src = r#"
agent support_triage {
    model: anthropic
    can [
        zendesk.read_ticket
    ]
    budget: $0.03 per ticket
}"#;
        let tokens = non_eof(lex_ok(src));
        // Spot-check key tokens exist
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Agent));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Model));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Can));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Budget));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Dollar(3)));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Per));
    }

    #[test]
    fn span_is_correct_for_ident() {
        let src = "agent";
        let tokens = lex_ok(src);
        let agent_tok = tokens.iter().find(|t| t.kind == TokenKind::Agent).unwrap();
        assert_eq!(agent_tok.span, Span::new(0, 5));
    }

    #[test]
    fn eof_is_always_last() {
        let tokens = lex_ok("agent foo {}");
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    // ── Error-path tests ──────────────────────────────────────────────────────

    #[test]
    fn error_on_invalid_char() {
        let result = tokenize("agent @ foo");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains('@'));
    }

    #[test]
    fn error_on_unterminated_block_comment() {
        let result = tokenize("/* never closed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("unterminated"));
    }

    #[test]
    fn error_span_points_to_bad_char() {
        let src = "foo @";
        let result = tokenize(src);
        let err = result.unwrap_err();
        // '@' is at byte offset 4
        assert_eq!(err.span.start, 4);
    }

    // ── Dollar / number error tests ───────────────────────────────────────────

    #[test]
    fn error_bare_dollar_eof() {
        let err = tokenize("$").unwrap_err();
        assert!(
            err.message.contains("expected a number"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_bare_dollar_space() {
        let err = tokenize("$ ").unwrap_err();
        assert!(
            err.message.contains("expected a number"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_dollar_alpha() {
        let err = tokenize("$abc").unwrap_err();
        assert!(err.message.contains("found 'a'"), "got: {}", err.message);
    }

    #[test]
    fn error_dollar_leading_dot() {
        // '$.' — dot is not a digit so we should get the "expected a number" error
        let err = tokenize("$.5").unwrap_err();
        assert!(
            err.message.contains("expected a number"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_dollar_trailing_dot() {
        let err = tokenize("$1.").unwrap_err();
        assert!(
            err.message.contains("expected digit after decimal"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn error_dollar_multiple_dots() {
        let err = tokenize("$1.2.3").unwrap_err();
        assert!(
            err.message.contains("too many decimal points"),
            "got: {}",
            err.message
        );
    }
}
