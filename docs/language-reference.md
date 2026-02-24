# Rein Language Reference

Rein is a declarative language for defining AI agent policies, workflows, and governance. This reference covers every block type and language feature supported by the parser.

> **Runtime status key:**
> ✅ Enforced at runtime | ⚠️ Parse-only (not yet enforced) | 🔧 Partial support

---

## Table of Contents

- [Comments](#comments)
- [Imports](#imports)
- [Defaults](#defaults)
- [Provider](#provider)
- [Tool](#tool)
- [Agent](#agent)
- [Archetype](#archetype)
- [Workflow](#workflow)
- [Step](#step)
- [Parallel](#parallel)
- [Route](#route)
- [Policy](#policy)
- [Guardrails](#guardrails)
- [Circuit Breaker](#circuit-breaker)
- [Observe](#observe)
- [Fleet](#fleet)
- [Channel](#channel)
- [Eval](#eval)
- [Consensus](#consensus)
- [Approval](#approval)
- [Escalate](#escalate)
- [Secrets](#secrets)
- [Memory](#memory)
- [Schedule](#schedule)
- [Scenario](#scenario)
- [Type Definitions](#type-definitions)
- [Pipe Expressions](#pipe-expressions)
- [Budget](#budget)
- [When Conditions](#when-conditions)
- [Env References](#env-references)

---

## Comments

```rein
// Line comment
# Also a line comment

/* Block comment
   spanning multiple lines */
```

✅ Fully supported.

---

## Imports

Import definitions from other `.rein` files or registries.

```rein
import "path/to/file.rein"              // file import
import { agent_name } from "file.rein"  // named import
import * from "file.rein"               // glob import
import "registry://package"             // registry import
```

⚠️ Parse-only. Imports are parsed and validated syntactically but not resolved.

---

## Defaults

Global defaults applied to all agents.

```rein
defaults {
    model: "gpt-4o"
    budget: $100 per day
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | value expr | No | Default model for all agents |
| `budget` | budget expr | No | Default budget constraint |

✅ Applied to agents at runtime.

---

## Provider

Configure an AI model provider.

```rein
provider openai {
    model: "gpt-4o"
    key: env("OPENAI_API_KEY")
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | value expr | No | Model to use |
| `key` | value expr | No | API key (typically via `env()`) |

> **Note:** The field is `key`, not `api_key`.

✅ Used for API key resolution and provider selection at runtime.

---

## Tool

Define an external tool integration.

```rein
tool web_search {
    endpoint: "https://api.search.example.com/v1"
    key: env("SEARCH_API_KEY")
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `endpoint` | value expr | No | Tool API endpoint |
| `provider` | value expr | No | Associated provider |
| `key` | value expr | No | Tool API key |

⚠️ Parse-only. Tool definitions are validated but external tool calls are not yet wired.

---

## Agent

The core building block. Defines an AI agent with capabilities, constraints, and budget.

```rein
agent support_bot {
    model: openai

    can [
        zendesk.read_ticket
        zendesk.reply_ticket
        zendesk.refund up to $50
    ]

    cannot [
        zendesk.delete_ticket
        zendesk.admin
    ]

    budget: $5 per request

    guardrails {
        output_filter {
            pii_detection: redact
            toxicity: block
            prompt_injection: block
        }
    }
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | value expr | Yes | AI model or provider name |
| `can [...]` | capability list | No | Allowed actions (bracket list, comma or newline-separated) |
| `cannot [...]` | capability list | No | Denied actions (bracket list, comma or newline-separated) |
| `budget` | budget expr | No | Spending limit |
| `guardrails` | block | No | Safety guardrails |

**Capability syntax:** Capabilities use `namespace.action` format. Both comma-separated and newline-separated styles are accepted:

```rein
# comma-separated (concise, consistent with metrics: [...])
can [search.web, files.read, stripe.refund up to $50]

# newline-separated (multi-line, existing files continue to work)
can [
    search.web
    files.read
    stripe.refund up to $50
]
```

The `up to $N` constraint is optional and sets a monetary cap on that capability.

✅ `model`, `can`, `cannot`, `budget` enforced at runtime.
✅ `guardrails` enforced at runtime (output filtering, redaction, blocking).

---

## Archetype

Reusable agent templates. Agents inherit via `from`.

```rein
archetype cautious_agent {
    can [
        files.read
    ]

    cannot [
        files.delete
        system.execute
    ]

    budget: $25 per day
}

agent my_agent from cautious_agent {
    model: openai

    can [
        search.web
    ]
}
```

> **Note:** Inheritance uses `agent name from archetype`, not `from:` as a field.

⚠️ Parse-only. Archetype inheritance is not resolved at runtime.

---

## Workflow

Define multi-step workflows with triggers.

```rein
workflow support_pipeline {
    trigger: ticket_created

    step classify {
        agent: triage
        goal: "Classify the incoming support ticket"
    }

    step resolve {
        agent: resolver
        goal: "Resolve the ticket automatically"
        when: priority > 3
    }
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `trigger` | identifier | No | Event that starts the workflow (no dots allowed) |
| `stages` | comma-separated identifiers | No | Named stage references |
| `step` | block (multiple) | No | Workflow steps |

> **Note:** Triggers must be plain identifiers (e.g., `ticket_created`). Function-style triggers like `schedule("0 9 * * *")` are not valid here; use the `schedule` block instead.

✅ Basic sequential execution works via `rein run`.

---

## Step

A unit of work within a workflow.

```rein
step review_code {
    agent: code_reviewer
    goal: "Review the pull request for security issues"
    when: file_count > 10
    approve: human via slack("#reviews") timeout "2h"
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agent` | identifier | **Yes** | Agent to execute this step |
| `goal` | string literal | No | Task description (must be a string, not `env()`) |
| `when` | condition | No | Guard condition |
| `send_to` | pipe expr | No | Output destination |
| `input` | pipe expr | No | Input data pipeline |
| `on failure` | retry policy | No | Failure handling (e.g., `retry 3 exponential then escalate`) |
| `fallback` | identifier | No | Fallback step on failure |
| `for_each` | identifier | No | Iterate over a collection |
| `escalate` | block | No | Escalation config |
| `approve` | approval expr | No | Human approval gate |

**Inline step shorthand:**

```rein
step classify: triage goal "Classify the ticket"
```

> **Note:** There is no `depends_on` field. Use `when:` conditions to express step ordering, or rely on sequential declaration order.

✅ `agent` and `goal` used at runtime.
✅ `approve` wired to approval handler.
⚠️ `when`, `on failure`, `fallback`, `for_each`, `escalate`, `send_to` are parse-only.

---

## Parallel

Execute steps concurrently within a workflow.

```rein
workflow analysis {
    trigger: data_ready

    parallel {
        step sentiment {
            agent: nlp_bot
            goal: "Analyze sentiment"
        }
        step entities {
            agent: nlp_bot
            goal: "Extract entities"
        }
    }
}
```

⚠️ Parse-only. Steps in `parallel` blocks currently execute sequentially.

---

## Route

Conditional routing based on pattern matching.

```rein
workflow router {
    trigger: incoming_request

    route on category {
        "billing" -> step billing_step {
            agent: billing_agent
            goal: "Handle billing request"
        }
        "technical" -> step tech_step {
            agent: tech_agent
            goal: "Handle technical request"
        }
        _ -> step general_step {
            agent: general_agent
            goal: "Handle general request"
        }
    }
}
```

**Patterns:** String literals, identifiers, or `_` (wildcard).

⚠️ Parse-only. Routing logic is not executed.

---

## Policy

Progressive trust tiers with promotion/demotion rules.

```rein
policy {
    tier supervised {
        promote when accuracy > 95%
    }
    tier autonomous {
        promote when accuracy > 99%
    }
    tier fully_autonomous {}
}
```

**Tier fields:**

| Field | Type | Description |
|-------|------|-------------|
| `promote when` | condition | Condition to promote to the next tier |
| `can [...]` | capability list | Capabilities at this tier |
| `cannot [...]` | capability list | Denied at this tier |
| `budget` | budget expr | Budget limit at this tier |

> **Note:** Policy blocks are top-level, not nested inside agents. They can optionally have a name: `policy data_access { ... }`.

🔧 Partial. Policy engine logs current tier at runtime; full tier promotion/demotion enforcement is in progress.

---

## Guardrails

Safety constraints applied to agent output. Nested inside agent blocks.

```rein
agent safe_bot {
    model: openai

    guardrails {
        output_filter {
            pii_detection: redact
            toxicity: block
            prompt_injection: block
        }
    }
}
```

Guardrails use named sections (e.g., `output_filter`, `safety`) containing key-value rules.

> **Note:** Guardrails are nested blocks inside agents, not flat key-value pairs. The section name (like `output_filter`) is required.

✅ Enforced at runtime. Output is checked after each LLM response; matching content is blocked or redacted.

> **Security notice — guardrails are heuristics, not classifiers.**
>
> `pii_detection`, `toxicity`, and `prompt_injection` are implemented using keyword and pattern matching. They are **not** ML classifiers and can be evaded by:
> - Rephrasing or misspelling trigger terms
> - Using synonyms, leetspeak, or Unicode homoglyphs
> - Splitting sensitive content across multiple responses
> - Encoding content in base64 or other encodings
>
> Do not rely on Rein guardrails as your sole security control in production. For adversarial or high-stakes environments, supplement with an LLM-based content classifier (e.g., a dedicated safety model or moderation API call) that runs on the raw output before it reaches downstream systems.

---

## Circuit Breaker

Failure detection and automatic recovery. Top-level block (not nested in agents).

```rein
circuit_breaker api_protection {
    open after: 3 failures in 5 min
    half_open after: 2 min
}
```

**Fields:**

| Field | Syntax | Description |
|-------|--------|-------------|
| `open after` | `N failures in M min` | Trip threshold |
| `half_open after` | `N min` | Recovery probe interval |

> **Note:** Circuit breakers are top-level blocks, not nested inside agent definitions.

✅ Enforced at runtime. Tracks provider errors, opens circuit on threshold, transitions through half-open for recovery.

---

## Observe

Observability and monitoring configuration.

```rein
observe system_health {
    trace: "structured"
    metrics: [cost, latency, errors]
    alert when {
        cost > $10.00
    }
    export: otlp
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `trace` | string/ident | Trace format |
| `metrics` | `[ident, ...]` | Metrics to collect (bracket list, comma-separated) |
| `alert when { ... }` | condition block | Alert condition (uses `when` expression syntax) |
| `export` | identifier | Export format. Supported at runtime: `otlp`, `stdout`. Other values parse but are not yet implemented. |

> **Note:** The fields are `trace`, `metrics`, `alert when { }`, and `export`. Not `watch`, `alert_when`, or `notify`.

⚠️ Parse-only. OTLP trace export is available via `rein run --otel` but observe blocks are not wired to it.
Using `export: prometheus` or `export: datadog` will parse successfully but produce a `W_EXPORT_UNSUPPORTED` warning in strict mode (`rein validate --strict`).

---

## Fleet

Multi-agent group definitions with optional scaling.

```rein
fleet support_team {
    agents: [support_bot, escalation_bot]
    policy: round_robin
    budget: $500 per day
    scaling {
        min: 2
        max: 10
    }
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `agents` | `[ident, ...]` | List of agent names |
| `policy` | identifier | Routing policy |
| `budget` | budget expr | Fleet-wide budget |
| `scaling` | block | Scaling configuration (`min`, `max`) |

⚠️ Parse-only.

---

## Channel

Communication channel definitions.

```rein
channel notifications {
    type: slack
    retention: 30d
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | identifier | Channel type (e.g., `slack`, `webhook`). Supports array syntax: `type[]` |
| `retention` | duration | Message retention period |

> **Note:** The block keyword is `channel`, and fields are `type` and `retention`. Not `provider`, `target`, or `agents`.

⚠️ Parse-only.

---

## Eval

Quality gates with dataset-based assertions.

```rein
eval quality_check {
    dataset: "./evals/quality.yaml"
    assert accuracy > 90%
    assert latency < 5000
    on failure: block deploy
}
```

**Fields:**

| Field | Syntax | Description |
|-------|--------|-------------|
| `dataset` | string path | Path to evaluation dataset (required) |
| `assert` | `metric op value` | Assertion (repeatable) |
| `on failure` | `block deploy` / `alert` / `log` | Failure action |

**Assertion operators:** `<`, `>`, `<=`, `>=`, `==`, `!=`

> **Note:** Eval blocks use `dataset`, `assert`, and `on failure`. Not `agent`, `given`, or `expect` (those belong in `scenario` blocks).

The eval name is optional: `eval { ... }` is valid.

⚠️ Parse-only. Assertion runner exists but is not wired to CLI.

---

## Consensus

Multi-agent verification strategies. Top-level block.

```rein
consensus review_panel {
    agents: [reviewer_a, reviewer_b, reviewer_c]
    strategy: majority
    quorum: 2
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `agents` | `[ident, ...]` | Participating agents |
| `strategy` | identifier | Voting strategy (`majority`, `unanimous`) |
| `quorum` | integer | Minimum votes required |

⚠️ Parse-only.

---

## Approval

Human-in-the-loop approval gates. Used as a field within workflow steps.

```rein
step deploy {
    agent: deployer
    goal: "Execute the deployment plan"
    approve: human via slack("#deployments") timeout "4h"
}
```

**Syntax:** `approve: human via channel("destination") timeout "duration"`

The approval handler supports:
- **CLI prompt** (interactive terminal)
- **Auto-approve / auto-reject** (for testing and CI)
- **Extensible** via the `ApprovalHandler` trait

✅ Approval handler runtime module is built. Wired to step execution.

---

## Escalate

Escalation within workflow steps (not a top-level block).

```rein
step handle_ticket {
    agent: support_bot
    goal: "Handle the support ticket"
    on failure: retry 3 linear then escalate
    escalate {
        to: senior_agent
        notify: slack("#escalations")
    }
}
```

The `escalate` keyword is recognized in step context. It is also used as part of `on failure` retry chains.

⚠️ Parse-only. Escalation paths are not executed.

---

## Secrets

Secure secret management with vault integration. Note: the keyword is `secrets` (plural).

```rein
secrets {
    openai_key: vault("secrets/ai/openai")
    stripe_key: env("STRIPE_API_KEY")
}
```

**Secret sources:**
- `vault("path")` — HashiCorp Vault or similar
- `env("VAR_NAME")` — Environment variable

Each binding maps a name to a source: `name: vault("path")` or `name: env("VAR")`.

> **Note:** The keyword is `secrets` (plural), not `secret`. Bindings use `vault()` or `env()` function syntax.

⚠️ Parse-only.

---

## Memory

Agent memory system with tiered storage.

```rein
memory agent_context {
    working {
        ttl: 3600
        max_tokens: 4000
    }
    session {
        ttl: 86400
    }
    knowledge {
        source: "knowledge_base/"
        embedding_model: "text-embedding-ada-002"
    }
}
```

**Tiers:** `working` (cleared per run), `session` (persists across runs), `knowledge` (long-term retrieval).

🔧 Partial. Working and session tiers are implemented in-process. Knowledge tier is deferred.

---

## Schedule

Time-based triggers.

```rein
schedule daily_report {
    cron: "0 9 * * *"
    workflow: generate_report
}

schedule health_check {
    every: 3600
    workflow: run_health_check
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `cron` | string | Cron expression |
| `every` | integer | Interval in seconds |
| `workflow` | identifier | Workflow to trigger |

⚠️ Parse-only.

---

## Scenario

Declarative test definitions with structured given/expect blocks.

```rein
scenario happy_path {
    given {
        query: "What is the capital of France?"
        context: "geography"
    }
    expect {
        answer: "Paris"
        confidence: "high"
    }
}
```

**Structure:** `given { key: "value" ... }` and `expect { key: "value" ... }` blocks with key-value pairs.

> **Note:** Scenarios use nested `given { }` and `expect { }` blocks with key-value pairs, not flat `given:` / `expect:` fields.

⚠️ Parse-only.

---

## Type Definitions

Custom types with field constraints.

```rein
type Priority {
    level: one of [low, medium, high, critical]
    confidence: percentage
}
```

**Field types:**
- `one of [value1, value2, ...]` — Enumerated values
- `percentage` — Percentage value

⚠️ Parse-only. Types are validated syntactically but not enforced at runtime.

---

## Pipe Expressions

Data transformation pipelines using `|>`. Used in step `input` and `send_to` fields.

```rein
step process {
    agent: processor
    goal: "Process the data"
    input: results |> select name, score |> where score > 80
}
```

**Transforms:**
- `select field1, field2` — Project specific fields
- `where field op value` — Filter rows
- `sort field [asc|desc]` — Order results
- `unique` — Deduplicate

**Operators in `where`:** `<`, `>`, `<=`, `>=`, `==`, `!=`

⚠️ Parse-only.

---

## Budget

Multi-currency budget constraints with time periods.

```rein
budget: $0.10 per request
budget: €500 per month
budget: £50 per hour
budget: ¥10000 per day
```

**Supported currencies:** `$` (USD), `€` (EUR), `£` (GBP), `¥` (JPY)
**Time periods:** Any identifier (commonly `request`, `day`, `hour`, `month`, `session`, `ticket`)

Amounts are stored internally as integer cents to avoid floating-point precision issues.

✅ Budget limits are enforced at runtime.

---

## When Conditions

Guard conditions on steps, policy tiers, and other blocks.

```rein
when: confidence < 70%
when: refund > $50.00
when: score >= 80 and priority < 3
when: status == "critical"
when: tier != "free"
when: risk > 50% or amount > $1000
```

**Operators:** `<`, `>`, `<=`, `>=`, `==`, `!=`
**Values:** numbers, percentages (`70%`), currency (`$50`), strings (`"critical"`), identifiers
**Logic:** `and` (binds tighter), `or`

⚠️ Parse-only. Conditions are not evaluated at runtime.

---

## Env References

Reference environment variables.

```rein
key: env("OPENAI_API_KEY")
```

✅ Resolved at runtime for provider configuration.

---

## Validation

Run the validator to check `.rein` files:

```bash
# Basic validation
rein validate policy.rein

# Output AST as JSON
rein validate policy.rein --ast

# JSON output format
rein validate policy.rein --format json

# Strict mode: warn on parse-only features
rein validate policy.rein --strict
```

### Strict Mode

With `--strict`, the validator warns about features that parse correctly but are not enforced at runtime. This prevents a false sense of security.

```
⚠ warning[W_UNENFORCED]: consensus blocks are parsed but not enforced at runtime.
```

Exit codes:
- `0` — Valid, no issues
- `1` — Parse or validation errors
- `2` — No errors, but strict warnings present

---

## Quick Syntax Reference

| Thing you want | Correct syntax | Common mistake |
|---------------|---------------|----------------|
| Provider API key | `key: env("...")` | `api_key: env("...")` |
| Capability list | `can [a.b, c.d]` or `can [\n  a.b\n  c.d\n]` | `can: a, b, c` (must use brackets) |
| Secrets block | `secrets { ... }` | `secret { ... }` (must be plural) |
| Workflow trigger | `trigger: event_name` | `trigger: schedule("...")` (use `schedule` block) |
| Step ordering | `when: condition` | `depends_on: step_name` (not supported) |
| Observe metrics | `metrics: [a, b]` | `watch: a, b` |
| Observe alerts | `alert when { cost > $10 }` | `alert_when: cost > $10` |
| Scenario test | `given { k: "v" }` | `given: "text"` (needs block syntax) |
| Eval quality | `dataset: "path"` + `assert metric > N` | `agent: x` + `given: "..."` (wrong block) |
| Circuit breaker | Top-level block | Nested in agent (not valid) |
| Escalate | Inside step block | Top-level block (not valid) |
