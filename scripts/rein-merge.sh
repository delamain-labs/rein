#!/bin/bash
# rein-merge: Merge an approved PR and clean up
# Usage: ./scripts/rein-merge.sh <pr-number>

set -e

REPO="delamain-labs/rein"
PR_NUM=$1

if [ -z "$PR_NUM" ]; then
  echo "Usage: ./scripts/rein-merge.sh <pr-number>"
  exit 1
fi

# Merge with squash
echo "🔀 Merging PR #$PR_NUM..."
gh pr merge "$PR_NUM" -R "$REPO" --squash --delete-branch

# Update local
git checkout master
git pull origin master

echo "✅ PR #$PR_NUM merged and branch cleaned up."
