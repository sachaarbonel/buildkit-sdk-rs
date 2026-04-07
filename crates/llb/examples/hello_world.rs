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

use std::io::Write;

use buildkit_rs_llb::*;

fn main() {
    // Pull an Alpine image as the base
    let alpine = Image::new("alpine:latest");

    // Run a simple echo command
    let command = Exec::shlex("/bin/sh -c \"echo 'hello world'\"")
        .with_custom_name("echo hello world")
        .with_mount(Mount::layer_readonly(alpine.output(), "/"));

    // Serialize and write to stdout
    let definition = Definition::new(command.output(0)).into_bytes();
    std::io::stdout().write_all(&definition).unwrap();
}
