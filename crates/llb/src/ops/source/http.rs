use std::{collections::HashMap, sync::Arc};

use buildkit_rs_proto::pb::{self, Op, op::Op as OpEnum};

use crate::{
    ops::{
        metadata::{OpMetadata, OpMetadataBuilder, attr::Attr},
        output::{SingleBorrowedOutput, SingleOwnedOutput},
    },
    serialize::{
        id::OperationId,
        node::{Context, Node, Operation},
    },
    utils::{OperationOutput, OutputIdx},
};

/// An HTTP source operation that fetches a file from an HTTP(S) URL.
#[derive(Debug, Clone)]
pub struct Http {
    id: OperationId,
    metadata: OpMetadata,

    /// The URL to fetch.
    url: String,
    /// Expected digest of the file (for verification).
    checksum: Option<String>,
    /// Override filename for the downloaded file.
    filename: Option<String>,
    /// File permissions (octal string, e.g. "0644").
    perm: Option<i32>,
    /// UID of the downloaded file.
    uid: Option<i32>,
    /// GID of the downloaded file.
    gid: Option<i32>,
}

impl Http {
    /// Create a new HTTP source with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            id: OperationId::new(),
            metadata: OpMetadata::new(),
            url: url.into(),
            checksum: None,
            filename: None,
            perm: None,
            uid: None,
            gid: None,
        }
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn with_perm(mut self, perm: i32) -> Self {
        self.perm = Some(perm);
        self
    }

    pub fn with_uid(mut self, uid: i32) -> Self {
        self.uid = Some(uid);
        self
    }

    pub fn with_gid(mut self, gid: i32) -> Self {
        self.gid = Some(gid);
        self
    }
}

impl Operation for Http {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, _: &mut Context) -> Option<Node> {
        let mut attrs = HashMap::default();

        if let Some(ref checksum) = self.checksum {
            attrs.insert(Attr::HTTP_CHECKSUM.into(), checksum.clone());
        }

        if let Some(ref filename) = self.filename {
            attrs.insert(Attr::HTTP_FILENAME.into(), filename.clone());
        }

        if let Some(perm) = self.perm {
            attrs.insert(Attr::HTTP_PERM.into(), format!("{perm:o}"));
        }

        if let Some(uid) = self.uid {
            attrs.insert(Attr::HTTP_UID.into(), uid.to_string());
        }

        if let Some(gid) = self.gid {
            attrs.insert(Attr::HTTP_GID.into(), gid.to_string());
        }

        Some(Node::new(
            Op {
                op: Some(OpEnum::Source(pb::SourceOp {
                    identifier: self.url.clone(),
                    attrs,
                })),

                ..Default::default()
            },
            self.metadata.clone().into(),
        ))
    }
}

impl OpMetadataBuilder for Http {
    fn metadata(&self) -> &OpMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut OpMetadata {
        &mut self.metadata
    }
}

impl<'a> SingleBorrowedOutput<'a> for Http {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl SingleOwnedOutput<'static> for Arc<Http> {
    fn output(&self) -> OperationOutput<'static> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}
