//! Cache mounts example.
//!
//! This example demonstrates how to use cache mounts to persist data
//! between builds, commonly used for package manager caches (apt, pip,
//! cargo, npm, etc.) to speed up repeated builds.
//!
//! Usage:
//!   cargo run --example cache_mounts --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    // Install packages with a persistent APK cache
    let st = image("alpine:latest")
        .run(shlex("apk add --no-cache curl git jq"))
        .with_custom_name("install packages with cached apk")
        .add_mount_cache("/var/cache/apk", "apk-cache", CacheSharingMode::Shared)
        .root();

    // Demonstrate a private cache (not shared between concurrent builds)
    let st = st
        .run(shlex(
            r#"sh -c "echo 'build artifact' > /tmp/build-cache/result.txt && cat /tmp/build-cache/result.txt""#,
        ))
        .with_custom_name("build step with private cache")
        .add_mount_cache(
            "/tmp/build-cache",
            "build-cache-v1",
            CacheSharingMode::Private,
        )
        .root();

    // Demonstrate a locked cache (exclusive access, only one build at a time)
    let st = st
        .run(shlex(
            r#"sh -c "echo 'writing to locked cache' > /tmp/locked/state.txt && cat /tmp/locked/state.txt""#,
        ))
        .with_custom_name("finalize with locked cache")
        .add_mount_cache("/tmp/locked", "locked-state", CacheSharingMode::Locked)
        .root();

    write_to(&st.marshal(), &mut std::io::stdout());
}
