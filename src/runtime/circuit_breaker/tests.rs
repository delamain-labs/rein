use super::*;

#[test]
fn new_breaker_is_closed() {
    let mut cb = CircuitBreaker::new("test", 3, 5, 1);
    assert_eq!(cb.state(), BreakerState::Closed);
    assert!(cb.check().is_ok());
}

#[test]
fn trips_open_after_threshold() {
    let mut cb = CircuitBreaker::new("test", 3, 5, 1);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), BreakerState::Closed);

    cb.record_failure();
    assert_eq!(cb.state(), BreakerState::Open);
    assert!(cb.check().is_err());
}

#[test]
fn check_error_message_includes_name() {
    let mut cb = CircuitBreaker::new("payments", 2, 5, 1);
    cb.record_failure();
    cb.record_failure();
    let err = cb.check().unwrap_err();
    assert!(err.contains("payments"));
    assert!(err.contains("open"));
}

#[test]
fn success_in_closed_state_is_noop() {
    let mut cb = CircuitBreaker::new("test", 3, 5, 1);
    cb.record_success();
    assert_eq!(cb.state(), BreakerState::Closed);
}

#[test]
fn half_open_success_resets_to_closed() {
    let mut cb = CircuitBreaker::new("test", 2, 5, 0);
    cb.record_failure();
    cb.record_failure();

    // With half_open_after = 0 minutes, state() transitions immediately to HalfOpen.
    assert_eq!(cb.state(), BreakerState::HalfOpen);
    assert!(cb.check().is_ok());

    cb.record_success();
    assert_eq!(cb.state(), BreakerState::Closed);
}

#[test]
fn half_open_failure_reopens() {
    let mut cb = CircuitBreaker::new("test", 2, 5, 0);
    cb.record_failure();
    cb.record_failure();

    // With 0 min wait, transitions immediately to half-open.
    assert_eq!(cb.state(), BreakerState::HalfOpen);

    cb.record_failure();
    // After failure in half-open, goes back to open, but 0-min wait
    // means next state() call transitions again.
    assert_eq!(cb.state(), BreakerState::HalfOpen);
}

#[test]
fn from_def_constructs_correctly() {
    let def = crate::ast::CircuitBreakerDef {
        name: "api_breaker".to_string(),
        failure_threshold: 5,
        window_minutes: 10,
        half_open_after_minutes: 2,
        span: crate::ast::Span { start: 0, end: 0 },
    };
    let mut cb = CircuitBreaker::from_def(&def);
    assert_eq!(cb.name(), "api_breaker");
    assert_eq!(cb.state(), BreakerState::Closed);
}

#[test]
fn single_failure_does_not_trip() {
    let mut cb = CircuitBreaker::new("test", 5, 10, 1);
    cb.record_failure();
    assert_eq!(cb.state(), BreakerState::Closed);
    assert!(cb.check().is_ok());
}

#[test]
fn threshold_of_one_trips_immediately() {
    let mut cb = CircuitBreaker::new("sensitive", 1, 5, 1);
    cb.record_failure();
    assert_eq!(cb.state(), BreakerState::Open);
}
