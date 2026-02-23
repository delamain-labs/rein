use std::path::PathBuf;

/// Run `rein eval <file>` — execute all scenario and eval blocks and report pass/fail.
///
/// Exit code 0 = all pass, exit code 1 = any failure.
pub fn run_eval_command(
    path: &PathBuf,
    scenario_filter: Option<&str>,
    _verbose: bool,
    demo: bool,
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

    if file.scenarios.is_empty() && file.evals.is_empty() {
        eprintln!("No scenario or eval blocks found in '{filename}'.");
        return 0;
    }

    let provider: Box<dyn rein::runtime::provider::Provider> =
        match resolve_provider_for_eval(&file, demo, &filename) {
            Ok(p) => p,
            Err(code) => return code,
        };

    let total_scenarios = file.scenarios.len();
    if !file.scenarios.is_empty() {
        eprintln!("Running {total_scenarios} scenario(s) in {filename}...\n");
    }

    let (passed, failed) = run_scenarios(&file, scenario_filter, provider.as_ref());

    if failed == 0 && passed > 0 {
        eprintln!("\n{passed} passed");
        0
    } else if failed > 0 {
        eprintln!("\n{failed} failed, {passed} passed");
        1
    } else {
        if let Some(filter) =
            scenario_filter.filter(|_| passed == 0 && failed == 0 && !file.scenarios.is_empty())
        {
            eprintln!("No scenario named '{filter}' found.");
        }
        0
    }
}

fn run_scenarios(
    file: &rein::ast::ReinFile,
    scenario_filter: Option<&str>,
    provider: &dyn rein::runtime::provider::Provider,
) -> (usize, usize) {
    let mut passed = 0usize;
    let mut failed = 0usize;

    for scenario in &file.scenarios {
        if scenario_filter.is_some_and(|f| scenario.name != f) {
            continue;
        }

        let user_message = scenario
            .given
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");

        let Some(agent) = file.agents.first() else {
            eprintln!("  ✗ {}: no agent to run", scenario.name);
            failed += 1;
            continue;
        };

        let registry = rein::runtime::permissions::ToolRegistry::from_agent(agent);
        let config = rein::runtime::engine::RunConfig {
            system_prompt: None,
            max_turns: 5,
            budget_cents: 0,
        };
        let executor = rein::runtime::executor::NoopExecutor;
        let engine = rein::runtime::engine::AgentEngine::new(
            provider,
            &executor,
            &registry,
            Vec::new(),
            config,
        );

        let handle = tokio::runtime::Handle::try_current();
        let response = match if let Ok(handle) = handle {
            tokio::task::block_in_place(|| handle.block_on(engine.run(&user_message)))
        } else {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(engine.run(&user_message))
        } {
            Ok(r) => r.response,
            Err(e) => {
                eprintln!("  ✗ {} — agent run failed: {e:?}", scenario.name);
                failed += 1;
                continue;
            }
        };

        if scenario.expect.is_empty() {
            eprintln!("  ✓ {} — ran successfully (no expectations)", scenario.name);
            passed += 1;
            continue;
        }

        let mut scenario_passed = true;
        for (key, expected_value) in &scenario.expect {
            if rein::runtime::scenario::check_expectation(&response, expected_value) {
                eprintln!(
                    "  ✓ {} — {key} contains \"{expected_value}\"",
                    scenario.name
                );
                passed += 1;
            } else {
                eprintln!(
                    "  ✗ {} — {key} expected \"{expected_value}\" but not found in response",
                    scenario.name
                );
                failed += 1;
                scenario_passed = false;
            }
        }
        let _ = scenario_passed;
    }

    (passed, failed)
}

fn resolve_provider_for_eval(
    file: &rein::ast::ReinFile,
    demo: bool,
    filename: &str,
) -> Result<Box<dyn rein::runtime::provider::Provider>, i32> {
    if demo {
        eprintln!("🎭 Demo mode: using mock provider (no API keys needed)\n");
        return Ok(Box::new(rein::runtime::provider::demo::DemoProvider::new()));
    }
    let Some(agent) = file.agents.first() else {
        eprintln!("No agents defined in '{filename}'. Eval requires at least one agent.");
        return Err(1);
    };
    resolve_provider(agent)
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
        eprintln!("hint: set OPENAI_API_KEY or ANTHROPIC_API_KEY, or use --demo");
        1
    })
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
