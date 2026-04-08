//! Simple test example.
//!
//! Usage:
//!   cargo run --example test --package buildkit-sdk-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    let st = image("alpine:latest")
        .run(shlex("echo 'hello world'"))
        .with_custom_name("create a dummy file")
        .add_mount_scratch("/out")
        .root();

    write_to(&st.marshal(), &mut std::io::stdout());
}
