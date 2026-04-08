# buildkit-sdk-reference

Docker and OCI image reference parsing for BuildKit-related tooling.

- Package: `buildkit-sdk-reference`
- Crate: `buildkit_rs_reference`

This crate parses normalized image references, tags, digests, domains, and
repository paths.

## Example

```rust
use buildkit_rs_reference::Reference;

let reference = Reference::parse_normalized_named("alpine:latest").unwrap();

assert_eq!(reference.domain(), "docker.io");
assert_eq!(reference.path().as_deref(), Some("library/alpine"));
assert_eq!(reference.tag(), Some("latest"));
```
