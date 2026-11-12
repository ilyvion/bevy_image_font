#!/usr/bin/env bash
# Prints the body of a version's changelog section, including its link reference definitions.
#
# Usage: release_notes.sh <version>
#
# Environment:
#   CHANGELOG  changelog path (default: CHANGELOG.md)
set -euo pipefail

fail() {
    echo "::error::$*" >&2
    exit 1
}

[[ $# -eq 1 ]] || fail "Usage: $0 <version>"
version="$1"
changelog="${CHANGELOG:-CHANGELOG.md}"

# The section ends at the next `## ` heading, or, for the oldest section, at the version compare
# link definitions at the bottom of the file. Leading and trailing blank lines are trimmed.
notes=$(awk -v heading="## [$version]" '
    !inside && index($0, heading) == 1 && (length($0) == length(heading) || substr($0, length(heading) + 1, 1) == " ") {
        inside = 1
        next
    }
    inside && /^## / { exit }
    inside && /^\[/ && substr($0, 2, 1) != "#" && ($0 ~ /\/compare\// || $0 ~ /\/releases\/tag\//) { exit }
    inside {
        lines[++n] = $0
        if ($0 ~ /[^ \t]/) { if (!first) first = n; last = n }
    }
    END { for (i = first; first && i <= last; i++) print lines[i] }
' "$changelog")

[[ -n "$notes" ]] || fail "No '## [$version]' section with content found in $changelog."
printf '%s\n' "$notes"
