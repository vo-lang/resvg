module github.com/vo-lang/resvg

vo ^0.1.0

[extension]
name = "resvg"

[extension.native]
path = "rust/target/{profile}/libvo_resvg"

[[extension.native.targets]]
target = "aarch64-apple-darwin"
library = "libvo_resvg.dylib"

[[extension.native.targets]]
target = "x86_64-unknown-linux-gnu"
library = "libvo_resvg.so"

[extension.wasm]
type = "standalone"
wasm = "resvg.wasm"
