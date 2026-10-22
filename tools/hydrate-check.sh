#!/bin/sh
# Builds an SSR + hydration fixture, serves it, and runs
# browser-tests/hydrate.mjs against it.
#
#   tools/hydrate-check.sh leptos    examples/leptos-hydrate, cargo-leptos, port 3400
#   tools/hydrate-check.sh dioxus    examples/dioxus-hydrate, dx, port 3410
#
# Builds in release mode. Needs cargo-leptos or the Dioxus CLI (dx), plus Node
# and Chrome for the check (set CHROMIUM to use a system browser).
set -eu

if [ $# -ne 1 ]; then
    echo "usage: $0 leptos|dioxus" >&2
    exit 2
fi

repo=$(cd "$(dirname "$0")/.." && pwd)
log=$(mktemp "${TMPDIR:-/tmp}/hydrate-check.XXXXXX")
server=""
cleanup() {
    if [ -n "$server" ]; then
        kill "$server" 2>/dev/null || true
        wait "$server" 2>/dev/null || true
    fi
    rm -f "$log"
}
trap cleanup EXIT

case "$1" in
    leptos)
        port=3400
        cd "$repo/examples/leptos-hydrate"
        cargo leptos build --release
        cargo leptos serve --release >"$log" 2>&1 &
        server=$!
        ;;
    dioxus)
        port=3410
        cd "$repo/examples/dioxus-hydrate"
        dx build --release --web
        cd "$repo/target/dx/dioxus-hydrate/release/web"
        IP=127.0.0.1 PORT=$port ./server >"$log" 2>&1 &
        server=$!
        ;;
    *)
        echo "unknown fixture '$1', expected leptos or dioxus" >&2
        exit 2
        ;;
esac

for _ in $(seq 1 120); do
    if curl -sf "http://127.0.0.1:$port/" | grep -q "monaco-$1 hydration"; then
        break
    fi
    if ! kill -0 "$server" 2>/dev/null; then
        cat "$log" >&2
        echo "fixture server exited" >&2
        exit 1
    fi
    sleep 1
done

cd "$repo/browser-tests"
[ -d node_modules ] || npm ci
node hydrate.mjs "http://127.0.0.1:$port/"
