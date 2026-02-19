#!/bin/bash
# rein-pr: Push current branch and open a PR
# Usage: ./scripts/rein-pr.sh <issue-number>

set -e

REPO="delamain-labs/rein"
ISSUE_NUM=$1
BRANCH=$(git branch --show-current)

if [ -z "$ISSUE_NUM" ]; then
  echo "Usage: ./scripts/rein-pr.sh <issue-number>"
  exit 1
fi

if [ "$BRANCH" = "master" ]; then
  echo "Error: Can't create PR from master"
  exit 1
fi

# Run tests first
echo "🧪 Running tests..."
cargo test --quiet
echo "✅ Tests pass"
echo ""

# Push branch
echo "⬆️  Pushing $BRANCH..."
git push -u origin "$BRANCH"

# Get issue title for PR
ISSUE_TITLE=$(gh issue view "$ISSUE_NUM" -R "$REPO" --json title -q .title)

# Get commit messages for PR body
COMMITS=$(git log master.."$BRANCH" --oneline)

# Create PR
echo "📝 Creating PR..."
gh pr create -R "$REPO" \
  --base master \
  --head "$BRANCH" \
  --title "$ISSUE_TITLE" \
  --body "## Summary
Addresses #$ISSUE_NUM

## Changes
$COMMITS

## Checklist
- [ ] Tests pass (\`cargo test\`)
- [ ] Follows SOLID principles
- [ ] Production-ready code quality"

echo ""
echo "✅ PR created. Ready for review."
