#!/bin/bash
# rein-fix: Launch Claude Code to address PR review feedback
# Usage: ./scripts/rein-fix.sh <review-file-path>

set -e

REVIEW_FILE=$1

if [ -z "$REVIEW_FILE" ]; then
  echo "Usage: ./scripts/rein-fix.sh <path-to-review-markdown>"
  exit 1
fi

if [ ! -f "$REVIEW_FILE" ]; then
  echo "Error: Review file not found: $REVIEW_FILE"
  exit 1
fi

REVIEW_CONTENT=$(cat "$REVIEW_FILE")
BRANCH=$(git branch --show-current)

echo "🔧 Fixing review feedback on branch: $BRANCH"
echo ""

PROMPT="You're on branch '$BRANCH' in the Rein project. A code review has been completed. Read CLAUDE.md first.

Address ALL review feedback below. For each item:
1. Fix the issue
2. Add/update tests if needed  
3. Run cargo test
4. Commit with 'fix: address review - <what you fixed>'

REVIEW FEEDBACK:
$REVIEW_CONTENT

When completely finished, run: openclaw system event --text 'Done: Review feedback addressed on $BRANCH' --mode now"

claude "$PROMPT" --allowedTools "Bash(git*),Bash(cargo*),Bash(mkdir*),Bash(cat*),Bash(echo*),Bash(openclaw*),Read,Write,Edit,Bash(rustc*)"
