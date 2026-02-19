# Rein

A declarative language and runtime for AI agent orchestration. Define what your agents can do, what they can't, and how much they can spend — in plain, readable policy files.

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

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Validate a .rein file
rein validate policy.rein

# Dump the AST as JSON
rein validate --ast policy.rein
```

## Status

Early development. The parser and validator are functional. Runtime execution (`rein run`) is next.

## License

MIT
