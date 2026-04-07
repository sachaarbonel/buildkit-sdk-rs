pub mod chown;
pub mod copy;
pub mod mkdir;
pub mod mkfile;
pub mod rm;
pub mod symlink;

use std::sync::Arc;

use buildkit_rs_proto::pb::{self, FileOp, Op, op::Op as OpEnum};
pub use copy::Copy;
pub use mkdir::Mkdir;
pub use mkfile::MkFile;
pub use rm::Rm;
pub use symlink::Symlink;

use crate::{
    MultiBorrowedOutput, MultiOwnedOutput, OpMetadataBuilder,
    serialize::{
        id::OperationId,
        node::{Context, Node, Operation},
    },
    utils::{OperationOutput, OutputIdx},
};

use super::metadata::OpMetadata;

#[derive(Debug)]
pub enum FileAction<'a> {
    Copy(Copy<'a>),
    Mkdir(Mkdir<'a>),
    MkFile(MkFile<'a>),
    Rm(Rm<'a>),
    Symlink(Symlink<'a>),
}

#[derive(Debug)]
pub struct FileActions<'a> {
    id: OperationId,
    metadata: OpMetadata,

    actions: Vec<FileAction<'a>>,
}

impl Default for FileActions<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl FileActions<'_> {
    pub fn new() -> Self {
        Self {
            id: OperationId::new(),
            metadata: OpMetadata::new(),
            actions: Vec::new(),
        }
    }
}

impl<'a> FileActions<'a> {
    pub fn with_action(mut self, action: impl Into<FileAction<'a>>) -> Self {
        self.actions.push(action.into());
        self
    }
}

impl<'b> MultiBorrowedOutput<'b> for FileActions<'b> {
    fn output(&'b self, index: u32) -> OperationOutput<'b> {
        // TODO: check if the requested index available.
        OperationOutput::borrowed(self, OutputIdx(index))
    }
}

impl<'a> MultiOwnedOutput<'a> for Arc<FileActions<'a>> {
    fn output(&self, index: u32) -> OperationOutput<'a> {
        // TODO: check if the requested index available.
        OperationOutput::owned(self.clone(), OutputIdx(index))
    }
}

impl Operation for FileActions<'_> {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, cx: &mut Context) -> Option<Node> {
        let mut actions = Vec::new();
        let mut inputs: Vec<pb::Input> = Vec::new();
        let mut input_index: i64 = 0;

        for action in &self.actions {
            let (pb_action, action_input, secondary_input, output) = match action {
                FileAction::Copy(copy) => {
                    let src_node = cx.register(copy.src_input.operation())?;
                    let src_idx = input_index;
                    inputs.push(pb::Input {
                        digest: src_node.digest.clone(),
                        index: copy.src_input.output().into(),
                    });
                    input_index += 1;

                    let dst_node = cx.register(copy.dest_input.operation())?;
                    let dst_idx = input_index;
                    inputs.push(pb::Input {
                        digest: dst_node.digest.clone(),
                        index: copy.dest_input.output().into(),
                    });
                    input_index += 1;

                    (
                        pb::file_action::Action::Copy(copy.to_pb()),
                        dst_idx,
                        src_idx,
                        0_i64,
                    )
                }
                FileAction::Mkdir(mkdir) => {
                    let node = cx.register(mkdir.input.operation())?;
                    let idx = input_index;
                    inputs.push(pb::Input {
                        digest: node.digest.clone(),
                        index: mkdir.input.output().into(),
                    });
                    input_index += 1;

                    (
                        pb::file_action::Action::Mkdir(mkdir.to_pb()),
                        idx,
                        -1_i64,
                        0_i64,
                    )
                }
                FileAction::MkFile(mkfile) => {
                    let node = cx.register(mkfile.input.operation())?;
                    let idx = input_index;
                    inputs.push(pb::Input {
                        digest: node.digest.clone(),
                        index: mkfile.input.output().into(),
                    });
                    input_index += 1;

                    (
                        pb::file_action::Action::Mkfile(mkfile.to_pb()),
                        idx,
                        -1_i64,
                        0_i64,
                    )
                }
                FileAction::Rm(rm) => {
                    let node = cx.register(rm.input.operation())?;
                    let idx = input_index;
                    inputs.push(pb::Input {
                        digest: node.digest.clone(),
                        index: rm.input.output().into(),
                    });
                    input_index += 1;

                    (pb::file_action::Action::Rm(rm.to_pb()), idx, -1_i64, 0_i64)
                }
                FileAction::Symlink(symlink) => {
                    let node = cx.register(symlink.input.operation())?;
                    let idx = input_index;
                    inputs.push(pb::Input {
                        digest: node.digest.clone(),
                        index: symlink.input.output().into(),
                    });
                    input_index += 1;

                    (
                        pb::file_action::Action::Symlink(symlink.to_pb()),
                        idx,
                        -1_i64,
                        0_i64,
                    )
                }
            };

            actions.push(pb::FileAction {
                input: action_input,
                secondary_input,
                output,
                action: Some(pb_action),
            });
        }

        Some(Node::new(
            Op {
                op: Some(OpEnum::File(FileOp { actions })),
                inputs,
                ..Default::default()
            },
            self.metadata.clone().into(),
        ))
    }
}

impl OpMetadataBuilder for FileActions<'_> {
    fn metadata(&self) -> &OpMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut OpMetadata {
        &mut self.metadata
    }
}
