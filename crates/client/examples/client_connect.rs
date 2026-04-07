//! Client connection example.
//!
//! This example demonstrates connecting to a running BuildKit daemon
//! using the buildkit-rs client library and querying basic information.
//!
//! Prerequisites:
//!   docker run -d --name buildkitd --privileged moby/buildkit:latest
//!
//! Usage:
//!   cargo run --example client_connect --package buildkit-rs-client

use buildkit_rs_client::Client;
use buildkit_rs_util::oci::OciBackend;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a BuildKit daemon running in Docker
    let mut client = Client::connect(OciBackend::Docker, "buildkitd".to_string()).await?;

    // Query the BuildKit daemon for general info
    let info = client.info().await?;
    println!("BuildKit info: {info:?}");

    // List available workers
    let workers = client.list_workers().await?;
    println!("Workers: {workers:?}");

    // Query disk usage
    let disk_usage = client.disk_usage().await?;
    println!("Disk usage: {disk_usage:?}");

    Ok(())
}
