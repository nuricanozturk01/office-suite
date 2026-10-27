#!/usr/bin/env bash
# Usage: scripts/rebrand.sh <app> <source-dir>
# Rewrites user-visible product names in an upstream checkout (run by build-embeds.sh after
# the patch is applied) and pins the interface language to English. See ADR 0003.
#   WordCraft -> Docs, GridCraft -> Sheets, DeckCraft -> Slides, PdfCraft -> PDF
# It also injects scripts/embed-patches/host-prelude.js (English, app theme) into index.html.
# Only text inside string literals changes: identifiers, env vars (WORDCRAFT_*), the
# `.deckcraft` extension and the translation catalogs are left alone.
set -euo pipefail

app=$1
src=$2

case $app in
    wordcraft) name=Docs ;;
    gridcraft) name=Sheets ;;
    deckcraft) name=Slides ;;
    pdfcraft) name=PDF ;;
    *) echo "unknown app: $app" >&2; exit 2 ;;
esac

dirs=()
for d in "$src/crates/ui-egui/src" "$src/crates/engine/src" "$src/apps/$app-web/src"; do
    [ -d "$d" ] && dirs+=("$d")
done

find "${dirs[@]}" -name '*.rs' ! -path '*/i18n/*' ! -name '*tests*.rs' ! -name 'tests.rs' -print0 |
    xargs -0 perl -i -pe '
        BEGIN { $n = shift @ARGV }
        next if /^\s*\/\//;
        s{"(?:[^"\\]|\\.)*"}{
            my $s = $&;
            # "DeckCraft" is also the name of the built-in theme, looked up by name.
            unless ($s eq q("DeckCraft") && /theme|scheme|album/i) {
                $s =~ s/ArtCraft Team and the //g;
                $s =~ s/ArtCraft Team//g;
                $s =~ s/\b(?:Word|Grid|Deck|Pdf)Craft\b(?![\w.])/$n/g;
            }
            $s
        }ge;
    ' "$name"

# Before the editor starts: English only, and the app's theme choice answers `prefers-color-scheme`.
html=$src/apps/$app-web/index.html
if ! grep -q 'office-suite-prelude' "$html"; then
    prelude=$(dirname "$0")/embed-patches/host-prelude.js
    PRELUDE=$(cat "$prelude") perl -0 -i -pe 's|(<head>)|$1\n  <script>/* office-suite-prelude */\n$ENV{PRELUDE}\n  </script>|' "$html"
fi
