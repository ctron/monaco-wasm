#!/bin/sh
# Builds the plain example as a fresh consumer of the packaged monaco-wasm
# crate, with node/npm/npx replaced by failing stubs, then runs the browser
# smoke test against the result.
#
#   tools/consumer-check.sh [--browser /path/to/chromium]
#
# Proves that consumers need only Cargo dependencies: the build uses nothing
# but the extracted .crate contents and never invokes Node. (The smoke test
# itself runs on Node; that is test environment, not consumer environment.)
set -eu

repo=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/monaco-wasm-consumer.XXXXXX")
trap 'rm -rf "$work"' EXIT

real_node=$(command -v node)

cd "$repo"
rm -f target/package/monaco-wasm-*.crate
cargo package -p monaco-wasm --locked --no-verify --allow-dirty
crate=$(ls target/package/monaco-wasm-*.crate)

mkdir -p "$work/vendor" "$work/app/src" "$work/stubs"
tar -xzf "$crate" -C "$work/vendor"
crate_dir=$(ls -d "$work"/vendor/monaco-wasm-*)
cp examples/wasm-basic/src/lib.rs "$work/app/src/"
cp examples/wasm-basic/index.html "$work/app/"
cat > "$work/app/Cargo.toml" <<EOF
[package]
name = "wasm-basic"
version = "0.0.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
monaco-wasm = { path = "$crate_dir" }
wasm-bindgen = "0.2.100"
web-sys = { version = "0.3.77", features = ["Document", "Element", "HtmlElement", "Window", "console"] }

[workspace]
EOF

for tool in node nodejs npm npx; do
    cat > "$work/stubs/$tool" <<EOF
#!/bin/sh
echo "\$0 \$*" >> "$work/stubs/invoked.log"
echo "consumer build must not run $tool" >&2
exit 97
EOF
    chmod +x "$work/stubs/$tool"
done

(
    cd "$work/app"
    PATH="$work/stubs:$PATH" CARGO_TARGET_DIR="$work/target" \
        wasm-pack build --target web --out-dir pkg --no-pack
)
if [ -s "$work/stubs/invoked.log" ]; then
    echo "consumer build invoked Node tooling:" >&2
    cat "$work/stubs/invoked.log" >&2
    exit 1
fi
echo "consumer build did not invoke Node"

cd "$repo/browser-tests"
[ -d node_modules ] || npm ci
"$real_node" smoke.mjs "$work/app" "$@"
