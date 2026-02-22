# Rein — Copilot Instructions

Rein is a declarative DSL for AI agent governance (`.rein` files).

## Syntax Basics
- Blocks: `agent name { ... }`, `workflow name { ... }`
- Fields: `key: value` (colon-separated)
- Lists: `can: action1, action2` or `can [ action1 \n action2 ]`
- Comments: `//`, `#`, `/* */`
- Budget: `$100 per day`, `€50 per month`
- Env refs: `env("KEY")`, `env("KEY", "default")`
- Conditions: `when: field > value and field < value`

## Project Conventions
- Rust, edition 2024
- Tests in `tests.rs` files (not inline `#[cfg(test)]`)
- clippy pedantic: `cargo clippy -- -D warnings`
- No file over 400 lines
- Squash-merge PRs to master
