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
  that relied on any failure producing exit `1` should check for non-zero exit
  with `[ $? -ne 0 ]` or test for `$? -ge 1`.

- **`AGENT_OUTPUT_PREVIEW_LIMIT` and `TRUNCATION_MARKER` moved to module level** (PR #521)

  Previously these were associated constants on `AuditingApprovalHandler`:
  ```
  rein::runtime::approval::AuditingApprovalHandler::AGENT_OUTPUT_PREVIEW_LIMIT
  rein::runtime::approval::AuditingApprovalHandler::TRUNCATION_MARKER
  ```
  They are now module-level `pub const` items:
  ```
  rein::runtime::approval::AGENT_OUTPUT_PREVIEW_LIMIT
  rein::runtime::approval::TRUNCATION_MARKER
  ```
  **Migration:** Update any reference to the old associated-constant path. The
  values are unchanged.

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

- `RunEvent::StepFailed` now carries an `error_kind: String` field — a
  stable `snake_case` identifier for the failure mode (e.g.
  `"agent_not_found"`, `"stage_failed"`). OTEL dashboards and alerting
  rules can filter on the new `rein.step.error_kind` span attribute
  instead of parsing the human-readable `reason` string with regex.

  Deserializes as `"unknown"` from JSON produced before this field was
  added, so replaying persisted event streams does not break. (#452)

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
