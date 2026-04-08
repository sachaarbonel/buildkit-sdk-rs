# buildkit-sdk-client

Async BuildKit client and session helpers for Rust.

- Package: `buildkit-sdk-client`
- Crate: `buildkit_rs_client`

This crate provides the `Client` type for connecting to a running `buildkitd`
instance, opening sessions, and submitting solve requests built either from LLB
definitions or BuildKit frontends such as `dockerfile.v0`.

## Example

```rust
use buildkit_rs_client::Client;
use buildkit_rs_util::oci::OciBackend;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut client = Client::connect(OciBackend::Docker, "buildkitd".to_owned()).await?;
let _info = client.info().await?;
# Ok(())
# }
```

See also the example binary:

```shell
cargo run --example client_connect --package buildkit-sdk-client
```
