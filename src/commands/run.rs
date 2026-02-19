pub fn run_agent(path: &std::path::Path, message: Option<&str>) -> i32 {
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

    if let Some(msg) = message {
        println!("Message: {msg}");
    }

    println!("Runtime not yet implemented");
    0
}
