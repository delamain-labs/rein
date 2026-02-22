# Rein Roadmap

From proof-of-life to the vendor-neutral control plane for production AI agents.

**Strategy:** Ship the parser+validator as a CI tool first. Runtime catches up later. Phase 6 (Cloud) is on hold until open source adoption happens.

---

## Completed

### Phase 1: Parser & Validator ✅
14 issues, 14 PRs. Lexer, parser, validator, CLI `rein validate`, error reporting.

### Phase 2: `rein run` — MVP Runtime ✅
20 issues, 15 PRs. Agent execution, OpenAI + Anthropic providers, tool permission enforcement, budget tracking, cost calculation, run traces.

### Phase 3: Workflows ✅
8 issues, 7 PRs. Sequential + parallel execution, conditional routing, workflow state persistence with crash recovery, circular route detection.

### Phase S.0: Core DSL ✅
6 issues (#112-#117). `#` comments, `env()` references, `provider` blocks, `step` syntax, `tool` blocks, `guardrails` blocks.

### Phase S.1: Language Features ✅
27 issues (#118-#144). Type system, imports, defaults, archetypes, trust policies, route_on, parallel blocks, retry policies, when expressions, auto_resolve, for_each, typed I/O, multi-currency, escalate, rein.toml, secrets, audit trail, human approval, durable execution, sandbox, prompt injection defense.

### Phase S.2: Extended Features ✅
27 issues (#145-#171). Consensus, scenarios, observe, fleet, channel, pipe expressions, circuit breaker, streaming, send_to, inline steps, fallback, rein.lock, REST API, webhooks, event streaming, observability exports, alerting, tool registry.

### Phase 4: Observability ✅
5 issues (#92-#96). Structured traces, `rein cost` CLI, alerting on budgets, OpenTelemetry export.

### Phase 5: Developer Experience ✅
7 issues (#97-#99, #135-#136, #152). `rein init`, `rein fmt`, tree-sitter grammar, `rein explain`, `rein validate --strict`, `rein validate --format json`, `rein run --dry-run`, `rein serve`.

### v0.1 Ship ✅ (mostly)
12 issues (#251-#262). README overhaul, language reference, getting started guide, example files, CI/release workflows, reusable validation action, --strict mode, --format json, fmt feedback, rein explain.

**Current state:** 517 tests, zero clippy warnings, ~19K lines of Rust.

---

## In Progress

### Distribution
| Issue | Title | Status |
|-------|-------|--------|
| #251 | Publish to crates.io | Blocked (name "rein" taken, need alt) |
| #252 | Homebrew formula | Blocked (needs first release tag) |

---

## Open — Post-v0.1

### Runtime Gaps (Council V3 findings)
The parser supports 25+ block types, but the runtime only executes basic agent runs and simple workflows. Safety features (guardrails, circuit breakers, policies, etc.) parse but do not enforce. `rein validate --strict` warns users about this gap.

| Issue | Title | Priority |
|-------|-------|----------|
| #244 | Runtime execution gap audit | P0 |
| #246 | Language specification document | P1 |
| #249 | Production readiness checklist | P1 |

### Feature Implementations
These features have parser support but need runtime implementation.

| Issue | Title | Priority |
|-------|-------|----------|
| #123 | Eval blocks and quality gates | P1 |
| #124 | Memory system (working/session/knowledge) | P1 |
| #130 | Schedule-based workflow triggers | P1 |
| #158 | MCP server | P2 |
| #93 | OpenTelemetry integration | P2 |

### Developer Tooling
| Issue | Title | Priority |
|-------|-------|----------|
| #100 | LSP server | P1 |
| #101 | VSCode extension | P1 |
| #102 | AI assistant skill files | P2 |
| #103 | `rein deploy` command | P2 |
| #95 | Local web dashboard | P2 |

### Enterprise Features
| Issue | Title | Priority |
|-------|-------|----------|
| #159 | Namespace-based multi-tenancy | P2 |
| #160 | RBAC with roles | P2 |
| #161 | SSO integration (OIDC/SAML) | P2 |
| #162 | API key management | P2 |
| #165 | Data residency enforcement | P3 |
| #166 | Blue-green and canary deployment | P3 |
| #168 | Python/Node SDK | P2 |

---

## Phase 6: Rein Cloud — ON HOLD

**Not starting until open source adoption happens.** The cloud is the monetization layer.

| Issue | Title |
|-------|-------|
| #104 | Cloud API design |
| #105 | Cloud backend service |
| #106 | Usage-based billing |
| #107 | Multi-tenant isolation |
| #108 | Team management & SSO |
| #109 | Compliance & audit reports |
| #110 | Cloud dashboard |
| #111 | Rein Studio visual editor |

---

## Summary

| Phase | Status |
|-------|--------|
| Phase 1-3: Foundation | ✅ Complete |
| Phase S: Spec Alignment | ✅ Complete (parser) |
| Phase 4: Observability | ✅ Complete |
| Phase 5: DX | ✅ Complete |
| v0.1 Ship | 🟡 2 distribution issues remaining |
| Post-v0.1 Features | 🔴 17 issues |
| Phase 6: Cloud | ⏸️ On hold (8 issues) |

517 tests. Zero clippy warnings. 19K lines of Rust. The parser is the product. Ship it.
