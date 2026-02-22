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
    model: gpt-4
    budget: $100 per day
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | identifier | No | Default model for all agents |
| `budget` | budget expr | No | Default budget constraint |

✅ Applied to agents at runtime.

---

## Provider

Configure an AI model provider.

```rein
provider openai {
    model: gpt-4
    key: env("OPENAI_API_KEY")
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | identifier | No | Model to use |
| `key` | value expr | No | API key (typically via `env()`) |

✅ Used for API key resolution at runtime.

---

## Tool

Define an external tool integration.

```rein
tool zendesk {
    endpoint: "https://api.zendesk.com"
    provider: zendesk_provider
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `endpoint` | string | No | Tool API endpoint |
| `provider` | identifier | No | Associated provider |

⚠️ Parse-only. Tool definitions are validated but not used by the runtime.

---

## Agent

The core building block. Defines an AI agent with capabilities, constraints, and budget.

```rein
agent support_bot {
    model: gpt-4
    can: read_tickets, respond_to_customers, check_order_status
    cannot: issue_refunds, delete_accounts
    budget: $50 per day
    guardrails {
        pii_redaction: on
    }
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | identifier | Yes | AI model to use |
| `can` | capability list | No | Allowed actions |
| `cannot` | capability list | No | Denied actions |
| `budget` | budget expr | No | Spending limit |
| `from` | identifier | No | Inherit from an archetype |
| `guardrails` | block | No | Safety guardrails (⚠️ parse-only) |

**Capabilities** can include `up_to` constraints:

```rein
agent finance_bot {
    model: gpt-4
    can: issue_refunds up_to $500, view_transactions
    cannot: wire_transfers, modify_accounts
}
```

✅ `model`, `can`, `cannot`, `budget` enforced at runtime.
⚠️ `guardrails`, `from` (archetype inheritance) are parse-only.

---

## Archetype

Reusable agent templates. Agents inherit via `from`.

```rein
archetype cautious_agent {
    model: gpt-4
    budget: $25 per day
    cannot: delete_data, send_emails
}

agent my_agent {
    from: cautious_agent
    can: read_files
}
```

**Fields:** Same as `agent`.

⚠️ Parse-only. Inheritance is not resolved at runtime.

---

## Workflow

Define multi-step workflows with triggers and stages.

```rein
workflow ticket_resolution {
    trigger: new_ticket
    stages: triage, investigation, resolution

    step classify {
        agent: support_bot
        goal: "Classify the incoming ticket by urgency and category"
    }

    step resolve {
        agent: support_bot
        goal: "Resolve the ticket based on classification"
        when: urgency > 3
    }
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `trigger` | identifier | No | Event that starts the workflow |
| `stages` | comma-separated identifiers | No | Named stages |
| `step` | block (multiple) | No | Workflow steps |
| `auto_resolve` | block | No | Auto-resolution conditions |

✅ Basic sequential execution works.
⚠️ Stages, auto_resolve, and advanced features are parse-only.

---

## Step

A unit of work within a workflow.

```rein
step review_code {
    agent: code_reviewer
    goal: "Review the pull request for security issues"
    when: file_count > 10
    send_to: slack_channel
    on failure: retry 3 exponential then escalate
    fallback: manual_review
    one_of: "security_review", "code_review"
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agent` | identifier | Yes | Agent to execute this step |
| `goal` | string | No | Task description |
| `when` | condition | No | Guard condition (⚠️ parse-only) |
| `send_to` | identifier | No | Output destination |
| `on failure` | retry policy | No | Failure handling (⚠️ parse-only) |
| `fallback` | identifier | No | Fallback step (⚠️ parse-only) |
| `one_of` | string list | No | Constraint group (⚠️ parse-only) |
| `escalate` | block | No | Escalation config (⚠️ parse-only) |
| `within` | block | No | Scoping constraints (⚠️ parse-only) |
| `input` / `output` | type ref | No | Typed I/O (⚠️ parse-only) |
| `for_each` | identifier | No | Iteration (⚠️ parse-only) |

✅ `agent` and `goal` used at runtime for basic execution.

---

## Parallel

Execute steps concurrently within a workflow.

```rein
workflow analysis {
    trigger: data_ready

    parallel {
        step sentiment { agent: nlp_bot, goal: "Analyze sentiment" }
        step entities  { agent: nlp_bot, goal: "Extract entities" }
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

    route_on category {
        "billing" -> billing_agent
        "technical" -> tech_agent
        _ -> general_agent
    }
}
```

**Patterns:** String literals, identifiers, or `_` (wildcard).

⚠️ Parse-only. Routing logic is not executed.

---

## Policy

Conditional governance rules with trust tiers.

```rein
policy data_access {
    tier low {
        can: read_public_data
        budget: $10 per day
        when: trust_score < 50%
    }

    tier high {
        can: read_public_data, read_private_data, export_data
        budget: $100 per day
        when: trust_score >= 80%
    }
}
```

**Tier fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `can` | capability list | No | Allowed actions at this tier |
| `budget` | budget expr | No | Budget at this tier |
| `when` | condition | No | Condition for this tier |

⚠️ Parse-only. Trust tiers are not enforced.

---

## Guardrails

Safety constraints applied to agents.

```rein
agent safe_bot {
    model: gpt-4
    guardrails {
        pii_redaction: on
        toxicity_filter: strict
        max_response_length: 500
    }
}
```

Guardrails are defined inline within an agent block.

⚠️ Parse-only. No runtime filtering or redaction occurs.

---

## Circuit Breaker

Failure detection and automatic recovery.

```rein
circuit_breaker api_safety {
    threshold: 5
    window: 60
    cooldown: 300
    action: halt
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `threshold` | integer | No | Failure count to trip |
| `window` | integer | No | Time window in seconds |
| `cooldown` | integer | No | Recovery period in seconds |
| `action` | identifier | No | Action when tripped |

⚠️ Parse-only.

---

## Observe

Observability and monitoring configuration.

```rein
observe metrics {
    export: prometheus
    endpoint: "http://localhost:9090"
    interval: 30
}
```

⚠️ Parse-only.

---

## Fleet

Multi-agent group definitions with scaling.

```rein
fleet support_team {
    agents: support_bot, escalation_bot
    min: 2
    max: 10
    scale_on: queue_depth
}
```

⚠️ Parse-only.

---

## Channel

Async communication channel definitions.

```rein
channel alerts {
    provider: slack
    target: "#incidents"
}
```

⚠️ Parse-only.

---

## Eval

Quality gates with dataset assertions.

```rein
eval accuracy_check {
    dataset: "test_cases.jsonl"
    assert accuracy >= 90%
    assert latency < 2000
    on_fail: block_deploy
}
```

**Assertion operators:** `<`, `>`, `<=`, `>=`, `==`, `!=`

⚠️ Parse-only.

---

## Consensus

Multi-agent verification strategies.

```rein
consensus review_panel {
    strategy: majority
    require: 3 agree
    agents: reviewer_1, reviewer_2, reviewer_3
    timeout: 300
}
```

**Strategies:** `majority`, `unanimous`

⚠️ Parse-only.

---

## Approval

Human-in-the-loop approval workflows.

```rein
approval expense_approval {
    kind: human
    via: slack
    timeout: 3600
    approve: manager
    collaborate {
        mode: suggest
    }
}
```

**Kinds:** `human`, `auto`
**Collaboration modes:** `edit`, `suggest`, `review`

⚠️ Parse-only.

---

## Escalate

Define escalation paths for agent handoff.

```rein
escalate to_human {
    target: support_team
    priority: high
    via: pagerduty
}
```

⚠️ Parse-only.

---

## Secrets

Vault-based secret management.

```rein
secrets api_keys {
    vault: hashicorp
    bind {
        openai_key: "secrets/ai/openai"
        stripe_key: "secrets/payments/stripe"
    }
}
```

**Vault sources:** `hashicorp`, `aws`, `gcp`, `azure`, `env`

⚠️ Parse-only.

---

## Memory

Agent memory system with tiered storage.

```rein
memory agent_memory {
    working {
        ttl: 3600
        max_tokens: 4000
    }
    session {
        ttl: 86400
    }
    knowledge {
        source: "knowledge_base/"
        embedding_model: text-embedding-ada-002
    }
}
```

**Tiers:** `working`, `session`, `knowledge`

⚠️ Parse-only.

---

## Schedule

Time-based workflow triggers.

```rein
schedule daily_report {
    cron: "0 9 * * *"
    workflow: generate_report
}

schedule every_hour {
    every: 3600
    workflow: health_check
}
```

**Formats:** `cron` (cron expression) or `every` (interval in seconds).

⚠️ Parse-only.

---

## Scenario

Declarative test definitions.

```rein
scenario happy_path {
    given: "Customer asks about order status"
    expect: "Agent retrieves order and responds with tracking info"
}
```

⚠️ Parse-only.

---

## Type Definitions

Custom types with field definitions and range constraints.

```rein
type Priority {
    level: int 1..5
    label: string
    urgent: bool
}

type Temperature {
    value: float -40.0..60.0
    unit: string
}
```

**Built-in types:** `int`, `float`, `string`, `bool`

Range syntax: `int 1..10`, `float 0.0..1.0`

⚠️ Parse-only. Types are validated syntactically but not enforced at runtime.

---

## Pipe Expressions

Data transformation pipelines using `|>`.

```rein
results |> select name, score |> where score > 80 |> sort score desc |> unique
```

**Transforms:**
- `select field1, field2` — project specific fields
- `where field op value` — filter rows
- `sort field [asc|desc]` — order results
- `unique` — deduplicate

**Operators in `where`:** `<`, `>`, `<=`, `>=`, `==`, `!=`

⚠️ Parse-only.

---

## Budget

Multi-currency budget constraints with time periods.

```rein
budget: $100 per day
budget: €500 per month
budget: £50 per hour
budget: ¥10000 per month
```

**Supported currencies:** `$`, `€`, `£`, `¥`
**Time periods:** `day`, `hour`, `month`

✅ Budget limits are enforced at runtime.

---

## When Conditions

Guard conditions on steps, policies, and auto-resolve blocks.

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

Reference environment variables with optional defaults.

```rein
key: env("OPENAI_API_KEY")
key: env("API_KEY", "default_value")
```

✅ Resolved at runtime for provider configuration.

---

## Validation

Run the validator to check `.rein` files:

```bash
# Basic validation
rein validate policy.rein

# Output AST as JSON
rein validate --ast policy.rein

# JSON output format
rein validate --format json policy.rein

# Strict mode: warn on parse-only features
rein validate --strict policy.rein
```

### Strict Mode

With `--strict`, the validator warns about features that parse correctly but are not enforced at runtime. This prevents a false sense of security.

```
⚠ warning[W_UNENFORCED]: Guardrails blocks are parsed but not enforced at runtime.
  Output filtering, PII redaction, and toxicity blocking will not be applied.
```

Exit codes:
- `0` — valid, no issues
- `1` — parse or validation errors
- `2` — no errors, but strict warnings present
