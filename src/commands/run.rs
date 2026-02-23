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
        match resolve_provider(agent) {
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

    // Log policy tier if defined.
    if let Some(policy_def) = file.policies.first() {
        let policy = rein::runtime::policy::PolicyEngine::from_def(policy_def);
        eprintln!(
            "Policy: starting at tier '{}' ({} tiers defined)",
            policy.current_tier(),
            policy.tier_count()
        );
    }

    // Execute.
    let start = Instant::now();
    let handle = tokio::runtime::Handle::try_current();
    let result = if let Ok(handle) = handle {
        // Already inside a tokio runtime (e.g. #[tokio::main])
        tokio::task::block_in_place(|| handle.block_on(engine.run(user_message)))
    } else {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(engine.run(user_message))
    };

    match result {
        Ok(run_result) => {
            let duration = start.elapsed();
            eprintln!();
            eprintln!("--- Run complete ---");
            eprintln!("{}", run_result.trace.summary());
            eprintln!("Duration: {duration:.2?}");

            if otel {
                write_otel_trace(&run_result.trace, &agent.name, duration);
            }
            0
        }
        Err(e) => {
            eprintln!();
            eprintln!("Run failed: {e:?}");
            1
        }
    }
}

fn resolve_provider(
    agent: &rein::ast::AgentDef,
) -> Result<Box<dyn rein::runtime::provider::Provider>, i32> {
    let model_field = agent
        .model
        .as_ref()
        .map_or("openai".to_string(), format_value_expr);

    let config = rein::runtime::provider::resolver::ProviderConfig {
        openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
        openai_base_url: std::env::var("OPENAI_BASE_URL").ok(),
        anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        anthropic_base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
    };

    rein::runtime::provider::resolver::resolve(&model_field, &config).map_err(|e| {
        eprintln!("error: {e}");
        eprintln!("hint: set OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable");
        1
    })
}

fn write_otel_trace(
    trace: &rein::runtime::RunTrace,
    agent_name: &str,
    duration: std::time::Duration,
) {
    let now = chrono::Utc::now();
    let started = now - duration;
    let structured = trace.to_structured(
        agent_name,
        &started.to_rfc3339(),
        &now.to_rfc3339(),
        duration.as_millis().try_into().unwrap_or(u64::MAX),
    );

    match rein::runtime::otel_export::to_otlp_json(&structured) {
        Ok(json) => {
            let path = format!("rein-trace-{}.json", now.format("%Y%m%d-%H%M%S"));
            match std::fs::write(&path, &json) {
                Ok(()) => eprintln!("OTLP trace written to {path}"),
                Err(e) => eprintln!("Failed to write OTLP trace: {e}"),
            }
        }
        Err(e) => eprintln!("Failed to serialize OTLP trace: {e}"),
    }
}

fn format_value_expr(v: &rein::ast::ValueExpr) -> String {
    match v {
        rein::ast::ValueExpr::Literal(s) => s.clone(),
        rein::ast::ValueExpr::EnvRef {
            var_name, default, ..
        } => match default {
            Some(d) => format!("env(\"{var_name}\", \"{d}\")"),
            None => format!("env(\"{var_name}\")"),
        },
    }
}

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
