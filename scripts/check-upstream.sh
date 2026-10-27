#!/usr/bin/env bash
# Usage: scripts/check-upstream.sh [--issue]
# Compares the pins (vendor/<repo>/VENDOR.toml and the SHA table in build-embeds.sh) with
# upstream HEAD of storytold/<repo>. Exit 0 when everything is current, 1 when a pin is behind.
# With --issue, opens one GitHub issue (needs `gh`) listing the repos that moved.
# Meant for a weekly CI schedule; see docs/runbooks/upstream-updates.md.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
open_issue=false
[ "${1:-}" = "--issue" ] && open_issue=true

vendor_sha() {
    sed -n 's/^sha = "\([0-9a-f]\{40\}\)".*/\1/p' "$root/vendor/$1/VENDOR.toml" 2>/dev/null || true
}
embed_sha() {
    sed -n "s/^ *\[$1\]=\([0-9a-f]\{40\}\).*/\1/p" "$root/scripts/build-embeds.sh"
}

report=""
for repo in gridcraft wordcraft deckcraft pdfcraft; do
    head=$(git ls-remote "https://github.com/storytold/$repo" HEAD | cut -c1-40)
    [ -n "$head" ] || { echo "cannot reach upstream $repo" >&2; exit 2; }
    for channel in vendor embed; do
        pin=$("${channel}_sha" "$repo")
        [ -n "$pin" ] || continue
        if [ "$pin" = "$head" ]; then
            printf '%-10s %-7s up to date (%s)\n' "$repo" "$channel" "${pin:0:9}"
        else
            line=$(printf '%-10s %-7s pinned %s, upstream %s' "$repo" "$channel" "${pin:0:9}" "${head:0:9}")
            echo "$line"
            report+="- $line"$'\n'
        fi
    done
done

[ -z "$report" ] && exit 0

if $open_issue; then
    gh issue create \
        --title "chore: upstream reference repositories moved" \
        --body "Pins behind upstream HEAD:

$report
Follow docs/runbooks/upstream-updates.md."
fi
exit 1
