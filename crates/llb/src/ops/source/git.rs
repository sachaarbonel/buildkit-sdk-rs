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

/// A Git source operation that fetches a Git repository.
#[derive(Debug, Clone)]
pub struct Git {
    id: OperationId,
    metadata: OpMetadata,

    /// The remote URL of the Git repository.
    remote: String,
    /// The ref to check out (branch, tag, or commit).
    git_ref: String,
    /// Whether to keep the `.git` directory.
    keep_git_dir: bool,
    /// Subdir within the repo to use as the root.
    subdir: Option<String>,
    /// Auth header secret name.
    auth_header_secret: Option<String>,
    /// Auth token secret name.
    auth_token_secret: Option<String>,
    /// Known SSH hosts content.
    known_ssh_hosts: Option<String>,
    /// Whether to mount the SSH socket.
    mount_ssh_sock: Option<String>,
}

impl Git {
    /// Create a new Git source with the given remote URL and ref.
    pub fn new(remote: impl Into<String>, git_ref: impl Into<String>) -> Self {
        Self {
            id: OperationId::new(),
            metadata: OpMetadata::new(),
            remote: remote.into(),
            git_ref: git_ref.into(),
            keep_git_dir: false,
            subdir: None,
            auth_header_secret: None,
            auth_token_secret: None,
            known_ssh_hosts: None,
            mount_ssh_sock: None,
        }
    }

    pub fn with_keep_git_dir(mut self, keep: bool) -> Self {
        self.keep_git_dir = keep;
        self
    }

    pub fn with_subdir(mut self, subdir: impl Into<String>) -> Self {
        self.subdir = Some(subdir.into());
        self
    }

    pub fn with_auth_header_secret(mut self, secret: impl Into<String>) -> Self {
        self.auth_header_secret = Some(secret.into());
        self
    }

    pub fn with_auth_token_secret(mut self, secret: impl Into<String>) -> Self {
        self.auth_token_secret = Some(secret.into());
        self
    }

    pub fn with_known_ssh_hosts(mut self, hosts: impl Into<String>) -> Self {
        self.known_ssh_hosts = Some(hosts.into());
        self
    }

    pub fn with_mount_ssh_sock(mut self, sock: impl Into<String>) -> Self {
        self.mount_ssh_sock = Some(sock.into());
        self
    }
}

impl Operation for Git {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, _: &mut Context) -> Option<Node> {
        let mut attrs = HashMap::default();

        if self.keep_git_dir {
            attrs.insert(Attr::KEEP_GIT_DIR.into(), "true".into());
        }

        attrs.insert(Attr::FULL_REMOTE_URL.into(), self.remote.clone());

        if let Some(ref secret) = self.auth_header_secret {
            attrs.insert(Attr::AUTH_HEADER_SECRET.into(), secret.clone());
        }

        if let Some(ref secret) = self.auth_token_secret {
            attrs.insert(Attr::AUTH_TOKEN_SECRET.into(), secret.clone());
        }

        if let Some(ref hosts) = self.known_ssh_hosts {
            attrs.insert(Attr::KNOWN_SSH_HOSTS.into(), hosts.clone());
        }

        if let Some(ref sock) = self.mount_ssh_sock {
            attrs.insert(Attr::MOUNT_SSH_SOCK.into(), sock.clone());
        }

        // Build the identifier: git://<remote>#<ref>
        let identifier = format!("git://{}#{}", self.remote, self.git_ref);

        Some(Node::new(
            Op {
                op: Some(OpEnum::Source(pb::SourceOp { identifier, attrs })),

                ..Default::default()
            },
            self.metadata.clone().into(),
        ))
    }
}

impl OpMetadataBuilder for Git {
    fn metadata(&self) -> &OpMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut OpMetadata {
        &mut self.metadata
    }
}

impl<'a> SingleBorrowedOutput<'a> for Git {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl SingleOwnedOutput<'static> for Arc<Git> {
    fn output(&self) -> OperationOutput<'static> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}
