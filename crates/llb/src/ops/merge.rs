use std::sync::Arc;

use buildkit_rs_proto::pb::{self, Op, op::Op as OpEnum};

use crate::{
    OpMetadataBuilder,
    ops::output::{SingleBorrowedOutput, SingleOwnedOutput},
    serialize::{
        id::OperationId,
        node::{Context, Node, Operation},
    },
    utils::{OperationOutput, OutputIdx},
};

use super::metadata::OpMetadata;

/// Merge multiple inputs into a single output by overlaying them.
#[derive(Debug)]
pub struct Merge<'a> {
    id: OperationId,
    metadata: OpMetadata,
    inputs: Vec<OperationOutput<'a>>,
}

impl Merge<'_> {
    pub fn new<'a>(inputs: Vec<OperationOutput<'a>>) -> Merge<'a> {
        Merge {
            id: OperationId::new(),
            metadata: OpMetadata::new(),
            inputs,
        }
    }
}

impl Operation for Merge<'_> {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, cx: &mut Context) -> Option<Node> {
        let mut pb_inputs = Vec::new();
        let mut merge_inputs = Vec::new();

        for (i, input) in self.inputs.iter().enumerate() {
            let node = cx.register(input.operation())?;
            pb_inputs.push(pb::Input {
                digest: node.digest.clone(),
                index: input.output().into(),
            });
            merge_inputs.push(pb::MergeInput { input: i as i64 });
        }

        Some(Node::new(
            Op {
                op: Some(OpEnum::Merge(pb::MergeOp {
                    inputs: merge_inputs,
                })),
                inputs: pb_inputs,
                ..Default::default()
            },
            self.metadata.clone().into(),
        ))
    }
}

impl OpMetadataBuilder for Merge<'_> {
    fn metadata(&self) -> &OpMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut OpMetadata {
        &mut self.metadata
    }
}

impl<'a> SingleBorrowedOutput<'a> for Merge<'a> {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl<'a> SingleOwnedOutput<'a> for Arc<Merge<'a>> {
    fn output(&self) -> OperationOutput<'a> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}
