#!/usr/bin/env bash
# Flags any Rust source file over the line-count threshold. Clippy has no lint
# for this — its size-related lints (too_many_lines, cognitive_complexity) are
# all scoped to a single function's body, not a file's aggregate size, so a file
# that accumulates many reasonably-sized functions passes clippy clean while
# still being a module that needs splitting by responsibility (same instinct as
# CLAUDE.md's _impl-suffix rule, triggered by size instead of naming).
set -euo pipefail

threshold=400
failed=0

while IFS= read -r -d '' file; do
    lines=$(wc -l < "$file")
    if [ "$lines" -gt "$threshold" ]; then
        echo "ERROR $file: $lines lines (limit $threshold) — split into a submodule directory by responsibility"
        failed=1
    fi
done < <(find records store cli kernel -name '*.rs' -not -path '*/target/*' -print0 2>/dev/null)

exit "$failed"
