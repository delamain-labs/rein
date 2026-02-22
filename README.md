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
# From source
cargo install --path .

# Or build from the repo
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
    key: env("ANTHROPIC_KEY")
}

// Agent permissions and budgets
agent reviewer from senior_template {
    model: anthropic
    can [ github.read_pr, github.comment ]
    cannot [ github.merge, github.delete_branch ]
    budget: $10 per day
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

// Multi-step workflows
workflow triage {
    stage classify {
        agent: classifier
    }
    stage handle {
        agent: reviewer
        when: confidence > 80%
    }
    route on classify.category {
        billing -> stage handle_billing { agent: billing_bot }
        _ -> stage escalate { agent: human_handoff }
    }
}

// Guardrails
guardrails {
    spending {
        soft_limit: $100 per day
        hard_limit: $500 per day
    }
    output_filter {
        pii: redact
        toxicity: block
    }
}

// Progressive trust
policy {
    tier supervised {
        promote when accuracy > 95%
    }
    tier autonomous {
        demote when error_rate > 5%
    }
}
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `rein init <dir>` | Scaffold a new project |
| `rein validate <files>` | Parse and validate `.rein` files |
| `rein validate --ast <file>` | Dump the AST as JSON |
| `rein fmt <files>` | Auto-format to canonical style |
| `rein fmt --check <files>` | Check formatting without modifying |
| `rein cost <traces>` | Aggregate costs from trace files |
| `rein run <file>` | Execute an agent (requires API keys) |

## Project Status

Rein is in active development. Here's what works today:

- ✅ **Parser and validator** — full DSL with 20+ block types, 513 tests, zero clippy warnings
- ✅ **CLI tooling** — init, validate, fmt, cost commands
- ✅ **Error messages** — precise diagnostics with source spans (powered by ariadne)
- ✅ **Tree-sitter grammar** — syntax highlighting for editors
- 🔧 **Runtime** — basic agent execution and workflow engine functional, advanced features in progress
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
