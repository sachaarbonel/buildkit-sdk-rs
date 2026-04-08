//! File operations example.
//!
//! This example demonstrates the various file operations available in
//! BuildKit: creating directories, creating files, copying files,
//! creating symlinks, and removing files.
//!
//! Usage:
//!   cargo run --example file_operations --package buildkit-sdk-llb | \
//!     buildctl build --progress plain --no-cache

use buildkit_rs_llb::state::*;

fn main() {
    let alpine = image("alpine:latest");

    // Chain file operations on the state
    let st = alpine
        // Create a directory
        .file(
            mkdir("/app", 0o755)
                .with_make_parents(true)
                .with_custom_name("create /app directory"),
        )
        // Create a config file
        .file(
            mkfile(
                "/app/config.toml",
                0o644,
                b"[server]\nhost = \"0.0.0.0\"\nport = 8080\n".to_vec(),
            )
            .with_custom_name("create config file"),
        );

    // Create a symlink
    let st = st.clone().file(
        symlink("/app/config.toml", "/app/current-config")
            .with_custom_name("create config symlink"),
    );

    // Copy the file to a backup location
    let st = st.clone().file(
        copy(&st, "/app/config.toml", "/app/config.toml.bak")
            .with_create_dest_path(true)
            .with_custom_name("backup config file"),
    );

    // Remove the symlink
    let st = st.file(rm("/app/current-config").with_custom_name("remove symlink"));

    // Verify the results
    let result = st
        .run(shlex(
            r#"sh -c "ls -la /app/ && echo '--- config ---' && cat /app/config.toml && echo '--- backup ---' && cat /app/config.toml.bak""#,
        ))
        .with_custom_name("verify file operations")
        .root();

    write_to(&result.marshal(), &mut std::io::stdout());
}
