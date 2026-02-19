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
- **Commit format:** `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`
- **All tests must pass** (`cargo test`) before committing.
- **Run `cargo fmt`** before committing. No formatting diffs allowed.
- **Run `cargo clippy`** before committing. No warnings allowed (pedantic enabled).

### File Structure Rules
- **Tests go in separate files.** Never inline `#[cfg(test)] mod tests { ... }` in source files.
- Use `mod.rs` + `tests.rs` pattern: logic in `mod.rs`, tests in `tests.rs` within a directory.
- Small modules (< 200 lines with no tests) can stay as flat files (e.g., `ast.rs`, `error.rs`).
- No file should exceed ~400 lines of logic. If it does, split it.

### Review Criteria (what reviewers check)
1. **Does this work?** — Tests pass, logic correct, edge cases handled
2. **Does this adhere to SOLID principles?**
3. **Would I merge this into production?** — No shortcuts, no tech debt

### Project Structure
```
src/
  main.rs              — CLI entry point (clap), thin dispatch
  lib.rs               — Public API re-exports
  ast.rs               — AST type definitions
  error.rs             — Error types with spans and pretty-printing
  commands/
    mod.rs             — Command module declarations + `rein run`
    validate.rs        — `rein validate` command
  lexer/
    mod.rs             — Tokenizer for .rein files
    tests.rs           — Lexer tests
  parser/
    mod.rs             — Recursive descent parser → AST
    tests.rs           — Parser tests
  validator/
    mod.rs             — Validation passes
    tests.rs           — Validator tests
  runtime/
    mod.rs             — Runtime types (ToolCall, ToolResult, RunEvent, RunTrace, RunError)
    permissions.rs     — Permission engine
    tests.rs           — Permission engine tests
    provider/
      mod.rs           — Provider trait, types, MockProvider
      tests.rs         — Provider trait tests
      openai/
        mod.rs         — OpenAI Chat Completions client
        tests.rs       — OpenAI tests (wiremock)
      resolver/
        mod.rs         — Model field → Provider mapping
        tests.rs       — Resolver tests
tests/
  cli.rs               — Integration tests
examples/
  basic.rein           — Simple agent definition
  multi_agent.rein     — Multiple agents
  invalid.rein         — Intentionally broken for error testing
```

### Tech Stack
- Repo: `delamain-labs/rein` (private)
- Language: Rust (single binary, no runtime deps)
- Error reporting: `ariadne` crate
- CLI: `clap` crate
- Serialization: `serde` + `serde_json`
- License: MIT

## Current State
- `rein validate` — fully functional parser + validator
- `rein run` — CLI command wired up (no execution yet)
- Runtime types, config, tool registry, permissions engine all implemented
- Provider trait with async-trait, OpenAI client, model resolver all implemented
- 162 tests (154 unit + 8 integration), all passing
- clippy pedantic enabled, zero warnings
- Phase 1 complete, Phase 2 in progress

### Runtime modules (src/runtime/)
```
runtime/
  mod.rs              — Runtime types (ToolCall, ToolResult, RunEvent, RunTrace, RunError)
  permissions.rs      — Permission engine (default-deny, cannot-overrides-can, monetary caps)
  tests.rs            — Permission engine tests (21 tests)
  provider/
    mod.rs            — Provider trait, types (Message, ChatResponse, ToolDef, etc.), MockProvider
    tests.rs          — Provider trait tests (10 tests)
    openai/
      mod.rs          — OpenAI Chat Completions API client
      tests.rs        — OpenAI tests with wiremock (7 tests)
    resolver/
      mod.rs          — Maps .rein model field to Provider instance
      tests.rs        — Resolver tests (8 tests)
```

### Dependencies
- `async-trait` — dyn-compatible async Provider trait
- `tokio` — async runtime (full features)
- `reqwest` — HTTP client (json feature)
- `wiremock` — dev dependency for HTTP mock testing

## Phase 2 Remaining Work
Issues to implement (in dependency order):
1. #45: Tool call interceptor — intercept tool calls, check permissions
2. #46: Monetary constraint enforcement — enforce spend limits per tool
3. #47: Token counting and cost calculation
4. #48: Budget enforcement — global budget tracking
5. #49: Agent execution loop — main loop: LLM → tool calls → results → repeat
6. #50: Mock tool executor — execute tools in test mode
7. #51: Run trace output — structured output of what happened
8. #52: Unit tests for permission enforcement (may already be covered)
9. #53: Unit tests for budget tracking
10. #54: Integration test: full agent run
11. #55: Example .rein files for rein run
12. #56: Anthropic provider
