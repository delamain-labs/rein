# Getting Started with Rein

Define governance for an AI agent in 5 minutes. No API keys required.

---

## Install

**From source (requires Rust):**

```bash
git clone https://github.com/delamain-labs/rein.git
cd rein
cargo install --path .
```

**Verify installation:**

```bash
rein --version
# rein 0.1.0
```

---

## 1. Initialize a Project

```bash
rein init my-project
cd my-project
```

This creates:

```
my-project/
├── agents/
│   └── assistant.rein    # Your first agent policy
├── rein.toml             # Project configuration
├── .env.example          # Environment variable template
├── .gitignore
└── README.md
```

---

## 2. Explore the Default Policy

Open `agents/assistant.rein`:

```rein
// My first Rein agent

agent assistant {
    model: openai

    can [
        search.web
        files.read
    ]

    cannot [
        files.delete
    ]

    budget: $0.10 per request
}
```

This defines an agent that:
- Uses OpenAI's model
- **Can** search the web and read files
- **Cannot** delete files
- Has a budget cap of $0.10 per request

---

## 3. Validate Your Policy

```bash
rein validate agents/assistant.rein
```

Expected output:

```
✓ Valid
```

The validator checks syntax, required fields, and semantic correctness. Try introducing an error to see the error reporting:

```bash
echo 'agent broken { }' > /tmp/broken.rein
rein validate /tmp/broken.rein
```

```
✗ Invalid: error at 1:16: expected field 'model' in agent block
```

---

## 4. Format Your Policy

```bash
rein fmt agents/assistant.rein
```

If the file is already formatted:

```
All 1 files already formatted
```

Use `--check` in CI to verify formatting without modifying files:

```bash
rein fmt --check agents/assistant.rein
```

---

## 5. Explain Your Policy

Get a human-readable summary of what a policy defines:

```bash
rein explain agents/assistant.rein
```

```
📋 Policy summary: agents/assistant.rein

Agents (1)
  • assistant (model: openai)
    can: search.web, files.read
    cannot: files.delete
    budget: USD10 per request
```

---

## 6. Write a More Complex Policy

Create `agents/support.rein`:

```rein
provider openai {
    model: gpt-4
    key: env("OPENAI_API_KEY")
}

agent support_bot {
    model: gpt-4
    can: read_tickets, respond_to_customers, check_order_status
    cannot: issue_refunds, delete_accounts, access_billing
    budget: $50 per day
}

agent escalation_bot {
    model: gpt-4
    can: issue_refunds up_to $200, escalate_to_human
    cannot: delete_accounts
    budget: $100 per day
}

workflow ticket_resolution {
    trigger: new_ticket

    step classify {
        agent: support_bot
        goal: "Classify the incoming ticket by urgency and type"
    }

    step resolve {
        agent: support_bot
        goal: "Attempt to resolve the ticket"
    }

    step escalate {
        agent: escalation_bot
        goal: "Handle cases requiring refunds or human escalation"
    }
}
```

Validate it:

```bash
rein validate agents/support.rein
```

Use strict mode to see which features are enforced vs. parse-only:

```bash
rein validate --strict agents/support.rein
```

---

## 7. Use in CI

Add validation to your CI pipeline:

```yaml
# .github/workflows/rein.yml
name: Validate Rein Policies
on: [push, pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rein
        run: cargo install --git https://github.com/delamain-labs/rein.git
      - name: Validate
        run: rein validate agents/*.rein
      - name: Check formatting
        run: rein fmt --check agents/*.rein
```

---

## Common Errors

**"expected field 'model' in agent block"**
Every agent needs a `model` field. Add one:
```rein
agent my_agent {
    model: gpt-4   // ← required
    can: do_stuff
}
```

**"unexpected character"**
Check for unsupported syntax. Rein uses `//` for comments, `:` for field assignment, and `{ }` for blocks. No semicolons, no `=` for assignment.

**"file not found"**
Verify the path to your `.rein` file. Rein does not recursively search directories.

---

## CLI Quick Reference

| Command | Description |
|---------|-------------|
| `rein init [name]` | Scaffold a new project |
| `rein validate <file>` | Parse and validate a `.rein` file |
| `rein validate --strict <file>` | Validate with unenforced feature warnings |
| `rein validate --ast <file>` | Output the AST as JSON |
| `rein validate --format json <file>` | JSON validation output |
| `rein fmt <files...>` | Auto-format `.rein` files |
| `rein fmt --check <files...>` | Check formatting (CI mode) |
| `rein explain <file>` | Human-readable policy summary |
| `rein cost <paths...>` | Aggregate cost stats from traces |
| `rein run <file> [-m "msg"]` | Run an agent (requires API key) |
| `rein serve <file> [--port N]` | Start the REST API server |

---

## Next Steps

- [Language Reference](./language-reference.md) — every block type and feature
- [Examples](../examples/) — real-world `.rein` files
- [README](../README.md) — project overview and architecture
