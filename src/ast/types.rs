use serde::{Deserialize, Serialize};

use super::Span;

/// A type expression constraining a field's value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeExpr {
    /// A built-in or user-defined named type (e.g. `string`, `int`, `MyType`).
    Named { name: String, array: bool },
    /// A union of allowed values: `one of [billing, technical, general]`.
    OneOf { variants: Vec<String>, span: Span },
    /// A numeric range: `1..10` or `0.0..1.0`.
    Range { min: String, max: String },
}

/// A field within a `type` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub span: Span,
}

/// A `type Name { ... }` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<TypeField>,
    pub span: Span,
}
