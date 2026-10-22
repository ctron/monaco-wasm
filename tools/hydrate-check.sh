#!/bin/sh
# Builds the cargo-leptos SSR + hydration fixture (examples/leptos-hydrate),
# serves it, and runs browser-tests/hydrate.mjs against it.
#
#   tools/hydrate-check.sh [--release]
#
# Needs cargo-leptos, plus Node and Chrome for the check (set CHROMIUM to use
# a system browser). The fixture listens on 127.0.0.1:3400.
set -eu

repo=$(cd "$(dirname "$0")/.." && pwd)
log=$(mktemp "${TMPDIR:-/tmp}/leptos-hydrate.XXXXXX")
server=""
cleanup() {
    if [ -n "$server" ]; then
        kill "$server" 2>/dev/null || true
        wait "$server" 2>/dev/null || true
    fi
    rm -f "$log"
}
trap cleanup EXIT

cd "$repo/examples/leptos-hydrate"
cargo leptos build "$@"
cargo leptos serve "$@" >"$log" 2>&1 &
server=$!

for _ in $(seq 1 120); do
    if curl -sf http://127.0.0.1:3400/ | grep -q "monaco-leptos hydration"; then
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
node hydrate.mjs http://127.0.0.1:3400/
