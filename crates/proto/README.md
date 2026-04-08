# buildkit-sdk-proto

Generated BuildKit protobuf and gRPC types for Rust.

- Package: `buildkit-sdk-proto`
- Crate: `buildkit_rs_proto`

This crate contains generated types for BuildKit's protobuf surface, including
LLB protobuf messages and gRPC service definitions used by the client crate.

## Example

```rust
use buildkit_rs_proto::pb::Definition;

let _definition = Definition::default();
```

Most users will consume this crate indirectly through `buildkit-sdk-llb` or
`buildkit-sdk-client`, but it is available directly when you need lower-level
access to the wire format.
