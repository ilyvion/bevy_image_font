#!/usr/bin/env bash
# Moves the changelog's Unreleased entries into a new release section and updates the compare links.
#
# Usage: prepare_changelog.sh <new-version>
#
# Must run before Cargo.toml is bumped: the previous version is read from Cargo.toml.
# Only the `## [Unreleased]` heading and the `[unreleased]:` link definition are located and
# rewritten; all other content, including other link reference definitions, is left untouched.
#
# Environment:
#   CHANGELOG          changelog path (default: CHANGELOG.md)
#   RELEASE_DATE       release date (default: today, UTC)
#   GITHUB_REPOSITORY  owner/repo (default: derived from the origin remote)
set -euo pipefail

fail() {
    echo "::error::$*" >&2
    exit 1
}

[[ $# -eq 1 ]] || fail "Usage: $0 <new-version>"
new_version="$1"
changelog="${CHANGELOG:-CHANGELOG.md}"
release_date="${RELEASE_DATE:-$(date -u +%Y-%m-%d)}"

[[ "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] ||
    fail "'$new_version' is not a valid version (expected e.g. 1.2.3 or 1.2.3-rc.1, without a 'v' prefix)."
[[ -f "$changelog" ]] || fail "$changelog not found."

old_version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
[[ -n "$old_version" && "$old_version" != "null" ]] || fail "Could not read the current version from Cargo.toml."
[[ "$new_version" != "$old_version" ]] || fail "Cargo.toml is already at $new_version."
[[ "$(printf '%s\n%s\n' "$old_version" "$new_version" | sort -V | tail -n1)" == "$new_version" ]] ||
    fail "$new_version is not newer than the current version $old_version."

if [[ -z "${GITHUB_REPOSITORY:-}" ]]; then
    origin_url=$(git remote get-url origin)
    GITHUB_REPOSITORY="${origin_url#git@github.com:}"
    GITHUB_REPOSITORY="${GITHUB_REPOSITORY#https://github.com/}"
    GITHUB_REPOSITORY="${GITHUB_REPOSITORY%.git}"
fi
repo_url="https://github.com/$GITHUB_REPOSITORY"

old_tag="v$old_version"
new_tag="v$new_version"

if [[ -n "$(git ls-remote --tags origin "refs/tags/$new_tag")" ]]; then
    fail "Tag $new_tag already exists on origin."
fi
if [[ -z "$(git ls-remote --tags origin "refs/tags/$old_tag")" ]]; then
    fail "Tag $old_tag for the current version $old_version does not exist on origin, so the new compare link would be broken."
fi

new_version_re="${new_version//./\\.}"

unreleased_heading_count=$(grep -ciE '^## \[unreleased\][[:space:]]*$' "$changelog" || true)
[[ "$unreleased_heading_count" -eq 1 ]] ||
    fail "Expected exactly one '## [Unreleased]' heading in $changelog, found $unreleased_heading_count."

unreleased_links=$(grep -iE '^\[unreleased\]:' "$changelog" || true)
[[ -n "$unreleased_links" && $(wc -l <<<"$unreleased_links") -eq 1 ]] ||
    fail "Expected exactly one '[unreleased]:' link definition in $changelog."
expected_unreleased_url="$repo_url/compare/$old_tag...HEAD"
actual_unreleased_url=$(sed -E 's/^\[[^]]*\]:[[:space:]]*//; s/[[:space:]]+$//' <<<"$unreleased_links")
[[ "$actual_unreleased_url" == "$expected_unreleased_url" ]] ||
    fail "The [unreleased] link is '$actual_unreleased_url', expected '$expected_unreleased_url'."

! grep -qE "^## \[$new_version_re\]" "$changelog" || fail "$changelog already has a '## [$new_version]' section."
! grep -qE "^\[$new_version_re\]:" "$changelog" || fail "$changelog already has a '[$new_version]:' link definition."

unreleased_entries=$(awk '
    tolower($0) ~ /^## \[unreleased\]/ { inside = 1; next }
    inside && /^## / { exit }
    inside && /^[ \t]*[-*+] / { count++ }
    END { print count + 0 }
' "$changelog")
[[ "$unreleased_entries" -gt 0 ]] || fail "The Unreleased section of $changelog has no entries; there is nothing to release."

tmp=$(mktemp)
awk \
    -v version="$new_version" \
    -v date="$release_date" \
    -v unreleased_url="$repo_url/compare/$new_tag...HEAD" \
    -v version_url="$repo_url/compare/$old_tag...$new_tag" '
    tolower($0) ~ /^## \[unreleased\]/ {
        print
        print ""
        print "## [" version "] - " date
        next
    }
    tolower($0) ~ /^\[unreleased\]:/ {
        label = substr($0, 2, index($0, "]") - 2)
        print "[" label "]: " unreleased_url
        print "[" version "]: " version_url
        next
    }
    { print }
' "$changelog" >"$tmp"
cat "$tmp" >"$changelog"
rm -f "$tmp"

echo "Prepared $changelog for $new_version ($release_date): $old_tag → $new_tag."
