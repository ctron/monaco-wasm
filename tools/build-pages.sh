#!/bin/sh
# Builds the GitHub Pages site: the landing page at the root and the Yew and
# Leptos quickstarts under yew/ and leptos/.
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

for framework in yew leptos; do
    (
        cd "$repo/examples/quickstart-$framework"
        trunk build --release --locked --public-url "${base}${framework}/" --dist "$out/$framework"
    )
done

cp "$repo/examples/pages/style.css" "$out/"
sed "s#__REPOSITORY_URL__#$repository#g" "$repo/examples/pages/index.html" >"$out/index.html"
rm -f "$out"/*/.trunk-lock

echo "built $out for $base"
