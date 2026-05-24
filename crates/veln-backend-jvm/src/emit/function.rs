use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{BinaryOp, PrefixOp};
use veln_ir::{IrCallTarget, IrExpr, IrExprKind, IrFunction, IrRecordField, IrStmt, IrStmtKind};

use crate::emit::program::ProgramEmitter;
use crate::java::{
    binary_method, java_string, sanitize_identifier_text, stdio_method, unique_java_identifier,
    veln_string_literal_value,
};

pub(crate) struct FunctionEmitter<'a, 'program> {
    program: &'a ProgramEmitter<'program>,
    function: &'a IrFunction,
    locals: BTreeMap<String, String>,
    used_names: BTreeSet<String>,
    temp_counter: usize,
}

impl<'a, 'program> FunctionEmitter<'a, 'program> {
    pub(crate) fn new(program: &'a ProgramEmitter<'program>, function: &'a IrFunction) -> Self {
        Self {
            program,
            function,
            locals: BTreeMap::new(),
            used_names: BTreeSet::new(),
            temp_counter: 0,
        }
    }

    pub(crate) fn emit(&mut self, out: &mut String) {
        let function_name = self.program.function_name(&self.function.name);
        let params = self
            .function
            .params
            .iter()
            .map(|param| {
                let java_name = self.bind_local(&param.name, "p");
                format!("Object {java_name}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    static Object {function_name}({params}) {{\n"));
        for stmt in &self.function.body {
            self.emit_stmt(out, stmt);
        }
        if !matches!(
            self.function.body.last().map(|stmt| &stmt.kind),
            Some(IrStmtKind::Return { .. })
        ) {
            out.push_str(&format!(
                "        return {}.UNIT;\n",
                self.program.options.runtime_class
            ));
        }
        out.push_str("    }\n");
    }

    fn emit_stmt(&mut self, out: &mut String, stmt: &IrStmt) {
        match &stmt.kind {
            IrStmtKind::Let { name, value, .. } => {
                let java_value = self.emit_expr(value);
                self.emit_prelude(out, &java_value.prelude);
                let java_name = self.bind_local(name, "v");
                out.push_str(&format!(
                    "        Object {java_name} = {};\n",
                    java_value.code
                ));
            }
            IrStmtKind::Expr { value } => {
                let java_value = self.emit_expr(value);
                self.emit_prelude(out, &java_value.prelude);
                out.push_str(&format!("        {};\n", java_value.code));
            }
            IrStmtKind::Return { value } => {
                let java_value = self.emit_expr(value);
                self.emit_prelude(out, &java_value.prelude);
                out.push_str(&format!("        return {};\n", java_value.code));
            }
        }
    }

    fn emit_expr(&mut self, expr: &IrExpr) -> JavaExpr {
        match &expr.kind {
            IrExprKind::Local(name) => JavaExpr::simple(self.local_name(name)),
            IrExprKind::BoolLiteral(value) => JavaExpr::simple(if *value {
                "Boolean.TRUE".to_string()
            } else {
                "Boolean.FALSE".to_string()
            }),
            IrExprKind::StringLiteral(value) => {
                JavaExpr::simple(java_string(&veln_string_literal_value(value)))
            }
            IrExprKind::IntLiteral(value) => JavaExpr::simple(format!("Long.valueOf({value}L)")),
            IrExprKind::FloatLiteral(value) => {
                JavaExpr::simple(format!("Double.valueOf({value}D)"))
            }
            IrExprKind::Unit => {
                JavaExpr::simple(format!("{}.UNIT", self.program.options.runtime_class))
            }
            IrExprKind::ResultOk(value) => self.emit_unary_runtime("ok", value),
            IrExprKind::ResultErr(value) => self.emit_unary_runtime("err", value),
            IrExprKind::OptionSome(value) => self.emit_unary_runtime("some", value),
            IrExprKind::Call { target, args } => self.emit_call(expr, target, args),
            IrExprKind::Try(value) => self.emit_try(value),
            IrExprKind::Record(fields) => self.emit_record(fields),
            IrExprKind::List(items) => self.emit_list(items),
            IrExprKind::Prefix { op, expr } => {
                let method = match op {
                    PrefixOp::Not => "not",
                    PrefixOp::Negate => "negate",
                };
                self.emit_unary_runtime(method, expr)
            }
            IrExprKind::Binary { op, left, right } => self.emit_binary(*op, left, right),
        }
    }

    fn emit_call(&mut self, expr: &IrExpr, target: &IrCallTarget, args: &[IrExpr]) -> JavaExpr {
        let mut prelude = Vec::new();
        let java_args = args
            .iter()
            .map(|arg| {
                let java_arg = self.emit_expr(arg);
                prelude.extend(java_arg.prelude);
                java_arg.code
            })
            .collect::<Vec<_>>();
        let code = match target {
            IrCallTarget::Function(name) => {
                format!(
                    "{}({})",
                    self.program.function_name(name),
                    java_args.join(", ")
                )
            }
            IrCallTarget::StdioBuiltin(name) => {
                let mut all_args = java_args;
                all_args.push(java_string(&expr.node_id.display("call")));
                all_args.push(java_string(expr.span.file.as_str()));
                format!(
                    "{}.{}({})",
                    self.program.options.runtime_class,
                    stdio_method(name),
                    all_args.join(", ")
                )
            }
            IrCallTarget::Value(name) => {
                let mut all_args = Vec::with_capacity(java_args.len() + 1);
                all_args.push(self.local_name(name));
                all_args.extend(java_args);
                format!(
                    "{}.call({})",
                    self.program.options.runtime_class,
                    all_args.join(", ")
                )
            }
        };
        JavaExpr { prelude, code }
    }

    fn emit_try(&mut self, value: &IrExpr) -> JavaExpr {
        let java_value = self.emit_expr(value);
        let temp = self.next_temp("try");
        let mut prelude = java_value.prelude;
        prelude.push(format!("Object {temp} = {};", java_value.code));
        prelude.push(format!(
            "if ({}.isErr({temp})) {{",
            self.program.options.runtime_class
        ));
        prelude.push(format!("    return {temp};"));
        prelude.push("}".to_string());
        JavaExpr {
            prelude,
            code: format!("{}.unwrapOk({temp})", self.program.options.runtime_class),
        }
    }

    fn emit_record(&mut self, fields: &[IrRecordField]) -> JavaExpr {
        let mut prelude = Vec::new();
        let mut args = Vec::new();
        for field in fields {
            let value = self.emit_expr(&field.value);
            prelude.extend(value.prelude);
            args.push(java_string(&field.name));
            args.push(value.code);
        }
        JavaExpr {
            prelude,
            code: format!(
                "{}.record({})",
                self.program.options.runtime_class,
                args.join(", ")
            ),
        }
    }

    fn emit_list(&mut self, items: &[IrExpr]) -> JavaExpr {
        let mut prelude = Vec::new();
        let args = items
            .iter()
            .map(|item| {
                let value = self.emit_expr(item);
                prelude.extend(value.prelude);
                value.code
            })
            .collect::<Vec<_>>();
        JavaExpr {
            prelude,
            code: format!(
                "{}.list({})",
                self.program.options.runtime_class,
                args.join(", ")
            ),
        }
    }

    fn emit_binary(&mut self, op: BinaryOp, left: &IrExpr, right: &IrExpr) -> JavaExpr {
        let left = self.emit_expr(left);
        let right = self.emit_expr(right);
        let mut prelude = left.prelude;
        prelude.extend(right.prelude);
        JavaExpr {
            prelude,
            code: format!(
                "{}.{}({}, {})",
                self.program.options.runtime_class,
                binary_method(op),
                left.code,
                right.code
            ),
        }
    }

    fn emit_unary_runtime(&mut self, method: &str, value: &IrExpr) -> JavaExpr {
        let value = self.emit_expr(value);
        JavaExpr {
            prelude: value.prelude,
            code: format!(
                "{}.{method}({})",
                self.program.options.runtime_class, value.code
            ),
        }
    }

    fn emit_prelude(&self, out: &mut String, prelude: &[String]) {
        for line in prelude {
            out.push_str("        ");
            out.push_str(line);
            out.push('\n');
        }
    }

    fn bind_local(&mut self, source_name: &str, prefix: &str) -> String {
        let java_name = unique_java_identifier(
            &format!("{prefix}_{}", sanitize_identifier_text(source_name)),
            &mut self.used_names,
        );
        self.locals
            .insert(source_name.to_string(), java_name.clone());
        java_name
    }

    fn local_name(&self, source_name: &str) -> String {
        self.locals
            .get(source_name)
            .cloned()
            .unwrap_or_else(|| format!("v_{}", sanitize_identifier_text(source_name)))
    }

    fn next_temp(&mut self, label: &str) -> String {
        let name = format!("__{label}{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
}

#[derive(Clone, Debug)]
struct JavaExpr {
    prelude: Vec<String>,
    code: String,
}

impl JavaExpr {
    fn simple(code: String) -> Self {
        Self {
            prelude: Vec::new(),
            code,
        }
    }
}
