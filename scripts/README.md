# Rein Dev Scripts

Automation for the Rein development workflow. Used by Del (OpenClaw) to orchestrate Claude Code + sub-agent reviewers.

## The Pipeline

```
rein-task.sh → rein-pr.sh → rein-review.sh → rein-fix.sh → rein-merge.sh
```

### 1. `rein-task.sh <issue-number>`
Picks up a GitHub issue, creates a branch, launches Claude Code to build it with TDD.

### 2. `rein-pr.sh <issue-number>`
Runs tests, pushes the branch, creates a PR linked to the issue.

### 3. `rein-review.sh <pr-number>`
Pulls PR details + diff for the reviewer sub-agent.

### 4. `rein-fix.sh <review-file>`
Launches Claude Code to address review feedback. Takes a markdown file with the review.

### 5. `rein-merge.sh <pr-number>`
Squash-merges an approved PR and cleans up the branch.

## Del's Orchestration Flow

When I (Del) work a Rein task, this is the sequence:

1. `exec: ./scripts/rein-task.sh 8` → Claude Code builds it
2. `exec: ./scripts/rein-pr.sh 8` → PR opens
3. `exec: ./scripts/rein-review.sh 1` → get diff
4. `sessions_spawn` → reviewer sub-agent evaluates (Does it work? SOLID? Production-ready?)
5. If feedback: `exec: ./scripts/rein-fix.sh review.md` → Claude Code fixes
6. Re-review until approved
7. `exec: ./scripts/rein-merge.sh 1` → merge + cleanup
