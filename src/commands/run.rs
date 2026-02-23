use std::sync::Arc;
use std::time::Instant;

pub fn run_agent(
    path: &std::path::Path,
    message: Option<&str>,
    dry_run: bool,
    demo: bool,
    otel: bool,
) -> i32 {
    let filename = path.to_string_lossy();

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{filename}': {e}");
            return 1;
        }
    };

    let file = match rein::parser::parse(&source) {
        Ok(f) => f,
        Err(e) => {
            rein::error::report_parse_error(&filename, &source, &e);
            return 1;
        }
    };

    let diags = rein::validator::validate(&file);
    let has_errors = diags.iter().any(rein::validator::Diagnostic::is_error);

    for diag in &diags {
        rein::error::report_diagnostic(&filename, &source, diag);
    }

    if has_errors {
        return 1;
    }

    if dry_run {
        return print_execution_plan(&file, message);
    }

    let Some(agent) = file.agents.first() else {
        eprintln!("error: no agents defined in '{filename}'");
        return 1;
    };

    let user_message = message.unwrap_or("Hello");

    let provider: Box<dyn rein::runtime::provider::Provider> = if demo {
        eprintln!("🎭 Demo mode: using mock provider (no API keys needed)\n");
        Box::new(rein::runtime::provider::demo::DemoProvider::new())
    } else {
        match super::provider::resolve(agent) {
            Ok(p) => p,
            Err(code) => return code,
        }
    };

    // Build permission registry from agent capabilities.
    let registry = rein::runtime::permissions::ToolRegistry::from_agent(agent);

    // Build run config.
    let config = rein::runtime::engine::RunConfig {
        system_prompt: None,
        max_turns: 10,
        budget_cents: agent.budget.as_ref().map_or(0, |b| b.amount),
    };

    // Build engine with enforcement.
    let executor = rein::runtime::executor::NoopExecutor;
    let mut engine = rein::runtime::engine::AgentEngine::new(
        provider.as_ref(),
        &executor,
        &registry,
        Vec::new(),
        config,
    )
    .with_stream(Box::new(rein::runtime::engine::StdoutStream));

    // Attach guardrails if defined.
    if let Some(ref guardrails_def) = agent.guardrails {
        let guardrail_engine = rein::runtime::guardrails::GuardrailEngine::from_def(guardrails_def);
        engine = engine.with_guardrails(guardrail_engine);
    }

    // Attach circuit breaker if defined.
    if let Some(cb_def) = file.circuit_breakers.first() {
        let cb = rein::runtime::circuit_breaker::CircuitBreaker::from_def(cb_def);
        engine = engine.with_circuit_breaker(cb);
    }

    // Attach policy engine if defined.
    if let Some(policy_def) = file.policies.first() {
        let policy = rein::runtime::policy::PolicyEngine::from_def(policy_def);
        eprintln!(
            "Policy: starting at tier '{}' ({} tiers defined)",
            policy.current_tier(),
            policy.tier_count()
        );
        engine = engine.with_policy(policy);
    }

    // Resolve OTEL mode from an observe block (matched by agent name, falling back to first)
    // or the --otel flag. `observe` blocks are file-level so we prefer the one whose name
    // matches the running agent; if none match, we take the first block as a file-wide default.
    let obs = file
        .observes
        .iter()
        .find(|o| o.name == agent.name)
        .or_else(|| file.observes.first());
    let otel_mode = resolve_otel_mode(obs, otel);
    engine = engine
        .with_otel_mode(otel_mode)
        .with_agent_name(agent.name.clone());

    // If the file has workflows, run the first workflow instead of single-agent execution.
    if let Some(workflow) = file.workflows.first() {
        let budget_cents = agent.budget.as_ref().map_or(0, |b| b.amount);
        return run_workflow_mode(workflow, &file, provider.as_ref(), &executor, budget_cents);
    }

    run_engine(&engine, user_message)
}

fn run_workflow_mode(
    workflow: &rein::ast::WorkflowDef,
    file: &rein::ast::ReinFile,
    provider: &dyn rein::runtime::provider::Provider,
    executor: &rein::runtime::executor::NoopExecutor,
    budget_cents: u64,
) -> i32 {
    let approval_handler = resolve_approval_handler();
    let wf_config = rein::runtime::engine::RunConfig {
        system_prompt: None,
        max_turns: 10,
        budget_cents,
    };
    let ctx = rein::runtime::workflow::WorkflowContext {
        file,
        provider,
        executor,
        tool_defs: &[],
        config: &wf_config,
        approval_handler: Some(approval_handler),
    };
    let start = Instant::now();
    let wf_result =
        super::provider::block_on(rein::runtime::workflow::run_workflow(workflow, &ctx));
    let duration = start.elapsed();
    match wf_result {
        Ok(result) => {
            eprintln!();
            eprintln!(
                "--- Workflow complete ({} stages) ---",
                result.stage_results.len()
            );
            eprintln!("Final output: {}", result.final_output);
            eprintln!("Duration: {duration:.2?}");
            0
        }
        Err(e) => {
            eprintln!("Workflow failed: {e}");
            1
        }
    }
}

fn run_engine(engine: &rein::runtime::engine::AgentEngine<'_>, user_message: &str) -> i32 {
    let start = Instant::now();
    let result = super::provider::block_on(engine.run(user_message));

    match result {
        Ok(run_result) => {
            let duration = start.elapsed();
            eprintln!();
            eprintln!("--- Run complete ---");
            eprintln!("{}", run_result.trace.summary());
            eprintln!("Duration: {duration:.2?}");
            0
        }
        Err(e) => {
            eprintln!();
            eprintln!("Run failed: {e:?}");
            1
        }
    }
}

/// Resolve the OTEL export mode from an optional `observe` block and the `--otel` flag.
fn resolve_otel_mode(
    observe: Option<&rein::ast::ObserveDef>,
    otel_flag: bool,
) -> rein::runtime::otel_export::OtelMode {
    use rein::runtime::otel_export::OtelMode;
    if let Some(obs) = observe {
        match obs.export.as_deref() {
            Some("stdout") => {
                return OtelMode::StdoutOnComplete {
                    metrics: obs.metrics.clone(),
                };
            }
            Some(_) => return OtelMode::FileOnComplete,
            None => {}
        }
    }
    if otel_flag {
        OtelMode::FileOnComplete
    } else {
        OtelMode::None
    }
}

fn resolve_approval_handler() -> Arc<dyn rein::runtime::approval::ApprovalHandler> {
    if std::env::var("REIN_AUTO_APPROVE").as_deref() == Ok("1") {
        Arc::new(rein::runtime::approval::AutoApproveHandler)
    } else if std::env::var("REIN_AUTO_REJECT").as_deref() == Ok("1") {
        Arc::new(rein::runtime::approval::AutoRejectHandler::new(
            "auto-rejected by REIN_AUTO_REJECT",
        ))
    } else {
        Arc::new(rein::runtime::approval::CliApprovalHandler)
    }
}

use super::provider::format_value_expr;

fn print_execution_plan(file: &rein::ast::ReinFile, message: Option<&str>) -> i32 {
    println!("📋 Execution Plan (dry run)\n");

    if !file.providers.is_empty() {
        println!("Providers ({}):", file.providers.len());
        for p in &file.providers {
            let model = p
                .model
                .as_ref()
                .map_or("default".to_string(), format_value_expr);
            println!("  • {} (model: {model})", p.name);
        }
        println!();
    }

    if !file.agents.is_empty() {
        println!("Agents ({}):", file.agents.len());
        for a in &file.agents {
            let can_count = a.can.len();
            let cannot_count = a.cannot.len();
            let budget_str = a.budget.as_ref().map_or_else(
                || "none".to_string(),
                |b| {
                    let sym = match b.currency.as_str() {
                        "EUR" => "€",
                        "GBP" => "£",
                        "JPY" => "¥",
                        _ => "$",
                    };
                    format!(
                        "{sym}{}.{:02} per {}",
                        b.amount / 100,
                        b.amount % 100,
                        b.unit
                    )
                },
            );
            let model = a
                .model
                .as_ref()
                .map_or("default".to_string(), format_value_expr);
            println!(
                "  • {} (model: {model}, can: {can_count}, cannot: {cannot_count}, budget: {budget_str})",
                a.name
            );
        }
        println!();
    }

    if !file.workflows.is_empty() {
        println!("Workflows ({}):", file.workflows.len());
        for w in &file.workflows {
            let trigger = &w.trigger;
            println!(
                "  • {} (trigger: {trigger}, steps: {})",
                w.name,
                w.steps.len()
            );
            for s in &w.steps {
                let guard = s
                    .when
                    .as_ref()
                    .map_or(String::new(), |_| " [guarded]".to_string());
                println!("    → {} (agent: {}){guard}", s.name, s.agent);
            }
        }
        println!();
    }

    if let Some(msg) = message {
        println!("Message: \"{msg}\"");
        println!();
    }

    println!("⚡ No API calls made. Use without --dry-run to execute.");
    0
}
