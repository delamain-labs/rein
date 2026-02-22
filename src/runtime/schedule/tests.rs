use super::*;
use crate::ast::ScheduleExpr;

#[test]
fn daily_at_interval_is_24_hours() {
    let expr = ScheduleExpr::DailyAt {
        time: "2am".to_string(),
    };
    assert_eq!(interval_seconds(&expr), 86_400);
}

#[test]
fn every_6_hours_interval() {
    let expr = ScheduleExpr::EveryNHours { hours: 6 };
    assert_eq!(interval_seconds(&expr), 21_600);
}

#[test]
fn every_30_minutes_interval() {
    let expr = ScheduleExpr::EveryNMinutes { minutes: 30 };
    assert_eq!(interval_seconds(&expr), 1_800);
}

#[test]
fn cron_default_interval() {
    let expr = ScheduleExpr::Cron {
        expr: "0 9 * * *".to_string(),
    };
    assert_eq!(interval_seconds(&expr), 3_600);
}

#[test]
fn describe_daily() {
    let expr = ScheduleExpr::DailyAt {
        time: "2am".to_string(),
    };
    assert_eq!(describe(&expr), "daily at 2am");
}

#[test]
fn describe_every_1_hour() {
    let expr = ScheduleExpr::EveryNHours { hours: 1 };
    assert_eq!(describe(&expr), "every hour");
}

#[test]
fn describe_every_n_hours() {
    let expr = ScheduleExpr::EveryNHours { hours: 6 };
    assert_eq!(describe(&expr), "every 6 hours");
}

#[test]
fn describe_every_1_minute() {
    let expr = ScheduleExpr::EveryNMinutes { minutes: 1 };
    assert_eq!(describe(&expr), "every minute");
}

#[test]
fn describe_cron() {
    let expr = ScheduleExpr::Cron {
        expr: "0 9 * * 1-5".to_string(),
    };
    assert_eq!(describe(&expr), "cron: 0 9 * * 1-5");
}

#[test]
fn validate_daily_ok() {
    let expr = ScheduleExpr::DailyAt {
        time: "14:30".to_string(),
    };
    assert!(validate(&expr).is_ok());
}

#[test]
fn validate_daily_empty_time() {
    let expr = ScheduleExpr::DailyAt {
        time: String::new(),
    };
    assert!(validate(&expr).is_err());
}

#[test]
fn validate_hours_zero() {
    let expr = ScheduleExpr::EveryNHours { hours: 0 };
    assert!(validate(&expr).is_err());
}

#[test]
fn validate_hours_too_large() {
    let expr = ScheduleExpr::EveryNHours { hours: 200 };
    assert!(validate(&expr).is_err());
}

#[test]
fn validate_cron_valid() {
    let expr = ScheduleExpr::Cron {
        expr: "0 9 * * *".to_string(),
    };
    assert!(validate(&expr).is_ok());
}

#[test]
fn validate_cron_wrong_fields() {
    let expr = ScheduleExpr::Cron {
        expr: "0 9 *".to_string(),
    };
    assert!(validate(&expr).is_err());
}
