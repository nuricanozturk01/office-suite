#!/usr/bin/env bash
# Usage: scripts/build-embeds.sh [wordcraft|gridcraft|deckcraft|pdfcraft]...
# Builds the browser apps of the reference repos (storytold/*craft) at pinned commits and
# copies them to apps/web/public/apps/<name> (git-ignored). The Vue pages embed them in an
# <iframe> with `?host=parent` and store the files through /api/v1/packages.
# Needs: rustup target wasm32-unknown-unknown, trunk (cargo install trunk --locked), git.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cache=${EMBED_CACHE:-$root/target/embeds}
out=$root/apps/web/public/apps

declare -A SHA=(
    [wordcraft]=2c99a427803364354181d0bc20865f05880605f7
    [gridcraft]=4eb2fb5c8ed30a580acf24234fffb2cb6256e335
    [deckcraft]=aaf6c1c38aa6c59beda8110bd014cc6c96c6ec74
    [pdfcraft]=01b696df2c79636ff6ea737e6b02eff634d99f6c
)

apps=("$@")
# pdfcraft is pinned one commit before its craft-menu switch: that craft-libs revision is not published.
[ "${#apps[@]}" -eq 0 ] && apps=(wordcraft gridcraft deckcraft pdfcraft)

for app in "${apps[@]}"; do
    sha=${SHA[$app]:-}
    [ -n "$sha" ] || { echo "unknown app: $app" >&2; exit 2; }
    src=$cache/$app
    if [ ! -d "$src/.git" ]; then
        mkdir -p "$cache"
        git clone -q "https://github.com/storytold/$app" "$src"
    fi
    git -C "$src" fetch -q origin || true
    git -C "$src" checkout -q -f "$sha"
    git -C "$src" clean -q -fd
    patch=$root/scripts/embed-patches/$app.patch
    if [ -f "$patch" ]; then
        git -C "$src" apply "$patch"
    fi
    "$root/scripts/rebrand.sh" "$app" "$src"
    (cd "$src/apps/$app-web" && trunk build --release --dist "$out/$app" --public-url "./")
    echo "built $app@$sha -> $out/$app"
done
