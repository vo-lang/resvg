format = 1
module = "github.com/vo-lang/resvg"
version = "0.1.0"
vo = "0.1.0"

[extension]
name = "resvg"

[extension.native]
library = "vo_resvg"
targets = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]

[extension.wasm]
kind = "standalone"
wasm = "resvg.wasm"

[build.native]
kind = "cargo"
manifest = "rust/Cargo.toml"
package = "vo-resvg"

[build.wasm]
wasm = "resvg.wasm"
