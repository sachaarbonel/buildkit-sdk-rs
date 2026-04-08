# buildkit-sdk-rs

This fork publishes under the `buildkit-sdk-*` package names on crates.io.

`buildkit-sdk-rs` is a Rust SDK for [BuildKit](https://github.com/moby/buildkit).
It provides Rust-first APIs for constructing LLB graphs, working with the
BuildKit protobuf surface, and talking to a running `buildkitd` daemon.

## What You Get

- A high-level state-based LLB builder inspired by BuildKit's Go API
- Lower-level LLB operation types for building graphs directly
- Generated protobuf and gRPC types for BuildKit services
- An async client crate for connecting to `buildkitd`
- Supporting utilities for image references, ignore files, and OCI backends

## Package Names

The published crates use the `buildkit-sdk-*` package names:

- `buildkit-sdk-rs`
- `buildkit-sdk-llb`
- `buildkit-sdk-client`
- `buildkit-sdk-proto`
- `buildkit-sdk-reference`
- `buildkit-sdk-ignore`
- `buildkit-sdk-util`

The Rust crate names stay unchanged for imports:

- `buildkit_sdk_rs`
- `buildkit_rs_llb`
- `buildkit_rs_client`
- `buildkit_rs_proto`
- `buildkit_rs_reference`
- `buildkit_rs_ignore`
- `buildkit_rs_util`

## How BuildKit Works

If you have not used BuildKit before, the rough mental model is:

1. You describe a build as a graph instead of a shell script.
2. BuildKit turns that graph into an internal format called LLB.
3. A BuildKit daemon (`buildkitd`) solves that graph, reusing cache where it can.
4. The result is exported somewhere useful, such as a Docker image, an OCI image,
   a local directory, or another BuildKit consumer.

In practice there are two common ways to drive it:

- Use a frontend such as `dockerfile.v0`, where BuildKit reads a Dockerfile and
  produces the LLB graph for you.
- Build the LLB graph directly in code, which is what the `buildkit-sdk-llb`
  crate is for.

This workspace maps to those pieces like this:

- `buildkit-sdk-llb` builds LLB graphs in Rust.
- `buildkit-sdk-proto` exposes the generated protobuf and gRPC types.
- `buildkit-sdk-client` connects to `buildkitd`, opens sessions, and submits
  solve requests.

Sessions are the mechanism BuildKit uses to access local build contexts,
secrets, auth, and file transfer while a solve is running.

## Quick Start

```rust
use buildkit_sdk_rs::llb::{image, shlex};

fn main() {
    let def = image("alpine:latest")
        .run(shlex("echo hello from BuildKit"))
        .root()
        .marshal();

    buildkit_sdk_rs::llb::write_to(&def, &mut std::io::stdout());
}
```

Pipe the generated definition into `buildctl`:

```shell
cargo run --example hello_world --package buildkit-sdk-llb | \
  buildctl build --progress plain --no-cache
```

## Workspace Crates

- `buildkit-sdk-rs`: top-level facade crate re-exporting the workspace crates
- `buildkit-sdk-llb`: LLB graph builders and state API
- `buildkit-sdk-client`: async BuildKit client and session helpers
- `buildkit-sdk-proto`: generated protobuf and gRPC types
- `buildkit-sdk-reference`: image reference parsing utilities
- `buildkit-sdk-ignore`: `.dockerignore` and `.containerignore` parsing
- `buildkit-sdk-util`: shared utilities such as OCI backend selection

## Running Against BuildKit

Start a local daemon:

```shell
docker run -d --name buildkitd --privileged moby/buildkit:latest
```

Run tests and lints:

```shell
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Maintaining BuildKit Protos

The pinned upstream BuildKit tag lives in `crates/proto/BUILDKIT_VERSION`.

Check whether the repo is behind the latest `moby/buildkit` release:

```shell
cargo xtask check-protos
```

Re-vendor the protobuf files for a specific BuildKit release:

```shell
cargo xtask update-protos --version v0.29.0
```

A weekly GitHub Actions workflow runs the same check, re-vendors a newer tag
when one is available, runs the workspace tests and clippy, and opens a PR
automatically. Maintainers should still subscribe to `moby/buildkit` releases
for awareness.

Try the example programs:

```shell
cargo run --example hello_world --package buildkit-sdk-llb | \
  buildctl build --progress plain --no-cache

cargo run --example client_connect --package buildkit-sdk-client
```

## Publishing This Fork

Authenticate with crates.io first:

```shell
cargo login
```

Or set `CARGO_REGISTRY_TOKEN` in your environment.

Publish in dependency order:

```shell
cargo publish -p buildkit-sdk-reference
cargo publish -p buildkit-sdk-proto
cargo publish -p buildkit-sdk-util
cargo publish -p buildkit-sdk-ignore
cargo publish -p buildkit-sdk-llb
cargo publish -p buildkit-sdk-client
cargo publish -p buildkit-sdk-rs
```

If you want to verify before uploading, use dry runs:

```shell
cargo publish --dry-run -p buildkit-sdk-reference
cargo publish --dry-run -p buildkit-sdk-rs
```

The top-level `buildkit-sdk-rs` crate will not publish successfully until its
dependent workspace crates are already available on crates.io.

## Goals

- Provide an idiomatic Rust SDK for BuildKit
- Stay close enough to the Go API to make BuildKit concepts familiar
- Keep the workspace modular so users can depend on only the crates they need
- Avoid unsafe code in the public implementation

## Status

The LLB layer and the generated protocol surface are the most mature parts of
the workspace. The client crate is usable, but its API surface is still fairly
close to the wire format and will likely continue to evolve.

## License

Licensed under either `Apache-2.0` or `MIT`.

Vendored protobuf files under `crates/proto/vendor` remain under their original
licenses.

## Contributing

Issues and pull requests are welcome. If you plan to add a larger feature, open
an issue first so the API shape can be discussed before implementation.
