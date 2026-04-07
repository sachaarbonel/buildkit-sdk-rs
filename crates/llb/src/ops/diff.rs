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

/// Compute the diff between two inputs.
///
/// The lower input is the base and the upper input is the changed state.
/// The output is the set of changes (additions, modifications, deletions)
/// between the lower and upper.
#[derive(Debug)]
pub struct Diff<'a> {
    id: OperationId,
    metadata: OpMetadata,
    lower: Option<OperationOutput<'a>>,
    upper: Option<OperationOutput<'a>>,
}

impl Diff<'_> {
    /// Create a new Diff operation.
    ///
    /// Either `lower` or `upper` can be `None` to represent scratch (empty filesystem).
    pub fn new<'a>(
        lower: Option<OperationOutput<'a>>,
        upper: Option<OperationOutput<'a>>,
    ) -> Diff<'a> {
        Diff {
            id: OperationId::new(),
            metadata: OpMetadata::new(),
            lower,
            upper,
        }
    }
}

impl Operation for Diff<'_> {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, cx: &mut Context) -> Option<Node> {
        let mut pb_inputs = Vec::new();
        let mut input_index: i64 = 0;

        let lower_pb = if let Some(ref lower) = self.lower {
            let node = cx.register(lower.operation())?;
            let idx = input_index;
            pb_inputs.push(pb::Input {
                digest: node.digest.clone(),
                index: lower.output().into(),
            });
            input_index += 1;
            Some(pb::LowerDiffInput { input: idx })
        } else {
            Some(pb::LowerDiffInput { input: -1 })
        };

        let upper_pb = if let Some(ref upper) = self.upper {
            let node = cx.register(upper.operation())?;
            let idx = input_index;
            pb_inputs.push(pb::Input {
                digest: node.digest.clone(),
                index: upper.output().into(),
            });
            let _ = idx;
            Some(pb::UpperDiffInput { input: idx })
        } else {
            Some(pb::UpperDiffInput { input: -1 })
        };

        Some(Node::new(
            Op {
                op: Some(OpEnum::Diff(pb::DiffOp {
                    lower: lower_pb,
                    upper: upper_pb,
                })),
                inputs: pb_inputs,
                ..Default::default()
            },
            self.metadata.clone().into(),
        ))
    }
}

impl OpMetadataBuilder for Diff<'_> {
    fn metadata(&self) -> &OpMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut OpMetadata {
        &mut self.metadata
    }
}

impl<'a> SingleBorrowedOutput<'a> for Diff<'a> {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl<'a> SingleOwnedOutput<'a> for Arc<Diff<'a>> {
    fn output(&self) -> OperationOutput<'a> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}
