use crate::parser::ParseError;
use crate::validator::{Diagnostic, Severity};
use ariadne::{Color, Label, Report, ReportKind, Source};

// ── Private helpers ───────────────────────────────────────────────────────────

/// Map a `Severity` to the ariadne `ReportKind` and label colour.
fn severity_to_kind_and_color(severity: &Severity) -> (ReportKind<'_>, Color) {
    match severity {
        Severity::Error => (ReportKind::Error, Color::Red),
        Severity::Warning => (ReportKind::Warning, Color::Yellow),
    }
}

/// Build a finished ariadne `Report` from raw components.
///
/// The returned report borrows `filename` for its source-ID, so the caller
/// must pass `filename` to the output method (`.eprint` / `.write`) as well.
#[allow(clippy::too_many_arguments)]
fn build_report<'a>(
    filename: &'a str,
    kind: ReportKind<'a>,
    code: &str,
    message: &str,
    label_message: &str,
    label_color: Color,
    span_start: usize,
    span_end: usize,
) -> Report<'a, (&'a str, std::ops::Range<usize>)> {
    Report::build(kind, filename, span_start)
        .with_code(code)
        .with_message(message)
        .with_label(
            Label::new((filename, span_start..span_end))
                .with_message(label_message)
                .with_color(label_color),
        )
        .finish()
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Render a `ParseError` to stderr as a coloured ariadne report.
pub fn report_parse_error(filename: &str, source: &str, error: &ParseError) {
    build_report(
        filename,
        ReportKind::Error,
        "E000",
        "parse error",
        &error.message,
        Color::Red,
        error.span.start,
        error.span.end,
    )
    .eprint((filename, Source::from(source)))
    .ok();
}

/// Render a `Diagnostic` to stderr as a coloured ariadne report.
pub fn report_diagnostic(filename: &str, source: &str, diag: &Diagnostic) {
    let (kind, color) = severity_to_kind_and_color(&diag.severity);
    build_report(
        filename,
        kind,
        diag.code,
        &diag.message,
        &diag.message,
        color,
        diag.span.start,
        diag.span.end,
    )
    .eprint((filename, Source::from(source)))
    .ok();
}

/// Render a `ParseError` to a `String` (useful for tests / captured output).
pub fn format_parse_error(filename: &str, source: &str, error: &ParseError) -> String {
    let mut buf = Vec::new();
    build_report(
        filename,
        ReportKind::Error,
        "E000",
        "parse error",
        &error.message,
        Color::Red,
        error.span.start,
        error.span.end,
    )
    .write((filename, Source::from(source)), &mut buf)
    .ok();
    String::from_utf8_lossy(&buf).into_owned()
}

/// Render a `Diagnostic` to a `String` (useful for tests / captured output).
pub fn format_diagnostic(filename: &str, source: &str, diag: &Diagnostic) -> String {
    let (kind, color) = severity_to_kind_and_color(&diag.severity);
    let mut buf = Vec::new();
    build_report(
        filename,
        kind,
        diag.code,
        &diag.message,
        &diag.message,
        color,
        diag.span.start,
        diag.span.end,
    )
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
