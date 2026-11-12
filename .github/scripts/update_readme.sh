#!/usr/bin/env bash
# Updates the README's installation snippet and Bevy version compatibility table for a new release.
#
# Usage: update_readme.sh <new-version>
#
# The Bevy version is read from the `bevy_app` dependency in Cargo.toml. If it matches the top row
# of the compatibility table, the new crate version is appended to that row; otherwise a new row is
# inserted above it. Crate versions are listed as major.minor, so patch releases leave the table
# unchanged. Pre-release versions leave the README untouched. The README is formatted with
# Prettier afterwards.
#
# Environment:
#   README  README path (default: README.md)
set -euo pipefail

# We manually set this for consistent formatting, update whenever you are comfortable with possible
# formatting changes.
PRETTIER_VERSION="3.10.1"

fail() {
    echo "::error::$*" >&2
    exit 1
}

[[ $# -eq 1 ]] || fail "Usage: $0 <new-version>"
new_version="$1"
readme="${README:-README.md}"

[[ "$new_version" =~ ^([0-9]+)\.([0-9]+)\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] ||
    fail "'$new_version' is not a valid version (expected e.g. 1.2.3 or 1.2.3-rc.1, without a 'v' prefix)."
if [[ -n "${BASH_REMATCH[3]}" ]]; then
    echo "$new_version is a pre-release; leaving $readme untouched."
    exit 0
fi
crate_version="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
[[ -f "$readme" ]] || fail "$readme not found."

bevy_req=$(cargo metadata --no-deps --format-version 1 |
    jq -r '.packages[0].dependencies[] | select(.name == "bevy_app") | .req')
[[ "$bevy_req" =~ ^\^?([0-9]+\.[0-9]+)(\.[0-9]+)?$ ]] ||
    fail "Could not read a Bevy version from the bevy_app requirement '$bevy_req' in Cargo.toml."
bevy_version="${BASH_REMATCH[1]}"

for crate in bevy bevy_image_font; do
    count=$(grep -cE "^$crate = \"[^\"]*\"$" "$readme" || true)
    [[ "$count" -eq 1 ]] || fail "Expected exactly one '$crate = \"...\"' line in $readme, found $count."
done

header_count=$(grep -cE '^\| *Bevy Version *\| *Crate Version *\|$' "$readme" || true)
[[ "$header_count" -eq 1 ]] ||
    fail "Expected exactly one '| Bevy Version | Crate Version |' table header in $readme, found $header_count."

top_row=$(awk '
    /^\| *Bevy Version *\| *Crate Version *\|$/ { getline; getline; print; exit }
' "$readme")
[[ "$top_row" =~ ^\|\ *([0-9]+\.[0-9]+)\ *\|\ *([0-9., ]+[0-9])\ *\|$ ]] ||
    fail "Could not parse the top row of the compatibility table in $readme: '$top_row'."
top_bevy="${BASH_REMATCH[1]}"
top_crates="${BASH_REMATCH[2]}"

if [[ "$top_bevy" == "$bevy_version" ]]; then
    if grep -qxF "$crate_version" <<<"${top_crates//, /$'\n'}"; then
        table_action="already lists $crate_version for Bevy $bevy_version"
        new_row=""
    else
        table_action="added $crate_version to the Bevy $bevy_version row"
        new_row="| $bevy_version | $top_crates, $crate_version |"
    fi
else
    [[ "$(printf '%s\n%s\n' "$top_bevy" "$bevy_version" | sort -V | tail -n1)" == "$bevy_version" ]] ||
        fail "Cargo.toml's Bevy version $bevy_version is older than the table's newest entry $top_bevy."
    table_action="added a row for Bevy $bevy_version"
    new_row="| $bevy_version | $crate_version |"
fi

tmp=$(mktemp)
awk \
    -v bevy_version="$bevy_version" \
    -v crate_version="$crate_version" \
    -v new_row="$new_row" \
    -v same_bevy="$([[ "$top_bevy" == "$bevy_version" ]] && echo 1 || echo 0)" '
    /^bevy = "[^"]*"$/ { print "bevy = \"" bevy_version "\""; next }
    /^bevy_image_font = "[^"]*"$/ { print "bevy_image_font = \"" crate_version "\""; next }
    /^\| *Bevy Version *\| *Crate Version *\|$/ { print; table_row = -1; next }
    table_row == -1 { print; table_row = 1; next }
    table_row == 1 {
        table_row = 0
        if (new_row != "") {
            print new_row
            if (same_bevy) next
        }
    }
    { print }
' "$readme" >"$tmp"
cat "$tmp" >"$readme"
rm -f "$tmp"

npx --yes "prettier@$PRETTIER_VERSION" --write "$readme" >/dev/null

echo "Updated $readme: installation snippet uses Bevy $bevy_version and $crate_version; compatibility table $table_action."
