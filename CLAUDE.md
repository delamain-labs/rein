# Rein

You are working on **Rein**, a declarative language + runtime for AI agent orchestration. "The Terraform of AI agents." This is the Rust runtime.

## What Rein Is
A vendor-neutral control plane for production AI agents. `.rein` files define agents with permissions (`can`/`cannot`), budgets, and tool access. The runtime **enforces** these at execution time — not via prompts, but via actual runtime interception.

## Development Rules (ALWAYS follow these)

### Workflow
1. Every piece of work starts as a **GitHub issue** on `delamain-labs/rein`
2. Create a **branch** from `master` for each issue
3. **TDD** — write tests FIRST, then implementation
4. Push branch, open a **PR** referencing the issue
5. PRs are reviewed before merge. Never merge without review.
6. Out-of-scope findings become new GitHub issues

### Code Standards
- **Idiomatic Rust.** Proper `Result<T, E>`. No `.unwrap()` outside tests.
- **SOLID principles.** Single responsibility per module. Open for extension.
- **Small commits.** One logical change per commit.
- **Commit format:** `feat:`, `fix:`, `docs:`, `test:`, `refactor:`
- **All tests must pass** (`cargo test`) before committing.
- **Run `cargo fmt`** before committing. No formatting diffs allowed.
- **Run `cargo clippy`** before committing. No warnings allowed.

### Review Criteria (what reviewers check)
1. **Does this work?** — Tests pass, logic correct, edge cases handled
2. **Does this adhere to SOLID principles?**
3. **Would I merge this into production?** — No shortcuts, no tech debt

### Project Structure
- Repo: `delamain-labs/rein` (private)
- Project board: GitHub Projects on delamain-labs
- Language: Rust (single binary, no runtime deps)
- Error reporting: `ariadne` crate
- CLI: `clap` crate

## Phase 1 Goal (This Session)
Build `rein validate` — a CLI that parses `.rein` files into a typed AST and validates them.

### The `.rein` Language (Subset for Phase 1)

```rein
agent support_triage {
    model: anthropic

    can [
        zendesk.read_ticket
        zendesk.reply_ticket
        zendesk.refund up to $50
    ]

    cannot [
        zendesk.delete_ticket
        zendesk.admin
    ]

    budget: $0.03 per ticket
}
```

#### Grammar rules for Phase 1:
- `agent <name> { ... }` — top-level agent definition
- `model: <provider>` — LLM provider (string)
- `can [ ... ]` — list of allowed tool capabilities
- `cannot [ ... ]` — list of denied tool capabilities
- Tool capabilities are `namespace.action` (e.g., `zendesk.read_ticket`)
- Tool capabilities can have constraints: `up to $<amount>` (monetary cap)
- `budget: $<amount> per <unit>` — spending limit per invocation unit
- Comments with `//` and `/* */`
- Whitespace-insensitive

#### AST types needed:
- `AgentDef` — name, model, capabilities (can/cannot), budget
- `Capability` — tool reference (namespace + action) + optional constraint (monetary cap)
- `Budget` — amount (f64), currency (USD for now), unit (string)

### CLI Interface
```
rein validate <file.rein>     # Parse + validate, print errors or "Valid ✓"
rein validate --ast <file.rein>  # Parse + dump AST as JSON
```

### Error Messages
Errors should be **human-readable**, with line/column, the offending text, and a suggestion. Example:
```
error[E001]: unknown tool namespace 'zendsk'
  --> agent.rein:5:9
   |
 5 |         zendsk.read_ticket
   |         ^^^^^^ did you mean 'zendesk'?
   |
   = note: tool namespaces must be registered before use
```

(For Phase 1, we won't have a registry — just parse and validate syntax. Semantic validation of tool names comes later.)

### Architecture
```
src/
  main.rs          — CLI entry point (clap)
  lexer.rs         — Tokenizer for .rein files
  parser.rs        — Recursive descent parser → AST
  ast.rs           — AST type definitions
  validator.rs     — Validation passes (syntax correctness, budget sanity, etc.)
  error.rs         — Error types with spans and pretty-printing
```

### Dependencies (keep minimal)
- `clap` — CLI argument parsing
- `serde` + `serde_json` — AST serialization for `--ast` flag
- `miette` or `ariadne` — pretty error reporting with source spans
- That's it. No other deps.

### Quality Bar
- `cargo build` produces a single binary
- `cargo test` has tests for: valid agent parsing, missing fields, malformed budgets, unknown syntax, constraint parsing
- Error messages are colorful and helpful, not stack traces
- Code is idiomatic Rust with proper error handling (no unwrap in non-test code)

### Example test files to create:
- `examples/basic.rein` — the support_triage agent above
- `examples/multi_agent.rein` — two agents in one file
- `examples/invalid.rein` — intentionally broken for error testing

## Non-Goals (Do NOT build these yet)
- No `rein run` (execution comes in Phase 1b)
- No LLM API calls
- No workflow syntax
- No durable execution
- No Tree-sitter grammar (yet)
- No LSP

## Style
- Idiomatic Rust. Proper `Result<T, E>` handling.
- Good module separation.
- Tests for every parser rule.
- Commit after each milestone (lexer done, parser done, validator done, CLI done).

When completely finished, run this command to notify me:
openclaw system event --text "Done: rein validate CLI — parses .rein files into typed AST with pretty error reporting" --mode now
