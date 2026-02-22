use crate::ast::ScheduleExpr;
use crate::parser::parse;

#[test]
fn schedule_daily_at() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            schedule: daily at 2am
            step s { agent: a }
        }
    "#,
    )
    .unwrap();
    let sched = f.workflows[0].schedule.as_ref().unwrap();
    assert!(matches!(&sched.expr, ScheduleExpr::DailyAt { time } if time == "2am"));
}

#[test]
fn schedule_every_n_hours() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            schedule: every 6 hours
            step s { agent: a }
        }
    "#,
    )
    .unwrap();
    let sched = f.workflows[0].schedule.as_ref().unwrap();
    assert!(matches!(
        &sched.expr,
        ScheduleExpr::EveryNHours { hours: 6 }
    ));
}

#[test]
fn schedule_cron_string() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            schedule: "0 2 * * *"
            step s { agent: a }
        }
    "#,
    )
    .unwrap();
    let sched = f.workflows[0].schedule.as_ref().unwrap();
    assert!(matches!(&sched.expr, ScheduleExpr::Cron { expr } if expr == "0 2 * * *"));
}
