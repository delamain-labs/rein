#!/bin/bash
# rein-review: Get PR diff and output it for the review sub-agent
# Usage: ./scripts/rein-review.sh <pr-number>
# (Meant to be called by Del/OpenClaw to feed into a sub-agent reviewer)

set -e

REPO="delamain-labs/rein"
PR_NUM=$1

if [ -z "$PR_NUM" ]; then
  echo "Usage: ./scripts/rein-review.sh <pr-number>"
  echo ""
  echo "Open PRs:"
  gh pr list -R "$REPO"
  exit 1
fi

echo "=== PR #$PR_NUM ==="
gh pr view "$PR_NUM" -R "$REPO"
echo ""
echo "=== DIFF ==="
gh pr diff "$PR_NUM" -R "$REPO"
echo ""
echo "=== CHANGED FILES ==="
gh pr diff "$PR_NUM" -R "$REPO" --name-only
