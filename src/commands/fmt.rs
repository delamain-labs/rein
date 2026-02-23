use std::fs;
use std::path::Path;

/// Run the `rein fmt` command. Auto-formats .rein files to canonical style.
pub fn run_fmt(files: &[std::path::PathBuf], check: bool) -> i32 {
    let mut any_changed = false;
    let mut any_error = false;

    for file in files {
        match format_file(file) {
            Ok(formatted) => {
                let original = match fs::read_to_string(file) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error reading {}: {e}", file.display());
                        any_error = true;
                        continue;
                    }
                };
                if original != formatted {
                    any_changed = true;
                    if check {
                        println!("{} needs formatting", file.display());
                    } else {
                        if let Err(e) = fs::write(file, &formatted) {
                            eprintln!("Error writing {}: {e}", file.display());
                            any_error = true;
                            continue;
                        }
                        println!("Formatted {}", file.display());
                    }
                }
            }
            Err(e) => {
                eprintln!("Error formatting {}: {e}", file.display());
                any_error = true;
            }
        }
    }

    if any_error {
        return 2;
    }
    if check && any_changed {
        return 1;
    }
    if !any_changed && !check {
        println!("All {} files already formatted", files.len());
    }
    0
}

/// Format a .rein file source to canonical style.
///
/// # Errors
/// Returns an error if the file cannot be read or contains syntax errors.
/// Refusing to format syntactically invalid files prevents silent data loss
/// (e.g. accidentally whitespace-normalising a broken file and masking errors).
pub fn format_file(path: &Path) -> Result<String, std::io::Error> {
    let source = fs::read_to_string(path)?;
    let (_, errors) = rein::parser::parse_collecting(&source);
    if !errors.is_empty() {
        let msg = errors
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("syntax error: {msg}"),
        ));
    }
    Ok(format_source(&source))
}

/// Format .rein source text to canonical style.
///
/// Rules:
/// - 4-space indentation, normalized per nesting depth
/// - Single blank line between top-level blocks
/// - No trailing whitespace
/// - Single newline at end of file
/// - Consistent spacing around colons in key-value pairs
pub fn format_source(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut depth: i32 = 0;
    let mut prev_was_blank = false;
    let mut prev_was_open_brace = false;
    let mut first_line = true;

    for line in source.lines() {
        let trimmed = line.trim();

        // Skip multiple consecutive blank lines
        if trimmed.is_empty() {
            if !prev_was_blank && !first_line && !prev_was_open_brace {
                output.push('\n');
                prev_was_blank = true;
            }
            continue;
        }

        // Adjust depth for closing braces/brackets before indenting
        let starts_with_close = trimmed.starts_with('}') || trimmed.starts_with(']');
        if starts_with_close {
            depth -= 1;
            if depth < 0 {
                depth = 0;
            }
        }

        // Remove blank line right before closing brace
        if starts_with_close && prev_was_blank {
            // Remove the trailing newline we added
            if output.ends_with("\n\n") {
                output.pop();
            }
        }

        // Add blank line between top-level blocks (depth 0 non-comment lines)
        // but not at the start
        if depth == 0 && !first_line && !prev_was_blank && !starts_with_close {
            let is_block_start = trimmed.starts_with("agent ")
                || trimmed.starts_with("workflow ")
                || trimmed.starts_with("provider ")
                || trimmed.starts_with("policy ")
                || trimmed.starts_with("fleet ")
                || trimmed.starts_with("channel ")
                || trimmed.starts_with("observe ")
                || trimmed.starts_with("type ")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("defaults ")
                || trimmed.starts_with("archetype ")
                || trimmed.starts_with("tool ")
                || trimmed.starts_with("circuit_breaker ");
            if is_block_start {
                output.push('\n');
            }
        }

        // Indent and write line
        let indent = "    ".repeat(usize::try_from(depth).unwrap_or(0));
        let formatted_line = format_line(trimmed);
        output.push_str(&indent);
        output.push_str(&formatted_line);
        output.push('\n');

        prev_was_blank = false;
        prev_was_open_brace = trimmed.ends_with('{') || trimmed.ends_with('[');
        first_line = false;

        // Adjust depth for opening braces/brackets
        let opens = trimmed.chars().filter(|&c| c == '{' || c == '[').count();
        let closes = trimmed.chars().filter(|&c| c == '}' || c == ']').count();
        // We already decremented for leading close, so only count non-leading closes
        let effective_closes = if starts_with_close {
            closes - 1
        } else {
            closes
        };
        depth += i32::try_from(opens).unwrap_or(0) - i32::try_from(effective_closes).unwrap_or(0);
        if depth < 0 {
            depth = 0;
        }
    }

    // Ensure single trailing newline
    while output.ends_with("\n\n") {
        output.pop();
    }
    if !output.ends_with('\n') && !output.is_empty() {
        output.push('\n');
    }

    output
}

/// Format a single trimmed line — normalize spacing around colons and collapse whitespace.
fn format_line(trimmed: &str) -> String {
    // Don't touch comments
    if trimmed.starts_with("//") {
        return trimmed.to_string();
    }

    // For key: value lines, normalize spacing
    if let Some(colon_pos) = find_kv_colon(trimmed) {
        let key = trimmed[..colon_pos].trim_end();
        let value = trimmed[colon_pos + 1..].trim_start();
        if !value.is_empty() {
            return format!("{key}: {value}");
        }
    }

    // Collapse multiple spaces into single spaces
    collapse_whitespace(trimmed)
}

/// Collapse runs of whitespace into single spaces, preserving string literals.
fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch == ' ' || ch == '\t' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result
}

/// Find the colon position for a key-value pair (not inside strings or URLs).
fn find_kv_colon(line: &str) -> Option<usize> {
    // Simple heuristic: first colon that's preceded by an identifier
    // and not part of a URL (://)
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            // Skip if part of URL (://)
            if i + 2 < bytes.len() && bytes[i + 1] == b'/' && bytes[i + 2] == b'/' {
                return None;
            }
            // Must have identifier chars before it
            if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_basic_agent() {
        let input = r#"agent   support_triage  {
  model:anthropic

  can  [
    zendesk.read_ticket
      zendesk.reply_ticket
  ]

  budget:   $0.03 per ticket
}"#;
        let expected = r#"agent support_triage {
    model: anthropic

    can [
        zendesk.read_ticket
        zendesk.reply_ticket
    ]

    budget: $0.03 per ticket
}
"#;
        assert_eq!(format_source(input), expected);
    }

    #[test]
    fn test_format_preserves_comments() {
        let input = "// This is a comment\nagent foo {\n  model: openai\n}\n";
        let output = format_source(input);
        assert!(output.starts_with("// This is a comment\n"));
    }

    #[test]
    fn test_format_removes_extra_blank_lines() {
        let input = "agent foo {\n\n\n    model: openai\n\n\n}\n";
        let output = format_source(input);
        // Should have at most one blank line
        assert!(!output.contains("\n\n\n"));
    }

    #[test]
    fn test_format_blank_between_top_level_blocks() {
        let input = "agent a {\n    model: openai\n}\nagent b {\n    model: openai\n}\n";
        let output = format_source(input);
        assert!(output.contains("}\n\nagent b"));
    }

    #[test]
    fn test_format_normalizes_colon_spacing() {
        let input = "agent foo {\n    model:openai\n    budget:  $0.10 per request\n}\n";
        let output = format_source(input);
        assert!(output.contains("model: openai"));
        assert!(output.contains("budget: $0.10 per request"));
    }

    #[test]
    fn test_format_single_trailing_newline() {
        let input = "agent foo {\n    model: openai\n}\n\n\n";
        let output = format_source(input);
        assert!(output.ends_with("}\n"));
        assert!(!output.ends_with("}\n\n"));
    }

    #[test]
    fn test_check_mode() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        write!(f, "agent foo {{\n  model:openai\n}}\n").unwrap();
        let path = f.path().to_path_buf();

        // Check mode should return 1 (needs formatting) without modifying file
        let code = run_fmt(&[path.clone()], true);
        assert_eq!(code, 1);

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("model:openai")); // unchanged
    }

    #[test]
    fn test_write_mode() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        write!(f, "agent foo {{\n  model:openai\n}}\n").unwrap();
        let path = f.path().to_path_buf();

        let code = run_fmt(&[path.clone()], false);
        assert_eq!(code, 0);

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("model: openai")); // fixed
    }
}
