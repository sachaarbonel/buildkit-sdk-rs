use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use buildkit_rs_proto::{
    fsutil::types::{Packet, Stat, packet::PacketType},
    moby::filesync::v1::file_sync_server::{FileSync, FileSyncServer},
};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::{Mutex, mpsc::Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, trace, warn};

use crate::util::file_mode::FileMode;

const KEY_INCLUDE_PATTERNS: &str = "include-patterns";
const KEY_EXCLUDE_PATTERNS: &str = "exclude-patterns";
const KEY_FOLLOW_PATHS: &str = "followpaths";
const KEY_DIR_NAME: &str = "dir-name";

const MAX_PACKET_SIZE: usize = 1024 * 1024 * 4;

fn file_can_request_data(mode: u32) -> bool {
    mode & FileMode::MODE_TYPE_MASK.bits() == 0
}

async fn read_data_chunks<R: AsyncRead + Unpin>(
    reader: &mut R,
    chunk_size: usize,
) -> std::io::Result<Vec<Vec<u8>>> {
    let mut chunks = Vec::new();
    let mut buffer = vec![0; chunk_size];

    loop {
        let read = reader.read(&mut buffer[..]).await?;
        if read == 0 {
            break;
        }
        chunks.push(buffer[..read].to_vec());
    }

    Ok(chunks)
}

pub struct FileSyncService {
    context: HashMap<String, PathBuf>,
}

impl FileSyncService {
    pub fn new(context: HashMap<String, PathBuf>) -> Self {
        Self { context }
    }

    pub fn into_server(self) -> FileSyncServer<Self> {
        FileSyncServer::new(self)
    }
}

#[tonic::async_trait]
impl FileSync for FileSyncService {
    type DiffCopyStream = ReceiverStream<Result<Packet, Status>>;
    type TarStreamStream = ReceiverStream<Result<Packet, Status>>;

    #[tracing::instrument(skip_all)]
    async fn diff_copy(
        &self,
        request: Request<Streaming<Packet>>,
    ) -> Result<Response<Self::DiffCopyStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);

        info!(?request);

        let dir_name = match request.metadata().get(KEY_DIR_NAME).map(|v| v.to_str()) {
            Some(Ok(dir_name)) => dir_name,
            Some(Err(e)) => {
                return Err(Status::invalid_argument(format!("invalid dir-name: {}", e)));
            }
            None => return Err(Status::invalid_argument("missing dir-name in metadata")),
        };

        let context_path = match self.context.get(dir_name) {
            Some(path) => path.clone(),
            None => return Err(Status::invalid_argument("dir-name not found in context")),
        };

        let include_patterns: Vec<String> = request
            .metadata()
            .get_all(KEY_INCLUDE_PATTERNS)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(Into::into)
            .collect();

        let exclude_patterns: Vec<String> = request
            .metadata()
            .get_all(KEY_EXCLUDE_PATTERNS)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(Into::into)
            .collect();

        let follow_paths: Vec<String> = request
            .metadata()
            .get_all(KEY_FOLLOW_PATHS)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(Into::into)
            .collect();

        tokio::spawn(async move {
            let files = Arc::new(Mutex::new(HashMap::<u32, PathBuf>::new()));
            let walker = tokio::spawn(walk(
                context_path.clone(),
                tx.clone(),
                exclude_patterns,
                include_patterns,
                follow_paths,
                Arc::clone(&files),
            ));

            let mut inner = request.into_inner();
            while let Ok(Some(packet)) = inner.message().await {
                trace!(?packet);
                match packet.r#type() {
                    PacketType::PacketReq => {
                        let id = packet.id;
                        let path = {
                            let mut locked = files.lock().await;
                            locked.remove(&id)
                        };
                        let Some(path) = path else {
                            let message = format!("requested file id {id} is out of range");
                            error!(%id, "{message}");
                            if let Err(err) = tx
                                .send(Ok(Packet {
                                    r#type: PacketType::PacketErr.into(),
                                    id,
                                    data: message.into_bytes(),
                                    ..Default::default()
                                }))
                                .await
                            {
                                error!(?err, "Error sending error packet");
                            }
                            continue;
                        };

                        info!(%id, path = %path.display(), "sending requested file");

                        let reader = match tokio::fs::File::open(&path).await {
                            Ok(reader) => reader,
                            Err(err) => {
                                let message = format!(
                                    "failed to open requested path {}: {err}",
                                    path.display()
                                );
                                error!(?err, path = %path.display(), "Error opening file");
                                if let Err(send_err) = tx
                                    .send(Ok(Packet {
                                        r#type: PacketType::PacketErr.into(),
                                        id,
                                        data: message.into_bytes(),
                                        ..Default::default()
                                    }))
                                    .await
                                {
                                    error!(?send_err, "Error sending error packet");
                                }
                                continue;
                            }
                        };
                        let mut buf_reader = tokio::io::BufReader::new(reader);
                        match read_data_chunks(&mut buf_reader, MAX_PACKET_SIZE).await {
                            Ok(chunks) => {
                                for chunk in chunks {
                                    if let Err(err) = tx
                                        .send(Ok(Packet {
                                            r#type: PacketType::PacketData.into(),
                                            id,
                                            data: chunk,
                                            ..Default::default()
                                        }))
                                        .await
                                    {
                                        error!(?err, "Error sending data packet");
                                    }
                                }
                            }
                            Err(err) => {
                                error!(?err, "Error reading file");
                            }
                        }

                        if let Err(err) = tx
                            .send(Ok(Packet {
                                r#type: PacketType::PacketData.into(),
                                id,
                                ..Default::default()
                            }))
                            .await
                        {
                            error!(?err, "Error sending data packet");
                        }
                    }
                    PacketType::PacketErr => {
                        error!(str =% String::from_utf8_lossy(&packet.data), "Error Packet");
                        break;
                    }
                    PacketType::PacketFin => {
                        info!("fin");
                        let _ = walker.await;
                        if let Err(err) = tx
                            .send(Ok(Packet {
                                r#type: PacketType::PacketFin.into(),
                                ..Default::default()
                            }))
                            .await
                        {
                            error!(?err, "Error sending fin packet");
                        }
                        return;
                    }
                    other => {
                        error!(?other, "Unexpected packet type");
                        break;
                    }
                }
            }

            let _ = walker.await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    #[tracing::instrument(skip_all)]
    async fn tar_stream(
        &self,
        _request: Request<Streaming<Packet>>,
    ) -> Result<Response<Self::TarStreamStream>, Status> {
        warn!("not implemented");
        Err(Status::unimplemented("not implemented"))
    }
}

async fn walk(
    root: PathBuf,
    tx: Sender<Result<Packet, Status>>,

    exclude_patterns: Vec<String>,
    include_patterns: Vec<String>,
    follow_paths: Vec<String>,
    files: Arc<Mutex<HashMap<u32, PathBuf>>>,
) {
    macro_rules! send_data_packet {
        ($t:ident, $data:expr) => {
            let _ = tx
                .send(Ok(Packet {
                    r#type: PacketType::$t.into(),
                    data: $data,
                    ..Default::default()
                }))
                .await;
        };
    }

    let root = root.as_path();
    let mut next_id: u32 = 0;

    for entry in walkdir::WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            let trimmed_path = entry
                .path()
                .strip_prefix(root)
                .unwrap_or_else(|_| Path::new(""));
            should_include_path(
                trimmed_path,
                &exclude_patterns,
                &include_patterns,
                &follow_paths,
            )
        })
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                send_data_packet!(PacketErr, err.to_string().into_bytes());
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                send_data_packet!(PacketErr, err.to_string().into_bytes());
                continue;
            }
        };

        let trimmed_path = entry.path().strip_prefix(root).unwrap();
        let clean_path = path_clean::clean(trimmed_path);

        #[cfg(unix)]
        let (uid, gid, size) = {
            use std::os::unix::prelude::MetadataExt;

            let uid = metadata.uid();
            let gid = metadata.gid();
            let size = metadata.size() as i64;

            (uid, gid, size)
        };

        #[cfg(windows)]
        let (uid, gid, size) = {
            use std::os::windows::prelude::MetadataExt;

            // TODO: this seems wrong, not sure what to do here for uid/gid, maybe default to 1000?
            let uid = 0;
            let gid = 0;

            let size = metadata.file_size() as i64;

            (uid, gid, size)
        };

        let stat = Stat {
            path: clean_path.to_string_lossy().into_owned(),
            mode: FileMode::from_metadata(&metadata).bits(),
            uid,
            gid,
            size,
            mod_time: metadata.modified().map_or(0, |t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
            }),
            ..Default::default()
        };

        if file_can_request_data(stat.mode) {
            files
                .lock()
                .await
                .insert(next_id, entry.path().to_path_buf());
        }
        next_id = next_id.saturating_add(1);

        if let Err(err) = tx
            .send(Ok(Packet {
                r#type: PacketType::PacketStat.into(),
                stat: Some(stat),
                ..Default::default()
            }))
            .await
        {
            error!(?err);
        }
    }

    // Send a final empty stat packet to indicate the end of the stream.
    if let Err(err) = tx
        .send(Ok(Packet {
            r#type: PacketType::PacketStat.into(),
            ..Default::default()
        }))
        .await
    {
        error!(?err);
    }
}

fn should_include_path(
    path: &Path,
    exclude_patterns: &[String],
    include_patterns: &[String],
    follow_paths: &[String],
) -> bool {
    let clean_path = path_clean::clean(path);

    if exclude_patterns
        .iter()
        .any(|pattern| path_matches_prefix(&clean_path, pattern))
    {
        return false;
    }

    if (include_patterns.is_empty() && follow_paths.is_empty()) || is_root_path(&clean_path) {
        return true;
    }

    include_patterns
        .iter()
        .chain(follow_paths.iter())
        .any(|pattern| {
            let pattern = path_clean::clean(pattern);
            is_root_path(&pattern)
                || clean_path.starts_with(&pattern)
                || pattern.starts_with(&clean_path)
        })
}

fn path_matches_prefix(path: &Path, pattern: &str) -> bool {
    let pattern = path_clean::clean(pattern);
    is_root_path(&pattern) || path.starts_with(&pattern)
}

fn is_root_path(path: &Path) -> bool {
    path.as_os_str().is_empty() || path == Path::new(".")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{read_data_chunks, should_include_path};
    use tokio::io::AsyncWriteExt;

    #[test]
    fn include_patterns_keep_parent_directories() {
        assert!(should_include_path(
            Path::new("src"),
            &[],
            &["src/main.rs".to_owned()],
            &[],
        ));
        assert!(should_include_path(
            Path::new("src/main.rs"),
            &[],
            &["src/main.rs".to_owned()],
            &[],
        ));
    }

    #[test]
    fn include_patterns_filter_unrelated_paths() {
        assert!(!should_include_path(
            Path::new("tests"),
            &[],
            &["src/main.rs".to_owned()],
            &[],
        ));
    }

    #[test]
    fn exclude_patterns_override_includes() {
        assert!(!should_include_path(
            Path::new("src/generated"),
            &["src/generated".to_owned()],
            &["src".to_owned()],
            &[],
        ));
    }

    #[test]
    fn follow_paths_keep_requested_file_and_parents() {
        assert!(should_include_path(
            Path::new("."),
            &[],
            &[],
            &["Dockerfile".to_owned()],
        ));
        assert!(should_include_path(
            Path::new("Dockerfile"),
            &[],
            &[],
            &["Dockerfile".to_owned()],
        ));
        assert!(!should_include_path(
            Path::new("src"),
            &[],
            &[],
            &["Dockerfile".to_owned()],
        ));
    }

    #[test]
    fn exclude_patterns_override_follow_paths() {
        assert!(!should_include_path(
            Path::new("Dockerfile"),
            &["Dockerfile".to_owned()],
            &[],
            &["Dockerfile".to_owned()],
        ));
    }

    #[tokio::test]
    async fn read_data_chunks_reads_non_empty_input() {
        let (mut writer, mut reader) = tokio::io::duplex(64);
        let payload = b"FROM scratch\nLABEL smoke=\"true\"\n";

        let writer_task = tokio::spawn(async move {
            writer.write_all(payload).await.unwrap();
            writer.shutdown().await.unwrap();
        });

        let chunks = read_data_chunks(&mut reader, 8).await.unwrap();
        writer_task.await.unwrap();

        assert_eq!(chunks.concat(), payload);
        assert!(chunks.iter().all(|chunk| !chunk.is_empty()));
    }
}
