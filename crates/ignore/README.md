# buildkit-sdk-ignore

Parse `.dockerignore` and `.containerignore` files for BuildKit contexts.

- Package: `buildkit-sdk-ignore`
- Crate: `buildkit_rs_ignore`

This crate normalizes ignore entries into a cleaned list of patterns that can be
used by BuildKit-related tooling when constructing local build contexts.

## Example

```rust
use buildkit_rs_ignore::read_ignore_to_list;

let patterns = read_ignore_to_list("target\n.git\n".as_bytes()).unwrap();

assert_eq!(patterns, vec!["target", ".git"]);
```
