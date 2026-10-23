#!/bin/sh
# Builds the GitHub Pages site: the landing page at the root and every example
# in examples/pages/examples.json that has a build, each under its id. Needs
# jq, Trunk, and wasm-pack.
#
#   tools/build-pages.sh <output dir> [base path] [repository URL]
#
# The base path is where the site is served from, for example /monaco-yew/
# for a project page (default: /). The repository URL is linked from the
# landing page.
set -eu

if [ $# -lt 1 ]; then
    echo "usage: $0 <output dir> [base path] [repository URL]" >&2
    exit 2
fi

repo=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$1"
out=$(cd "$1" && pwd)
base=${2:-/}
# Normalize to exactly one leading and one trailing slash.
base="/$(printf '%s' "$base" | sed 's#^/*##; s#/*$##')/"
base=$(printf '%s' "$base" | sed 's#^//$#/#')
repository=${3:-https://github.com/}

rm -rf "$out"
mkdir -p "$out"

pages="$repo/examples/pages"
# Assigned first, so a jq failure stops the script.
builds=$(jq -r '.examples[] | select(.build) | "\(.id) \(.source) \(.build)"' "$pages/examples.json")
printf '%s\n' "$builds" |
    while read -r id source build; do
        src="$repo/examples/$source"
        case "$build" in
            trunk)
                (cd "$src" && trunk build --release --locked --public-url "${base}${id}/" --dist "$out/$id" </dev/null)
                ;;
            wasm-pack)
                wasm-pack build "$src" --target web --out-dir "$out/$id/pkg" --no-pack -- --locked </dev/null
                cp "$src"/*.html "$src"/*.js "$src"/*.css "$out/$id/"
                ;;
            *)
                echo "unknown build '$build' for $id" >&2
                exit 1
                ;;
        esac
    done

cp "$pages/style.css" "$pages/examples.js" "$pages/examples.json" "$out/"
sed "s#__REPOSITORY_URL__#$repository#g" "$pages/index.html" >"$out/index.html"
rm -f "$out"/*/.trunk-lock "$out"/*/pkg/.gitignore

echo "built $out for $base"
