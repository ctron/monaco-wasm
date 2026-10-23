#!/bin/sh
# Builds the GitHub Pages site: the landing page at the root and every example
# in examples/pages/examples.json that has a build, each under its id. Needs
# jq, Trunk, and wasm-pack.
#
#   tools/build-pages.sh [--example <id> | --landing] <output dir> [base path] [repository URL]
#
# Without an option, the output directory is replaced with the whole site.
# --example builds only that example into <output dir>/<id>, and --landing
# writes only the landing page, next to examples that are already there; the
# Pages workflow builds the examples in parallel this way.
#
# The base path is where the site is served from, for example /monaco-yew/
# for a project page (default: /). The repository URL is linked from the
# landing page.
set -eu

usage() {
    echo "usage: $0 [--example <id> | --landing] <output dir> [base path] [repository URL]" >&2
    exit 2
}

only=""
landing=true
case "${1-}" in
    --example)
        [ $# -ge 2 ] || usage
        only=$2
        landing=false
        shift 2
        ;;
    --landing)
        only=none
        shift
        ;;
esac
[ $# -ge 1 ] || usage

repo=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$1"
out=$(cd "$1" && pwd)
base=${2:-/}
# Normalize to exactly one leading and one trailing slash.
base="/$(printf '%s' "$base" | sed 's#^/*##; s#/*$##')/"
base=$(printf '%s' "$base" | sed 's#^//$#/#')
repository=${3:-https://github.com/}

if [ -z "$only" ]; then
    rm -rf "$out"
    mkdir -p "$out"
fi

pages="$repo/examples/pages"
# Assigned first, so a jq failure stops the script.
builds=$(jq -r --arg only "$only" \
    '.examples[] | select(.build and ($only == "" or .id == $only)) | "\(.id) \(.source) \(.build)"' \
    "$pages/examples.json")
if [ -n "$only" ] && [ "$only" != none ] && [ -z "$builds" ]; then
    echo "no example '$only' with a build in examples.json" >&2
    exit 1
fi
[ -z "$builds" ] || printf '%s\n' "$builds" |
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
        rm -f "$out/$id/.trunk-lock" "$out/$id/pkg/.gitignore"
    done

if [ "$landing" = true ]; then
    cp "$pages/style.css" "$pages/examples.js" "$pages/examples.json" "$out/"
    sed "s#__REPOSITORY_URL__#$repository#g" "$pages/index.html" >"$out/index.html"
fi

echo "built $out for $base"
