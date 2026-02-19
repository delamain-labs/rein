# Rein Roadmap

From proof-of-life to the vendor-neutral control plane for production AI agents.

**Strategy:** Spec gaps first (make the runtime match what the docs promise), then observability, then developer experience. Phase 6 (Cloud) is on hold until open source adoption happens. DX is weighted heavily throughout — it's how open source wins.

---

## Completed

### Phase 1: Parser & Validator ✅
14 issues, 14 PRs merged. Lexer, parser, validator, CLI `rein validate`, error reporting.

### Phase 2: `rein run` — MVP Runtime ✅
20 issues, 15 PRs merged. Agent execution engine, OpenAI + Anthropic providers, tool permission enforcement (default-deny), budget tracking, cost calculation, run traces.

### Phase 3: Workflows ✅
8 issues, 7 PRs merged. Sequential + parallel execution, conditional routing with `RouteRule`, workflow state persistence with crash recovery, circular route detection.

**Current state:** 256 tests, zero clippy warnings, master clean.

---

## Phase S: Spec Alignment — Make the Runtime Match the Docs

The docs site defines a rich DSL. The runtime supports ~10% of it. This phase closes every gap between what the docs promise and what `rein` actually does. **This is the highest priority work.**

### S.0: Core DSL (P0 — Must have for any public release)

| Issue | Title | Description | Size |
|-------|-------|-------------|------|
| #112 | `#` line comments | Docs use `#`, lexer only supports `//` and `/* */` | S |
| #113 | `env()` function | `key: env("ANTHROPIC_KEY")` — function call syntax + runtime resolution | M |
| #114 | `provider` block | `provider anthropic { model: claude-sonnet, key: env(...) }` | M |
| #115 | `step` block syntax | Named steps with `agent`, `goal`, `output`, `input`, `when`, `approve` | L |
| #116 | `tool` block definitions | `tool zendesk { provider: rest_api, auth: oauth2(...), capabilities {...} }` | L |
| #117 | `guardrails` block | Spending tiers, output filters (PII, toxicity), rate limits, escalation rules | XL |

### S.1: Language Features (P1 — Important for usable product)

| Issue | Title | Description | Size |
|-------|-------|-------------|------|
| #118 | Type system | `type Ticket { category: one of [...], confidence: percent }`, built-in types, arrays, ranges | L |
| #119 | Import system | `import { agent } from "./file.rein"`, glob imports, registry imports | L |
| #120 | `defaults` block | Project-level defaults inherited by all agents | M |
| #121 | `archetype`/`from` templates | `archetype base { ... }` + `agent x from base { ... }` | M |
| #122 | Progressive trust system | `policy { tier supervised { promote when accuracy > 95% } }` | XL |
| #123 | Eval blocks | `eval { dataset: ./evals/data.yaml, assert accuracy > 90%, on failure: block deploy }` | L |
| #124 | Memory system | Three-tier: working (in-memory), session (sqlite), knowledge (postgres) | XL |
| #125 | `route on` block syntax | `route on classify.category { billing → step handle_billing {...} }` | M |
| #126 | Inline `parallel` blocks | `parallel { step a {...} step b {...} }` within workflows | M |
| #127 | Retry policies | `on failure: retry 3 exponential then escalate`, `on timeout 30s: ...` | M |
| #128 | `when` expressions | `step escalate { when: confidence < 70% or refund > $50 }` | M |
| #129 | `auto resolve when` | `auto resolve when { confidence > 90%, action is one of [...] }` | M |
| #130 | Schedule triggers | `schedule: daily at 2am`, `schedule: every 6 hours` | M |
| #131 | `for each` iteration | `step rewrite { agent: copywriter, for each: underperformers }` | M |
| #132 | Typed step I/O | `step x { output: items: Product[] }`, `step y { input: items }` | L |
| #133 | Multi-currency | `€`, `£`, `¥` tokens in lexer, currency field in AST | S |
| #134 | `escalate` keyword | `escalate to human via slack(#refunds)` | M |
| #135 | `rein.toml` config | `[project]`, `[runtime]`, `[registry]`, `[deploy]`, `[observability]` sections | M |
| #136 | Environment overrides | `rein.env.dev`, `rein.env.staging`, `rein.env.production` | M |
| #137 | Secrets management | `secrets { key: vault("secret/rein/key") }` — vault, aws, gcp, keychain, env backends | L |
| #138 | Persistent audit trail | Immutable log, query interface, SOC2/CSV export | L |
| #139 | Human approval workflows | `approve: human via slack(#approvals) timeout 4h`, collaborate modes | L |
| #140 | Harden durable execution | Execution IDs, tool call deduplication, idempotent retries | L |
| #141 | Agent sandbox isolation | Process-level sandboxes per agent, no env/fs/network leakage | L |
| #142 | Prompt injection defense | Input sanitization, structural separation, output validation, dual-agent verify | L |
| #143 | Rich trigger expressions | `trigger: new ticket in zendesk` as string/expression | S |
| #144 | `one of` union type | `category: one of [billing, technical, general]` in type defs | S |

### S.2: Extended Features (P2 — Production scale, later)

| Issue | Title | Size |
|-------|-------|------|
| #145 | Consensus blocks (multi-agent verification) | L |
| #146 | Scenario blocks (declarative testing) | M |
| #147 | Observe blocks (declarative observability) | L |
| #148 | Fleet blocks (agent group management) | L |
| #149 | Channel blocks (async agent messaging) | L |
| #150 | Pipe expressions (`\|` transforms) | L |
| #151 | Circuit breaker | M |
| #152 | Streaming output with per-chunk guardrails | L |
| #153 | `send to` notification steps | M |
| #154 | Inline step shorthand syntax | S |
| #155 | Fallback step for retry exhaustion | S |
| #156 | `rein.lock` for tool version pinning | M |
| #157 | REST API server | XL |
| #158 | MCP server | XL |
| #159 | Namespace-based multi-tenancy | XL |
| #160 | RBAC with roles | L |
| #161 | SSO integration (OIDC/SAML) | L |
| #162 | API key management | M |
| #163 | Webhook configuration | M |
| #164 | Event streaming (Kafka, Pub/Sub, EventBridge) | L |
| #165 | Data residency enforcement | L |
| #166 | Blue-green and canary deployment | L |
| #167 | `within()` cost/latency constraints | M |
| #168 | Python/Node SDK with `@govern` decorator | L |
| #169 | Observability exports (OTLP, Datadog, Prometheus) | L |
| #170 | Alerting system | L |
| #171 | Tool registry client | L |

---

## Phase 4: Observability

Tracing, cost analysis, and monitoring. Builds on the runtime's existing `RunTrace`.

| Issue | Title | Priority | Size |
|-------|-------|----------|------|
| #92 | Structured trace format (JSON, timestamps, decisions) | P0 | M |
| #93 | OpenTelemetry integration | P1 | L |
| #94 | `rein cost` CLI command | P1 | M |
| #95 | Local web dashboard (`rein dashboard`) | P2 | XL |
| #96 | Alerting on budget thresholds | P2 | M |

---

## Phase 5: Developer Experience ⭐

**This is how open source wins.** The first five minutes with Rein must be flawless.

| Issue | Title | Priority | Size |
|-------|-------|----------|------|
| #97 | `rein init` — scaffold a project | P1 | S |
| #98 | `rein fmt` — auto-format .rein files | P1 | M |
| #99 | Tree-sitter grammar for .rein | P2 | L |
| #100 | LSP server (autocomplete, diagnostics, hover) | P2 | XL |
| #101 | VSCode extension | P2 | M |
| #102 | AI assistant skill files (Claude, Cursor, Copilot) | P1 | S |
| #103 | `rein deploy` command | P2 | M |

**Additional DX gaps identified:**
- Error messages that teach (the docs promise this — verify every error path)
- `rein validate` with `--fix` suggestions
- `rein explain <file>` — human-readable summary of what a .rein file does
- Getting started tutorial that works end-to-end

---

## Phase 6: Rein Cloud — ON HOLD

**Not starting until open source adoption happens.** The cloud is the monetization layer: governance, compliance, team management, and persistent execution that you can't get from a CLI. But it only matters when people are using the CLI.

| Issue | Title | Priority | Size |
|-------|-------|----------|------|
| #104 | Cloud API design (OpenAPI spec) | P0 | M |
| #105 | Cloud backend service | P0 | XL |
| #106 | Usage-based billing (Stripe) | P0 | XL |
| #107 | Multi-tenant isolation | P0 | XL |
| #108 | Team management & SSO | P1 | L |
| #109 | Compliance & audit reports | P1 | L |
| #110 | Cloud dashboard | P1 | XL |
| #111 | Rein Studio — Visual Editor | P2 | XL |

---

## Tech Debt (Phase 3 cleanup)

| Issue | Title | Size |
|-------|-------|------|
| #88 | Extensible condition matching (regex, JSON path) | M |
| #89 | Version field in WorkflowState | S |
| #90 | Extract WorkflowContext struct | S |
| #91 | Move find_stage to WorkflowDef method | S |

---

## Execution Order

```
1. Phase S.0 (Core DSL P0s)     — 6 issues  — runtime parses the full base language
2. Phase S.1 (Language P1s)      — 23 issues — runtime supports the important features
3. Phase 4 (Observability)       — 5 issues  — traces, costs, monitoring
4. Phase 5 (DX) ⭐               — 7+ issues — first-five-minutes experience
5. Phase S.2 (Extended P2s)      — 27 issues — production scale features
6. Phase 6 (Cloud)               — ON HOLD   — after open source traction
```

DX work (Phase 5) interleaves with everything. `rein init` and skill files can ship as soon as the core DSL stabilizes. Error messages improve continuously.

---

## Summary

| Phase | Issues | Status |
|-------|--------|--------|
| Phase 1: Parser/Validator | 14 | ✅ Complete |
| Phase 2: Runtime MVP | 20 | ✅ Complete |
| Phase 3: Workflows | 8 | ✅ Complete |
| Phase S: Spec Alignment | 60 | 🔴 Not started |
| Phase 4: Observability | 5 | 🔴 Not started |
| Phase 5: Dev Experience | 7+ | 🔴 Not started |
| Phase 6: Cloud | 8 | ⏸️ On hold |
| Tech Debt | 4 | 🟡 Backlog |
| **Total open** | **84** | — |

256 tests. Zero clippy warnings. The foundation is solid. Now we build the full language.
