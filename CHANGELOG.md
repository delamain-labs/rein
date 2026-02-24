# Changelog

All notable changes to rein are documented here.

This project follows [Semantic Versioning](https://semver.org/). Changes are
grouped by type: **Breaking**, **Added**, **Changed**, **Fixed**, **Removed**.

---

## [Unreleased]

### Breaking

- **`rein run` exit codes changed for workflow files** (PR #374)

  | Exit code | Meaning |
  |-----------|---------|
  | `0` | All workflow steps succeeded |
  | `1` | Partial success — at least one step failed or was skipped; others may have completed |
  | `2` | Hard abort — a fatal `WorkflowError` stopped execution (e.g. cyclic dependency, approval rejected, timed-out stage) |

  Previously any step failure would propagate as an error and exit `1` via
  the error path. Now soft failures (agent not found, LLM error) are absorbed
  into a step result with `StageResultStatus::Failed`, dependent steps are
  cascade-skipped, and the overall exit code reflects the partial-success
  outcome rather than a hard error.

  **Migration:** Shell scripts or CI pipelines that treated exit `1` as "hard
  failure" must now also handle exit `2` for fatal workflow errors. Scripts
  that relied on any failure producing exit `1` should switch to `exit $? -ne 0`
  or explicitly test for `$? -ge 1`.

- **`RunError::BudgetExceeded` wire format changed** (PR #479)

  Previously serialized as the bare string `"budget_exceeded"`. Now serializes
  as `{"budget_exceeded": {}}` because `partial_trace` was added as a
  (skipped) struct field. Consumers deserializing raw JSON must update their
  pattern matching.

- **`RunError::CircuitBreakerOpen` wire format changed** (PR #479)

  Same change as `BudgetExceeded` — bare string `"circuit_breaker_open"`
  becomes `{"circuit_breaker_open": {}}`.

- **`RunError::Timeout` wire format changed** (PR #485)

  Previously serialized `partial_trace` events on the wire as
  `{"timeout": {"events": [...]}}`. Now serializes as `{"timeout": {}}`
  (empty object) because `partial_trace` is marked `#[serde(skip)]`.

### Added

- `WorkflowError::ApprovalPending` — new hard-error variant for steps where
  an `ApprovalHandler` returns `ApprovalStatus::Pending` (deferred /
  async approval dispatch). Previously `Pending` was silently aliased to
  `ApprovalTimedOut`. (#419)

- `RunEvent` is now `#[non_exhaustive]` — downstream library consumers
  can add wildcard match arms and receive new event variants in minor
  versions without a compile break. (#416)

- Partial OTEL traces from timed-out stages now carry
  `rein.run.partial = "true"` on the root span. (#430)

### Fixed

- `StageTimeout` display now uses 1-indexed turn numbers (consistent with
  `LlmCall` display convention). (#423)

- OTEL `rein.stage.turn` attribute now uses `i64::MAX` as the overflow
  sentinel instead of `-1`, avoiding ambiguity with valid turn indices. (#425)

- `CircuitBreakerTripped` event now emits real `failure_count()` and
  `threshold()` values instead of hardcoded `0`. (#389)

- `BudgetUpdate` event on the exceeded path now correctly reports
  `spent_cents` and `limit_cents` from the tracker. (#390)

---

## [0.2.1] — 2025-12-01

Initial changelog entry. See git history for pre-changelog changes.
