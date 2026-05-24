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

    fn lower_source(text: &str) -> SurfaceModule {
        let source = SourceFile::new("main.veln", text);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        lower_surface_ast(&parsed.tree)
    }

    fn lower_source_allowing_diagnostics(text: &str) -> SurfaceModule {
        let source = SourceFile::new("main.veln", text);
        let parsed = parse(&source);
        lower_surface_ast(&parsed.tree)
    }

    fn expr_line(function: &Function, index: usize) -> &Expr {
        let BodyLineKind::Expr { expr } = &function.body[index].kind else {
            panic!("expected expression line");
        };
        expr
    }

    fn let_line(function: &Function, index: usize) -> (&Option<String>, &Option<String>, &Expr) {
        let BodyLineKind::Let {
            name,
            annotation,
            expr,
        } = &function.body[index].kind
        else {
            panic!("expected let line");
        };
        (name, annotation, expr)
    }

    fn collect_module_node_ids(module: &SurfaceModule) -> Vec<u32> {
        let mut ids = Vec::new();
        for function in &module.functions {
            collect_function_node_ids(function, &mut ids);
        }
        ids
    }

    fn collect_function_node_ids(function: &Function, ids: &mut Vec<u32>) {
        ids.push(function.node_id.as_u32());
        ids.extend(function.params.iter().map(|param| param.node_id.as_u32()));
        ids.extend(
            function
                .contracts
                .iter()
                .map(|contract| contract.node_id.as_u32()),
        );
        for line in &function.body {
            ids.push(line.node_id.as_u32());
            match &line.kind {
                BodyLineKind::Let { expr, .. } | BodyLineKind::Expr { expr } => {
                    collect_expr_node_ids(expr, ids);
                }
            }
        }
    }

    fn collect_expr_node_ids(expr: &Expr, ids: &mut Vec<u32>) {
        ids.push(expr.node_id.as_u32());
        match &expr.kind {
            ExprKind::Call { callee, args } => {
                collect_expr_node_ids(callee, ids);
                for arg in args {
                    collect_expr_node_ids(arg, ids);
                }
            }
            ExprKind::Try(expr) => collect_expr_node_ids(expr, ids),
            ExprKind::Record(fields) => {
                for field in fields {
                    ids.push(field.node_id.as_u32());
                    collect_expr_node_ids(&field.expr, ids);
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    collect_expr_node_ids(item, ids);
                }
            }
            ExprKind::Prefix { expr, .. } => collect_expr_node_ids(expr, ids),
            ExprKind::Binary { left, right, .. } => {
                collect_expr_node_ids(left, ids);
                collect_expr_node_ids(right, ids);
            }
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    #[test]
    fn assigns_session_stable_node_ids() {
        let module = lower_source("fn id(value: Int) -> Int\n  value\nend\n");

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
        let module = lower_source("fn todo() -> Unit\n  _answer\nend\n");

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

    #[test]
    fn lowers_function_metadata_contracts_and_let_lines() {
        let module = lower_source(concat!(
            "pub fn publish(user: User, count: Int) -> Result(Unit, Error) effects [db, log]\n",
            "  require user ready\n",
            "  ensure result ok\n",
            "  let message: String = \"ready\"\n",
            "  message\n",
            "end\n",
        ));

        let function = &module.functions[0];
        assert_eq!(function.visibility, Visibility::Public);
        assert_eq!(function.name.as_deref(), Some("publish"));
        assert_eq!(function.return_type.as_deref(), Some("Result(Unit, Error)"));
        assert_eq!(
            function.effects,
            Some(vec!["db".to_string(), "log".to_string()])
        );

        assert_eq!(function.params.len(), 2);
        assert_eq!(function.params[0].name, "user");
        assert_eq!(function.params[0].ty.as_deref(), Some("User"));
        assert_eq!(function.params[1].name, "count");
        assert_eq!(function.params[1].ty.as_deref(), Some("Int"));

        assert_eq!(function.contracts.len(), 2);
        assert_eq!(function.contracts[0].kind, ContractKind::Require);
        assert_eq!(function.contracts[0].text, "user ready");
        assert_eq!(function.contracts[1].kind, ContractKind::Ensure);
        assert_eq!(function.contracts[1].text, "result ok");

        let (name, annotation, expr) = let_line(function, 0);
        assert_eq!(name.as_deref(), Some("message"));
        assert_eq!(annotation.as_deref(), Some("String"));
        assert!(matches!(&expr.kind, ExprKind::StringLiteral(value) if value == "\"ready\""));

        assert!(matches!(
            &expr_line(function, 1).kind,
            ExprKind::NamePath(segments) if segments == &vec!["message".to_string()]
        ));
    }

    #[test]
    fn lowers_nested_expression_edge_cases() {
        let module = lower_source(concat!(
            "fn build(input: Int) -> Unit\n",
            "  let data = {answer: [1, 2.5, -input?], check: _value satisfy candidate => candidate > 0}\n",
            "  data |> sink(\"ok\", ())\n",
            "end\n",
        ));
        let function = &module.functions[0];

        let (_, _, expr) = let_line(function, 0);
        let ExprKind::Record(fields) = &expr.kind else {
            panic!("expected record expression");
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "answer");
        let ExprKind::List(items) = &fields[0].expr.kind else {
            panic!("expected list field expression");
        };
        assert!(matches!(&items[0].kind, ExprKind::IntLiteral(value) if value == "1"));
        assert!(matches!(&items[1].kind, ExprKind::FloatLiteral(value) if value == "2.5"));
        assert!(matches!(
            &items[2].kind,
            ExprKind::Prefix {
                op: PrefixOp::Negate,
                expr,
            } if matches!(
                &expr.kind,
                ExprKind::Try(inner)
                    if matches!(&inner.kind, ExprKind::NamePath(segments) if segments == &vec!["input".to_string()])
            )
        ));

        assert_eq!(fields[1].name, "check");
        let ExprKind::Hole {
            name,
            satisfy: Some(satisfy),
        } = &fields[1].expr.kind
        else {
            panic!("expected hole with satisfy clause");
        };
        assert_eq!(name.as_deref(), Some("value"));
        assert_eq!(satisfy.candidate.as_deref(), Some("candidate"));
        assert_eq!(satisfy.predicate, "candidate > 0");

        let ExprKind::Binary {
            op: BinaryOp::PipeGreater,
            left,
            right,
        } = &expr_line(function, 1).kind
        else {
            panic!("expected pipe expression");
        };
        assert!(
            matches!(&left.kind, ExprKind::NamePath(segments) if segments == &vec!["data".to_string()])
        );
        let ExprKind::Call { callee, args } = &right.kind else {
            panic!("expected call on right side of pipe");
        };
        assert!(
            matches!(&callee.kind, ExprKind::NamePath(segments) if segments == &vec!["sink".to_string()])
        );
        assert!(matches!(&args[0].kind, ExprKind::StringLiteral(value) if value == "\"ok\""));
        assert!(matches!(&args[1].kind, ExprKind::Unit));
    }

    #[test]
    fn allocates_unique_contiguous_node_ids_across_nested_nodes_and_functions() {
        let module = lower_source(concat!(
            "fn first() -> Unit\n",
            "  {x: [1]}\n",
            "end\n",
            "fn second() -> Unit\n",
            "  _\n",
            "end\n",
        ));

        let mut ids = collect_module_node_ids(&module);
        ids.sort_unstable();
        assert_eq!(ids, (1..=ids.len() as u32).collect::<Vec<_>>());
        assert_eq!(module.functions[0].node_id.as_u32(), 1);
        assert!(module.functions[1].node_id > module.functions[0].node_id);
    }

    #[test]
    fn lowers_missing_let_initializers_to_missing_expressions() {
        let module = lower_source_allowing_diagnostics("fn broken() -> Unit\n  let value =\nend\n");
        let (_, _, expr) = let_line(&module.functions[0], 0);

        assert!(matches!(&expr.kind, ExprKind::Missing));
    }
}
