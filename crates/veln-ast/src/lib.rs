//! Arena-backed surface AST and node handles.

use veln_source::SourceSpan;
use veln_syntax::{
    BinaryOp as SyntaxBinaryOp, BodyLine as SyntaxBodyLine, ContractKind as SyntaxContractKind,
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, FunctionDecl as SyntaxFunction,
    PrefixOp as SyntaxPrefixOp, RecordField as SyntaxRecordField,
    SatisfyClause as SyntaxSatisfyClause, SyntaxItem, SyntaxTree, Visibility as SyntaxVisibility,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    pub fn as_u32(self) -> u32 {
        self.0
    }

    pub fn display(self, prefix: &str) -> String {
        format!("{prefix}-{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceModule {
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub node_id: NodeId,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub effects: Option<Vec<String>>,
    pub contracts: Vec<Contract>,
    pub body: Vec<BodyLine>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub node_id: NodeId,
    pub name: String,
    pub ty: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct Contract {
    pub node_id: NodeId,
    pub kind: ContractKind,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Require,
    Ensure,
}

#[derive(Clone, Debug)]
pub struct BodyLine {
    pub node_id: NodeId,
    pub kind: BodyLineKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum BodyLineKind {
    Let {
        name: Option<String>,
        annotation: Option<String>,
        expr: Expr,
    },
    Expr {
        expr: Expr,
    },
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub node_id: NodeId,
    pub kind: ExprKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Missing,
    Hole {
        name: Option<String>,
        satisfy: Option<SatisfyClause>,
    },
    NamePath(Vec<String>),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    Unit,
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Try(Box<Expr>),
    Record(Vec<RecordField>),
    List(Vec<Expr>),
    Prefix {
        op: PrefixOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Clone, Debug)]
pub struct SatisfyClause {
    pub candidate: Option<String>,
    pub predicate: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct RecordField {
    pub node_id: NodeId,
    pub name: String,
    pub expr: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    PipeGreater,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

pub fn lower_surface_ast(tree: &SyntaxTree) -> SurfaceModule {
    let mut builder = AstBuilder { next_node_id: 1 };
    let mut functions = Vec::new();

    for item in &tree.items {
        let SyntaxItem::Function(function) = item;
        functions.push(builder.lower_function(function));
    }

    SurfaceModule { functions }
}

struct AstBuilder {
    next_node_id: u32,
}

impl AstBuilder {
    fn alloc(&mut self) -> NodeId {
        let node_id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        node_id
    }

    fn lower_function(&mut self, function: &SyntaxFunction) -> Function {
        Function {
            node_id: self.alloc(),
            visibility: match function.visibility {
                SyntaxVisibility::Public => Visibility::Public,
                SyntaxVisibility::Private => Visibility::Private,
            },
            name: function.name.clone(),
            params: function
                .params
                .iter()
                .map(|param| Param {
                    node_id: self.alloc(),
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    span: param.span.clone(),
                })
                .collect(),
            return_type: function.return_type.clone(),
            effects: function.effects.clone(),
            contracts: function
                .contracts
                .iter()
                .map(|contract| Contract {
                    node_id: self.alloc(),
                    kind: match contract.kind {
                        SyntaxContractKind::Require => ContractKind::Require,
                        SyntaxContractKind::Ensure => ContractKind::Ensure,
                    },
                    text: contract.text.clone(),
                    span: contract.span.clone(),
                })
                .collect(),
            body: function
                .body
                .iter()
                .map(|line| match line {
                    SyntaxBodyLine::Let {
                        name,
                        annotation,
                        expr,
                        span,
                    } => BodyLine {
                        node_id: self.alloc(),
                        kind: BodyLineKind::Let {
                            name: name.clone(),
                            annotation: annotation.clone(),
                            expr: self.lower_expr(expr),
                        },
                        span: span.clone(),
                    },
                    SyntaxBodyLine::Expr { expr, span } => BodyLine {
                        node_id: self.alloc(),
                        kind: BodyLineKind::Expr {
                            expr: self.lower_expr(expr),
                        },
                        span: span.clone(),
                    },
                })
                .collect(),
            span: function.span.clone(),
        }
    }

    fn lower_expr(&mut self, expr: &SyntaxExpr) -> Expr {
        Expr {
            node_id: self.alloc(),
            kind: match &expr.kind {
                SyntaxExprKind::Missing => ExprKind::Missing,
                SyntaxExprKind::Hole { name, satisfy } => ExprKind::Hole {
                    name: name.clone(),
                    satisfy: satisfy.as_ref().map(lower_satisfy_clause),
                },
                SyntaxExprKind::NamePath(segments) => ExprKind::NamePath(segments.clone()),
                SyntaxExprKind::StringLiteral(value) => ExprKind::StringLiteral(value.clone()),
                SyntaxExprKind::IntLiteral(value) => ExprKind::IntLiteral(value.clone()),
                SyntaxExprKind::FloatLiteral(value) => ExprKind::FloatLiteral(value.clone()),
                SyntaxExprKind::Unit => ExprKind::Unit,
                SyntaxExprKind::Call { callee, args } => ExprKind::Call {
                    callee: Box::new(self.lower_expr(callee)),
                    args: args.iter().map(|arg| self.lower_expr(arg)).collect(),
                },
                SyntaxExprKind::Try(expr) => ExprKind::Try(Box::new(self.lower_expr(expr))),
                SyntaxExprKind::Record(fields) => ExprKind::Record(
                    fields
                        .iter()
                        .map(|field| self.lower_record_field(field))
                        .collect(),
                ),
                SyntaxExprKind::List(items) => {
                    ExprKind::List(items.iter().map(|item| self.lower_expr(item)).collect())
                }
                SyntaxExprKind::Prefix { op, expr } => ExprKind::Prefix {
                    op: match op {
                        SyntaxPrefixOp::Not => PrefixOp::Not,
                        SyntaxPrefixOp::Negate => PrefixOp::Negate,
                    },
                    expr: Box::new(self.lower_expr(expr)),
                },
                SyntaxExprKind::Binary { op, left, right } => ExprKind::Binary {
                    op: match op {
                        SyntaxBinaryOp::PipeGreater => BinaryOp::PipeGreater,
                        SyntaxBinaryOp::Or => BinaryOp::Or,
                        SyntaxBinaryOp::And => BinaryOp::And,
                        SyntaxBinaryOp::Equal => BinaryOp::Equal,
                        SyntaxBinaryOp::NotEqual => BinaryOp::NotEqual,
                        SyntaxBinaryOp::Less => BinaryOp::Less,
                        SyntaxBinaryOp::LessEqual => BinaryOp::LessEqual,
                        SyntaxBinaryOp::Greater => BinaryOp::Greater,
                        SyntaxBinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
                        SyntaxBinaryOp::Add => BinaryOp::Add,
                        SyntaxBinaryOp::Subtract => BinaryOp::Subtract,
                        SyntaxBinaryOp::Multiply => BinaryOp::Multiply,
                        SyntaxBinaryOp::Divide => BinaryOp::Divide,
                    },
                    left: Box::new(self.lower_expr(left)),
                    right: Box::new(self.lower_expr(right)),
                },
            },
            span: expr.span.clone(),
        }
    }

    fn lower_record_field(&mut self, field: &SyntaxRecordField) -> RecordField {
        RecordField {
            node_id: self.alloc(),
            name: field.name.clone(),
            expr: self.lower_expr(&field.expr),
            span: field.span.clone(),
        }
    }
}

fn lower_satisfy_clause(clause: &SyntaxSatisfyClause) -> SatisfyClause {
    SatisfyClause {
        candidate: clause.candidate.clone(),
        predicate: clause.predicate.clone(),
        span: clause.span.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_source::SourceFile;
    use veln_syntax::parse;

    #[test]
    fn assigns_session_stable_node_ids() {
        let source = SourceFile::new("main.veln", "fn id(value: Int) -> Int\n  value\nend\n");
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        assert_eq!(module.functions[0].node_id.display("fn"), "fn-1");
        assert_eq!(
            module.functions[0].params[0].node_id.display("param"),
            "param-2"
        );
        assert_eq!(
            module.functions[0].body[0].node_id.display("expr"),
            "expr-3"
        );
        let BodyLineKind::Expr { expr } = &module.functions[0].body[0].kind else {
            panic!("expected expression line");
        };
        assert_eq!(expr.node_id.display("expr"), "expr-4");
    }

    #[test]
    fn lowers_holes_to_node_id_backed_expression_nodes() {
        let source = SourceFile::new("main.veln", "fn todo() -> Unit\n  _answer\nend\n");
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let BodyLineKind::Expr { expr } = &module.functions[0].body[0].kind else {
            panic!("expected expression line");
        };
        assert_eq!(expr.node_id.display("expr"), "expr-3");
        assert!(matches!(
            &expr.kind,
            ExprKind::Hole {
                name: Some(name), ..
            } if name == "answer"
        ));
    }
}
