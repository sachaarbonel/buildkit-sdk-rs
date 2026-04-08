//! HTTP source example.
//!
//! This example demonstrates downloading a file from a URL and using
//! it in a build step.
//!
//! Usage:
//!   cargo run --example http_source --package buildkit-sdk-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    // Download a file via HTTP (use Http directly for filename option)
    let downloaded = State::from(
        Http::new("https://raw.githubusercontent.com/moby/buildkit/master/README.md")
            .with_filename("README.md")
            .with_custom_name("download README from buildkit repo"),
    );

    // Display the downloaded file using an Alpine image
    let st = image("alpine:latest")
        .run(shlex("cat /downloads/README.md | head -20"))
        .with_custom_name("display downloaded file")
        .add_mount("/downloads", downloaded)
        .root();

    write_to(&st.marshal(), &mut std::io::stdout());
}
