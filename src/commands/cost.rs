use std::fs;
use std::path::Path;

use rein::runtime::StructuredTrace;

/// Aggregate cost statistics from one or more trace files.
pub fn run_cost(trace_dirs: &[std::path::PathBuf]) -> i32 {
    let mut traces = Vec::new();

    for path in trace_dirs {
        if path.is_file() {
            match load_trace(path) {
                Ok(t) => traces.push(t),
                Err(e) => {
                    eprintln!("Warning: skipping {}: {e}", path.display());
                }
            }
        } else if path.is_dir() {
            match collect_traces_from_dir(path) {
                Ok(mut ts) => traces.append(&mut ts),
                Err(e) => {
                    eprintln!("Warning: error reading {}: {e}", path.display());
                }
            }
        } else {
            eprintln!("Warning: {} not found", path.display());
        }
    }

    if traces.is_empty() {
        eprintln!("No trace files found");
        return 1;
    }

    print_summary(&traces);
    0
}

fn load_trace(path: &Path) -> Result<StructuredTrace, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let trace: StructuredTrace = serde_json::from_str(&content)?;
    Ok(trace)
}

fn collect_traces_from_dir(dir: &Path) -> Result<Vec<StructuredTrace>, std::io::Error> {
    let mut traces = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json")
            && let Ok(t) = load_trace(&path)
        {
            traces.push(t);
        }
    }
    Ok(traces)
}

fn print_summary(traces: &[StructuredTrace]) {
    let total_cost: u64 = traces.iter().map(|t| t.stats.total_cost_cents).sum();
    let total_tokens: u64 = traces.iter().map(|t| t.stats.total_tokens).sum();
    let total_llm_calls: u64 = traces.iter().map(|t| t.stats.llm_calls).sum();
    let total_tool_calls: u64 = traces.iter().map(|t| t.stats.tool_calls).sum();
    let total_denied: u64 = traces.iter().map(|t| t.stats.tool_calls_denied).sum();
    let total_duration: u64 = traces.iter().map(|t| t.stats.duration_ms).sum();
    let total_timeouts: u64 = traces.iter().map(|t| t.stats.timeout_count).sum();

    // Per-agent breakdown
    let mut agent_costs: std::collections::HashMap<String, (u64, u64, u64)> =
        std::collections::HashMap::new();
    for trace in traces {
        let entry = agent_costs.entry(trace.agent.clone()).or_insert((0, 0, 0));
        entry.0 += trace.stats.total_cost_cents;
        entry.1 += trace.stats.total_tokens;
        entry.2 += 1;
    }

    println!("Cost Summary");
    println!("============");
    println!("Runs:         {}", traces.len());
    println!(
        "Total cost:   ${}.{:02}",
        total_cost / 100,
        total_cost % 100
    );
    println!("Total tokens: {total_tokens}");
    println!("LLM calls:    {total_llm_calls}");
    println!("Tool calls:   {total_tool_calls} ({total_denied} denied)");
    if total_timeouts > 0 {
        println!("Timeouts:     {total_timeouts}");
    }
    println!(
        "Total time:   {}.{:01}s",
        total_duration / 1000,
        (total_duration % 1000) / 100
    );

    if agent_costs.len() > 1 {
        println!();
        println!("Per Agent");
        println!("---------");
        let mut agents: Vec<_> = agent_costs.into_iter().collect();
        agents.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        for (agent, (cost, tokens, runs)) in agents {
            println!(
                "  {agent}: ${}.{:02} ({tokens} tokens, {runs} runs)",
                cost / 100,
                cost % 100
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rein::runtime::TraceStats;
    use tempfile::TempDir;

    fn make_trace(agent: &str, cost: u64, tokens: u64) -> StructuredTrace {
        StructuredTrace {
            version: "1.0".to_string(),
            started_at: "2024-01-01T00:00:00Z".to_string(),
            completed_at: "2024-01-01T00:01:00Z".to_string(),
            agent: agent.to_string(),
            events: vec![],
            stats: TraceStats {
                total_tokens: tokens,
                total_cost_cents: cost,
                llm_calls: 1,
                tool_calls: 2,
                tool_calls_denied: 0,
                duration_ms: 1000,
                timeout_count: 0,
            },
            is_partial: false,
        }
    }

    #[test]
    fn test_load_trace_from_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("trace.json");
        let trace = make_trace("agent_a", 150, 1000);
        std::fs::write(&path, serde_json::to_string(&trace).unwrap()).unwrap();

        let loaded = load_trace(&path).unwrap();
        assert_eq!(loaded.agent, "agent_a");
        assert_eq!(loaded.stats.total_cost_cents, 150);
    }

    #[test]
    fn test_collect_traces_from_dir() {
        let tmp = TempDir::new().unwrap();
        for i in 0..3 {
            let path = tmp.path().join(format!("trace_{i}.json"));
            let trace = make_trace(&format!("agent_{i}"), (i + 1) * 100, (i + 1) * 500);
            std::fs::write(&path, serde_json::to_string(&trace).unwrap()).unwrap();
        }

        let traces = collect_traces_from_dir(tmp.path()).unwrap();
        assert_eq!(traces.len(), 3);
    }

    #[test]
    fn test_run_cost_with_dir() {
        let tmp = TempDir::new().unwrap();
        let trace = make_trace("bot", 50, 200);
        std::fs::write(
            tmp.path().join("run.json"),
            serde_json::to_string(&trace).unwrap(),
        )
        .unwrap();

        let code = run_cost(&[tmp.path().to_path_buf()]);
        assert_eq!(code, 0);
    }

    #[test]
    fn test_run_cost_empty_returns_error() {
        let tmp = TempDir::new().unwrap();
        let code = run_cost(&[tmp.path().to_path_buf()]);
        assert_eq!(code, 1);
    }
}
