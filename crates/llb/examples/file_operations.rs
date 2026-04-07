//! File operations example.
//!
//! This example demonstrates the various file operations available in
//! BuildKit: creating directories, creating files, copying files,
//! creating symlinks, and removing files.
//!
//! Usage:
//!   cargo run --example file_operations --package buildkit-rs-llb | \
//!     buildctl build --progress plain --no-cache

use std::io::Write;

use buildkit_rs_llb::*;

fn main() {
    let alpine = Image::new("alpine:latest");

    // Create a directory structure
    let mkdir = FileActions::new()
        .with_action(
            Mkdir::new("/app", alpine.output())
                .with_make_parents(true)
                .with_mode(0o755),
        )
        .with_custom_name("create /app directory");

    // Create a config file
    let mkfile = FileActions::new()
        .with_action(
            MkFile::new(
                "/app/config.toml",
                mkdir.output(0),
                b"[server]\nhost = \"0.0.0.0\"\nport = 8080\n".to_vec(),
            )
            .with_mode(0o644),
        )
        .with_custom_name("create config file");

    // Create a symlink
    let symlink = FileActions::new()
        .with_action(Symlink::new(
            "/app/config.toml",
            "/app/current-config",
            mkfile.output(0),
        ))
        .with_custom_name("create config symlink");

    // Copy the file to a backup location
    let copy = FileActions::new()
        .with_action(
            Copy::new(
                "/app/config.toml",
                symlink.output(0),
                "/app/config.toml.bak",
                symlink.output(0),
            )
            .with_create_dest_path(true),
        )
        .with_custom_name("backup config file");

    // Remove the symlink
    let rm = FileActions::new()
        .with_action(Rm::new("/app/current-config", copy.output(0)))
        .with_custom_name("remove symlink");

    // Verify the results
    let verify = Exec::shlex("/bin/sh -c \"ls -la /app/ && echo '--- config ---' && cat /app/config.toml && echo '--- backup ---' && cat /app/config.toml.bak\"")
        .with_custom_name("verify file operations")
        .with_mount(Mount::layer_readonly(rm.output(0), "/"));

    // Serialize and write to stdout
    let definition = Definition::new(verify.output(0)).into_bytes();
    std::io::stdout().write_all(&definition).unwrap();
}
