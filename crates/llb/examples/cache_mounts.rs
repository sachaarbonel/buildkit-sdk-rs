//! Cache mounts example.
//!
//! This example demonstrates how to use cache mounts to persist data
//! between builds, commonly used for package manager caches (apt, pip,
//! cargo, npm, etc.) to speed up repeated builds.
//!
//! Usage:
//!   cargo run --example cache_mounts --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use std::io::Write;

use buildkit_rs_llb::*;

fn main() {
    let alpine = Image::new("alpine:latest").with_custom_name("pull alpine");

    // Install packages with a persistent APK cache
    // The cache mount at /var/cache/apk persists between builds,
    // speeding up subsequent package installations
    let install_packages = Exec::shlex("/bin/sh -c \"apk add --no-cache curl git jq\"")
        .with_custom_name("install packages with cached apk")
        .with_mount(Mount::layer(alpine.output(), "/", 0))
        .with_mount(Mount::cache(
            "/var/cache/apk",
            "apk-cache",
            CacheSharingMode::Shared,
        ));

    // Demonstrate a private cache (not shared between concurrent builds)
    let build_step = Exec::shlex(
        "/bin/sh -c \"echo 'build artifact' > /tmp/build-cache/result.txt && cat /tmp/build-cache/result.txt\"",
    )
    .with_custom_name("build step with private cache")
    .with_mount(Mount::layer(install_packages.output(0), "/", 0))
    .with_mount(Mount::cache(
        "/tmp/build-cache",
        "build-cache-v1",
        CacheSharingMode::Private,
    ));

    // Demonstrate a locked cache (exclusive access, only one build at a time)
    let finalize = Exec::shlex(
        "/bin/sh -c \"echo 'writing to locked cache' > /tmp/locked/state.txt && cat /tmp/locked/state.txt\"",
    )
    .with_custom_name("finalize with locked cache")
    .with_mount(Mount::layer(build_step.output(0), "/", 0))
    .with_mount(Mount::cache(
        "/tmp/locked",
        "locked-state",
        CacheSharingMode::Locked,
    ));

    // Serialize and write to stdout
    let definition = Definition::new(finalize.output(0)).into_bytes();
    std::io::stdout().write_all(&definition).unwrap();
}
