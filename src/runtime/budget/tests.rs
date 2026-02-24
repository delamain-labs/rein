use super::*;
use crate::runtime::provider::Usage;

// ---------------------------------------------------------------------------
// Cost calculation tests
// ---------------------------------------------------------------------------

#[test]
fn gpt4o_cost_calculation() {
    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 500,
    };
    let cost = calculate_cost("gpt-4o", &usage);
    // input: 1000 * 250 / 1M = 0.25 cents
    // output: 500 * 1000 / 1M = 0.5 cents
    // total: 0.75, rounded up = 1 cent
    assert_eq!(cost, 1);
}

#[test]
fn gpt4o_mini_is_cheaper() {
    let usage = Usage {
        input_tokens: 100_000,
        output_tokens: 50_000,
    };
    let gpt4o_cost = calculate_cost("gpt-4o", &usage);
    let mini_cost = calculate_cost("gpt-4o-mini", &usage);
    assert!(mini_cost < gpt4o_cost, "mini={mini_cost}, 4o={gpt4o_cost}");
}

#[test]
fn gpt35_pricing() {
    let usage = Usage {
        input_tokens: 10_000,
        output_tokens: 5_000,
    };
    let cost = calculate_cost("gpt-3.5-turbo", &usage);
    // input: 10000 * 50 / 1M = 0.5 cents
    // output: 5000 * 150 / 1M = 0.75 cents
    // total: 1.25, rounded up = 2 cents
    assert_eq!(cost, 2);
}

#[test]
fn unknown_model_uses_default() {
    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 1000,
    };
    let cost = calculate_cost("some-unknown-model", &usage);
    // default: 500 input, 1500 output per M
    // 1000*500 + 1000*1500 = 2_000_000 / 1M = 2 cents
    assert_eq!(cost, 2);
}

#[test]
fn zero_tokens_costs_zero() {
    let usage = Usage {
        input_tokens: 0,
        output_tokens: 0,
    };
    let cost = calculate_cost("gpt-4o", &usage);
    assert_eq!(cost, 0);
}

#[test]
fn large_token_count() {
    let usage = Usage {
        input_tokens: 100_000,
        output_tokens: 50_000,
    };
    let cost = calculate_cost("gpt-4o", &usage);
    // input: 100000 * 250 = 25_000_000 / 1M = 25 cents
    // output: 50000 * 1000 = 50_000_000 / 1M = 50 cents
    // total: 75 cents (+ rounding) = 75 or 76
    assert!(cost >= 75 && cost <= 76, "cost={cost}");
}

#[test]
fn claude_pricing() {
    let usage = Usage {
        input_tokens: 1_000_000,
        output_tokens: 0,
    };
    let cost = calculate_cost("claude-sonnet-4-20250514", &usage);
    // 1M * 300 / 1M = 300 cents exactly
    assert_eq!(cost, 300);
}

// ---------------------------------------------------------------------------
// Budget tracker tests
// ---------------------------------------------------------------------------

#[test]
fn new_tracker_has_full_budget() {
    let tracker = BudgetTracker::new(100);
    assert_eq!(tracker.remaining_cents(), 100);
    assert_eq!(tracker.spent_cents(), 0);
    assert!(!tracker.is_exceeded());
}

#[test]
fn record_usage_reduces_remaining() {
    let mut tracker = BudgetTracker::new(100);
    tracker.record_usage(30).expect("within budget");
    assert_eq!(tracker.remaining_cents(), 70);
    assert_eq!(tracker.spent_cents(), 30);
}

#[test]
fn record_usage_at_exact_limit() {
    let mut tracker = BudgetTracker::new(100);
    tracker.record_usage(100).expect("at limit");
    assert_eq!(tracker.remaining_cents(), 0);
    assert!(!tracker.is_exceeded());
}

#[test]
fn record_usage_over_limit_returns_error() {
    let mut tracker = BudgetTracker::new(100);
    tracker.record_usage(50).expect("within budget");
    let err = tracker.record_usage(60).unwrap_err();
    assert_eq!(err.spent_cents, 110);
    assert_eq!(err.limit_cents, 100);
    // Budget should NOT have been updated on failure
    assert_eq!(tracker.spent_cents(), 50);
}

#[test]
fn budget_exceeded_display() {
    let err = BudgetExceeded {
        spent_cents: 150,
        limit_cents: 100,
    };
    let msg = err.to_string();
    assert!(msg.contains("150"), "msg: {msg}");
    assert!(msg.contains("100"), "msg: {msg}");
}

#[test]
fn multiple_recordings() {
    let mut tracker = BudgetTracker::new(100);
    tracker.record_usage(20).expect("ok");
    tracker.record_usage(30).expect("ok");
    tracker.record_usage(40).expect("ok");
    assert_eq!(tracker.spent_cents(), 90);
    assert_eq!(tracker.remaining_cents(), 10);

    let err = tracker.record_usage(20).unwrap_err();
    assert_eq!(err.spent_cents, 110);
    assert_eq!(tracker.spent_cents(), 90); // unchanged on error
}

// ── #390: BudgetExceeded struct carries both spent_cents and limit_cents ──
// These are the exact field values that get written into RunEvent::BudgetUpdate
// on the exceeded path inside AgentEngine::check_budget().

#[test]
fn budget_exceeded_carries_spent_and_limit_cents() {
    let mut tracker = BudgetTracker::new(50);
    // Push spend close to limit
    tracker.record_usage(40).expect("ok");
    // Exceed by 20 (40 spent + 20 new = 60 > 50 limit)
    let err = tracker.record_usage(20).unwrap_err();
    assert_eq!(err.limit_cents, 50, "limit_cents must equal the configured budget limit");
    assert_eq!(err.spent_cents, 60, "spent_cents must equal the would-be total after the overage");
}

#[test]
fn alert_triggers_at_threshold() {
    let mut tracker = BudgetTracker::with_thresholds(100, vec![50, 80]);
    tracker.record_usage(49).unwrap();
    assert!(tracker.check_alerts().is_empty());

    tracker.record_usage(2).unwrap(); // 51%
    let alerts = tracker.check_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].threshold_pct, 50);

    // Same threshold doesn't fire again
    assert!(tracker.check_alerts().is_empty());

    tracker.record_usage(30).unwrap(); // 81%
    let alerts = tracker.check_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].threshold_pct, 80);
}

#[test]
fn alert_multiple_thresholds_at_once() {
    let mut tracker = BudgetTracker::with_thresholds(100, vec![50, 80, 90]);
    tracker.record_usage(95).unwrap(); // crosses all three at once
    let alerts = tracker.check_alerts();
    assert_eq!(alerts.len(), 3);
}

#[test]
fn alert_display_format() {
    let alert = BudgetAlert {
        threshold_pct: 80,
        spent_cents: 85,
        limit_cents: 100,
    };
    let s = format!("{alert}");
    assert!(s.contains("80%"));
    assert!(s.contains("85"));
}
