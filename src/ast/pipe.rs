use serde::{Deserialize, Serialize};

use super::{CompareOp, Span, WhenValue};

/// A pipe expression: `source | transform1 | transform2 | ...`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipeExpr {
    /// The source identifier (e.g. `products`).
    pub source: String,
    /// Ordered chain of transforms.
    pub transforms: Vec<PipeTransform>,
    pub span: Span,
}

/// Sort direction for `sort by` transforms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// A single transform in a pipe chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PipeTransform {
    /// `where field op value` — filter rows.
    Where {
        field: String,
        op: CompareOp,
        value: WhenValue,
    },
    /// `sort by field [asc|desc]` — order rows.
    SortBy {
        field: String,
        direction: SortDirection,
    },
    /// `take N` — limit to first N items.
    Take { count: u32 },
    /// `skip N` — skip first N items.
    Skip { count: u32 },
    /// `select field1, field2, ...` — project specific fields.
    Select { fields: Vec<String> },
    /// `unique` or `unique field` — deduplicate.
    Unique { field: Option<String> },
}
