#!/bin/bash
# rein-task: Pick up a GitHub issue, create branch, launch Claude Code
# Usage: ./scripts/rein-task.sh <issue-number> [additional-instructions]

set -e

REPO="delamain-labs/rein"
ISSUE_NUM=$1
EXTRA_INSTRUCTIONS="${@:2}"

if [ -z "$ISSUE_NUM" ]; then
  echo "Usage: ./scripts/rein-task.sh <issue-number> [additional instructions]"
  echo ""
  echo "Available issues:"
  gh issue list -R "$REPO" --state open
  exit 1
fi

# Fetch issue details
ISSUE_TITLE=$(gh issue view "$ISSUE_NUM" -R "$REPO" --json title -q .title)
ISSUE_BODY=$(gh issue view "$ISSUE_NUM" -R "$REPO" --json body -q .body)

if [ -z "$ISSUE_TITLE" ]; then
  echo "Error: Issue #$ISSUE_NUM not found"
  exit 1
fi

# Create branch name from issue title
BRANCH_NAME=$(echo "$ISSUE_TITLE" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g' | sed 's/^-//' | sed 's/-$//' | cut -c1-50)
BRANCH_NAME="issue-${ISSUE_NUM}/${BRANCH_NAME}"

echo "📋 Issue #$ISSUE_NUM: $ISSUE_TITLE"
echo "🌿 Branch: $BRANCH_NAME"
echo ""

# Create and checkout branch
git checkout master
git pull origin master 2>/dev/null || true
git checkout -b "$BRANCH_NAME"

echo "🚀 Launching Claude Code..."
echo ""

# Build the prompt
PROMPT="You're working on issue #$ISSUE_NUM for the Rein project.

ISSUE TITLE: $ISSUE_TITLE

ISSUE BODY:
$ISSUE_BODY

Read CLAUDE.md first for project rules. Follow TDD: write tests first, then implementation. 
Commit after each logical milestone with descriptive messages (feat:, fix:, test:, etc.).
Run cargo test before every commit.

$EXTRA_INSTRUCTIONS

When completely finished, run: openclaw system event --text 'Done: Issue #$ISSUE_NUM - $ISSUE_TITLE' --mode now"

claude "$PROMPT" --allowedTools "Bash(git*),Bash(cargo*),Bash(mkdir*),Bash(cat*),Bash(echo*),Bash(openclaw*),Read,Write,Edit,Bash(rustc*)"
