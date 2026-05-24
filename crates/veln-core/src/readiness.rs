use veln_ast::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreReadiness {
    Complete,
    Blocked(Vec<CoreBlocker>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreBlocker {
    Hole { node_id: NodeId },
    MissingExpression { node_id: NodeId },
    UnsupportedExpression { node_id: NodeId, reason: String },
}
