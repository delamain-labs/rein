//! Schedule evaluation for workflow triggers.
//!
//! Evaluates `ScheduleExpr` definitions to determine when workflows
//! should run. Supports daily-at, every-N-hours/minutes, and cron expressions.

use crate::ast::ScheduleExpr;

#[cfg(test)]
mod tests;

/// A resolved schedule with the next run time as seconds from now.
#[derive(Debug, Clone, PartialEq)]
pub struct NextRun {
    /// Seconds until the next scheduled run.
    pub seconds_from_now: u64,
    /// Human-readable description of the schedule.
    pub description: String,
}

/// Evaluate a schedule expression and return the interval in seconds.
///
/// For `daily at` schedules, this returns 86400 (24 hours).
/// For `every N hours/minutes`, this returns the interval directly.
/// For cron expressions, this returns an approximate interval.
#[must_use]
pub fn interval_seconds(expr: &ScheduleExpr) -> u64 {
    match expr {
        ScheduleExpr::DailyAt { .. } => 86_400,
        ScheduleExpr::EveryNHours { hours } => hours * 3_600,
        ScheduleExpr::EveryNMinutes { minutes } => minutes * 60,
        ScheduleExpr::Cron { .. } => {
            // Without a full cron parser, return a default interval.
            // In production, this would use a cron library to compute
            // the next occurrence.
            3_600 // default to hourly
        }
    }
}

/// Describe a schedule expression in human-readable form.
#[must_use]
pub fn describe(expr: &ScheduleExpr) -> String {
    match expr {
        ScheduleExpr::DailyAt { time } => format!("daily at {time}"),
        ScheduleExpr::EveryNHours { hours } => {
            if *hours == 1 {
                "every hour".to_string()
            } else {
                format!("every {hours} hours")
            }
        }
        ScheduleExpr::EveryNMinutes { minutes } => {
            if *minutes == 1 {
                "every minute".to_string()
            } else {
                format!("every {minutes} minutes")
            }
        }
        ScheduleExpr::Cron { expr } => format!("cron: {expr}"),
    }
}

/// Validate a schedule expression for basic correctness.
///
/// # Errors
/// Returns a description of the problem if the schedule is invalid.
pub fn validate(expr: &ScheduleExpr) -> Result<(), String> {
    match expr {
        ScheduleExpr::DailyAt { time } => {
            if time.is_empty() {
                return Err("daily schedule requires a time".to_string());
            }
            Ok(())
        }
        ScheduleExpr::EveryNHours { hours } => {
            if *hours == 0 || *hours > 168 {
                return Err(format!("invalid interval: {hours} hours (must be 1-168)"));
            }
            Ok(())
        }
        ScheduleExpr::EveryNMinutes { minutes } => {
            if *minutes == 0 || *minutes > 10_080 {
                return Err(format!("invalid interval: {minutes} minutes (must be 1-10080)"));
            }
            Ok(())
        }
        ScheduleExpr::Cron { expr } => {
            // Basic validation: cron should have 5 fields
            let fields: Vec<&str> = expr.split_whitespace().collect();
            if fields.len() != 5 {
                return Err(format!(
                    "cron expression should have 5 fields, got {}",
                    fields.len()
                ));
            }
            Ok(())
        }
    }
}
