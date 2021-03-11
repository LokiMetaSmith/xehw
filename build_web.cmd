
set RUSTFLAGS=--cfg=web_sys_unstable_apis

del docs/xehw_bg.wasm

@echo "Building rust…"
cargo build --release -p xehw --lib --target wasm32-unknown-unknown

@echo "Generating JS bindings for wasm"
wasm-bindgen "target/wasm32-unknown-unknown/release/xehw.wasm" --out-dir docs --no-modules --no-typescript
