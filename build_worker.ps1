cargo build -p takumi-worker --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/takumi_worker.wasm --out-dir ./workers/takumi_worker --target no-modules