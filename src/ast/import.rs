use serde::{Deserialize, Serialize};

use super::Span;

/// An import declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportDef {
    /// Named imports: `import { name1, name2 } from "./path.rein"`.
    Named {
        names: Vec<String>,
        source: String,
        span: Span,
    },
    /// Glob import: `import all from "./dir/"`.
    Glob { source: String, span: Span },
    /// Registry import: `import from @scope/name`.
    Registry {
        scope: String,
        name: String,
        span: Span,
    },
}
