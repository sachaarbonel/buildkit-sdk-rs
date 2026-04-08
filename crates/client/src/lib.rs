pub mod connhelper;
pub mod error;
pub mod session;
pub(crate) mod util;

pub use error::Error;

use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;

use buildkit_rs_llb::Definition;
use buildkit_rs_proto::moby::buildkit::secrets::v1::secrets_server::SecretsServer;
use buildkit_rs_proto::moby::buildkit::v1::BytesMessage;
use buildkit_rs_proto::moby::buildkit::v1::{
    DiskUsageRequest, DiskUsageResponse, InfoRequest, InfoResponse, ListWorkersRequest,
    ListWorkersResponse, SolveResponse, control_client::ControlClient,
};
use buildkit_rs_proto::moby::buildkit::v1::{StatusRequest, StatusResponse};
use buildkit_rs_proto::moby::filesync::v1::auth_server::AuthServer;
use buildkit_rs_proto::moby::filesync::v1::file_send_server::FileSendServer;
use buildkit_rs_proto::moby::filesync::v1::file_sync_server::FileSyncServer;
use buildkit_rs_util::oci::OciBackend;
use connhelper::{docker::docker_connect, podman::podman_connect};
use futures::stream::StreamExt;
use hyper_util::rt::TokioIo;
use session::filesend::FileSendService;
use session::secret::SecretSource;
use session::{auth::AuthService, filesync::FileSyncService};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tonic::{
    Request, Response,
    transport::{Channel, Uri},
};
use tonic::{Status, Streaming};
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;
use tracing::{debug, info};

use crate::session::secret::SecretService;
pub use crate::util::id::random_id;

const HEADER_SESSION_ID: &str = "x-docker-expose-session-uuid";
const HEADER_SESSION_NAME: &str = "x-docker-expose-session-name";
const HEADER_SESSION_SHARED_KEY: &str = "x-docker-expose-session-sharedkey";
const HEADER_SESSION_METHOD: &str = "x-docker-expose-session-grpc-method";

/// Options for submitting a solve request to the BuildKit daemon.
///
/// Either `definition` (a pre-built LLB graph) or `frontend` (e.g.
/// `"dockerfile.v0"`) should be provided. When using a frontend the daemon
/// generates the LLB internally, so `definition` can be `None`.
#[derive(Debug, Default)]
pub struct SolveOptions<'a> {
    /// Unique reference id for this solve.
    pub id: String,
    /// Session id to associate with this solve.
    pub session: String,
    /// Pre-built LLB definition. `None` when a frontend is used instead.
    pub definition: Option<Definition<'a>>,
    /// Frontend to use (e.g. `"dockerfile.v0"`). Leave empty when providing
    /// an LLB definition directly.
    pub frontend: String,
    /// Key-value attributes passed to the frontend.
    pub frontend_attrs: HashMap<String, String>,
    /// Exporter to use for build results (e.g. `"docker"`, `"image"`,
    /// `"local"`). Leave empty when no explicit exporter is needed.
    pub exporter: String,
    /// Key-value attributes passed to the exporter, such as image names or
    /// output paths. These require `exporter` to be set.
    pub exporter_attrs: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionOptions {
    /// Human-readable session name advertised to BuildKit.
    pub name: String,
    /// Local directories exposed to the session by name.
    pub local: HashMap<String, PathBuf>,
    /// Secrets exposed to the session by id.
    pub secrets: HashMap<String, SecretSource>,
}

pub struct Session {
    /// Opaque BuildKit session identifier.
    pub id: String,
}

#[derive(Debug)]
pub struct Client {
    control: ControlClient<Channel>,
    backend: OciBackend,
}

impl Client {
    pub async fn connect(backend: OciBackend, container_name: String) -> Result<Client, Error> {
        let channel = Channel::from_static("http://[::1]:50051")
            .connect_with_connector(tower::service_fn(move |_: Uri| {
                let container_name = container_name.clone();
                async move {
                    let io = match backend {
                        OciBackend::Docker => docker_connect(container_name).await,
                        OciBackend::Podman => podman_connect(container_name).await,
                    }?;
                    Ok::<_, std::io::Error>(TokioIo::new(io))
                }
            }))
            .await?;

        Ok(Client {
            control: ControlClient::new(channel),
            backend,
        })
    }

    pub async fn info(&mut self) -> Result<InfoResponse, tonic::Status> {
        self.control
            .info(InfoRequest {})
            .await
            .map(Response::into_inner)
    }

    pub async fn disk_usage(&mut self) -> Result<DiskUsageResponse, tonic::Status> {
        self.control
            .disk_usage(DiskUsageRequest {
                filter: vec![],
                ..Default::default()
            })
            .await
            .map(Response::into_inner)
    }

    pub async fn list_workers(&mut self) -> Result<ListWorkersResponse, tonic::Status> {
        self.control
            .list_workers(ListWorkersRequest { filter: vec![] })
            .await
            .map(Response::into_inner)
    }

    pub async fn solve(
        &mut self,
        options: SolveOptions<'_>,
    ) -> Result<SolveResponse, tonic::Status> {
        if options.definition.is_none() && options.frontend.is_empty() {
            return Err(tonic::Status::invalid_argument(
                "solve requires either `definition` or `frontend`",
            ));
        }

        if options.exporter.is_empty() && !options.exporter_attrs.is_empty() {
            return Err(tonic::Status::invalid_argument(
                "exporter attributes require `exporter` to be set",
            ));
        }

        self.control
            .solve(Request::new(
                buildkit_rs_proto::moby::buildkit::v1::SolveRequest {
                    r#ref: options.id,
                    definition: options.definition.map(|d| d.into_pb()),
                    frontend: options.frontend,
                    frontend_attrs: options.frontend_attrs,
                    session: options.session,
                    exporter_deprecated: options.exporter,
                    exporter_attrs_deprecated: options.exporter_attrs,
                    ..Default::default()
                },
            ))
            .await
            .map(|res| res.into_inner())
    }

    pub async fn session(&mut self, options: SessionOptions) -> Result<Session, tonic::Status> {
        let (server_stream, client_stream) = tokio::io::duplex(4096);

        let (health_reporter, health_server) = tonic_health::server::health_reporter();

        let auth = AuthService::new().into_server();
        let file_sync = FileSyncService::new(options.local).into_server();
        let file_send = FileSendService::new(self.backend).into_server();
        let secret = SecretService::new(options.secrets).into_server();

        health_reporter
            .set_serving::<AuthServer<AuthService>>()
            .await;

        health_reporter
            .set_serving::<FileSyncServer<FileSyncService>>()
            .await;

        health_reporter
            .set_serving::<SecretsServer<SecretService>>()
            .await;

        health_reporter
            .set_serving::<FileSendServer<FileSendService>>()
            .await;

        let layer = ServiceBuilder::new().trace_for_grpc().into_inner();

        tokio::spawn(async move {
            match tonic::transport::Server::builder()
                .trace_fn(|_| tracing::info_span!("session server"))
                .layer(layer)
                .add_service(health_server)
                .add_service(auth)
                .add_service(file_sync)
                .add_service(file_send)
                .add_service(secret)
                .serve_with_incoming(futures::stream::iter(vec![Ok::<_, std::io::Error>(
                    server_stream,
                )]))
                .await
            {
                Ok(()) => debug!("Server finished"),
                Err(err) => tracing::error!(?err, "Server error"),
            }
        });

        // In memory client
        // let mut client = Some(client_stream);
        // let channel = Endpoint::try_from("http://[::]:50051")
        //     .unwrap()
        //     .connect_with_connector(service_fn(move |_: Uri| {
        //         let client = client.take();

        //         async move {
        //             if let Some(client) = client {
        //                 Ok(client)
        //             } else {
        //                 Err(std::io::Error::new(
        //                     std::io::ErrorKind::Other,
        //                     "Client already taken",
        //                 ))
        //             }
        //         }
        //     }))
        //     .await
        //     .unwrap();

        // loop {
        //     tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        //     // health check
        //     let mut client = tonic_health::pb::health_client::HealthClient::new(channel.clone());
        //     let res = client
        //         .check(tonic_health::pb::HealthCheckRequest {
        //             // service: "moby.filesync.v1.Auth".into(),
        //             service: "".into()
        //         })
        //         .await;

        //     info!(?res, "Health check");
        // }

        let (client_read, mut client_write) = tokio::io::split(client_stream);

        let request_stream = ReaderStream::new(client_read).filter_map(|bytes| async move {
            match bytes {
                Ok(bytes) => Some(BytesMessage {
                    data: bytes.to_vec(),
                }),
                Err(err) => {
                    tracing::error!(?err, "Error reading session stream");
                    None
                }
            }
        });
        let mut request = Request::new(request_stream);

        let id = random_id();

        request
            .metadata_mut()
            .append(HEADER_SESSION_ID, id.parse().expect("valid header value"));

        // Map the name to a valid header value so we make sure it doesn't panic
        let header_name_bytes = options
            .name
            .bytes()
            .map(|b| if (32..127).contains(&b) { b } else { b'?' })
            .collect::<Vec<_>>();
        let header_name = String::from_utf8_lossy(&header_name_bytes);

        request.metadata_mut().append(
            HEADER_SESSION_NAME,
            header_name.parse().expect("valid header value"),
        );

        request.metadata_mut().append(
            HEADER_SESSION_SHARED_KEY,
            "".parse().expect("valid header value"),
        );

        // request.metadata_mut().append(
        //     HEADER_SESSION_METHOD,
        //     "/moby.filesync.v1.Auth/Credentials"
        //         .parse()
        //         .expect("valid header value"),
        // );

        // request.metadata_mut().append(
        //     HEADER_SESSION_METHOD,
        //     "/moby.filesync.v1.Auth/FetchToken"
        //         .parse()
        //         .expect("valid header value"),
        // );

        // request.metadata_mut().append(
        //     HEADER_SESSION_METHOD,
        //     "/moby.filesync.v1.Auth/GetTokenAuthority"
        //         .parse()
        //         .expect("valid header value"),
        // );

        // request.metadata_mut().append(
        //     HEADER_SESSION_METHOD,
        //     "/moby.filesync.v1.Auth/VerifyTokenAuthority"
        //         .parse()
        //         .expect("valid header value"),
        // );

        // TODO: make these dynamic
        request.metadata_mut().append(
            HEADER_SESSION_METHOD,
            "/moby.filesync.v1.FileSync/DiffCopy"
                .parse()
                .expect("valid header value"),
        );

        request.metadata_mut().append(
            HEADER_SESSION_METHOD,
            "/moby.filesync.v1.FileSync/TarStream"
                .parse()
                .expect("valid header value"),
        );

        request.metadata_mut().append(
            HEADER_SESSION_METHOD,
            "/moby.filesync.v1.FileSend/DiffCopy"
                .parse()
                .expect("valid header value"),
        );

        request.metadata_mut().append(
            HEADER_SESSION_METHOD,
            "/moby.buildkit.secrets.v1.Secrets/GetSecret"
                .parse()
                .expect("valid header value"),
        );

        let res = self.control.session(request).await?;

        tokio::spawn(async move {
            let mut inner = res.into_inner();

            loop {
                match inner.message().await {
                    Ok(Some(msg)) => {
                        if let Err(err) = client_write.write_all(&msg.data).await {
                            tracing::error!(?err, "Error writing to client");
                            break;
                        }
                    }
                    Ok(None) => {
                        info!("Session finished");
                        break;
                    }
                    Err(err) => {
                        tracing::error!(?err, "Error");
                        break;
                    }
                }
            }

            info!("Client finished")
        });

        Ok(Session { id })
    }

    pub async fn status(&mut self, id: String) -> Result<Streaming<StatusResponse>, Status> {
        self.control
            .status(StatusRequest { r#ref: id })
            .await
            .map(Response::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::process::Stdio;

    use super::*;

    async fn ensure_buildkit_container(container_name: &str) {
        let _ = tokio::process::Command::new("docker")
            .args(["rm", "-f", container_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        let status = tokio::process::Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                container_name,
                "--privileged",
                "moby/buildkit:latest",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .expect("start buildkit container");
        assert!(status.success(), "failed to start buildkit container");

        for _ in 0..30 {
            let output = tokio::process::Command::new("docker")
                .args(["inspect", "-f", "{{.State.Running}}", container_name])
                .output()
                .await
                .expect("inspect buildkit container");
            if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true" {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        panic!("buildkit container did not become ready");
    }

    async fn remove_buildkit_container(container_name: &str) {
        let _ = tokio::process::Command::new("docker")
            .args(["rm", "-f", container_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }

    #[tokio::test]
    #[ignore] // Requires a running BuildKit container
    async fn test_connect() {
        // SAFETY: This is only called from a single-threaded test context
        unsafe {
            std::env::set_var("RUST_LOG", "debug");
        }
        tracing_subscriber::fmt::init();

        let mut conn = Client::connect(OciBackend::Docker, "buildkitd".to_owned())
            .await
            .unwrap();
        dbg!(conn.info().await.unwrap());

        let _session = conn
            .session(SessionOptions {
                name: "buildkit-rs".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(8)).await;

        // sleep for 5 sec
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    #[tokio::test]
    #[ignore] // Requires Docker and a BuildKit container
    async fn dockerfile_frontend_reads_local_dockerfile() {
        let _ = tracing_subscriber::fmt().try_init();

        let container_name = format!("buildkit-sdk-test-{}", random_id());
        ensure_buildkit_container(&container_name).await;

        let temp_dir = std::env::temp_dir().join(format!("buildkit-sdk-{}", random_id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(
            temp_dir.join("Dockerfile"),
            "FROM scratch\nLABEL smoke=\"true\"\n",
        )
        .unwrap();

        let result = async {
            let mut client = Client::connect(OciBackend::Docker, container_name.clone())
                .await
                .unwrap();

            let session = client
                .session(SessionOptions {
                    name: "dockerfile-smoke".into(),
                    local: HashMap::<String, PathBuf>::from([
                        ("context".into(), temp_dir.clone()),
                        ("dockerfile".into(), temp_dir.clone()),
                    ]),
                    ..Default::default()
                })
                .await
                .unwrap();

            client
                .solve(SolveOptions {
                    id: random_id(),
                    session: session.id,
                    definition: None,
                    frontend: "dockerfile.v0".into(),
                    frontend_attrs: HashMap::from([("filename".into(), "Dockerfile".into())]),
                    ..Default::default()
                })
                .await
        }
        .await;

        remove_buildkit_container(&container_name).await;
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(
            result.is_ok(),
            "dockerfile frontend solve failed: {:?}",
            result.err()
        );
    }
}
