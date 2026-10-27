#!/usr/bin/env bash
# Usage: scripts/upgrade-upstream.sh <repo> [<sha>] [--dry-run] [--no-verify] [--full]
#                                              [--skip-vendor] [--skip-embed]
# Moves both pins of one reference repo (wordcraft, gridcraft, deckcraft or pdfcraft) to <sha>
# (default: upstream HEAD) and checks the result:
#   1. vendor/<repo>   re-imported with the crate list in its VENDOR.toml, root NOTICE updated
#   2. editor build    pin in scripts/build-embeds.sh moved; the patch in scripts/embed-patches
#                      is re-applied, rebased with a 3-way merge when it no longer applies, and
#                      the editor is built with trunk
#   3. verification    `just vendor-check`, the adapter tests of the engine, and with --full
#                      `just check`
# Nothing is committed. Any failure puts the files back as they were (a copy is kept in
# target/upgrade/); a patch that conflicts is left for you to resolve in target/embeds/<repo>.
# See docs/runbooks/upstream-updates.md.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

repo=""
new_sha=""
dry_run=false
verify=true
full=false
do_vendor=true
do_embed=true
for arg in "$@"; do
    case $arg in
        --dry-run) dry_run=true ;;
        --no-verify) verify=false ;;
        --full) full=true ;;
        --skip-vendor) do_vendor=false ;;
        --skip-embed) do_embed=false ;;
        -*) echo "unknown option: $arg" >&2; exit 2 ;;
        *)
            if [ -z "$repo" ]; then repo=$arg
            elif [ -z "$new_sha" ]; then new_sha=$arg
            else echo "too many arguments" >&2; exit 2
            fi
            ;;
    esac
done

case $repo in
    wordcraft) adapter=documents-engine ;;
    gridcraft) adapter=spreadsheets-engine ;;
    deckcraft) adapter=presentations-engine ;;
    pdfcraft) adapter="" ;;
    *) echo "usage: $0 <wordcraft|gridcraft|deckcraft|pdfcraft> [<sha>] [options]" >&2; exit 2 ;;
esac

if [ -z "$new_sha" ]; then
    new_sha=$(git ls-remote "https://github.com/storytold/$repo" HEAD | cut -c1-40)
fi
[[ $new_sha =~ ^[0-9a-f]{40}$ ]] || { echo "error: need a full 40-character sha (got '$new_sha')" >&2; exit 2; }

has_vendor=false
[ -f "vendor/$repo/VENDOR.toml" ] && has_vendor=true
$has_vendor || do_vendor=false

old_embed=$(sed -n "s/^ *\[$repo\]=\([0-9a-f]\{40\}\).*/\1/p" scripts/build-embeds.sh)
old_vendor=""
$has_vendor && old_vendor=$(sed -n 's/^sha = "\([0-9a-f]\{40\}\)".*/\1/p' "vendor/$repo/VENDOR.toml")
[ -n "$old_embed" ] || { echo "error: no pin for $repo in scripts/build-embeds.sh" >&2; exit 1; }

echo "== $repo -> ${new_sha:0:9}"
$do_vendor && echo "   engine crates: ${old_vendor:0:9}"
$do_embed && echo "   editor:        ${old_embed:0:9}"
if { ! $do_vendor || [ "$old_vendor" = "$new_sha" ]; } && { ! $do_embed || [ "$old_embed" = "$new_sha" ]; }; then
    echo "already current"
    exit 0
fi

clone=$root/target/embeds/$repo
if [ -d "$clone/.git" ]; then
    git -C "$clone" fetch -q origin || true
    if git -C "$clone" cat-file -e "$old_embed^{commit}" 2>/dev/null \
        && git -C "$clone" cat-file -e "$new_sha^{commit}" 2>/dev/null; then
        echo "-- upstream changes (newest first, at most 40):"
        git -C "$clone" log --oneline -40 "$old_embed..$new_sha" || true
    fi
fi
$dry_run && { echo "dry run: nothing changed"; exit 0; }

stamp=$(date +%Y%m%d-%H%M%S)
backup=$root/target/upgrade/$repo-$stamp
mkdir -p "$backup"
cp scripts/build-embeds.sh "$backup/build-embeds.sh"
cp NOTICE "$backup/NOTICE"
[ -f "scripts/embed-patches/$repo.patch" ] && cp "scripts/embed-patches/$repo.patch" "$backup/$repo.patch"
$has_vendor && cp -a "vendor/$repo" "$backup/vendor-$repo"

finished=false
rollback() {
    $finished && return 0
    echo "!! failed: putting the pins and files back (copy kept in $backup)" >&2
    cp "$backup/build-embeds.sh" scripts/build-embeds.sh
    cp "$backup/NOTICE" NOTICE
    [ -f "$backup/$repo.patch" ] && cp "$backup/$repo.patch" "scripts/embed-patches/$repo.patch"
    if $has_vendor && [ -d "$backup/vendor-$repo" ]; then
        rm -rf "vendor/$repo"
        cp -a "$backup/vendor-$repo" "vendor/$repo"
    fi
}
trap rollback EXIT

if $do_vendor && [ "$old_vendor" != "$new_sha" ]; then
    echo "-- vendoring engine crates"
    crates=$(sed -n 's/^crates = \[\(.*\)\]/\1/p' "vendor/$repo/VENDOR.toml" | tr -d '"' | tr ',' ' ')
    # shellcheck disable=SC2086
    scripts/vendor-import.sh "$repo" "$new_sha" $crates
    sed -i "s/$old_vendor/$new_sha/g" NOTICE
fi

if $do_embed && [ "$old_embed" != "$new_sha" ]; then
    echo "-- moving the editor pin"
    sed -i "s/\[$repo\]=$old_embed/[$repo]=$new_sha/" scripts/build-embeds.sh
    patch=scripts/embed-patches/$repo.patch
    if [ -f "$patch" ]; then
        [ -d "$clone/.git" ] || git clone -q "https://github.com/storytold/$repo" "$clone"
        git -C "$clone" fetch -q origin || true
        git -C "$clone" checkout -q -f "$new_sha"
        git -C "$clone" clean -q -fd
        if git -C "$clone" apply --check "$root/$patch" 2>/dev/null; then
            echo "   patch applies as is"
        else
            echo "   patch no longer applies: rebasing it with a 3-way merge"
            if ! git -C "$clone" apply --3way "$root/$patch"; then
                echo "!! the patch conflicts. Resolve the markers in $clone, then regenerate it:" >&2
                echo "     cd $clone && git add -N . && git diff HEAD -- . ':!target' ':!Cargo.lock' > $root/$patch" >&2
                echo "   and finish with: scripts/build-embeds.sh $repo" >&2
                # The conflicted tree stays for the manual work; the pins stay moved.
                finished=true
                exit 1
            fi
            git -C "$clone" add -N .
            git -C "$clone" diff HEAD -- . ':!target' ':!Cargo.lock' > "$root/$patch.new"
            mv "$root/$patch.new" "$root/$patch"
            echo "   patch rebased and rewritten"
        fi
    fi
    echo "-- building the editor (trunk)"
    scripts/build-embeds.sh "$repo"
fi

if $verify; then
    echo "-- verifying"
    cargo run -q -p xtask -- vendor-check
    if $do_vendor && [ -n "$adapter" ]; then
        SQLX_OFFLINE=true cargo test -q -p "$adapter"
    fi
    if $full; then
        just check
    fi
fi

finished=true
echo
echo "done: $repo is at ${new_sha:0:9}. Nothing is committed."
echo "Next: restart 'just dev-web' (the editor files changed), open the editor and try"
echo "open, edit + autosave, rename, share, and that the view stays put after a remote save."
echo "Commit as: chore(vendor,web): update $repo to ${new_sha:0:9}  (put <old>..<new> in the body)"
