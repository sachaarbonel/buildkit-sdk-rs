use buildkit_rs_proto::pb;
use camino::Utf8PathBuf;

use crate::utils::OperationOutput;

use super::FileAction;
use super::chown::ChownOpt;

/// Create a symbolic link.
#[derive(Debug)]
pub struct Symlink<'a> {
    pub(crate) oldpath: Utf8PathBuf,
    pub(crate) newpath: Utf8PathBuf,
    pub(crate) input: OperationOutput<'a>,
    owner: Option<ChownOpt>,
    timestamp: i64,
}

impl<'a> Symlink<'a> {
    pub fn new(
        oldpath: impl Into<Utf8PathBuf>,
        newpath: impl Into<Utf8PathBuf>,
        input: OperationOutput<'a>,
    ) -> Self {
        Self {
            oldpath: oldpath.into(),
            newpath: newpath.into(),
            input,
            owner: None,
            timestamp: -1,
        }
    }

    pub fn with_owner(mut self, owner: ChownOpt) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub(crate) fn to_pb(&self) -> pb::FileActionSymlink {
        pb::FileActionSymlink {
            oldpath: self.oldpath.to_string(),
            newpath: self.newpath.to_string(),
            owner: self.owner.as_ref().map(|o| o.to_pb()),
            timestamp: self.timestamp,
        }
    }
}

impl<'a> From<Symlink<'a>> for FileAction<'a> {
    fn from(symlink: Symlink<'a>) -> Self {
        Self::Symlink(symlink)
    }
}
