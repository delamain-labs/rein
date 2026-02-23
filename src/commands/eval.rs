use super::provider::{block_on, resolve};

/// Run `rein eval <file>` — execute all scenario blocks and report pass/fail.
///
/// Exit code 0 = all pass, exit code 1 = any failure.
///
/// Note: `eval` blocks (dataset-based assertions) are not yet executed by the
/// CLI — use `rein validate --strict` to surface unenforced eval blocks.
pub fn run_eval_command(
    path: &std::path::PathBuf,
    scenario_filter: Option<&str>,
    verbose: bool,
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

    if file.scenarios.is_empty() {
        if file.evals.is_empty() {
            eprintln!("No scenario blocks found in '{filename}'.");
        } else {
            eprintln!(
                "note: {filename} has {} eval block(s) — dataset-based eval is not yet \
                 supported by the CLI. Use `rein validate --strict` to check coverage.",
                file.evals.len()
            );
        }
        return 0;
    }

    let provider: Box<dyn rein::runtime::provider::Provider> =
        match resolve_provider_for_eval(&file, demo, &filename) {
            Ok(p) => p,
            Err(code) => return code,
        };

    eprintln!(
        "Running {} scenario(s) in {filename}...\n",
        file.scenarios.len()
    );

    let (passed, failed) = run_scenarios(&file, scenario_filter, provider.as_ref(), verbose);

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
    verbose: bool,
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
            stage_timeout_secs: None,
        };
        let executor = rein::runtime::executor::NoopExecutor;
        let engine = rein::runtime::engine::AgentEngine::new(
            provider,
            &executor,
            &registry,
            Vec::new(),
            config,
        );

        let response = match block_on(engine.run(&user_message)) {
            Ok(r) => r.response,
            Err(e) => {
                eprintln!("  ✗ {} — agent run failed: {e:?}", scenario.name);
                failed += 1;
                continue;
            }
        };

        if verbose {
            eprintln!("  [response] {response}");
        }

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
            } else {
                eprintln!(
                    "  ✗ {} — {key} expected \"{expected_value}\" but not found in response",
                    scenario.name
                );
                scenario_passed = false;
            }
        }

        if scenario_passed {
            passed += 1;
        } else {
            failed += 1;
        }
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
    resolve(agent)
}
