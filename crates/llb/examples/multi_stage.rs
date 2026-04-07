//! Multi-stage build example.
//!
//! This example demonstrates a multi-stage build pattern where:
//! 1. A builder stage compiles source code
//! 2. A runtime stage copies only the build artifact
//!
//! This pattern produces smaller final images by discarding build-time
//! dependencies.
//!
//! Usage:
//!   cargo run --example multi_stage --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use std::io::Write;

use buildkit_rs_llb::*;

fn main() {
    // Stage 1: Builder image with build tools
    let builder_image = Image::new("golang:1.22-alpine").with_custom_name("pull builder image");

    // Install dependencies and build the application
    let build = Exec::shlex(
        "/bin/sh -c \"cd /src && echo 'package main\nimport \\\"fmt\\\"\nfunc main() { fmt.Println(\\\"hello from buildkit\\\") }' > main.go && go build -o /out/app main.go\"",
    )
    .with_custom_name("build the application")
    .with_mount(Mount::layer_readonly(builder_image.output(), "/"))
    .with_mount(Mount::scratch("/src", 1))
    .with_mount(Mount::scratch("/out", 2));

    // Stage 2: Minimal runtime image
    let runtime_image = Image::new("alpine:latest").with_custom_name("pull runtime image");

    // Copy only the compiled binary into the runtime image
    let copy_artifact = FileActions::new()
        .with_action(Copy::new(
            "/out/app",
            build.output(2),
            "/usr/local/bin/app",
            runtime_image.output(),
        ))
        .with_custom_name("copy binary to runtime image");

    // Serialize and write to stdout
    let definition = Definition::new(copy_artifact.output(0)).into_bytes();
    std::io::stdout().write_all(&definition).unwrap();
}
