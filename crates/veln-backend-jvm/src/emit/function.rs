use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{BinaryOp, ContractKind, PrefixOp};
use veln_ir::{
    ContractObligationStatus, IrCallTarget, IrContract, IrDictEntry, IrExpr, IrExprKind,
    IrFunction, IrMatchArm, IrPattern, IrPatternField, IrPatternKind, IrRecordField, IrStmt,
    IrStmtKind,
};

use crate::emit::program::ProgramEmitter;
use crate::java::{
    binary_method, concurrency_method, java_string, prelude_method, sanitize_identifier_text,
    stdio_method, unique_java_identifier, veln_string_literal_value,
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
        for contract in self
            .function
            .contracts
            .iter()
            .filter(|contract| contract.kind == ContractKind::Require)
        {
            self.emit_contract_check(out, contract);
        }
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
                if self.has_ensure_contracts() {
                    let result = self.next_temp("result");
                    out.push_str(&format!("        Object {result} = {};\n", java_value.code));
                    self.emit_ensure_checks_for_result(out, &result);
                    out.push_str(&format!("        return {result};\n"));
                } else {
                    out.push_str(&format!("        return {};\n", java_value.code));
                }
            }
        }
    }

    fn emit_contract_check(&mut self, out: &mut String, contract: &IrContract) {
        if contract.obligation_status != ContractObligationStatus::RuntimeRequired {
            return;
        }
        let Some(line) = self.contract_check_line(contract) else {
            return;
        };
        out.push_str("        ");
        out.push_str(&line);
        out.push('\n');
    }

    fn contract_check_line(&self, contract: &IrContract) -> Option<String> {
        let Some(predicate) = ContractParser::new(&contract.predicate).parse() else {
            return None;
        };
        let java_predicate = self.emit_contract_expr(&predicate);
        let blame = match contract.kind {
            ContractKind::Require => "caller",
            ContractKind::Ensure => "implementation",
        };
        let clause = match contract.kind {
            ContractKind::Require => "require",
            ContractKind::Ensure => "ensure",
        };
        Some(format!(
            "{}.checkContract({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {});",
            self.program.options.runtime_class,
            java_predicate,
            java_string(clause),
            java_string(&contract.predicate),
            java_string(&self.function.name),
            java_string(blame),
            java_string(&contract.node_id.display("contract")),
            java_string(contract.span.file.as_str()),
            contract.span.start.line,
            contract.span.start.column,
            contract.span.end.line,
            contract.span.end.column
        ))
    }

    fn has_ensure_contracts(&self) -> bool {
        self.function
            .contracts
            .iter()
            .any(|contract| contract.kind == ContractKind::Ensure)
    }

    fn emit_ensure_checks_for_result(&mut self, out: &mut String, result: &str) {
        for line in self.ensure_check_lines_for_result(result) {
            out.push_str("        ");
            out.push_str(&line);
            out.push('\n');
        }
    }

    fn ensure_check_lines_for_result(&mut self, result: &str) -> Vec<String> {
        let previous = self.function.return_binding.as_ref().map(|binding| {
            (
                binding.clone(),
                self.locals.insert(binding.clone(), result.to_string()),
            )
        });
        let lines = self
            .function
            .contracts
            .iter()
            .filter(|contract| contract.kind == ContractKind::Ensure)
            .filter_map(|contract| self.contract_check_line(contract))
            .collect::<Vec<_>>();
        if let Some((binding, old)) = previous {
            if let Some(old) = old {
                self.locals.insert(binding, old);
            } else {
                self.locals.remove(&binding);
            }
        }
        lines
    }

    fn emit_contract_expr(&self, expr: &ContractExpr) -> String {
        match expr {
            ContractExpr::Bool(value) => {
                if *value {
                    "Boolean.TRUE".to_string()
                } else {
                    "Boolean.FALSE".to_string()
                }
            }
            ContractExpr::String(value) => java_string(value),
            ContractExpr::Int(value) => format!("Long.valueOf({value}L)"),
            ContractExpr::Float(value) => format!("Double.valueOf({value}D)"),
            ContractExpr::Unit => format!("{}.UNIT", self.program.options.runtime_class),
            ContractExpr::Name(name) => self.local_name(name),
            ContractExpr::Call { callee, args } => {
                let args = args
                    .iter()
                    .map(|arg| self.emit_contract_expr(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                let function = callee.rsplit("::").next().unwrap_or(callee);
                format!("{}({args})", self.program.function_name(function))
            }
            ContractExpr::FieldAccess { base, field } => format!(
                "{}.recordField({}, {})",
                self.program.options.runtime_class,
                self.emit_contract_expr(base),
                java_string(field)
            ),
            ContractExpr::Prefix { op, expr } => {
                let method = match op {
                    PrefixOp::Not => "not",
                    PrefixOp::Negate => "negate",
                };
                format!(
                    "{}.{method}({})",
                    self.program.options.runtime_class,
                    self.emit_contract_expr(expr)
                )
            }
            ContractExpr::Binary { op, left, right } => format!(
                "{}.{}({}, {})",
                self.program.options.runtime_class,
                binary_method(*op),
                self.emit_contract_expr(left),
                self.emit_contract_expr(right)
            ),
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
            IrExprKind::FunctionValue(name) => self.emit_function_value(name),
            IrExprKind::ResultOk(value) => self.emit_unary_runtime("ok", value),
            IrExprKind::ResultErr(value) => self.emit_unary_runtime("err", value),
            IrExprKind::OptionSome(value) => self.emit_unary_runtime("some", value),
            IrExprKind::OptionNone => {
                JavaExpr::simple(format!("{}.none()", self.program.options.runtime_class))
            }
            IrExprKind::Call { target, args } => self.emit_call(expr, target, args),
            IrExprKind::FieldAccess { base, field } => self.emit_field_access(base, field),
            IrExprKind::Try(value) => self.emit_try(value),
            IrExprKind::Record(fields) => self.emit_record(fields),
            IrExprKind::Dict(entries) => self.emit_dict(entries),
            IrExprKind::List(items) => self.emit_list(items),
            IrExprKind::Match { scrutinee, arms } => self.emit_match(scrutinee, arms),
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
            IrCallTarget::ConcurrencyBuiltin(name) => {
                format!(
                    "{}.{}({})",
                    self.program.options.runtime_class,
                    concurrency_method(name),
                    java_args.join(", ")
                )
            }
            IrCallTarget::PreludeBuiltin(name) => {
                format!(
                    "{}.{}({})",
                    self.program.options.runtime_class,
                    prelude_method(name),
                    java_args.join(", ")
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

    fn emit_function_value(&self, name: &str) -> JavaExpr {
        let function_name = self.program.function_name(name);
        let Some(function) = self.program.function(name) else {
            return JavaExpr::simple(format!(
                "({}.Fn) ((Object... fnArgs) -> {function_name}())",
                self.program.options.runtime_class
            ));
        };
        let args = (0..function.params.len())
            .map(|index| format!("fnArgs[{index}]"))
            .collect::<Vec<_>>()
            .join(", ");
        JavaExpr::simple(format!(
            "({}.Fn) ((Object... fnArgs) -> {function_name}({args}))",
            self.program.options.runtime_class
        ))
    }

    fn emit_field_access(&mut self, base: &IrExpr, field: &str) -> JavaExpr {
        let base = self.emit_expr(base);
        JavaExpr {
            prelude: base.prelude,
            code: format!(
                "{}.recordField({}, {})",
                self.program.options.runtime_class,
                base.code,
                java_string(field)
            ),
        }
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
        prelude.extend(
            self.ensure_check_lines_for_result(&temp)
                .into_iter()
                .map(|line| format!("    {line}")),
        );
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

    fn emit_dict(&mut self, entries: &[IrDictEntry]) -> JavaExpr {
        let mut prelude = Vec::new();
        let mut args = Vec::new();
        for entry in entries {
            let key = self.emit_expr(&entry.key);
            prelude.extend(key.prelude);
            args.push(key.code);
            let value = self.emit_expr(&entry.value);
            prelude.extend(value.prelude);
            args.push(value.code);
        }
        JavaExpr {
            prelude,
            code: format!(
                "{}.dict({})",
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

    fn emit_match(&mut self, scrutinee: &IrExpr, arms: &[IrMatchArm]) -> JavaExpr {
        let scrutinee_value = self.emit_expr(scrutinee);
        let scrutinee_temp = self.next_temp("match_value");
        let result_temp = self.next_temp("match_result");
        let mut prelude = scrutinee_value.prelude;
        prelude.push(format!(
            "Object {scrutinee_temp} = {};",
            scrutinee_value.code
        ));
        prelude.push(format!("Object {result_temp};"));
        for (index, arm) in arms.iter().enumerate() {
            let saved_locals = self.locals.clone();
            let saved_used_names = self.used_names.clone();
            let pattern = self.emit_pattern(&arm.pattern, &scrutinee_temp);
            let condition = if index == 0 {
                format!("if ({}) {{", pattern.condition)
            } else {
                format!("else if ({}) {{", pattern.condition)
            };
            prelude.push(condition);
            for binding in pattern.bindings {
                prelude.push(format!("    {binding}"));
            }
            let arm_value = self.emit_expr(&arm.value);
            for line in arm_value.prelude {
                prelude.push(format!("    {line}"));
            }
            prelude.push(format!("    {result_temp} = {};", arm_value.code));
            prelude.push("}".to_string());
            self.locals = saved_locals;
            self.used_names = saved_used_names;
        }
        prelude.push("else {".to_string());
        prelude.push("    throw new IllegalStateException(\"non-exhaustive match\");".to_string());
        prelude.push("}".to_string());
        JavaExpr {
            prelude,
            code: result_temp,
        }
    }

    fn emit_pattern(&mut self, pattern: &IrPattern, value: &str) -> JavaPattern {
        match &pattern.kind {
            IrPatternKind::Wildcard => JavaPattern::matches(),
            IrPatternKind::Binding(name) => {
                let java_name = self.bind_local(name, "p");
                JavaPattern {
                    condition: "true".to_string(),
                    bindings: vec![format!("Object {java_name} = {value};")],
                }
            }
            IrPatternKind::StringLiteral(text) => JavaPattern {
                condition: format!(
                    "java.util.Objects.equals({value}, {})",
                    java_string(&veln_string_literal_value(text))
                ),
                bindings: Vec::new(),
            },
            IrPatternKind::IntLiteral(text) => JavaPattern {
                condition: format!("java.util.Objects.equals({value}, Long.valueOf({text}L))"),
                bindings: Vec::new(),
            },
            IrPatternKind::FloatLiteral(text) => JavaPattern {
                condition: format!("java.util.Objects.equals({value}, Double.valueOf({text}D))"),
                bindings: Vec::new(),
            },
            IrPatternKind::BoolLiteral(value_bool) => JavaPattern {
                condition: format!(
                    "java.util.Objects.equals({value}, {})",
                    if *value_bool {
                        "Boolean.TRUE"
                    } else {
                        "Boolean.FALSE"
                    }
                ),
                bindings: Vec::new(),
            },
            IrPatternKind::Unit => JavaPattern {
                condition: format!(
                    "java.util.Objects.equals({value}, {}.UNIT)",
                    self.program.options.runtime_class
                ),
                bindings: Vec::new(),
            },
            IrPatternKind::Record(fields) => self.emit_record_pattern(fields, value),
            IrPatternKind::Constructor { name, args } => {
                self.emit_constructor_pattern(name, args, value)
            }
        }
    }

    fn emit_record_pattern(&mut self, fields: &[IrPatternField], value: &str) -> JavaPattern {
        let mut conditions = Vec::new();
        let mut bindings = Vec::new();
        for field in fields {
            let field_value = format!(
                "{}.recordField({}, {})",
                self.program.options.runtime_class,
                value,
                java_string(&field.name)
            );
            conditions.push(format!(
                "{}.recordHasField({}, {})",
                self.program.options.runtime_class,
                value,
                java_string(&field.name)
            ));
            let mut nested = self.emit_pattern(&field.pattern, &field_value);
            conditions.push(nested.condition);
            bindings.append(&mut nested.bindings);
        }
        JavaPattern {
            condition: if conditions.is_empty() {
                "true".to_string()
            } else {
                conditions.join(" && ")
            },
            bindings,
        }
    }

    fn emit_constructor_pattern(
        &mut self,
        name: &[String],
        args: &[IrPattern],
        value: &str,
    ) -> JavaPattern {
        let Some(constructor) = name.last().map(String::as_str) else {
            return JavaPattern::never();
        };
        match constructor {
            "None" if args.is_empty() => JavaPattern {
                condition: format!("{}.isNone({value})", self.program.options.runtime_class),
                bindings: Vec::new(),
            },
            "Some" if args.len() == 1 => {
                let inner = format!(
                    "{}.optionValue({value})",
                    self.program.options.runtime_class
                );
                let mut nested = self.emit_pattern(&args[0], &inner);
                let condition = format!(
                    "{}.isSome({value}) && {}",
                    self.program.options.runtime_class, nested.condition
                );
                let mut bindings = Vec::new();
                bindings.append(&mut nested.bindings);
                JavaPattern {
                    condition,
                    bindings,
                }
            }
            "Ok" if args.len() == 1 => {
                let inner = format!(
                    "{}.resultValue({value})",
                    self.program.options.runtime_class
                );
                let mut nested = self.emit_pattern(&args[0], &inner);
                let condition = format!(
                    "{}.isOk({value}) && {}",
                    self.program.options.runtime_class, nested.condition
                );
                let mut bindings = Vec::new();
                bindings.append(&mut nested.bindings);
                JavaPattern {
                    condition,
                    bindings,
                }
            }
            "Err" if args.len() == 1 => {
                let inner = format!(
                    "{}.resultValue({value})",
                    self.program.options.runtime_class
                );
                let mut nested = self.emit_pattern(&args[0], &inner);
                let condition = format!(
                    "{}.isErr({value}) && {}",
                    self.program.options.runtime_class, nested.condition
                );
                let mut bindings = Vec::new();
                bindings.append(&mut nested.bindings);
                JavaPattern {
                    condition,
                    bindings,
                }
            }
            _ => JavaPattern::never(),
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

struct JavaPattern {
    condition: String,
    bindings: Vec<String>,
}

impl JavaPattern {
    fn matches() -> Self {
        Self {
            condition: "true".to_string(),
            bindings: Vec::new(),
        }
    }

    fn never() -> Self {
        Self {
            condition: "false".to_string(),
            bindings: Vec::new(),
        }
    }
}

impl JavaExpr {
    fn simple(code: String) -> Self {
        Self {
            prelude: Vec::new(),
            code,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ContractExpr {
    Bool(bool),
    String(String),
    Int(String),
    Float(String),
    Unit,
    Name(String),
    Call {
        callee: String,
        args: Vec<ContractExpr>,
    },
    FieldAccess {
        base: Box<ContractExpr>,
        field: String,
    },
    Prefix {
        op: PrefixOp,
        expr: Box<ContractExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<ContractExpr>,
        right: Box<ContractExpr>,
    },
}

struct ContractParser<'a> {
    tokens: Vec<ContractToken<'a>>,
    cursor: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContractToken<'a> {
    Ident(&'a str),
    String(&'a str),
    Int(&'a str),
    Float(&'a str),
    LParen,
    RParen,
    Comma,
    Dot,
    ColonColon,
    Op(&'a str),
    Eof,
}

impl<'a> ContractParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            tokens: tokenize_contract(text),
            cursor: 0,
        }
    }

    fn parse(&mut self) -> Option<ContractExpr> {
        let expr = self.parse_prec(0)?;
        matches!(self.current(), ContractToken::Eof).then_some(expr)
    }

    fn parse_prec(&mut self, min_prec: u8) -> Option<ContractExpr> {
        let mut left = self.parse_prefix()?;
        while let Some((op, prec)) = self.current_binary_op() {
            if prec < min_prec {
                break;
            }
            self.bump();
            let right = self.parse_prec(prec + 1)?;
            left = ContractExpr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Some(left)
    }

    fn parse_prefix(&mut self) -> Option<ContractExpr> {
        match self.current() {
            ContractToken::Ident("not") => {
                self.bump();
                Some(ContractExpr::Prefix {
                    op: PrefixOp::Not,
                    expr: Box::new(self.parse_prefix()?),
                })
            }
            ContractToken::Op("-") => {
                self.bump();
                Some(ContractExpr::Prefix {
                    op: PrefixOp::Negate,
                    expr: Box::new(self.parse_prefix()?),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Option<ContractExpr> {
        let mut expr = self.parse_primary()?;
        while matches!(self.current(), ContractToken::Dot) {
            self.bump();
            let ContractToken::Ident(field) = self.current() else {
                return None;
            };
            let field = field.to_string();
            self.bump();
            expr = ContractExpr::FieldAccess {
                base: Box::new(expr),
                field,
            };
        }
        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<ContractExpr> {
        match self.current() {
            ContractToken::Ident("true") => {
                self.bump();
                Some(ContractExpr::Bool(true))
            }
            ContractToken::Ident("false") => {
                self.bump();
                Some(ContractExpr::Bool(false))
            }
            ContractToken::Ident(_) => self.parse_name_or_call(),
            ContractToken::String(value) => {
                let value = veln_string_literal_value(value);
                self.bump();
                Some(ContractExpr::String(value))
            }
            ContractToken::Int(value) => {
                let value = value.to_string();
                self.bump();
                Some(ContractExpr::Int(value))
            }
            ContractToken::Float(value) => {
                let value = value.to_string();
                self.bump();
                Some(ContractExpr::Float(value))
            }
            ContractToken::LParen => {
                self.bump();
                if matches!(self.current(), ContractToken::RParen) {
                    self.bump();
                    return Some(ContractExpr::Unit);
                }
                let expr = self.parse_prec(0)?;
                if !matches!(self.current(), ContractToken::RParen) {
                    return None;
                }
                self.bump();
                Some(expr)
            }
            _ => None,
        }
    }

    fn parse_name_or_call(&mut self) -> Option<ContractExpr> {
        let mut parts = Vec::new();
        let ContractToken::Ident(first) = self.current() else {
            return None;
        };
        parts.push(first);
        self.bump();
        while matches!(self.current(), ContractToken::ColonColon) {
            self.bump();
            let ContractToken::Ident(part) = self.current() else {
                return None;
            };
            parts.push(part);
            self.bump();
        }
        let name = parts.join("::");
        if !matches!(self.current(), ContractToken::LParen) {
            return Some(ContractExpr::Name(name));
        }
        self.bump();
        let mut args = Vec::new();
        while !matches!(self.current(), ContractToken::RParen | ContractToken::Eof) {
            args.push(self.parse_prec(0)?);
            if !matches!(self.current(), ContractToken::Comma) {
                break;
            }
            self.bump();
        }
        if !matches!(self.current(), ContractToken::RParen) {
            return None;
        }
        self.bump();
        Some(ContractExpr::Call { callee: name, args })
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.current() {
            ContractToken::Ident("or") => Some((BinaryOp::Or, 1)),
            ContractToken::Ident("and") => Some((BinaryOp::And, 2)),
            ContractToken::Op("==") => Some((BinaryOp::Equal, 3)),
            ContractToken::Op("!=") => Some((BinaryOp::NotEqual, 3)),
            ContractToken::Op("<") => Some((BinaryOp::Less, 4)),
            ContractToken::Op("<=") => Some((BinaryOp::LessEqual, 4)),
            ContractToken::Op(">") => Some((BinaryOp::Greater, 4)),
            ContractToken::Op(">=") => Some((BinaryOp::GreaterEqual, 4)),
            ContractToken::Op("+") => Some((BinaryOp::Add, 5)),
            ContractToken::Op("-") => Some((BinaryOp::Subtract, 5)),
            ContractToken::Op("*") => Some((BinaryOp::Multiply, 6)),
            ContractToken::Op("/") => Some((BinaryOp::Divide, 6)),
            _ => None,
        }
    }

    fn current(&self) -> ContractToken<'a> {
        self.tokens
            .get(self.cursor)
            .copied()
            .unwrap_or(ContractToken::Eof)
    }

    fn bump(&mut self) {
        self.cursor += 1;
    }
}

fn tokenize_contract(text: &str) -> Vec<ContractToken<'_>> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = cursor;
            cursor += ch.len_utf8();
            while cursor < text.len() {
                let ch = text[cursor..]
                    .chars()
                    .next()
                    .expect("cursor should stay on a char boundary");
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    cursor += ch.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(ContractToken::Ident(&text[start..cursor]));
            continue;
        }
        if ch.is_ascii_digit() {
            let start = cursor;
            cursor += ch.len_utf8();
            let mut float = false;
            while cursor < text.len() {
                let ch = text[cursor..]
                    .chars()
                    .next()
                    .expect("cursor should stay on a char boundary");
                if ch.is_ascii_digit() {
                    cursor += ch.len_utf8();
                } else if ch == '.' && !float {
                    float = true;
                    cursor += ch.len_utf8();
                } else {
                    break;
                }
            }
            let value = &text[start..cursor];
            tokens.push(if float {
                ContractToken::Float(value)
            } else {
                ContractToken::Int(value)
            });
            continue;
        }
        if ch == '"' {
            let start = cursor;
            cursor += 1;
            let mut escaped = false;
            while cursor < text.len() {
                let ch = text[cursor..]
                    .chars()
                    .next()
                    .expect("cursor should stay on a char boundary");
                cursor += ch.len_utf8();
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    break;
                }
            }
            tokens.push(ContractToken::String(&text[start..cursor]));
            continue;
        }
        let rest = &text[cursor..];
        let (token, width) = if rest.starts_with("::") {
            (ContractToken::ColonColon, 2)
        } else if rest.starts_with("==")
            || rest.starts_with("!=")
            || rest.starts_with("<=")
            || rest.starts_with(">=")
        {
            (ContractToken::Op(&rest[..2]), 2)
        } else {
            match ch {
                '(' => (ContractToken::LParen, 1),
                ')' => (ContractToken::RParen, 1),
                ',' => (ContractToken::Comma, 1),
                '.' => (ContractToken::Dot, 1),
                '<' | '>' | '+' | '-' | '*' | '/' => (ContractToken::Op(&rest[..1]), 1),
                _ => {
                    cursor += ch.len_utf8();
                    continue;
                }
            }
        };
        tokens.push(token);
        cursor += width;
    }
    tokens.push(ContractToken::Eof);
    tokens
}
