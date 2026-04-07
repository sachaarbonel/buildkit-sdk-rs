use buildkit_rs_proto::pb;
use camino::Utf8PathBuf;

use crate::utils::OperationOutput;

use super::FileAction;

/// Remove a file or directory.
#[derive(Debug)]
pub struct Rm<'a> {
    pub(crate) path: Utf8PathBuf,
    pub(crate) input: OperationOutput<'a>,
    allow_not_found: bool,
    allow_wildcard: bool,
}

impl<'a> Rm<'a> {
    pub fn new(path: impl Into<Utf8PathBuf>, input: OperationOutput<'a>) -> Self {
        Self {
            path: path.into(),
            input,
            allow_not_found: false,
            allow_wildcard: false,
        }
    }

    pub fn with_allow_not_found(mut self, allow: bool) -> Self {
        self.allow_not_found = allow;
        self
    }

    pub fn with_allow_wildcard(mut self, allow: bool) -> Self {
        self.allow_wildcard = allow;
        self
    }

    pub(crate) fn to_pb(&self) -> pb::FileActionRm {
        pb::FileActionRm {
            path: self.path.to_string(),
            allow_not_found: self.allow_not_found,
            allow_wildcard: self.allow_wildcard,
        }
    }
}

impl<'a> From<Rm<'a>> for FileAction<'a> {
    fn from(rm: Rm<'a>) -> Self {
        Self::Rm(rm)
    }
}
