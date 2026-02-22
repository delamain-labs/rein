# CLAUDE.md — Rein Project Context

Rein is a declarative language and runtime for AI agent orchestration. Think "Terraform for AI agents."

## Project Structure
```
src/
  ast/         — AST types (25+ block types)
  lexer/       — Tokenizer (scanner.rs, mod.rs, tests.rs)
  parser/      — Recursive descent parser (mod.rs + specialized parsers)
  validator/   — Semantic validation + strict mode
  runtime/     — Agent execution, workflows, tracing
  commands/    — CLI commands (validate, fmt, run, serve, etc.)
  server/      — REST API (axum)
  main.rs      — CLI entry point (clap)
examples/      — Example .rein files
docs/          — Language reference, spec, getting started
```

## Key Conventions
- **TDD always.** Tests in separate `tests.rs` files alongside `mod.rs`.
- **No file over ~400 lines.** Split when approaching.
- **clippy pedantic.** `cargo clippy -- -D warnings` must pass.
- **Branch per task.** PR, squash-merge to master. Never stack PRs.
- **Commit format:** `feat(scope): description. Closes #XX`

## The Language
- `.rein` files define agents, workflows, policies, and governance
- 25+ block types, full parser support
- Runtime only executes basic agent runs + sequential workflows
- Most features are parse-only (use `--strict` to see which)

## CLI
```
rein validate [--ast] [--strict] [--format json] <file>
rein fmt [--check] <files>
rein init [name]
rein explain <file>
rein cost <paths>
rein run [--dry-run] [-m "msg"] <file>
rein serve [--host H] [--port P] <file>
```

## Testing
```bash
cargo test                      # 517 tests
cargo clippy -- -D warnings     # zero warnings
cargo test -- --test-threads=1  # if tests conflict
```

## What's Enforced at Runtime
- Agent model, can/cannot lists, basic budget tracking
- Sequential workflow step execution
- Tool permission enforcement (default-deny)
- Everything else parses but doesn't execute (yet)
