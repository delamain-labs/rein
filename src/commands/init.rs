use std::fs;
use std::path::Path;

const EXAMPLE_REIN: &str = r#"// My first Rein agent
// Try: rein run agents/assistant.rein --demo

provider openai {
    model: "gpt-4o"
    key: env("OPENAI_API_KEY")
}

agent assistant {
    model: openai

    can [
        search.web
        files.read
    ]

    cannot [
        files.delete
        admin.anything
    ]

    budget: $0.10 per request

    guardrails {
        output_filter {
            pii_detection: redact
            toxicity: block
        }
    }
}
"#;

const ENV_TEMPLATE: &str = r"# Rein environment configuration
# OPENAI_API_KEY=sk-...
# ANTHROPIC_API_KEY=sk-ant-...
";

const REIN_TOML_TEMPLATE: &str = r#"[project]
name = "{name}"
version = "0.1.0"

[runtime]
default_model = "gpt-4o"
timeout_secs = 30
max_retries = 3
log_level = "info"

[dev]
watch = false
hot_reload = false
"#;

const GITIGNORE: &str = ".env\n";

const README_TEMPLATE: &str = r#"# My Rein Project

An AI agent orchestration project built with [Rein](https://github.com/delamain-labs/rein).

## Getting Started

1. Copy `.env.example` to `.env` and add your API keys
2. Run your agent:

```bash
rein run agents/assistant.rein --message "Hello!"
```

## Project Structure

- `agents/` — Agent definitions (`.rein` files)
- `.env.example` — Environment variable template
"#;

/// Run the `rein init` command. Creates a new project scaffold in the given directory.
pub fn run_init(dir: &Path) -> i32 {
    let project_name = dir.file_name().map_or_else(
        || "my-rein-project".to_string(),
        |n| n.to_string_lossy().to_string(),
    );

    if dir.exists() && dir.read_dir().is_ok_and(|mut d| d.next().is_some()) {
        eprintln!("Error: directory '{}' is not empty", dir.display());
        return 1;
    }

    if let Err(e) = scaffold(dir, &project_name) {
        eprintln!("Error initializing project: {e}");
        return 1;
    }

    println!("✨ Initialized new Rein project in {}", dir.display());
    println!();
    println!("  cd {project_name}");
    println!("  cp .env.example .env");
    println!("  rein run agents/assistant.rein --message \"Hello!\"");
    0
}

fn scaffold(dir: &Path, project_name: &str) -> std::io::Result<()> {
    let agents_dir = dir.join("agents");
    fs::create_dir_all(&agents_dir)?;
    fs::write(agents_dir.join("assistant.rein"), EXAMPLE_REIN)?;
    fs::write(dir.join(".env.example"), ENV_TEMPLATE)?;
    fs::write(dir.join(".gitignore"), GITIGNORE)?;
    fs::write(dir.join("README.md"), README_TEMPLATE)?;
    fs::write(
        dir.join("rein.toml"),
        REIN_TOML_TEMPLATE.replace("{name}", project_name),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_scaffold() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("my-project");
        let code = run_init(&project_dir);
        assert_eq!(code, 0);
        assert!(project_dir.join("agents/assistant.rein").exists());
        assert!(project_dir.join(".env.example").exists());
        assert!(project_dir.join(".gitignore").exists());
        assert!(project_dir.join("README.md").exists());

        let rein_content = fs::read_to_string(project_dir.join("agents/assistant.rein")).unwrap();
        assert!(rein_content.contains("agent assistant"));
    }

    #[test]
    fn test_init_fails_on_nonempty_dir() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("existing");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join("file.txt"), "hello").unwrap();
        let code = run_init(&project_dir);
        assert_eq!(code, 1);
    }

    #[test]
    fn test_init_succeeds_on_empty_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("empty");
        fs::create_dir_all(&project_dir).unwrap();
        let code = run_init(&project_dir);
        assert_eq!(code, 0);
        assert!(project_dir.join("agents/assistant.rein").exists());
    }
}
