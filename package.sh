wasm-pack build                         \
    --out-name wasmito-tools        \
    --target nodejs                     \
    --dev                               \
    --no-opt                            \
    --out-dir ../../pkg                 \
    ./crates/wasmito-tools-bindings
