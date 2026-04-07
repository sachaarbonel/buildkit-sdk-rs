//! Basic "hello world" example demonstrating the simplest BuildKit LLB usage.
//!
//! This example creates an LLB definition that:
//! 1. Pulls an Alpine Linux image
//! 2. Runs `echo "hello world"` inside the container
//! 3. Serializes the definition and writes it to stdout
//!
//! Usage:
//!   cargo run --example hello_world --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    let st = image("alpine:latest")
        .run(shlex("echo 'hello world'"))
        .with_custom_name("echo hello world")
        .root();

    write_to(&st.marshal(), &mut std::io::stdout());
}
