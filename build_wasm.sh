export PATH="$PATH:/home/cody/.cargo/bin"
cargo install wasm-bindgen-cli
cargo build --release --target wasm32-unknown-unknown

rm -rf web/dist/
mkdir web/dist/
cp -r assets/ web/dist/
cp -r web/index.html web/dist/

wasm-bindgen --out-dir web/dist --target web target/wasm32-unknown-unknown/release/gameoflife.wasm
