# Integration Guide

How to use Rein from application code and CI pipelines.

---

## Python

### Subprocess integration

```python
import subprocess
import json
import sys

def run_rein_agent(policy_path: str, message: str) -> dict:
    """Run a Rein agent and return parsed output."""
    result = subprocess.run(
        ["rein", "run", policy_path, "--message", message],
        capture_output=True,
        text=True,
    )

    if result.returncode == 2:
        # Hard abort — policy violation, cyclic dependency, infra failure
        raise RuntimeError(f"rein hard abort: {result.stderr.strip()}")

    if result.returncode == 1:
        # Partial success — some steps failed, workflow ran to completion
        sys.stderr.write(f"rein partial success: {result.stderr.strip()}\n")

    return {
        "exit_code": result.returncode,
        "stdout": result.stdout.strip(),
        "stderr": result.stderr.strip(),
    }

# Example
output = run_rein_agent("agents/assistant.rein", "Summarize this document")
print(output["stdout"])
```

### Parsing `rein validate --format json`

```python
def validate_policy(policy_path: str) -> bool:
    result = subprocess.run(
        ["rein", "validate", "--format", "json", policy_path],
        capture_output=True,
        text=True,
    )
    data = json.loads(result.stdout)
    if not data["valid"]:
        for error in data.get("errors", []):
            print(f"Error at {error['location']}: {error['message']}")
    return data["valid"]
```

---

## Node.js

### Subprocess integration

```javascript
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

async function runReinAgent(policyPath, message) {
  try {
    const { stdout, stderr } = await execFileAsync('rein', [
      'run', policyPath, '--message', message,
    ]);
    return { exitCode: 0, stdout: stdout.trim(), stderr: stderr.trim() };
  } catch (err) {
    // err.code is the exit code when execFile rejects
    if (err.code === 2) {
      throw new Error(`rein hard abort: ${err.stderr}`);
    }
    // exit code 1 = partial success, not a throw
    return {
      exitCode: err.code,
      stdout: err.stdout?.trim() ?? '',
      stderr: err.stderr?.trim() ?? '',
    };
  }
}

// Example
const result = await runReinAgent('agents/assistant.rein', 'Hello');
console.log(result.stdout);
```

---

## REST API (`rein serve`)

Start the server (default port is `3000`):

```bash
rein serve agents/assistant.rein --port 4000
```

### Endpoints

#### `GET /health`

Returns `200 OK` with body `{"status": "ok"}` when the server is running.

#### `GET /api/v1/agents`

Returns a JSON array of agents defined in the loaded `.rein` file.

#### `GET /api/v1/workflows`

Returns a JSON array of workflows defined in the loaded `.rein` file.

#### `GET /api/v1/types`

Returns metadata about supported block types and field names.

#### `GET /api/v1/audit`

Returns the audit log entries recorded during the server's lifetime.

> **Note:** `rein serve` exposes a read-only inspection API over the loaded file.
> Agent execution (`rein run`) is a CLI-only operation — there is no `POST /run`
> HTTP endpoint. Use the subprocess approach above to invoke agents from other services.

---

## CI/CD integration

### GitHub Actions

```yaml
# .github/workflows/rein.yml
name: Validate and Run Rein Policies
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rein
        run: cargo install rein-lang
      - name: Validate policies
        run: rein validate agents/*.rein
      - name: Check formatting
        run: rein fmt --check agents/*.rein

  dry-run:
    runs-on: ubuntu-latest
    needs: validate
    steps:
      - uses: actions/checkout@v4
      - name: Install Rein
        run: cargo install rein-lang
      - name: Dry run
        # --demo uses a mock provider — no API keys required in CI
        run: rein run --demo agents/assistant.rein --message "smoke test"
```

### Exit code handling in shell scripts

```bash
rein run workflow.rein --message "process batch"
EXIT=$?

case $EXIT in
  0) echo "All steps succeeded" ;;
  1) echo "Partial success — check trace for StepFailed events" ;;
  2) echo "Hard abort — run terminated early (policy violation, cyclic dep, infra)" ; exit 1 ;;
esac
```

---

## Error reference

| Condition | Exit code | `rein serve` status |
|-----------|-----------|---------------------|
| All steps succeeded | `0` | `200` |
| Partial success (soft failures) | `1` | `200` with partial output |
| Budget exceeded | `2` | `422` |
| Permission denied | `2` | `422` |
| Provider error | `2` | `500` |
| Policy violation / approval rejected | `2` | `422` |
| Cyclic dependency | `2` | `500` |
