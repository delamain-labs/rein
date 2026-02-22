use crate::ast::{ConsensusDef, ConsensusRequirement, ConsensusStrategy, Span};
use crate::lexer::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// Parse `consensus <name> { agents: [...], strategy: majority, require: N of M agree }`
    pub(super) fn parse_consensus(&mut self) -> Result<ConsensusDef, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Consensus)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut agents = Vec::new();
        let mut strategy = None;
        let mut require = None;

        while *self.peek() != TokenKind::RBrace {
            self.skip_comments();
            match self.peek() {
                TokenKind::Agents => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    agents = self.parse_ident_list()?;
                }
                TokenKind::Strategy => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    strategy = Some(self.parse_consensus_strategy()?);
                }
                TokenKind::Require => {
                    self.advance();
                    self.expect(&TokenKind::Colon)?;
                    require = Some(self.parse_consensus_requirement()?);
                }
                _ => {
                    return Err(ParseError::new(
                        format!("unexpected token in consensus block: {}", self.peek()),
                        self.current_span(),
                    ));
                }
            }
        }

        let end = self.current_span().end;
        self.advance(); // consume RBrace

        Ok(ConsensusDef {
            name,
            agents,
            strategy: strategy.unwrap_or(ConsensusStrategy::Majority),
            require,
            span: Span::new(start, end),
        })
    }

    fn parse_consensus_strategy(&mut self) -> Result<ConsensusStrategy, ParseError> {
        match self.peek() {
            TokenKind::Majority => {
                self.advance();
                Ok(ConsensusStrategy::Majority)
            }
            TokenKind::Unanimous => {
                self.advance();
                Ok(ConsensusStrategy::Unanimous)
            }
            _ => {
                let (name, _) = self.expect_ident()?;
                Ok(ConsensusStrategy::Custom(name))
            }
        }
    }

    /// Parse `N of M agree`.
    fn parse_consensus_requirement(&mut self) -> Result<ConsensusRequirement, ParseError> {
        let required = u32::try_from(self.expect_integer()?).unwrap_or(u32::MAX);
        self.expect(&TokenKind::Of)?;
        let total = u32::try_from(self.expect_integer()?).unwrap_or(u32::MAX);
        self.expect(&TokenKind::Agree)?;
        Ok(ConsensusRequirement { required, total })
    }
}
