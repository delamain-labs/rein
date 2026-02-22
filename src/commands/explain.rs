/// `rein explain` — human-readable summary of what a .rein file defines.
pub fn run_explain(path: &std::path::Path) -> i32 {
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

    println!("📋 Policy summary: {filename}");
    println!();

    explain_agents(&file);
    explain_workflows(&file);
    explain_types(&file);
    explain_providers(&file);
    explain_safety(&file);
    explain_other(&file);

    0
}

fn format_model(model: Option<&rein::ast::ValueExpr>) -> String {
    match model {
        Some(rein::ast::ValueExpr::Literal(s)) => s.clone(),
        Some(rein::ast::ValueExpr::EnvRef { var_name, .. }) => format!("env({var_name})"),
        None => "unspecified".to_string(),
    }
}

fn format_budget(budget: &rein::ast::Budget) -> String {
    let symbol = match budget.currency.as_str() {
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        _ => "$",
    };
    let dollars = budget.amount / 100;
    let cents = budget.amount % 100;
    format!("{symbol}{dollars}.{cents:02} per {}", budget.unit)
}

fn explain_agents(file: &rein::ast::ReinFile) {
    if file.agents.is_empty() {
        return;
    }
    println!("Agents ({})", file.agents.len());
    for agent in &file.agents {
        let model = format_model(agent.model.as_ref());
        print!("  • {} (model: {model})", agent.name);
        if let Some(ref archetype) = agent.from {
            print!(" extends {archetype}");
        }
        println!();

        if !agent.can.is_empty() {
            println!("    can: {}", summarize_capabilities(&agent.can));
        }
        if !agent.cannot.is_empty() {
            println!("    cannot: {}", summarize_capabilities(&agent.cannot));
        }
        if let Some(ref budget) = agent.budget {
            println!("    budget: {}", format_budget(budget));
        }
        if agent.guardrails.is_some() {
            println!("    guardrails: enabled");
        }
    }
    println!();
}

fn explain_workflows(file: &rein::ast::ReinFile) {
    if file.workflows.is_empty() {
        return;
    }
    println!("Workflows ({})", file.workflows.len());
    for wf in &file.workflows {
        let step_count = wf.steps.len();
        let stage_count = wf.stages.len();
        print!("  • {}", wf.name);
        if step_count > 0 {
            print!(", {step_count} steps");
        }
        if stage_count > 0 {
            print!(", {stage_count} stages");
        }
        println!();

        for step in &wf.steps {
            print!("    step {}: agent {}", step.name, step.agent);
            if step.when.is_some() {
                print!(" (conditional)");
            }
            if step.on_failure.is_some() {
                print!(" (retries)");
            }
            if step.escalate.is_some() {
                print!(" (escalates)");
            }
            println!();
        }
    }
    println!();
}

fn explain_types(file: &rein::ast::ReinFile) {
    if file.types.is_empty() {
        return;
    }
    println!("Types ({})", file.types.len());
    for t in &file.types {
        println!("  • {} ({} fields)", t.name, t.fields.len());
    }
    println!();
}

fn explain_providers(file: &rein::ast::ReinFile) {
    if file.providers.is_empty() {
        return;
    }
    println!("Providers ({})", file.providers.len());
    for p in &file.providers {
        let model = format_model(p.model.as_ref());
        println!("  • {} (model: {model})", p.name);
    }
    println!();
}

fn explain_safety(file: &rein::ast::ReinFile) {
    let mut features: Vec<&str> = Vec::new();
    if !file.policies.is_empty() {
        features.push("trust tiers");
    }
    if !file.circuit_breakers.is_empty() {
        features.push("circuit breakers");
    }
    if !file.consensus_blocks.is_empty() {
        features.push("consensus verification");
    }
    if !file.evals.is_empty() {
        features.push("eval quality gates");
    }
    if file.agents.iter().any(|a| a.guardrails.is_some()) {
        features.push("guardrails");
    }

    if features.is_empty() {
        return;
    }
    println!("Safety features: {}", features.join(", "));
    println!("  ⚠ Note: not yet enforced at runtime (use --strict to see details)");
    println!();
}

fn explain_other(file: &rein::ast::ReinFile) {
    let mut extras: Vec<String> = Vec::new();
    if !file.imports.is_empty() {
        extras.push(format!("{} imports", file.imports.len()));
    }
    if !file.tools.is_empty() {
        extras.push(format!("{} tools", file.tools.len()));
    }
    if !file.archetypes.is_empty() {
        extras.push(format!("{} archetypes", file.archetypes.len()));
    }
    if !file.observes.is_empty() {
        extras.push(format!("{} observe blocks", file.observes.len()));
    }
    if !file.fleets.is_empty() {
        extras.push(format!("{} fleets", file.fleets.len()));
    }
    if !file.channels.is_empty() {
        extras.push(format!("{} channels", file.channels.len()));
    }
    if !file.secrets.is_empty() {
        extras.push("secrets management".to_string());
    }
    if !file.scenarios.is_empty() {
        extras.push(format!("{} test scenarios", file.scenarios.len()));
    }
    if file.defaults.is_some() {
        extras.push("defaults block".to_string());
    }

    if !extras.is_empty() {
        println!("Also includes: {}", extras.join(", "));
    }
}

fn summarize_capabilities(caps: &[rein::ast::Capability]) -> String {
    if caps.len() <= 3 {
        caps.iter()
            .map(|c| format!("{}.{}", c.namespace, c.action))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let first: Vec<String> = caps
            .iter()
            .take(2)
            .map(|c| format!("{}.{}", c.namespace, c.action))
            .collect();
        format!("{} (+{} more)", first.join(", "), caps.len() - 2)
    }
}
