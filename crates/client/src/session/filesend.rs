use buildkit_rs_proto::moby::filesync::v1::{
    BytesMessage,
    file_send_server::{FileSend, FileSendServer},
};
use buildkit_rs_util::oci::OciBackend;
use tokio::io::AsyncWriteExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::{info, error};

pub(crate) struct FileSendService {
    backend: OciBackend,
}

impl FileSendService {
    pub fn new(backend: OciBackend) -> Self {
        Self { backend }
    }

    pub fn into_server(self) -> FileSendServer<Self> {
        FileSendServer::new(self)
    }
}

#[tonic::async_trait]
impl FileSend for FileSendService {
    type DiffCopyStream = ReceiverStream<Result<BytesMessage, Status>>;

    async fn diff_copy(
        &self,
        request: tonic::Request<tonic::Streaming<BytesMessage>>,
    ) -> Result<Response<Self::DiffCopyStream>, Status> {
        info!(?request);

        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let stream = request.into_inner();
        let backend = self.backend;

        tokio::spawn(async move {
            let command = backend.as_str();
            let mut load_process = match tokio::process::Command::new(command)
                .arg("load")
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(err) => {
                    error!(?err, %command, "failed to spawn image load process");
                    return;
                }
            };

            let mut load_stdin = match load_process.stdin.take() {
                Some(stdin) => stdin,
                None => {
                    error!("failed to capture stdin of image load process");
                    return;
                }
            };

            let _tx = tx;
            let mut stream = stream;
            while let Ok(Some(message)) = stream.message().await {
                let data = message.data;
                if let Err(err) = load_stdin.write_all(&data).await {
                    error!(?err, "failed to write to image load process");
                    return;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
