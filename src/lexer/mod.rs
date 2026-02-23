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
    Eval,
    Dataset,
    Assert,
    Block,
    Deploy,
    Memory,
    Working,
    Session,
    Knowledge,
    Schedule,
    Daily,
    Every,
    Hours,
    For,
    Each,
    Input,
    Output,
    Human,
    Via,
    Secrets,
    Vault,
    Approve,
    Collaborate,
    Mode,
    Timeout,
    Edit,
    Suggest,
    Review,
    Consensus,
    Strategy,
    Majority,
    Unanimous,
    Require,
    Agree,
    Scenario,
    Given,
    Expect,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEq,
    BangEq,
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
    #[allow(clippy::too_many_lines)]
    fn keyword_str(&self) -> Option<&'static str> {
        let s = match self {
            Self::Agent => "agent",
            Self::Archetype => "archetype",
            Self::Can => "can",
            Self::Cannot => "cannot",
            Self::Model => "model",
            Self::Budget => "budget",
            Self::Per => "per",
            Self::Up => "up",
            Self::To => "to",
            Self::Workflow => "workflow",
            Self::Trigger => "trigger",
            Self::Stages => "stages",
            Self::Provider => "provider",
            Self::Step => "step",
            Self::Goal => "goal",
            Self::Tool => "tool",
            Self::Endpoint => "endpoint",
            Self::Guardrails => "guardrails",
            Self::Defaults => "defaults",
            Self::One => "one",
            Self::Of => "of",
            Self::Type => "type",
            Self::Import => "import",
            Self::From => "from",
            Self::All => "all",
            Self::At => "@",
            Self::Slash => "/",
            Self::Parallel => "parallel",
            Self::When => "when",
            Self::Failure => "failure",
            Self::Retry => "retry",
            Self::Then => "then",
            Self::Exponential => "exponential",
            Self::Linear => "linear",
            Self::Fixed => "fixed",
            Self::Escalate => "escalate",
            Self::Fallback => "fallback",
            Self::Where => "where",
            Self::Sort => "sort",
            Self::By => "by",
            Self::Take => "take",
            Self::Skip => "skip",
            Self::Select => "select",
            Self::Unique => "unique",
            Self::Asc => "asc",
            Self::Desc => "desc",
            Self::Pipe => "|",
            Self::Observe => "observe",
            Self::Fleet => "fleet",
            Self::Channel => "channel",
            Self::Trace => "trace",
            Self::Metrics => "metrics",
            Self::Alert => "alert",
            Self::Export => "export",
            Self::Agents => "agents",
            Self::Scaling => "scaling",
            Self::Min => "min",
            Self::Max => "max",
            Self::Retention => "retention",
            Self::Send => "send",
            Self::Within => "within",
            Self::CircuitBreaker => "circuit_breaker",
            Self::Auto => "auto",
            Self::Resolve => "resolve",
            Self::Is => "is",
            Self::Policy => "policy",
            Self::Tier => "tier",
            Self::Promote => "promote",
            Self::Or => "or",
            Self::And => "and",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::LtEq => "<=",
            Self::GtEq => ">=",
            Self::EqEq => "==",
            Self::BangEq => "!=",
            Self::Percent => "%",
            Self::Arrow => "->",
            Self::Route => "route",
            Self::On => "on",
            Self::Underscore => "_",
            Self::DotDot => "..",
            Self::LBrace => "{",
            Self::RBrace => "}",
            Self::LBracket => "[",
            Self::RBracket => "]",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::Colon => ":",
            Self::Dot => ".",
            Self::Comma => ",",
            Self::Eval => "eval",
            Self::Dataset => "dataset",
            Self::Assert => "assert",
            Self::Block => "block",
            Self::Deploy => "deploy",
            Self::Memory => "memory",
            Self::Working => "working",
            Self::Session => "session",
            Self::Knowledge => "knowledge",
            Self::Schedule => "schedule",
            Self::Daily => "daily",
            Self::Every => "every",
            Self::Hours => "hours",
            Self::For => "for",
            Self::Each => "each",
            Self::Input => "input",
            Self::Output => "output",
            Self::Human => "human",
            Self::Via => "via",
            Self::Secrets => "secrets",
            Self::Vault => "vault",
            Self::Approve => "approve",
            Self::Collaborate => "collaborate",
            Self::Mode => "mode",
            Self::Timeout => "timeout",
            Self::Edit => "edit",
            Self::Suggest => "suggest",
            Self::Review => "review",
            Self::Consensus => "consensus",
            Self::Strategy => "strategy",
            Self::Majority => "majority",
            Self::Unanimous => "unanimous",
            Self::Require => "require",
            Self::Agree => "agree",
            Self::Scenario => "scenario",
            Self::Given => "given",
            Self::Expect => "expect",
            Self::Comment => "comment",
            Self::Eof => "end of file",
            _ => return None,
        };
        Some(s)
    }

    /// Map a word to its keyword `TokenKind`, or `None` if it's a plain identifier.
    #[allow(clippy::too_many_lines)]
    pub fn from_word(word: &str) -> Option<TokenKind> {
        let kind = match word {
            "agent" => Self::Agent,
            "archetype" => Self::Archetype,
            "can" => Self::Can,
            "cannot" => Self::Cannot,
            "model" => Self::Model,
            "budget" => Self::Budget,
            "per" => Self::Per,
            "up" => Self::Up,
            "to" => Self::To,
            "workflow" => Self::Workflow,
            "trigger" => Self::Trigger,
            "stages" => Self::Stages,
            "provider" => Self::Provider,
            "step" => Self::Step,
            "goal" => Self::Goal,
            "tool" => Self::Tool,
            "endpoint" => Self::Endpoint,
            "guardrails" => Self::Guardrails,
            "defaults" => Self::Defaults,
            "one" => Self::One,
            "of" => Self::Of,
            "type" => Self::Type,
            "import" => Self::Import,
            "from" => Self::From,
            "all" => Self::All,
            "parallel" => Self::Parallel,
            "when" => Self::When,
            "failure" => Self::Failure,
            "retry" => Self::Retry,
            "then" => Self::Then,
            "exponential" => Self::Exponential,
            "linear" => Self::Linear,
            "fixed" => Self::Fixed,
            "escalate" => Self::Escalate,
            "fallback" => Self::Fallback,
            "where" => Self::Where,
            "sort" => Self::Sort,
            "by" => Self::By,
            "take" => Self::Take,
            "skip" => Self::Skip,
            "select" => Self::Select,
            "unique" => Self::Unique,
            "asc" => Self::Asc,
            "desc" => Self::Desc,
            "observe" => Self::Observe,
            "fleet" => Self::Fleet,
            "channel" => Self::Channel,
            "trace" => Self::Trace,
            "metrics" | "watch" => Self::Metrics,
            "alert" => Self::Alert,
            "export" => Self::Export,
            "agents" => Self::Agents,
            "scaling" => Self::Scaling,
            "min" => Self::Min,
            "max" => Self::Max,
            "retention" => Self::Retention,
            "send" => Self::Send,
            "within" => Self::Within,
            "circuit_breaker" => Self::CircuitBreaker,
            "auto" => Self::Auto,
            "resolve" => Self::Resolve,
            "is" => Self::Is,
            "policy" => Self::Policy,
            "tier" => Self::Tier,
            "promote" => Self::Promote,
            "or" => Self::Or,
            "and" => Self::And,
            "route" => Self::Route,
            "on" => Self::On,
            "_" => Self::Underscore,
            "eval" => Self::Eval,
            "dataset" => Self::Dataset,
            "assert" => Self::Assert,
            "block" => Self::Block,
            "deploy" => Self::Deploy,
            "memory" => Self::Memory,
            "working" => Self::Working,
            "session" => Self::Session,
            "knowledge" => Self::Knowledge,
            "schedule" => Self::Schedule,
            "daily" => Self::Daily,
            "every" => Self::Every,
            "hours" => Self::Hours,
            "for" => Self::For,
            "each" => Self::Each,
            "input" => Self::Input,
            "output" => Self::Output,
            "human" => Self::Human,
            "via" => Self::Via,
            "secrets" | "secret" => Self::Secrets,
            "vault" => Self::Vault,
            "approve" => Self::Approve,
            "collaborate" => Self::Collaborate,
            "mode" => Self::Mode,
            "timeout" => Self::Timeout,
            "edit" => Self::Edit,
            "suggest" => Self::Suggest,
            "review" => Self::Review,
            "consensus" => Self::Consensus,
            "strategy" => Self::Strategy,
            "majority" => Self::Majority,
            "unanimous" => Self::Unanimous,
            "require" => Self::Require,
            "agree" => Self::Agree,
            "scenario" => Self::Scenario,
            "given" => Self::Given,
            "expect" => Self::Expect,
            _ => return None,
        };
        Some(kind)
    }

    /// If this keyword token can be used as an identifier in value positions,
    /// return its string representation. Returns `None` for non-keyword tokens
    /// and for structural keywords that start top-level blocks.
    pub fn keyword_as_ident(&self) -> Option<&'static str> {
        match self {
            Self::Failure
            | Self::Retry
            | Self::Then
            | Self::Exponential
            | Self::Linear
            | Self::Fixed
            | Self::Escalate
            | Self::One
            | Self::Of
            | Self::On
            | Self::All
            | Self::From
            | Self::When
            | Self::Route
            | Self::Parallel
            | Self::Auto
            | Self::Resolve
            | Self::Is
            | Self::Policy
            | Self::Tier
            | Self::Fallback
            | Self::Where
            | Self::Sort
            | Self::By
            | Self::Take
            | Self::Skip
            | Self::Select
            | Self::Unique
            | Self::Asc
            | Self::Desc
            | Self::Observe
            | Self::Fleet
            | Self::Channel
            | Self::Trace
            | Self::Metrics
            | Self::Alert
            | Self::Export
            | Self::Agents
            | Self::Scaling
            | Self::Min
            | Self::Max
            | Self::Retention
            | Self::Send
            | Self::To
            | Self::Within
            | Self::CircuitBreaker
            | Self::Promote
            | Self::Eval
            | Self::Dataset
            | Self::Assert
            | Self::Block
            | Self::Deploy
            | Self::Memory
            | Self::Working
            | Self::Session
            | Self::Knowledge
            | Self::Schedule
            | Self::Daily
            | Self::Every
            | Self::Hours
            | Self::For
            | Self::Each
            | Self::Input
            | Self::Output
            | Self::Human
            | Self::Via
            | Self::Secrets
            | Self::Vault
            | Self::Approve
            | Self::Collaborate
            | Self::Mode
            | Self::Timeout
            | Self::Edit
            | Self::Suggest
            | Self::Review
            | Self::Consensus
            | Self::Strategy
            | Self::Majority
            | Self::Unanimous
            | Self::Require
            | Self::Agree
            | Self::Scenario
            | Self::Given
            | Self::Expect => self.keyword_str(),
            _ => None,
        }
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

mod scanner;
#[cfg(test)]
pub(crate) use scanner::parse_cents;
pub use scanner::tokenize;

#[cfg(test)]
mod tests;
