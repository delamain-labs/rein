pub fn run_agent(path: &std::path::Path, message: Option<&str>, dry_run: bool) -> i32 {
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

    if let Some(msg) = message {
        println!("Message: {msg}");
    }

    println!("Runtime not yet implemented");
    0
}

fn format_value_expr(v: &rein::ast::ValueExpr) -> String {
    match v {
        rein::ast::ValueExpr::Literal(s) => s.clone(),
        rein::ast::ValueExpr::EnvRef { var_name, default, .. } => {
            match default {
                Some(d) => format!("env(\"{var_name}\", \"{d}\")"),
                None => format!("env(\"{var_name}\")"),
            }
        }
    }
}

fn print_execution_plan(file: &rein::ast::ReinFile, message: Option<&str>) -> i32 {
    println!("📋 Execution Plan (dry run)\n");

    if !file.providers.is_empty() {
        println!("Providers ({}):", file.providers.len());
        for p in &file.providers {
            let model = p.model.as_ref().map_or("default".to_string(), format_value_expr);
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
                |b| format!("{}{} per {}", b.currency, b.amount, b.unit),
            );
            let model = a.model.as_ref().map_or("default".to_string(), format_value_expr);
            println!("  • {} (model: {model}, can: {can_count}, cannot: {cannot_count}, budget: {budget_str})",
                a.name);
        }
        println!();
    }

    if !file.workflows.is_empty() {
        println!("Workflows ({}):", file.workflows.len());
        for w in &file.workflows {
            let trigger = &w.trigger;
            println!("  • {} (trigger: {trigger}, steps: {})", w.name, w.steps.len());
            for s in &w.steps {
                let guard = s.when.as_ref().map_or(String::new(), |_| " [guarded]".to_string());
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
