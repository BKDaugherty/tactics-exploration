set -ueox pipefail

rm -r out dist
mkdir -p out dist

cargo run --bin package_assets
cargo build --bin tactics-exploration --profile release-wasm --target wasm32-unknown-unknown 
wasm-bindgen --no-typescript --target web \
    --out-dir ./out/ \
    --out-name "tactics-exploration" \
    ./target/wasm32-unknown-unknown/release-wasm/tactics-exploration.wasm

# Can't seem to install this guy
# wasm-opt -Oz -o .wasm ./out/tactics-exploration ./out/tactics-exploration-unopt

cp web/index.html ./out/index.html
cat web/fix-audio.js ./out/tactics-exploration.js > ./out/tactics-exploration-audio-fix.js
pushd out
mv tactics-exploration-audio-fix.js tactics-exploration.js
zip -vr tactics-exploration.zip * -x "*.DS_Store"
popd
mv out/tactics-exploration.zip dist/

# Open a Firefox window to edit the Game on itch.io
open -a Firefox "https://itch.io/game/edit/4146661"

