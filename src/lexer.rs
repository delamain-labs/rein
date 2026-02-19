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
    Dollar(f64),
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
        let num_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9') | Some(b'.')) {
            self.advance();
        }
        let num_str = std::str::from_utf8(&self.src[num_start..self.pos]).unwrap();
        let amount: f64 = num_str.parse().map_err(|_| LexError {
            message: format!("invalid dollar amount: ${}", num_str),
            span: Span::new(start, self.pos),
        })?;
        Ok(Token::new(TokenKind::Dollar(amount), start, self.pos))
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
        assert_eq!(kinds(&tokens), vec![&TokenKind::Dollar(0.03)]);
    }

    #[test]
    fn tokenize_dollar_integer() {
        let tokens = non_eof(lex_ok("$50"));
        assert_eq!(kinds(&tokens), vec![&TokenKind::Dollar(50.0)]);
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
            vec![&TokenKind::Up, &TokenKind::To, &TokenKind::Dollar(50.0)]
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
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Dollar(0.03)));
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
}
