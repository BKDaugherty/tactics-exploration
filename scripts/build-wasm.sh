set -ueox pipefail

cargo build --profile release-wasm --target wasm32-unknown-unknown --features wasm
wasm-bindgen --no-typescript --target web \
    --out-dir ./out/ \
    --out-name "tactics-exploration" \
    ./target/wasm32-unknown-unknown/release-wasm/tactics-exploration.wasm

# Can't seem to install this guy
# wasm-opt -Oz -o .wasm ./out/tactics-exploration ./out/tactics-exploration-unopt
cp -r assets ./out/assets
cp web/index.html ./out/index.html

