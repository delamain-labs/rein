use serde::Serialize;

#[derive(Serialize)]
struct JsonReport {
    file: String,
    valid: bool,
    errors: usize,
    warnings: usize,
    diagnostics: Vec<JsonDiagnostic>,
}

#[derive(Serialize)]
struct JsonDiagnostic {
    severity: String,
    code: String,
    message: String,
    span: JsonSpan,
}

#[derive(Serialize)]
struct JsonSpan {
    start: usize,
    end: usize,
}

impl JsonReport {
    fn print(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }

    fn io_error(filename: &str, msg: String) -> Self {
        Self {
            file: filename.to_string(),
            valid: false,
            errors: 1,
            warnings: 0,
            diagnostics: vec![JsonDiagnostic {
                severity: "error".to_string(),
                code: "IO".to_string(),
                message: msg,
                span: JsonSpan { start: 0, end: 0 },
            }],
        }
    }

    fn parse_error(filename: &str, e: &rein::parser::ParseError) -> Self {
        Self {
            file: filename.to_string(),
            valid: false,
            errors: 1,
            warnings: 0,
            diagnostics: vec![JsonDiagnostic {
                severity: "error".to_string(),
                code: "PARSE".to_string(),
                message: e.message.clone(),
                span: JsonSpan {
                    start: e.span.start,
                    end: e.span.end,
                },
            }],
        }
    }

    fn from_diagnostics(filename: &str, diags: &[rein::validator::Diagnostic]) -> Self {
        let errors = diags.iter().filter(|d| d.is_error()).count();
        let warnings = diags.len() - errors;
        Self {
            file: filename.to_string(),
            valid: errors == 0,
            errors,
            warnings,
            diagnostics: diags
                .iter()
                .map(|d| JsonDiagnostic {
                    severity: if d.is_error() { "error" } else { "warning" }.to_string(),
                    code: d.code.to_string(),
                    message: d.message.clone(),
                    span: JsonSpan {
                        start: d.span.start,
                        end: d.span.end,
                    },
                })
                .collect(),
        }
    }
}

pub fn run_validate(path: &std::path::Path, dump_ast: bool, format: &str) -> i32 {
    let filename = path.to_string_lossy();
    let json_mode = format == "json";

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            if json_mode {
                JsonReport::io_error(&filename, format!("cannot read file: {e}")).print();
            } else {
                eprintln!("error: cannot read '{filename}': {e}");
            }
            return 1;
        }
    };

    let file = match rein::parser::parse(&source) {
        Ok(f) => f,
        Err(e) => {
            if json_mode {
                JsonReport::parse_error(&filename, &e).print();
            } else {
                rein::error::report_parse_error(&filename, &source, &e);
            }
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

    if json_mode {
        JsonReport::from_diagnostics(&filename, &diags).print();
    } else {
        for diag in &diags {
            rein::error::report_diagnostic(&filename, &source, diag);
        }
        if !has_errors {
            if diags.is_empty() {
                println!("✓ Valid");
            } else {
                println!("✓ Valid (with warnings)");
            }
        }
    }

    i32::from(has_errors)
}
