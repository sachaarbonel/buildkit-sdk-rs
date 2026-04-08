# buildkit-sdk-util

Lightweight utilities shared across the BuildKit Rust SDK.

- Package: `buildkit-sdk-util`
- Crate: `buildkit_rs_util`

This crate is intentionally small. It currently provides utilities such as OCI
backend selection and platform-specific executable search path helpers.

## Example

```rust
use buildkit_rs_util::oci::OciBackend;

let backend: OciBackend = "docker".parse().unwrap();
assert_eq!(backend.as_str(), "docker");
```
