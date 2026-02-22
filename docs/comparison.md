# Rein vs. The Alternatives

## The Problem

You're deploying AI agents in production. You need: permission controls, budget limits, guardrails, approval gates, audit trails, and circuit breakers.

Here's what that looks like today:

## Without Rein: Python + LangChain (~200 lines)

```python
import os
import re
from langchain.agents import AgentExecutor
from langchain.callbacks import CallbackManager
from langchain_openai import ChatOpenAI

# Permission system (you build this)
ALLOWED_TOOLS = ["search.web", "files.read"]
DENIED_TOOLS = ["files.delete", "admin.modify"]

class BudgetTracker:
    def __init__(self, limit_cents):
        self.limit = limit_cents
        self.spent = 0
    
    def record(self, cost):
        self.spent += cost
        if self.spent > self.limit:
            raise Exception(f"Budget exceeded: {self.spent}/{self.limit}")

class GuardrailFilter:
    PII_PATTERNS = [
        r'\b[\w.-]+@[\w.-]+\.\w+\b',  # email
        r'\b\d{3}-\d{2}-\d{4}\b',      # SSN
    ]
    TOXIC_PHRASES = ["kill yourself", "harm yourself"]
    
    def check(self, text):
        for pattern in self.PII_PATTERNS:
            if re.search(pattern, text):
                return "PII detected"
        for phrase in self.TOXIC_PHRASES:
            if phrase in text.lower():
                return "Toxic content"
        return None

class CircuitBreaker:
    def __init__(self, threshold=3, window=300):
        self.threshold = threshold
        self.window = window
        self.failures = []
        self.state = "closed"
    # ... 50 more lines for state management

# Wire it all together
budget = BudgetTracker(500)  # $5.00
guardrails = GuardrailFilter()
breaker = CircuitBreaker()

# Now build your agent with all this bolted on...
# And repeat for every agent in your system.
# And hope your junior dev doesn't skip the guardrails.
# And good luck auditing any of it.
```

## With Rein: 15 lines

```rein
agent support_bot {
    model: openai

    can [
        search.web
        files.read
    ]

    cannot [
        files.delete
        admin.modify
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

circuit_breaker api_protection {
    open after: 3 failures in 5 min
    half_open after: 2 min
}
```

Then:

```bash
rein validate my-agent.rein        # Catch policy errors in CI
rein run my-agent.rein -m "Hello"  # Execute with enforcement
rein validate --strict             # Warn about unenforced features
```

## The Difference

| | Python/LangChain | Rein |
|---|---|---|
| **Permissions** | You build it | `can` / `cannot` blocks |
| **Budget** | You build it | `budget: $5 per request` |
| **Guardrails** | You build it | `guardrails { }` block |
| **Circuit breaker** | You build it | `circuit_breaker { }` block |
| **Approval gates** | You build it | `approve: human via slack("#ch")` |
| **Audit trail** | You build it | Built-in OTLP traces |
| **Policy as code** | Scattered in Python | One `.rein` file |
| **CI validation** | Nothing to lint | `rein validate` in GitHub Actions |
| **Non-engineer readable** | No | Yes |
| **Vendor lock-in** | Per-framework | Swap `model: openai` to `model: anthropic` |

## Who Is Rein For?

- **Platform teams** deploying multiple AI agents who need consistent governance
- **Compliance-sensitive orgs** (fintech, healthcare) who need audit trails
- **Teams using multiple LLM providers** who want vendor-neutral policy
- **Anyone tired of reimplementing budget tracking and guardrails** for every new agent

## Quick Start

```bash
brew tap delamain-labs/tap && brew install rein
rein init my-project
cd my-project
rein validate agents/assistant.rein
rein run agents/assistant.rein -m "Hello"
```

[Full docs](https://rein-docs.vercel.app) | [GitHub](https://github.com/delamain-labs/rein) | [Examples](https://github.com/delamain-labs/rein/tree/master/examples)
