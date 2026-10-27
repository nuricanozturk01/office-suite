#!/usr/bin/env bash
# Usage: scripts/vendor-import.sh <repo> <sha> <crate>...
# Copies engine crates from github.com/storytold/<repo> at <sha> into vendor/<repo> (ADR 0001).
# Env: VENDOR_DEST (default: vendor), VENDOR_SRC (use a local clone instead of cloning).
set -euo pipefail

if [ "$#" -lt 3 ]; then
    echo "usage: $0 <repo> <sha> <crate>..." >&2
    exit 2
fi
repo=$1
sha=$2
shift 2
crates=("$@")
dest_root=${VENDOR_DEST:-vendor}
dest="$dest_root/$repo"

name_re='^[a-z0-9][a-z0-9-]*$'
for n in "$repo" "${crates[@]}"; do
    if ! [[ $n =~ $name_re ]]; then
        echo "error: invalid name: '$n' (allowed: $name_re)" >&2
        exit 2
    fi
done
if ! [[ $sha =~ ^[0-9a-f]{40}$ ]]; then
    echo "error: sha must be 40 hex chars" >&2
    exit 2
fi

work=
cleanup() { [ -n "$work" ] && rm -rf "$work"; return 0; }
trap cleanup EXIT

if [ -n "${VENDOR_SRC:-}" ]; then
    src=$VENDOR_SRC
else
    work=$(mktemp -d)
    src=$work/src
    git clone -q "https://github.com/storytold/$repo" "$src"
    git -C "$src" checkout -q "$sha"
fi
if [ -n "$(git -C "$src" status --porcelain --ignored -- crates Cargo.toml LICENSE-MIT LICENSE-APACHE NOTICE)" ]; then
    echo "error: source tree has local changes; vendoring must match the pinned sha" >&2
    exit 1
fi
head=$(git -C "$src" rev-parse HEAD)
if [ "$head" != "$sha" ]; then
    echo "error: source HEAD $head != $sha" >&2
    exit 1
fi

# Every intra-repo crate a requested crate names must be requested too.
missing=()
for c in "${crates[@]}"; do
    [ -d "$src/crates/$c" ] || { echo "error: no such crate: $c" >&2; exit 1; }
    # Dev-dependencies are dropped from the copy below, so they need not be vendored.
    for dep in $(sed '/^\[dev-dependencies\]/,$d' "$src/crates/$c/Cargo.toml" | grep -oE "^$repo-[a-z0-9-]+" | sort -u); do
        line=$(grep -E "^$dep = " "$src/Cargo.toml" || true)
        dir=$(sed -nE 's/.*path = "crates\/([^"]+)".*/\1/p' <<<"$line")
        [ -n "$dir" ] || continue
        case " ${crates[*]} " in *" $dir "*) ;; *) missing+=("$dir (needed by $c)") ;; esac
    done
done
if [ "${#missing[@]}" -gt 0 ]; then
    printf 'error: add these crates: %s\n' "${missing[@]}" >&2
    exit 1
fi

rm -rf "$dest"
mkdir -p "$dest/crates"
for f in LICENSE-MIT LICENSE-APACHE NOTICE; do cp "$src/$f" "$dest/$f"; done
# Upstream formatting config, so `cargo fmt` (which also visits path dependencies) leaves the code as is.
[ -f "$src/rustfmt.toml" ] && cp "$src/rustfmt.toml" "$dest/rustfmt.toml"
for c in "${crates[@]}"; do cp -R "$src/crates/$c" "$dest/crates/$c"; done

# Test-only dependencies of vendored crates are not built here; dropping them keeps the nested
# workspace small (manifest-only change, no source edits).
for c in "${crates[@]}"; do
    sed -i -E '/^\[(target\.[^]]*\.)?dev-dependencies\]/,/^\[/{/^\[(target\.[^]]*\.)?dev-dependencies\]/d;/^\[/!d}' "$dest/crates/$c/Cargo.toml"
done

# Only the nested workspace root is adjusted; crate sources stay byte-identical.
sed -E -e 's|^members = .*|members = ["crates/*"]|' -e '/^default-members/d' "$src/Cargo.toml" > "$dest/Cargo.toml"

list=$(printf '"%s", ' "${crates[@]}")
cat > "$dest/VENDOR.toml" <<TOML
upstream = "https://github.com/storytold/$repo"
sha = "$sha"
crates = [${list%, }]
note = "root Cargo.toml: members rewritten to crates/*, default-members removed; dev-dependencies sections removed from crate manifests"
TOML
# The nested workspace must resolve, and its dependency licenses must be permitted (ADR 0001).
cargo metadata --manifest-path "$dest/Cargo.toml" --format-version 1 >/dev/null
cargo deny --manifest-path "$dest/Cargo.toml" --config "$(dirname "$0")/../deny.toml" check licenses sources
echo "vendored $repo@$sha: ${crates[*]}"
