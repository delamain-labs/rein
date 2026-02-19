use crate::parser::ParseError;
use crate::validator::{Diagnostic, Severity};
use ariadne::{Color, Label, Report, ReportKind, Source};

/// Render a `ParseError` to stderr as a coloured ariadne report.
pub fn report_parse_error(filename: &str, source: &str, error: &ParseError) {
    Report::build(ReportKind::Error, filename, error.span.start)
        .with_code("E000")
        .with_message("parse error")
        .with_label(
            Label::new((filename, error.span.start..error.span.end))
                .with_message(&error.message)
                .with_color(Color::Red),
        )
        .finish()
        .eprint((filename, Source::from(source)))
        .ok();
}

/// Render a `Diagnostic` to stderr as a coloured ariadne report.
pub fn report_diagnostic(filename: &str, source: &str, diag: &Diagnostic) {
    let kind = match diag.severity {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
    };
    let color = match diag.severity {
        Severity::Error => Color::Red,
        Severity::Warning => Color::Yellow,
    };
    Report::build(kind, filename, diag.span.start)
        .with_code(diag.code)
        .with_message(&diag.message)
        .with_label(
            Label::new((filename, diag.span.start..diag.span.end))
                .with_message(&diag.message)
                .with_color(color),
        )
        .finish()
        .eprint((filename, Source::from(source)))
        .ok();
}

/// Render a `ParseError` to a `String` (useful for tests / captured output).
pub fn format_parse_error(filename: &str, source: &str, error: &ParseError) -> String {
    let mut buf = Vec::new();
    Report::build(ReportKind::Error, filename, error.span.start)
        .with_code("E000")
        .with_message("parse error")
        .with_label(
            Label::new((filename, error.span.start..error.span.end))
                .with_message(&error.message)
                .with_color(Color::Red),
        )
        .finish()
        .write((filename, Source::from(source)), &mut buf)
        .ok();
    String::from_utf8_lossy(&buf).into_owned()
}

/// Render a `Diagnostic` to a `String` (useful for tests / captured output).
pub fn format_diagnostic(filename: &str, source: &str, diag: &Diagnostic) -> String {
    let kind = match diag.severity {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
    };
    let color = match diag.severity {
        Severity::Error => Color::Red,
        Severity::Warning => Color::Yellow,
    };
    let mut buf = Vec::new();
    Report::build(kind, filename, diag.span.start)
        .with_code(diag.code)
        .with_message(&diag.message)
        .with_label(
            Label::new((filename, diag.span.start..diag.span.end))
                .with_message(&diag.message)
                .with_color(color),
        )
        .finish()
        .write((filename, Source::from(source)), &mut buf)
        .ok();
    String::from_utf8_lossy(&buf).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;
    use crate::validator::{Diagnostic, Severity};

    fn make_error_diag(code: &'static str, message: &str, start: usize, end: usize) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            code,
            message: message.to_string(),
            span: Span::new(start, end),
        }
    }

    fn make_warning_diag(
        code: &'static str,
        message: &str,
        start: usize,
        end: usize,
    ) -> Diagnostic {
        Diagnostic {
            severity: Severity::Warning,
            code,
            message: message.to_string(),
            span: Span::new(start, end),
        }
    }

    #[test]
    fn format_error_contains_code() {
        let src = "agent foo { model: anthropic }";
        let diag = make_error_diag("E001", "duplicate agent name 'foo'", 0, 5);
        let output = format_diagnostic("test.rein", src, &diag);
        assert!(
            output.contains("E001"),
            "expected 'E001' in output:\n{}",
            output
        );
    }

    #[test]
    fn format_error_contains_message() {
        let src = "agent foo { model: anthropic }";
        let diag = make_error_diag("E002", "capability 'x.y' in both can and cannot", 0, 5);
        let output = format_diagnostic("test.rein", src, &diag);
        assert!(
            output.contains("x.y") || output.contains("can and cannot"),
            "expected message content in output:\n{}",
            output
        );
    }

    #[test]
    fn format_warning_contains_code() {
        let src = "agent foo { }";
        let diag = make_warning_diag("W001", "agent 'foo' has no model", 0, 13);
        let output = format_diagnostic("test.rein", src, &diag);
        assert!(
            output.contains("W001"),
            "expected 'W001' in output:\n{}",
            output
        );
    }

    #[test]
    fn format_parse_error_contains_e000() {
        use crate::parser::ParseError;
        let src = "agent { }";
        let err = ParseError {
            message: "expected identifier".to_string(),
            span: Span::new(6, 7),
        };
        let output = format_parse_error("test.rein", src, &err);
        assert!(
            output.contains("E000"),
            "expected 'E000' in output:\n{}",
            output
        );
    }

    #[test]
    fn format_parse_error_contains_message_text() {
        use crate::parser::ParseError;
        let src = "agent { }";
        let err = ParseError {
            message: "expected identifier, got LBrace".to_string(),
            span: Span::new(6, 7),
        };
        let output = format_parse_error("test.rein", src, &err);
        assert!(
            output.contains("identifier") || output.contains("LBrace"),
            "expected error message in output:\n{}",
            output
        );
    }
}
