# Rein Language Specification

**Version:** 0.1.0
**Status:** Draft

This document defines the syntax and semantics of the Rein declarative language for AI agent governance.

---

## 1. Overview

Rein is a declarative, statically-validated domain-specific language for defining AI agent policies, capabilities, workflows, and governance. Files use the `.rein` extension and are UTF-8 encoded.

**Design principles:**
- **Declarative over imperative** — describe what agents can do, not how
- **Safety-first** — constraints are first-class, not afterthoughts
- **Readable by non-engineers** — a PM or compliance officer should understand a `.rein` file
- **Vendor-neutral** — works with any AI provider

---

## 2. Lexical Structure

### 2.1 Comments

```
// Line comment
# Line comment (alternative)
/* Block comment */
```

### 2.2 Keywords

**Block keywords:** `agent`, `archetype`, `defaults`, `provider`, `tool`, `workflow`, `step`, `policy`, `observe`, `fleet`, `channel`, `circuit_breaker`, `eval`, `memory`, `secrets`, `consensus`, `scenario`, `schedule`, `approval`, `escalate`, `type`, `import`

**Field keywords:** `model`, `can`, `cannot`, `budget`, `from`, `trigger`, `goal`, `when`, `on`, `failure`, `retry`, `then`, `escalate`, `fallback`, `one_of`, `send_to`, `input`, `output`, `for`, `each`, `auto`, `resolve`, `per`, `up`, `to`, `within`, `via`, `approve`, `collaborate`, `mode`, `timeout`, `require`, `agree`, `strategy`, `given`, `expect`, `assert`, `dataset`

**Operator keywords:** `and`, `or`, `is`

**Strategy keywords:** `exponential`, `linear`, `fixed`, `majority`, `unanimous`

**Execution keywords:** `parallel`, `route`, `pipe`

### 2.3 Identifiers

Identifiers start with an ASCII letter or underscore, followed by letters, digits, underscores, or dots.

```
agent_name
my.namespaced.agent
_private
```

Keywords can appear as identifiers in value positions (e.g., `model: openai` where `openai` is an identifier, not a keyword).

### 2.4 Literals

**Strings:** Double-quoted. `"hello world"`

**Numbers:** Integer or floating-point. `42`, `3.14`

**Percentages:** Number followed by `%`. `70%`, `95.5%`

**Currency:** Symbol followed by amount. `$100`, `€50`, `£25.50`, `¥10000`
- Amounts are stored in minor units (cents): `$50.00` = 5000
- Supported symbols: `$` (USD), `€` (EUR), `£` (GBP), `¥` (JPY)

**Booleans:** `on`, `off`, `true`, `false` (in field values)

### 2.5 Operators

**Comparison:** `<`, `>`, `<=`, `>=`, `==`, `!=`

**Logical:** `and` (higher precedence), `or` (lower precedence)

**Other:** `|>` (pipe), `->` (arrow/route), `..` (range), `:` (field assignment)

### 2.6 Delimiters

`{` `}` (blocks), `[` `]` (lists), `(` `)` (function calls), `,` (separators)

---

## 3. File Structure

A `.rein` file consists of zero or more top-level declarations in any order:

```
file = (import | defaults | provider | tool | archetype | agent
       | workflow | type | policy | observe | fleet | channel
       | circuit_breaker | eval | memory | secrets | consensus
       | scenario | schedule | approval | escalate)*
```

There is no required ordering, but conventionally: imports, defaults, providers, types, agents, workflows.

---

## 4. Type System

### 4.1 Built-in Types

`int`, `float`, `string`, `bool`

### 4.2 Range Types

```
int 1..5        // integer range, inclusive
float 0.0..1.0  // float range, inclusive
```

### 4.3 Custom Types

```
type Priority {
    level: int 1..5
    label: string
    urgent: bool
}
```

### 4.4 Union Types

```
category: one of [billing, technical, general]
```

---

## 5. Expressions

### 5.1 Value Expressions

A value can be:
- A string literal: `"hello"`
- An identifier: `openai`
- An env reference: `env("API_KEY")` or `env("API_KEY", "default")`

### 5.2 When Expressions

Guard conditions with boolean logic:

```
when: field op value
when: expr and expr
when: expr or expr
```

Precedence: `and` binds tighter than `or`.

Operators: `<`, `>`, `<=`, `>=`, `==`, `!=`

Values: numbers, percentages, currency amounts, string literals, identifiers.

### 5.3 Pipe Expressions

Data transformation chains:

```
data |> select field1, field2 |> where field > value |> sort field desc |> unique
```

Transforms: `select`, `where`, `sort` (with `asc`/`desc`), `unique`

### 5.4 Budget Expressions

```
budget: $100 per day
budget: €500 per month
budget: £50 per hour
```

### 5.5 Capability Expressions

```
can: action1, action2
can: action up_to $500
can [
    action1
    action2
]
```

---

## 6. Top-Level Blocks

See [Language Reference](./language-reference.md) for detailed syntax, fields, examples, and runtime status of all 25+ block types.

---

## 7. Validation

The Rein validator performs:

1. **Syntax validation** — correct grammar and structure
2. **Semantic validation** — required fields present, valid references, no duplicates
3. **Strict validation** (`--strict`) — warns when safety features parse but are not enforced at runtime

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Valid, no issues |
| 1 | Parse or validation errors |
| 2 | No errors, but strict warnings present |

---

## 8. Runtime Enforcement

Not all parsed features are enforced at runtime. The language specification defines the *intended* semantics. The current runtime status is documented in the [Language Reference](./language-reference.md) with ✅ (enforced) and ⚠️ (parse-only) markers.

Features that parse but do not enforce will be progressively implemented. Use `rein validate --strict` to identify unenforced features in your policies.

---

## Appendix A: Grammar (Simplified EBNF)

```ebnf
file        = declaration*
declaration = agent_def | workflow_def | provider_def | defaults_def
            | archetype_def | tool_def | type_def | policy_def
            | observe_def | fleet_def | channel_def | circuit_breaker_def
            | eval_def | memory_def | secrets_def | consensus_def
            | scenario_def | schedule_def | approval_def | escalate_def
            | import_def

agent_def   = "agent" IDENT "{" agent_field* "}"
agent_field = "model" ":" value_expr
            | "can" ":" cap_list | "can" "[" cap_list "]"
            | "cannot" ":" cap_list | "cannot" "[" cap_list "]"
            | "budget" ":" budget_expr
            | "from" ":" IDENT
            | "guardrails" "{" kv_pair* "}"

workflow_def = "workflow" IDENT "{" workflow_field* "}"
workflow_field = "trigger" ":" value_expr
               | "stages" ":" ident_list
               | step_def | parallel_block | route_block
               | auto_resolve_block

step_def    = "step" IDENT "{" step_field* "}"
step_field  = "agent" ":" IDENT
            | "goal" ":" STRING
            | "when" ":" when_expr
            | "send_to" ":" IDENT
            | "fallback" ":" IDENT
            | "one_of" ":" string_list
            | "input" ":" IDENT
            | "output" ":" IDENT
            | "for" "each" ":" IDENT
            | retry_policy
            | escalate_block
            | within_block

value_expr  = STRING | IDENT | env_ref
env_ref     = "env" "(" STRING ("," STRING)? ")"
when_expr   = or_expr
or_expr     = and_expr ("or" and_expr)*
and_expr    = comparison ("and" comparison)*
comparison  = IDENT compare_op when_value
compare_op  = "<" | ">" | "<=" | ">=" | "==" | "!="
when_value  = NUMBER | NUMBER "%" | CURRENCY | STRING | IDENT
budget_expr = CURRENCY "per" IDENT
cap_list    = capability ("," capability)*
capability  = IDENT ("up_to" CURRENCY)?
```

This is a simplified grammar. The full grammar is defined by the parser implementation in `src/parser/`.
