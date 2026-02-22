use serde::{Deserialize, Serialize};

use super::Span;

/// A schedule expression for triggering workflows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScheduleExpr {
    /// `daily at <time>` (e.g. "2am", "14:30").
    DailyAt { time: String },
    /// `every <n> hours`.
    EveryNHours { hours: u64 },
    /// `every <n> minutes`.
    EveryNMinutes { minutes: u64 },
    /// A cron expression string.
    Cron { expr: String },
}

/// A `schedule: ...` trigger attached to a workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleDef {
    pub expr: ScheduleExpr,
    pub span: Span,
}
