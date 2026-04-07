use std::{collections::HashMap, sync::Arc};

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

/// A nested build invocation.
///
/// BuildOp is experimental and can break without backwards compatibility.
#[derive(Debug, Clone)]
pub struct Build {
    id: OperationId,
    metadata: OpMetadata,

    /// Builder input index.
    builder: i64,
    /// Definition for the nested build.
    def: Option<pb::Definition>,
    /// Attributes for the build.
    attrs: HashMap<String, String>,
}

impl Default for Build {
    fn default() -> Self {
        Self::new()
    }
}

impl Build {
    /// Create a new BuildOp.
    pub fn new() -> Self {
        Self {
            id: OperationId::new(),
            metadata: OpMetadata::new(),
            builder: 0,
            def: None,
            attrs: HashMap::new(),
        }
    }

    pub fn with_builder(mut self, builder: i64) -> Self {
        self.builder = builder;
        self
    }

    pub fn with_definition(mut self, def: pb::Definition) -> Self {
        self.def = Some(def);
        self
    }

    pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(key.into(), value.into());
        self
    }
}

impl Operation for Build {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, _cx: &mut Context) -> Option<Node> {
        Some(Node::new(
            Op {
                op: Some(OpEnum::Build(pb::BuildOp {
                    builder: self.builder,
                    inputs: HashMap::new(),
                    def: self.def.clone(),
                    attrs: self.attrs.clone(),
                })),
                ..Default::default()
            },
            self.metadata.clone().into(),
        ))
    }
}

impl OpMetadataBuilder for Build {
    fn metadata(&self) -> &OpMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut OpMetadata {
        &mut self.metadata
    }
}

impl<'a> SingleBorrowedOutput<'a> for Build {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl SingleOwnedOutput<'static> for Arc<Build> {
    fn output(&self) -> OperationOutput<'static> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}
