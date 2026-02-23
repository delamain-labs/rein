# Rein

**Define what your AI agents can do, what they can't, and how much they can spend.**

Rein is a declarative policy language for AI agent governance. Write readable `.rein` files that enforce permissions, budgets, and guardrails on your agents, then validate them in CI or enforce them at runtime.

```rein
agent support_bot {
    model: "gpt-4o"

    can [
        zendesk.read_ticket
        zendesk.reply
    ]

    cannot [
        zendesk.delete_ticket
        stripe.refund
    ]

    budget: $50 per day
}
```

No YAML. No JSON. Just policy that reads like English.

## Why Rein?

AI agents are shipping to production, but **trust** is the bottleneck, not capability. Rein gives you:

- **Default-deny permissions** — agents can only use tools you explicitly allow
- **Budget controls** — `$50 per day`, `$0.03 per ticket`, hard limits that can't be bypassed
- **Readable policies** — non-engineers can review what an agent is allowed to do
- **Validation in CI** — catch policy errors before deployment, not after

## Install

```bash
# From crates.io
cargo install rein-lang

# Or from source
git clone https://github.com/delamain-labs/rein.git
cd rein && cargo build --release
```

## Quick Start

```bash
# Scaffold a new project
rein init my-project
cd my-project

# Edit your agent policy
$EDITOR agents/main.rein

# Validate it
rein validate agents/main.rein

# Format it
rein fmt agents/main.rein
```

## What You Can Define

Rein supports a rich policy language with 20+ block types:

```rein
// Providers and models
provider anthropic {
    model: "claude-sonnet-4-20250514"
    key: env("ANTHROPIC_API_KEY")
}

// Agent permissions and budgets
agent reviewer from senior_template {
    model: anthropic

    can [
        github.read_pr
        github.comment
    ]

    cannot [
        github.merge
        github.delete_branch
    ]

    budget: $10 per day

    guardrails {
        output_filter {
            pii: redact
            toxicity: block
        }
    }
}

// Reusable templates
archetype senior_template {
    budget: $25 per day
}

// Type-safe data contracts
type TicketCategory {
    label: one of [billing, technical, general]
    confidence: percentage
}

// Multi-step workflows with approval gates
workflow triage {
    trigger: new_ticket

    step classify {
        agent: reviewer
        goal: "Classify the incoming ticket"
    }

    step handle {
        agent: reviewer
        goal: "Handle the classified ticket"
        when: confidence > 80%
        approve: human via slack("#reviews") timeout "1h"
    }
}

// Progressive trust
policy {
    tier supervised {
        promote when accuracy > 95%
    }
    tier autonomous {
        promote when accuracy > 99%
    }
    tier fully_autonomous {}
}

// Circuit breaker
circuit_breaker api_safety {
    open after: 5 failures in 10 min
    half_open after: 2 min
}
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `rein init [dir]` | Scaffold a new project |
| `rein validate <files>` | Parse and validate `.rein` files |
| `rein validate --strict <file>` | Warn on unenforced features |
| `rein validate --ast <file>` | Dump the AST as JSON |
| `rein fmt <files>` | Auto-format to canonical style |
| `rein fmt --check <files>` | Check formatting (CI mode) |
| `rein explain <file>` | Human-readable policy summary |
| `rein run <file> [-m "msg"]` | Execute an agent (requires API keys) |
| `rein run --dry-run <file>` | Show execution plan without calling APIs |
| `rein cost <traces>` | Aggregate costs from trace files |
| `rein serve <file>` | Start the REST API server |
| `rein lsp` | Start the language server |

## Project Status

Rein is in active development. Here's what works today:

- ✅ **Parser and validator** — full DSL with 20+ block types, 649 tests, zero clippy warnings
- ✅ **CLI tooling** — init, validate, fmt, cost, explain, run, serve
- ✅ **Runtime enforcement** — guardrails, circuit breakers, approval gates, policy engine, budget limits
- ✅ **Agent execution** — `rein run` talks to OpenAI/Anthropic with OTLP trace export
- ✅ **Error messages** — precise diagnostics with source spans (powered by ariadne)
- ✅ **Tree-sitter grammar** — syntax highlighting for editors
- ✅ **LSP server** — editor integration via `rein lsp`
- 📋 **[Language Reference](docs/language-reference.md)** — every block type and feature
- 🚀 **[Getting Started](docs/getting-started.md)** — zero to validated policy in 5 minutes

## Architecture

Rein is built in Rust for correctness and speed.

```
src/
├── lexer/       # Tokenizer with 50+ token types
├── parser/      # Recursive descent parser, split into submodules
├── ast/         # Typed AST for all block types
├── validator/   # Semantic analysis and cross-reference checks
├── runtime/     # Agent execution, workflow engine, tracing
├── cli/         # Command implementations
└── server/      # REST API (axum)
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

[MIT](LICENSE)
