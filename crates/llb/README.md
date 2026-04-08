# buildkit-sdk-llb

Rust builders for BuildKit LLB graphs and state-based operations.

- Package: `buildkit-sdk-llb`
- Crate: `buildkit_rs_llb`

This crate contains both low-level LLB operation types and a higher-level
state-based API modeled after BuildKit's Go SDK.

## Example

```rust
use buildkit_rs_llb::{image, shlex};

let def = image("alpine:latest")
    .run(shlex("echo hello"))
    .root()
    .marshal();

buildkit_rs_llb::write_to(&def, &mut std::io::stdout());
```

You can pipe generated definitions into `buildctl`:

```shell
cargo run --example hello_world --package buildkit-sdk-llb | \
  buildctl build --progress plain --no-cache
```
