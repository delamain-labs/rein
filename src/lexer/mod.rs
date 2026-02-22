use crate::ast::Span;

/// All token variants produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Agent,
    Archetype,
    Can,
    Cannot,
    Model,
    Budget,
    Per,
    Up,
    To,
    Workflow,
    Trigger,
    Stages,
    Provider,
    Step,
    Goal,
    Tool,
    Endpoint,
    Guardrails,
    Defaults,
    One,
    Of,
    Type,
    Import,
    From,
    All,
    At,
    Slash,
    Arrow,
    Parallel,
    Route,
    On,
    When,
    Or,
    And,
    Failure,
    Retry,
    Then,
    Exponential,
    Linear,
    Fixed,
    Escalate,
    Fallback,
    Where,
    Sort,
    By,
    Take,
    Skip,
    Select,
    Unique,
    Asc,
    Desc,
    Pipe,
    Observe,
    Fleet,
    Channel,
    Trace,
    Metrics,
    Alert,
    Export,
    Agents,
    Scaling,
    Min,
    Max,
    Retention,
    Send,
    Within,
    CircuitBreaker,
    Auto,
    Resolve,
    Is,
    Policy,
    Tier,
    Promote,
    Underscore,
    // S.1 remaining keywords
    Eval, Dataset, Assert, Block, Deploy,
    Memory, Working, Session, Knowledge,
    Schedule, Daily, Every, Hours,
    For, Each, Input, Output,
    Human, Via, Secrets, Vault,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Percent,
    // Symbols
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Colon,
    Dot,
    DotDot,
    Comma,
    // Literals
    Ident(String),
    StringLiteral(String),
    /// A numeric literal (integer or float, stored as string for flexibility).
    Number(String),
    /// A monetary amount with currency symbol and value in minor units (cents).
    Currency {
        symbol: char,
        amount: u64,
    },
    // Trivia
    Comment,
    Eof,
}

impl TokenKind {
    /// Return the display string for keyword/symbol tokens.
    fn keyword_str(&self) -> Option<&'static str> {
        let s = match self {
            Self::Agent => "agent", Self::Archetype => "archetype",
            Self::Can => "can", Self::Cannot => "cannot",
            Self::Model => "model", Self::Budget => "budget",
            Self::Per => "per", Self::Up => "up", Self::To => "to",
            Self::Workflow => "workflow", Self::Trigger => "trigger",
            Self::Stages => "stages", Self::Provider => "provider",
            Self::Step => "step", Self::Goal => "goal",
            Self::Tool => "tool", Self::Endpoint => "endpoint",
            Self::Guardrails => "guardrails", Self::Defaults => "defaults",
            Self::One => "one", Self::Of => "of", Self::Type => "type",
            Self::Import => "import", Self::From => "from", Self::All => "all",
            Self::At => "@", Self::Slash => "/",
            Self::Parallel => "parallel", Self::When => "when",
            Self::Failure => "failure", Self::Retry => "retry",
            Self::Then => "then", Self::Exponential => "exponential",
            Self::Linear => "linear", Self::Fixed => "fixed",
            Self::Escalate => "escalate", Self::Fallback => "fallback",
            Self::Where => "where", Self::Sort => "sort", Self::By => "by",
            Self::Take => "take", Self::Skip => "skip", Self::Select => "select",
            Self::Unique => "unique", Self::Asc => "asc", Self::Desc => "desc",
            Self::Pipe => "|", Self::Observe => "observe",
            Self::Fleet => "fleet", Self::Channel => "channel",
            Self::Trace => "trace", Self::Metrics => "metrics",
            Self::Alert => "alert", Self::Export => "export",
            Self::Agents => "agents", Self::Scaling => "scaling",
            Self::Min => "min", Self::Max => "max",
            Self::Retention => "retention", Self::Send => "send",
            Self::Within => "within", Self::CircuitBreaker => "circuit_breaker",
            Self::Auto => "auto", Self::Resolve => "resolve",
            Self::Is => "is", Self::Policy => "policy",
            Self::Tier => "tier", Self::Promote => "promote",
            Self::Or => "or", Self::And => "and",
            Self::Lt => "<", Self::Gt => ">",
            Self::LtEq => "<=", Self::GtEq => ">=",
            Self::Percent => "%", Self::Arrow => "->",
            Self::Route => "route", Self::On => "on",
            Self::Underscore => "_", Self::DotDot => "..",
            Self::LBrace => "{", Self::RBrace => "}",
            Self::LBracket => "[", Self::RBracket => "]",
            Self::LParen => "(", Self::RParen => ")",
            Self::Colon => ":", Self::Dot => ".", Self::Comma => ",",
            Self::Eval => "eval", Self::Dataset => "dataset",
            Self::Assert => "assert", Self::Block => "block",
            Self::Deploy => "deploy", Self::Memory => "memory",
            Self::Working => "working", Self::Session => "session",
            Self::Knowledge => "knowledge", Self::Schedule => "schedule",
            Self::Daily => "daily", Self::Every => "every",
            Self::Hours => "hours", Self::For => "for", Self::Each => "each",
            Self::Input => "input", Self::Output => "output",
            Self::Human => "human", Self::Via => "via",
            Self::Secrets => "secrets", Self::Vault => "vault",
            Self::Comment => "comment", Self::Eof => "end of file",
            _ => return None,
        };
        Some(s)
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = self.keyword_str() {
            return write!(f, "{s}");
        }
        match self {
            TokenKind::Number(n) => write!(f, "{n}"),
            TokenKind::Ident(s) => write!(f, "{s}"),
            TokenKind::Currency { symbol, amount } => {
                write!(f, "{symbol}{}.{:02}", amount / 100, amount % 100)
            }
            TokenKind::StringLiteral(s) => write!(f, "\"{s}\""),
            _ => unreachable!(),
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
            "archetype" => TokenKind::Archetype,
            "can" => TokenKind::Can,
            "cannot" => TokenKind::Cannot,
            "model" => TokenKind::Model,
            "budget" => TokenKind::Budget,
            "per" => TokenKind::Per,
            "up" => TokenKind::Up,
            "to" => TokenKind::To,
            "workflow" => TokenKind::Workflow,
            "trigger" => TokenKind::Trigger,
            "stages" => TokenKind::Stages,
            "provider" => TokenKind::Provider,
            // "key" is context-sensitive; lexed as Ident, matched in parser
            "key" => TokenKind::Ident("key".to_string()),
            "step" => TokenKind::Step,
            "goal" => TokenKind::Goal,
            "tool" => TokenKind::Tool,
            "endpoint" => TokenKind::Endpoint,
            "guardrails" => TokenKind::Guardrails,
            "defaults" => TokenKind::Defaults,
            "one" => TokenKind::One,
            "of" => TokenKind::Of,
            "type" => TokenKind::Type,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "all" => TokenKind::All,
            "parallel" => TokenKind::Parallel,
            "when" => TokenKind::When,
            "failure" => TokenKind::Failure,
            "retry" => TokenKind::Retry,
            "then" => TokenKind::Then,
            "exponential" => TokenKind::Exponential,
            "linear" => TokenKind::Linear,
            "fixed" => TokenKind::Fixed,
            "escalate" => TokenKind::Escalate,
            "fallback" => TokenKind::Fallback,
            "where" => TokenKind::Where,
            "sort" => TokenKind::Sort,
            "by" => TokenKind::By,
            "take" => TokenKind::Take,
            "skip" => TokenKind::Skip,
            "select" => TokenKind::Select,
            "unique" => TokenKind::Unique,
            "asc" => TokenKind::Asc,
            "desc" => TokenKind::Desc,
            "observe" => TokenKind::Observe,
            "fleet" => TokenKind::Fleet,
            "channel" => TokenKind::Channel,
            "trace" => TokenKind::Trace,
            "metrics" => TokenKind::Metrics,
            "alert" => TokenKind::Alert,
            "export" => TokenKind::Export,
            "agents" => TokenKind::Agents,
            "scaling" => TokenKind::Scaling,
            "min" => TokenKind::Min,
            "max" => TokenKind::Max,
            "retention" => TokenKind::Retention,
            "send" => TokenKind::Send,
            "within" => TokenKind::Within,
            "circuit_breaker" => TokenKind::CircuitBreaker,
            "auto" => TokenKind::Auto,
            "resolve" => TokenKind::Resolve,
            "is" => TokenKind::Is,
            "policy" => TokenKind::Policy,
            "tier" => TokenKind::Tier,
            "promote" => TokenKind::Promote,
            "or" => TokenKind::Or,
            "and" => TokenKind::And,
            "route" => TokenKind::Route,
            "on" => TokenKind::On,
            "_" => TokenKind::Underscore,
            "eval" => TokenKind::Eval, "dataset" => TokenKind::Dataset,
            "assert" => TokenKind::Assert, "block" => TokenKind::Block,
            "deploy" => TokenKind::Deploy, "memory" => TokenKind::Memory,
            "working" => TokenKind::Working, "session" => TokenKind::Session,
            "knowledge" => TokenKind::Knowledge, "schedule" => TokenKind::Schedule,
            "daily" => TokenKind::Daily, "every" => TokenKind::Every,
            "hours" => TokenKind::Hours, "for" => TokenKind::For,
            "each" => TokenKind::Each, "input" => TokenKind::Input,
            "output" => TokenKind::Output, "human" => TokenKind::Human,
            "via" => TokenKind::Via, "secrets" => TokenKind::Secrets,
            "vault" => TokenKind::Vault,
            _ => TokenKind::Ident(word.to_string()),
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
        let text = std::str::from_utf8(&self.src[start..end]).unwrap().to_string();
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

        let num_str = std::str::from_utf8(&self.src[num_start..self.pos]).unwrap();
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
                Some(b'<') => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::LtEq, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Lt, start, self.pos));
                    }
                }
                Some(b'>') => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::GtEq, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Gt, start, self.pos));
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

#[cfg(test)]
mod tests;
