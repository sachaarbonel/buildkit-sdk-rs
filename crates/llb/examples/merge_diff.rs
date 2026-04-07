//! Merge and Diff operations example.
//!
//! This example demonstrates how to use Merge and Diff operations to
//! combine and compare filesystem layers.
//!
//! - Merge: combines multiple filesystem layers into one
//! - Diff: computes the difference between two filesystem states
//!
//! Usage:
//!   cargo run --example merge_diff --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use std::io::Write;

use buildkit_rs_llb::*;

fn main() {
    let alpine = Image::new("alpine:latest");

    // Create first layer: a directory with file_a
    let layer_a = FileActions::new()
        .with_action(Mkdir::new("/data", alpine.output()).with_make_parents(true))
        .with_custom_name("create /data directory");
    let layer_a = FileActions::new()
        .with_action(MkFile::new(
            "/data/file_a.txt",
            layer_a.output(0),
            b"contents of file A\n".to_vec(),
        ))
        .with_custom_name("create file_a");

    // Create second layer: add file_b on top of layer_a
    let layer_b = FileActions::new()
        .with_action(MkFile::new(
            "/data/file_b.txt",
            layer_a.output(0),
            b"contents of file B\n".to_vec(),
        ))
        .with_custom_name("create file_b");

    // Diff: compute only the changes between layer_a and layer_b
    // This will produce a layer containing only file_b.txt
    let diff = Diff::new(Some(layer_a.output(0)), Some(layer_b.output(0)))
        .with_custom_name("diff: only new files");

    // Merge: combine the diff (file_b only) with a fresh alpine base
    let merged =
        Merge::new(vec![alpine.output(), diff.output()]).with_custom_name("merge diff onto alpine");

    // Verify the result
    let verify = Exec::shlex(
        "/bin/sh -c \"echo '--- merged contents ---' && ls -la /data/ && cat /data/file_b.txt\"",
    )
    .with_custom_name("verify merge result")
    .with_mount(Mount::layer_readonly(merged.output(), "/"));

    // Serialize and write to stdout
    let definition = Definition::new(verify.output(0)).into_bytes();
    std::io::stdout().write_all(&definition).unwrap();
}
