#!/usr/bin/env bash
# Validate .rein code blocks in markdown files that are complete, standalone files.
# Skips snippet fragments (blocks that don't start with a top-level keyword or comment).
# Usage: ./scripts/validate-docs.sh

set -euo pipefail

REIN="${REIN:-rein}"
FAIL=0
TOTAL=0
SKIPPED=0

TOP_LEVEL_RE='^(//|#|/\*|agent |workflow |provider |defaults |tool |archetype |policy |circuit_breaker |observe |fleet |channel |eval |consensus |secrets |secret |scenario |escalate |schedule |memory |import |type )'

# Validate complete files only. Language reference contains intentional fragments.
for md in README.md docs/getting-started.md; do
    [ -f "$md" ] || continue

    blocks=$(python3 -c "
import re
content = open('$md').read()
blocks = re.findall(r'\`\`\`rein\n(.*?)\`\`\`', content, re.DOTALL)
for i, b in enumerate(blocks):
    path = f'/tmp/rein_doc_block_{i}.rein'
    open(path, 'w').write(b)
    print(path)
" 2>/dev/null || true)

    block_idx=0
    for block in $blocks; do
        first_line=$(head -1 "$block" | sed 's/^[[:space:]]*//')
        if echo "$first_line" | grep -qE "$TOP_LEVEL_RE"; then
            TOTAL=$((TOTAL + 1))
            if ! $REIN validate "$block" > /dev/null 2>&1; then
                echo "FAIL: $md block $block_idx"
                $REIN validate "$block" 2>&1 | head -10
                echo ""
                FAIL=$((FAIL + 1))
            fi
        else
            SKIPPED=$((SKIPPED + 1))
        fi
        block_idx=$((block_idx + 1))
    done
done

if [ "$FAIL" -gt 0 ]; then
    echo "❌ $FAIL of $TOTAL complete blocks failed validation ($SKIPPED snippets skipped)"
    exit 1
else
    echo "✅ All $TOTAL complete blocks valid ($SKIPPED snippets skipped)"
fi
