//! HTTP source example.
//!
//! This example demonstrates downloading a file from a URL and using
//! it in a build step.
//!
//! Usage:
//!   cargo run --example http_source --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use std::io::Write;

use buildkit_rs_llb::*;

fn main() {
    // Download a file via HTTP
    let downloaded = Http::new("https://raw.githubusercontent.com/moby/buildkit/master/README.md")
        .with_filename("README.md")
        .with_custom_name("download README from buildkit repo");

    // Use an Alpine image to process the file
    let alpine = Image::new("alpine:latest");

    // Display the downloaded file
    let command = Exec::shlex("/bin/sh -c \"cat /downloads/README.md | head -20\"")
        .with_custom_name("display downloaded file")
        .with_mount(Mount::layer_readonly(alpine.output(), "/"))
        .with_mount(Mount::layer_readonly(downloaded.output(), "/downloads"));

    // Serialize and write to stdout
    let definition = Definition::new(command.output(0)).into_bytes();
    std::io::stdout().write_all(&definition).unwrap();
}
