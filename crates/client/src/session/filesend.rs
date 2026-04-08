use buildkit_rs_proto::moby::filesync::v1::{
    BytesMessage,
    file_send_server::{FileSend, FileSendServer},
};
use buildkit_rs_util::oci::OciBackend;
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin},
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::{error, info, warn};

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

async fn finish_load_process(
    mut load_stdin: ChildStdin,
    load_process: Child,
    command: &str,
) -> Result<Option<String>, String> {
    load_stdin
        .shutdown()
        .await
        .map_err(|err| format!("failed to close stdin for {command} load: {err}"))?;
    drop(load_stdin);

    let output = load_process
        .wait_with_output()
        .await
        .map_err(|err| format!("failed to wait for {command} load: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();

    if !output.status.success() {
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("process exited with status {}", output.status)
        };

        return Err(format!("{command} load failed: {detail}"));
    }

    if !stderr.is_empty() {
        warn!(%command, %stderr, "image load process wrote to stderr");
    }

    if stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(stdout))
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
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(err) => {
                    error!(?err, %command, "failed to spawn image load process");
                    let _ = tx
                        .send(Err(Status::internal(format!(
                            "failed to spawn {command} load process: {err}"
                        ))))
                        .await;
                    return;
                }
            };

            let mut load_stdin = match load_process.stdin.take() {
                Some(stdin) => stdin,
                None => {
                    error!("failed to capture stdin of image load process");
                    let _ = tx
                        .send(Err(Status::internal(format!(
                            "failed to capture stdin of {command} load process"
                        ))))
                        .await;
                    return;
                }
            };

            let mut stream = stream;
            loop {
                match stream.message().await {
                    Ok(Some(message)) => {
                        let data = message.data;
                        if let Err(err) = load_stdin.write_all(&data).await {
                            error!(?err, "failed to write to image load process");
                            let _ = tx
                                .send(Err(Status::internal(format!(
                                    "failed to write to {command} load process: {err}"
                                ))))
                                .await;
                            return;
                        }
                    }
                    Ok(None) => break,
                    Err(err) => {
                        error!(?err, "failed to read from stream");
                        let _ = tx
                            .send(Err(Status::internal(format!(
                                "failed to read exporter stream: {err}"
                            ))))
                            .await;
                        return;
                    }
                }
            }

            match finish_load_process(load_stdin, load_process, command).await {
                Ok(Some(stdout)) => info!(%command, %stdout, "image load completed"),
                Ok(None) => info!(%command, "image load completed"),
                Err(message) => {
                    error!(%command, %message, "image load failed");
                    let _ = tx.send(Err(Status::internal(message))).await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[cfg(test)]
mod tests {
    use super::finish_load_process;
    use std::{
        env,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    use tokio::{fs, io::AsyncWriteExt, process::Command};

    #[cfg(unix)]
    #[tokio::test]
    async fn finish_load_process_waits_for_loader_exit() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("buildkit-sdk-filesend-{stamp}"));
        let output_path = dir.join("loaded.txt");
        fs::create_dir_all(&dir).await.unwrap();

        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg("cat > \"$OUT\"; sleep 1; echo loaded")
            .env("OUT", &output_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"hello from exporter").await.unwrap();

        let start = Instant::now();
        let stdout = finish_load_process(stdin, child, "sh").await.unwrap();
        let elapsed = start.elapsed();
        let contents = fs::read(&output_path).await.unwrap();

        assert_eq!(stdout.as_deref(), Some("loaded"));
        assert_eq!(contents, b"hello from exporter");
        assert!(elapsed >= Duration::from_millis(900));

        fs::remove_dir_all(&dir).await.unwrap();
    }
}
