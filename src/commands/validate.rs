pub fn run_validate(path: &std::path::Path, dump_ast: bool) -> i32 {
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

    if dump_ast {
        match serde_json::to_string_pretty(&file) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("error: failed to serialise AST: {e}");
                return 1;
            }
        }
        return 0;
    }

    let diags = rein::validator::validate(&file);
    let has_errors = diags.iter().any(rein::validator::Diagnostic::is_error);

    for diag in &diags {
        rein::error::report_diagnostic(&filename, &source, diag);
    }

    if has_errors {
        1
    } else {
        if diags.is_empty() {
            println!("✓ Valid");
        } else {
            // warnings only
            println!("✓ Valid (with warnings)");
        }
        0
    }
}
