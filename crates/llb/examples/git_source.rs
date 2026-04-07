//! Git source example.
//!
//! This example demonstrates cloning a Git repository and using its
//! contents in a build. It clones a repository, lists its files, and
//! writes the output.
//!
//! Usage:
//!   cargo run --example git_source --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use std::io::Write;

use buildkit_rs_llb::*;

fn main() {
    // Clone a Git repository at a specific ref
    let repo = Git::new("https://github.com/moby/buildkit.git", "v0.12.0")
        .with_custom_name("clone buildkit repository");

    // Use an Alpine image to process the cloned source
    let alpine = Image::new("alpine:latest");

    // List the files in the cloned repository
    let command = Exec::shlex("/bin/sh -c \"ls -la /src && cat /src/README.md\"")
        .with_custom_name("list repo contents")
        .with_mount(Mount::layer_readonly(alpine.output(), "/"))
        .with_mount(Mount::layer_readonly(repo.output(), "/src"));

    // Serialize and write to stdout
    let definition = Definition::new(command.output(0)).into_bytes();
    std::io::stdout().write_all(&definition).unwrap();
}
