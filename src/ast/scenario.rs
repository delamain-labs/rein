use serde::{Deserialize, Serialize};

use super::Span;

/// A `scenario <name> { given { ... } expect { ... } }` test block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioDef {
    pub name: String,
    /// Key-value pairs for the given context.
    pub given: Vec<(String, String)>,
    /// Expected outcomes.
    pub expect: Vec<(String, String)>,
    pub span: Span,
}
