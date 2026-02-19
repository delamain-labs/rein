# Rein — Phase 1 Implementation Plan

## Goal
Build `rein validate` CLI: parse `.rein` files into a typed AST, validate them, and print human-readable errors.

## Architecture
```
src/
  main.rs          — CLI (clap): rein validate <file> [--ast]
  ast.rs           — AST type definitions (AgentDef, Capability, Budget, etc.)
  lexer.rs         — Tokenizer: .rein source → Token stream
  parser.rs        — Recursive descent parser: Token stream → AST
  validator.rs     — Validation passes on the AST
  error.rs         — Error types with source spans + pretty printing
  lib.rs           — Re-exports for testing
```

## Steps (TDD — tests first, then implementation)

### Step 1: AST Types + Cargo Setup (15 min)
- Add deps to Cargo.toml: `clap` (derive), `serde` + `serde_json`, `ariadne` (error reporting)
- Define AST types in `ast.rs`:
  - `ReinFile { agents: Vec<AgentDef> }`
  - `AgentDef { name: String, model: Option<String>, can: Vec<Capability>, cannot: Vec<Capability>, budget: Option<Budget>, span: Span }`
  - `Capability { namespace: String, action: String, constraint: Option<Constraint>, span: Span }`
  - `Constraint::MonetaryCap { amount: f64, currency: String }`
  - `Budget { amount: f64, currency: String, unit: String, span: Span }`
  - `Span { start: usize, end: usize }` (byte offsets)
- **Test:** AST types serialize to JSON correctly
- **Commit:** `feat: define AST types`

### Step 2: Lexer (20 min)
- Token types:
  - Keywords: `agent`, `can`, `cannot`, `model`, `budget`, `per`, `up`, `to`
  - Symbols: `{`, `}`, `[`, `]`, `:`, `.`
  - Literals: `Ident(String)`, `Dollar(f64)`, `Percent(f64)`
  - `Newline`, `Comment`, `Eof`
- Lexer struct: takes `&str`, produces `Vec<Token>` with spans
- **Tests (write first):**
  - Tokenize `agent foo {` → `[Agent, Ident("foo"), LBrace]`
  - Tokenize `$0.03` → `[Dollar(0.03)]`
  - Tokenize `zendesk.read_ticket` → `[Ident("zendesk"), Dot, Ident("read_ticket")]`
  - Tokenize `up to $50` → `[Up, To, Dollar(50.0)]`
  - Tokenize `// comment` → `[Comment]`
  - Error on invalid chars
- **Commit:** `feat: lexer tokenizes .rein files`

### Step 3: Parser (25 min)
- Recursive descent:
  - `parse_file()` → `ReinFile`
  - `parse_agent()` → `AgentDef`
  - `parse_model()` → `String`
  - `parse_capability_list()` → `Vec<Capability>`
  - `parse_capability()` → `Capability` (with optional `up to $X` constraint)
  - `parse_budget()` → `Budget`
- **Tests (write first):**
  - Parse minimal agent: `agent foo { model: anthropic }` → correct AST
  - Parse full agent with can/cannot/budget → correct AST
  - Parse `up to $50` constraint → MonetaryCap
  - Parse multiple agents in one file
  - Error on missing `{`, missing `}`, missing agent name
  - Error on `can` without `[`
- **Commit:** `feat: parser produces typed AST from .rein tokens`

### Step 4: Validator (10 min)
- Validation passes:
  - Duplicate agent names → error
  - Capability in both `can` and `cannot` → error
  - Budget amount ≤ 0 → error
  - Missing model → warning
- **Tests (write first):**
  - Duplicate agent names detected
  - Same tool in can + cannot detected
  - Zero/negative budget detected
- **Commit:** `feat: validator catches semantic errors`

### Step 5: Error Reporting (10 min)
- Use `ariadne` for pretty errors with source spans
- Error format:
  ```
  error[E001]: duplicate agent name 'support'
    --> agent.rein:8:1
     |
   8 | agent support {
     | ^^^^^^^^^^^^^ 'support' already defined on line 1
  ```
- Warning format similar but yellow
- **Test:** Error messages contain expected text
- **Commit:** `feat: pretty error reporting with ariadne`

### Step 6: CLI (10 min)
- `rein validate <file>` — parse + validate, print errors or "✓ Valid"
- `rein validate --ast <file>` — dump AST as JSON
- Exit code 0 on success, 1 on errors
- **Test:** Integration test: run binary on example files, check exit codes
- **Commit:** `feat: rein validate CLI`

### Step 7: Example Files + Polish (10 min)
- `examples/basic.rein` — single agent
- `examples/multi_agent.rein` — two agents
- `examples/invalid.rein` — intentionally broken
- README.md with usage
- **Commit:** `docs: examples and README`

## Total Estimated Time: ~100 minutes

## Dependencies
```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ariadne = "0.4"
```

## Test Strategy (TDD)
1. Write failing test
2. Write minimal code to pass
3. Refactor
4. Repeat

All parser rules get at least: one happy path, one error path.
Integration tests run the actual binary on example files.
