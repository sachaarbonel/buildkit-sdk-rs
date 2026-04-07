//! Git source example.
//!
//! This example demonstrates cloning a Git repository and using its
//! contents in a build. It clones a repository, lists its files, and
//! writes the output.
//!
//! Usage:
//!   cargo run --example git_source --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    // Clone a Git repository at a specific ref
    let repo = git("https://github.com/moby/buildkit.git", "v0.12.0");

    // List the files in the cloned repository
    let st = image("alpine:latest")
        .run(shlex("sh -c \"ls -la /src && cat /src/README.md\""))
        .with_custom_name("list repo contents")
        .add_mount("/src", repo)
        .root();

    write_to(&st.marshal(), &mut std::io::stdout());
}
