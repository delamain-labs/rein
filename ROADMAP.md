# Rein Roadmap

Every ticket needed to go from `rein validate` to production-ready AI agent orchestration platform.

---

## Phase 2: `rein run` — MVP Runtime

The proof of life. An agent executes within .rein constraints.

### 2.1 Foundation

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 2.1.1 | Add async runtime and HTTP client deps | Add `tokio`, `reqwest`, and `dotenv` to Cargo.toml. Set up async main. | P0 | — | S |
| 2.1.2 | Define runtime config and environment | Load `.env` for API keys. Create `src/config.rs` for runtime configuration (API key, model defaults, timeouts). | P0 | — | S |
| 2.1.3 | Create `rein run` CLI command | Add `run` subcommand to clap CLI. Takes a `.rein` file path. Parses and validates before executing. Create `src/commands/run.rs`. | P0 | 2.1.1 | S |
| 2.1.4 | Define runtime types | Create `src/runtime/mod.rs` with core types: `AgentRuntime`, `ToolCall`, `ToolResult`, `RunTrace`, `RunError`. | P0 | — | M |

### 2.2 LLM Integration

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 2.2.1 | OpenAI API client | Create `src/providers/openai.rs`. Implement chat completion with function calling. Handle streaming and non-streaming responses. | P0 | 2.1.1, 2.1.2 | M |
| 2.2.2 | Provider trait abstraction | Define `src/providers/mod.rs` with `LlmProvider` trait (send message, define tools, parse tool calls). OpenAI implements it. | P0 | 2.2.1 | M |
| 2.2.3 | Map .rein model field to provider | Resolve `model: "gpt-4o"` or `model: anthropic` to the correct provider and model string. | P1 | 2.2.2 | S |

### 2.3 Permission Enforcement

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 2.3.1 | Tool registry from .rein capabilities | Convert `can`/`cannot` lists into a runtime tool registry. `can` items become available tools. `cannot` items are explicitly blocked. | P0 | 2.1.4 | M |
| 2.3.2 | Tool call interceptor | Before executing any tool call from the LLM, check it against the permission registry. Block unauthorized calls with a clear error message back to the LLM. | P0 | 2.3.1 | M |
| 2.3.3 | Monetary constraint enforcement | For capabilities with `up to $X`, track spending per capability and block when the limit is reached. | P0 | 2.3.2 | M |

### 2.4 Budget Tracking

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 2.4.1 | Token counting and cost calculation | Track input/output tokens per LLM call. Calculate cost based on model pricing (hardcoded table initially). | P0 | 2.2.1 | M |
| 2.4.2 | Budget enforcement | Track cumulative cost per budget unit. When `budget: $0.03 per ticket` is exceeded, halt execution with a budget-exceeded error. | P0 | 2.4.1 | M |

### 2.5 Execution Loop

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 2.5.1 | Agent execution loop | Core loop: send message to LLM → receive response → if tool call, check permissions, execute, send result back → repeat until done or budget exceeded. | P0 | 2.2.2, 2.3.2, 2.4.2 | L |
| 2.5.2 | Mock tool executor | For proof-of-life, implement mock tool execution (tools return canned responses). Real tool execution comes later. | P0 | 2.5.1 | S |
| 2.5.3 | Run trace output | Print a structured trace of the run: each LLM call, tool call decisions (allowed/blocked), tokens used, cost, final result. | P0 | 2.5.1 | M |

### 2.6 Testing & Polish

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 2.6.1 | Unit tests for permission enforcement | Test that allowed tools pass, blocked tools are rejected, monetary caps are enforced. | P0 | 2.3.2 | M |
| 2.6.2 | Unit tests for budget tracking | Test token counting, cost calculation, budget exceeded halting. | P0 | 2.4.2 | M |
| 2.6.3 | Integration test: full agent run | End-to-end test with mocked LLM responses: agent runs, makes tool calls, respects permissions, stays within budget. | P0 | 2.5.1 | L |
| 2.6.4 | Example .rein files for `rein run` | Create example files demonstrating runtime features: basic agent, budget-limited agent, permission-restricted agent. | P1 | 2.5.1 | S |
| 2.6.5 | Anthropic provider | Add `src/providers/anthropic.rs` implementing the `LlmProvider` trait for Claude models. | P1 | 2.2.2 | M |

---

## Phase 3: Workflows — Multi-Agent Orchestration

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 3.1 | Workflow syntax in .rein DSL | Extend the lexer/parser to support `workflow <name> { trigger: ... stages: [...] }` blocks. | P0 | Phase 2 | L |
| 3.2 | Workflow AST types | Define `WorkflowDef`, `Stage`, `Trigger`, `Route` types in ast.rs. | P0 | 3.1 | M |
| 3.3 | Workflow validator | Validate workflow definitions: stages reference existing agents, no circular dependencies, triggers are valid. | P0 | 3.2 | M |
| 3.4 | Sequential workflow executor | Execute workflows as a chain: trigger → agent1 → agent2 → ... Pass output of each stage as input to the next. | P0 | 3.2, Phase 2 | L |
| 3.5 | Conditional routing | Support `route: { if <condition> then <stage> else <stage> }` in workflows. Route based on agent output. | P1 | 3.4 | M |
| 3.6 | Parallel execution | Support `parallel: [stage_a, stage_b]` to run multiple agents concurrently and merge results. | P1 | 3.4 | L |
| 3.7 | Workflow state persistence | Save workflow state to disk (JSON) so a crashed workflow can resume from the last completed stage. | P1 | 3.4 | L |
| 3.8 | Workflow integration tests | End-to-end tests for sequential, conditional, and parallel workflows with mocked agents. | P0 | 3.4 | M |

---

## Phase 4: Observability

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 4.1 | Structured trace format | Define a JSON trace format for runs: timestamps, decisions, tool calls, tokens, costs. Write to file or stdout. | P0 | 2.5.3 | M |
| 4.2 | OpenTelemetry integration | Export traces as OpenTelemetry spans. Add `opentelemetry` crate. Enable with `--otel` flag or env var. | P1 | 4.1 | L |
| 4.3 | Cost aggregation | CLI command `rein cost` that reads trace files and shows cost breakdown by agent, workflow, time period. | P1 | 4.1 | M |
| 4.4 | `rein dashboard` — local web UI | Serve a local web dashboard showing recent runs, costs, and traces. Use `axum` for the server, minimal HTML/JS frontend. | P2 | 4.1 | XL |
| 4.5 | Alerting on budget thresholds | Configurable alerts when agents approach budget limits (80%, 90%, 100%). Output to stderr or webhook. | P2 | 2.4.2 | M |

---

## Phase 5: Developer Experience

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 5.1 | `rein init` command | Scaffold a new Rein project: create directory structure, example .rein files, .env template, README. | P1 | — | S |
| 5.2 | `rein fmt` command | Auto-format .rein files to canonical style. Consistent indentation, spacing, ordering. | P1 | — | M |
| 5.3 | Tree-sitter grammar | Write a Tree-sitter grammar for .rein files. Enables syntax highlighting in any editor. | P2 | — | L |
| 5.4 | LSP server | Language Server Protocol implementation for .rein files: autocomplete, diagnostics, hover docs, go-to-definition. | P2 | 5.3 | XL |
| 5.5 | VSCode extension | Package the LSP + Tree-sitter into a VSCode extension. Publish to marketplace. | P2 | 5.4 | M |
| 5.6 | AI assistant skill files | Create SKILL.md / rules files for Claude Code, Cursor, Copilot, Windsurf that teach them .rein syntax. | P1 | — | S |
| 5.7 | `rein deploy` command | Deploy agents/workflows to Rein Cloud (Phase 6). Reads .rein files + config, pushes to API. | P2 | Phase 6 | M |

---

## Phase 6: Rein Cloud — The Money

| # | Title | Description | Priority | Deps | Size |
|---|-------|-------------|----------|------|------|
| 6.1 | Cloud API design | Design the Rein Cloud REST API: deploy, status, logs, cost, manage. OpenAPI spec. | P0 | Phase 2 | M |
| 6.2 | Cloud backend service | Build the hosted backend: receives .rein deployments, runs agents in isolated containers, tracks usage. | P0 | 6.1 | XL |
| 6.3 | Usage-based billing | Meter agent runs, token usage, tool calls. Integrate with Stripe for billing. | P0 | 6.2 | XL |
| 6.4 | Multi-tenant isolation | Process isolation per customer. Agents run in sandboxed environments with no cross-tenant access. | P0 | 6.2 | XL |
| 6.5 | Team management & SSO | Multi-user accounts, role-based access, SSO via SAML/OIDC for enterprise customers. | P1 | 6.2 | L |
| 6.6 | Compliance & audit reports | Generate compliance reports: who deployed what, which agents ran, what tools were accessed, spending. PDF/CSV export. | P1 | 4.1, 6.2 | L |
| 6.7 | Cloud dashboard | Web dashboard for Rein Cloud: deploy, monitor, manage agents and workflows. React or similar. | P1 | 6.2 | XL |
| 6.8 | Rein Studio — Visual Editor | Drag-and-drop editor for building .rein files visually. Generates valid .rein syntax. | P2 | 6.7 | XL |

---

## Summary

| Phase | Tickets | P0s | Est. Total |
|-------|---------|-----|------------|
| Phase 2: `rein run` | 19 | 14 | 1-2 weeks |
| Phase 3: Workflows | 8 | 4 | 2-3 weeks |
| Phase 4: Observability | 5 | 1 | 1-2 weeks |
| Phase 5: Dev Experience | 7 | 0 | Ongoing |
| Phase 6: Rein Cloud | 8 | 4 | 2-3 months |
| **Total** | **47** | **23** | — |

Phase 2 is the gate. Everything depends on agents actually running.
