use veln_syntax::{
    BinaryOp as SyntaxBinaryOp, BodyLine as SyntaxBodyLine, ContractKind as SyntaxContractKind,
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, FunctionDecl as SyntaxFunction,
    PrefixOp as SyntaxPrefixOp, RecordField as SyntaxRecordField, SyntaxItem, SyntaxTree,
    Visibility as SyntaxVisibility,
};

use crate::{
    BinaryOp, BodyLine, BodyLineKind, Contract, ContractKind, Expr, ExprKind, Function,
    FunctionKind, NodeId, Param, PrefixOp, RecordField, SurfaceModule, Visibility,
};

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
        let node_id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;
        node_id
    }

    fn lower_function(&mut self, function: &SyntaxFunction) -> Function {
        Function {
            node_id: self.alloc(),
            kind: match function.kind {
                veln_syntax::FunctionKind::Function => FunctionKind::Function,
                veln_syntax::FunctionKind::Test => FunctionKind::Test,
            },
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
                    satisfy: satisfy.as_ref().map(crate::satisfy::lower_satisfy_clause),
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
