//! Merge and Diff operations example.
//!
//! This example demonstrates how to use Merge and Diff operations to
//! combine and compare filesystem layers.
//!
//! - Merge: combines multiple filesystem layers into one
//! - Diff: computes the difference between two filesystem states
//!
//! Usage:
//!   cargo run --example merge_diff --package buildkit-sdk-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    let alpine = image("alpine:latest");

    // Create first layer: a directory with file_a
    let layer_a = alpine
        .clone()
        .file(
            mkdir("/data", 0o755)
                .with_make_parents(true)
                .with_custom_name("create /data directory"),
        )
        .file(
            mkfile("/data/file_a.txt", 0o644, b"contents of file A\n".to_vec())
                .with_custom_name("create file_a"),
        );

    // Create second layer: add file_b on top of layer_a
    let layer_b = layer_a.clone().file(
        mkfile("/data/file_b.txt", 0o644, b"contents of file B\n".to_vec())
            .with_custom_name("create file_b"),
    );

    // Diff: compute only the changes between layer_a and layer_b
    // This will produce a layer containing only file_b.txt
    let changes = diff(&layer_a, &layer_b);

    // Merge: combine the diff (file_b only) with a fresh alpine base
    let merged = merge(vec![alpine, changes]);

    // Verify the result
    let st = merged
        .run(shlex(
            "sh -c \"echo '--- merged contents ---' && ls -la /data/ && cat /data/file_b.txt\"",
        ))
        .with_custom_name("verify merge result")
        .root();

    write_to(&st.marshal(), &mut std::io::stdout());
}
