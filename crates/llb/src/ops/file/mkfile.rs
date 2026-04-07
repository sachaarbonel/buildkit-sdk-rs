use buildkit_rs_proto::pb;
use camino::Utf8PathBuf;

use crate::utils::OperationOutput;

use super::FileAction;
use super::chown::ChownOpt;

/// Create a new file with specified contents.
#[derive(Debug)]
pub struct MkFile<'a> {
    pub(crate) path: Utf8PathBuf,
    pub(crate) input: OperationOutput<'a>,
    mode: i32,
    data: Vec<u8>,
    owner: Option<ChownOpt>,
    timestamp: i64,
}

impl<'a> MkFile<'a> {
    pub fn new(
        path: impl Into<Utf8PathBuf>,
        input: OperationOutput<'a>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            path: path.into(),
            input,
            mode: 0o644,
            data: data.into(),
            owner: None,
            timestamp: -1,
        }
    }

    pub fn with_mode(mut self, mode: i32) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_owner(mut self, owner: ChownOpt) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub(crate) fn to_pb(&self) -> pb::FileActionMkFile {
        pb::FileActionMkFile {
            path: self.path.to_string(),
            mode: self.mode,
            data: self.data.clone(),
            owner: self.owner.as_ref().map(|o| o.to_pb()),
            timestamp: self.timestamp,
        }
    }
}

impl<'a> From<MkFile<'a>> for FileAction<'a> {
    fn from(mkfile: MkFile<'a>) -> Self {
        Self::MkFile(mkfile)
    }
}
