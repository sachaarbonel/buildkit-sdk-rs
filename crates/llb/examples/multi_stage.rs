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
//!   cargo run --example multi_stage --package buildkit-sdk-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    // Stage 1: Builder image with build tools
    let build = image("golang:1.22-alpine")
        .run(shlex(
            r#"/bin/sh -c "cd /src && echo 'package main
import \"fmt\"
func main() { fmt.Println(\"hello from buildkit\") }' > main.go && go build -o /out/app main.go""#,
        ))
        .with_custom_name("build the application")
        .add_mount_scratch("/src")
        .add_mount_scratch("/out");

    let build_output = build.get_mount("/out");

    // Stage 2: Copy only the compiled binary into a minimal runtime image
    let runtime = image("alpine:latest").file(
        copy(&build_output, "/out/app", "/usr/local/bin/app")
            .with_custom_name("copy binary to runtime image"),
    );

    write_to(&runtime.marshal(), &mut std::io::stdout());
}
