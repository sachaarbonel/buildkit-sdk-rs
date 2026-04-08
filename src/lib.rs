//! # BuildKit Rust SDK
//!
//! This crate is the top-level entry point for the BuildKit Rust SDK. It
//! re-exports all sub-crates so users can work from a single dependency.
//! Each sub-crate can also be used independently for finer-grained control.
//!
//! ## Architecture
//!
//! The SDK mirrors the Go BuildKit client library and exposes three key layers:
//!
//! 1. **[`llb`]** — LLB State builder: a fluent API for constructing build
//!    graphs programmatically (equivalent to Go's `client/llb/` package).
//! 2. **[`proto`]** — Protobuf/gRPC generated types: the wire format for all
//!    LLB operations (equivalent to `solver/pb/ops.proto`).
//! 3. **[`client`]** — gRPC client: connects to a `buildkitd` daemon and
//!    submits builds (equivalent to Go's `client/` package).
//!
//! ## Quick Start
//!
//! ```no_run
//! use buildkit_sdk_rs::llb::{image, shlex};
//!
//! // Build a graph: pull alpine, run a command, serialize to bytes
//! let definition_bytes = image("alpine:latest")
//!     .run(shlex("echo hello"))
//!     .root()
//!     .marshal();
//!
//! // Write the serialized LLB definition to stdout (pipe to `buildctl build`)
//! buildkit_sdk_rs::llb::write_to(&definition_bytes, &mut std::io::stdout());
//! ```
//!
//! ## Connecting to a BuildKit daemon
//!
//! ```no_run
//! use buildkit_sdk_rs::client::{Client, SolveOptions, SessionOptions};
//! use buildkit_sdk_rs::llb::{image, shlex};
//! use buildkit_sdk_rs::util::oci::OciBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = Client::connect(OciBackend::Docker, "buildkitd".to_owned()).await?;
//!
//! let session = client.session(SessionOptions {
//!     name: "my-build".into(),
//!     ..Default::default()
//! }).await?;
//!
//! let id = buildkit_sdk_rs::client::random_id();
//! let state = image("alpine:latest").run(shlex("echo hello")).root();
//! // Serialize the state into a Definition and submit
//! // let definition = buildkit_sdk_rs::llb::Definition::new(state.output().unwrap().clone());
//! // client.solve(SolveOptions { id, session: session.id, definition }).await?;
//! # Ok(())
//! # }
//! ```

pub use buildkit_rs_client as client;
pub use buildkit_rs_ignore as ignore;
pub use buildkit_rs_llb as llb;
pub use buildkit_rs_proto as proto;
pub use buildkit_rs_reference as reference;
pub use buildkit_rs_util as util;
