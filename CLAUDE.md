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
    mod.rs             — Command module declarations
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
- 99 tests (91 unit + 8 integration), all passing
- clippy pedantic enabled, zero warnings
- All Phase 1 backlog issues (#9–#22) resolved

## Next Phase: `rein run`
Build the runtime that executes agents within their declared constraints.

When completely finished with a task, run:
```
openclaw system event --text "Done: <description>" --mode now
```
