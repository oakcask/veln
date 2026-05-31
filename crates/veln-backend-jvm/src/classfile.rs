use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use veln_ast::{BinaryOp, ContractKind, PrefixOp};
use veln_ir::{
    ContractObligationStatus, IrCallTarget, IrContract, IrDictEntry, IrExpr, IrExprKind,
    IrFunction, IrMatchArm, IrPattern, IrPatternField, IrPatternKind, IrRecordField, IrStmt,
    IrStmtKind, TypedProgram,
};

use crate::api::{EntryArgType, JvmClassFile, JvmProgram, SanitizedOptions};
use crate::java::{sanitize_identifier_text, unique_java_identifier, veln_string_literal_value};
use crate::runtime::{
    binary_method, concurrency_method, prelude_method, standard_library_method, stdio_method,
};

const PROGRAM_MAJOR_VERSION: u16 = 49;
const JAVA_LANG_OBJECT: &str = "java/lang/Object";
const VELN_ENTRY: &str = "VelnEntry";

pub(crate) struct ClassfileEmitter<'a> {
    program: &'a TypedProgram,
    options: SanitizedOptions,
    function_names: BTreeMap<String, String>,
}

impl<'a> ClassfileEmitter<'a> {
    pub(crate) fn new(program: &'a TypedProgram, options: SanitizedOptions) -> Self {
        let mut function_names = BTreeMap::new();
        let mut used_names = BTreeSet::new();
        for function in &program.functions {
            let name = unique_java_identifier(
                &format!("fn_{}", sanitize_identifier_text(&function.name)),
                &mut used_names,
            );
            function_names.insert(function.name.clone(), name);
        }
        Self {
            program,
            options,
            function_names,
        }
    }

    pub(crate) fn emit(
        &self,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) -> JvmProgram {
        let mut classes = runtime_classes();
        classes.push(JvmClassFile {
            path: format!("{}.class", self.options.program_class),
            contents: self.emit_program_class(),
        });
        classes.push(JvmClassFile {
            path: "VelnEntry.class".to_string(),
            contents: self.emit_entry_class(entry_function, entry_arg_types),
        });
        for function in &self.program.functions {
            classes.push(JvmClassFile {
                path: format!(
                    "{}${}.class",
                    self.options.program_class,
                    self.function_name(&function.name)
                ),
                contents: self.emit_function_adapter(function),
            });
        }
        JvmProgram { classes }
    }

    fn emit_program_class(&self) -> Vec<u8> {
        let mut class = ClassBuilder::new(&self.options.program_class);
        class.access_flags = 0x0031;
        class.add_default_constructor(0x0002);
        for function in &self.program.functions {
            let mut method = MethodCode::new(Rc::clone(&class.constant_pool));
            let mut emitter = FunctionBytecodeEmitter::new(self, function);
            emitter.emit(&mut method);
            class.add_method(MethodInfo {
                access_flags: 0x0008,
                name: self.function_name(&function.name),
                descriptor: object_method_descriptor(function.params.len()),
                max_stack: method.max_stack,
                max_locals: method.max_locals,
                code: method.code,
                exceptions: method.exceptions,
            });
        }
        class.finish()
    }

    fn emit_function_adapter(&self, function: &IrFunction) -> Vec<u8> {
        let adapter = format!(
            "{}${}",
            self.options.program_class,
            self.function_name(&function.name)
        );
        let mut class = ClassBuilder::new(&adapter);
        class.access_flags = 0x0031;
        class.interfaces.push(self.runtime_nested("Fn"));
        class.add_default_constructor(0x0001);

        let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
        for index in 0..function.params.len() {
            code.aload(1);
            code.push_i32(index as i32);
            code.op(0x32);
        }
        code.invokestatic(
            &self.options.program_class,
            &self.function_name(&function.name),
            &object_method_descriptor(function.params.len()),
        );
        code.op(0xb0);
        class.add_method(MethodInfo {
            access_flags: 0x0081,
            name: "call".to_string(),
            descriptor: "([Ljava/lang/Object;)Ljava/lang/Object;".to_string(),
            max_stack: code.max_stack,
            max_locals: 2,
            code: code.code,
            exceptions: Vec::new(),
        });
        class.finish()
    }

    fn emit_entry_class(&self, entry_function: &str, entry_arg_types: &[EntryArgType]) -> Vec<u8> {
        let mut class = ClassBuilder::new(VELN_ENTRY);
        class.access_flags = 0x0031;
        class.add_default_constructor(0x0002);

        let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
        let try_start = code.mark();
        code.aload(0);
        code.invokestatic(
            &self.options.runtime_class,
            "setProcessArgs",
            "([Ljava/lang/String;)V",
        );
        self.emit_entry_argument_conversions(&mut code, entry_arg_types);
        self.emit_entry_invocation_and_result(&mut code, entry_function, entry_arg_types);
        let try_end = code.mark();
        code.op(0xb1);

        self.emit_entry_contract_failure_handler(&mut code, try_start, try_end);
        self.add_entry_main_method(&mut class, code);
        class.finish()
    }

    fn emit_entry_argument_conversions(
        &self,
        code: &mut MethodCode,
        entry_arg_types: &[EntryArgType],
    ) {
        for (index, ty) in entry_arg_types.iter().enumerate() {
            code.aload(0);
            code.push_i32(index as i32);
            code.op(0x32);
            self.emit_entry_argument_conversion(code, ty);
        }
    }

    fn emit_entry_argument_conversion(&self, code: &mut MethodCode, ty: &EntryArgType) {
        match ty {
            EntryArgType::String => {}
            EntryArgType::Int => {
                code.invokestatic("java/lang/Long", "parseLong", "(Ljava/lang/String;)J");
                code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
            }
            EntryArgType::Float => {
                code.invokestatic("java/lang/Double", "parseDouble", "(Ljava/lang/String;)D");
                code.invokestatic("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;");
            }
            EntryArgType::Bool => {
                code.invokestatic(
                    "java/lang/Boolean",
                    "valueOf",
                    "(Ljava/lang/String;)Ljava/lang/Boolean;",
                );
            }
        }
    }

    fn emit_entry_invocation_and_result(
        &self,
        code: &mut MethodCode,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) {
        code.invokestatic(
            &self.options.program_class,
            &self.function_name(entry_function),
            &object_method_descriptor(entry_arg_types.len()),
        );
        code.astore(1);
        code.aload(1);
        code.invokestatic(
            &self.options.runtime_class,
            "isErr",
            "(Ljava/lang/Object;)Z",
        );
        let ok = code.branch(0x99);
        code.aload(1);
        code.invokestatic(
            &self.options.runtime_class,
            "recordResultFailure",
            "(Ljava/lang/Object;)V",
        );
        code.getstatic("java/lang/System", "err", "Ljava/io/PrintStream;");
        code.aload(1);
        code.invokestatic(
            &self.options.runtime_class,
            "format",
            "(Ljava/lang/Object;)Ljava/lang/String;",
        );
        code.invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V");
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
        code.bind(ok);
    }

    fn emit_entry_contract_failure_handler(
        &self,
        code: &mut MethodCode,
        try_start: usize,
        try_end: usize,
    ) {
        let handler = code.mark();
        code.astore(2);
        code.aload(2);
        code.invokestatic(
            &self.options.runtime_class,
            "recordContractFailure",
            &format!("(L{}$ContractFailure;)V", self.options.runtime_class),
        );
        code.getstatic("java/lang/System", "err", "Ljava/io/PrintStream;");
        code.aload(2);
        code.invokevirtual("java/lang/Throwable", "getMessage", "()Ljava/lang/String;");
        code.invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V");
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
        code.op(0xb1);
        code.exceptions.push(ExceptionHandler {
            start_pc: try_start,
            end_pc: try_end,
            handler_pc: handler,
            catch_type: self.runtime_nested("ContractFailure"),
        });
    }

    fn add_entry_main_method(&self, class: &mut ClassBuilder, code: MethodCode) {
        class.add_method(MethodInfo {
            access_flags: 0x0009,
            name: "main".to_string(),
            descriptor: "([Ljava/lang/String;)V".to_string(),
            max_stack: code.max_stack,
            max_locals: 3,
            code: code.code,
            exceptions: code.exceptions,
        });
    }

    fn function_name(&self, name: &str) -> String {
        self.function_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("fn_{}", sanitize_identifier_text(name)))
    }

    fn runtime_nested(&self, nested: &str) -> String {
        format!("{}${nested}", self.options.runtime_class)
    }
}

struct FunctionBytecodeEmitter<'a, 'program> {
    program: &'a ClassfileEmitter<'program>,
    function: &'a IrFunction,
    locals: BTreeMap<String, u16>,
    next_local: u16,
    max_local: u16,
    tail_loop_start: Option<usize>,
}

#[derive(Clone, Copy)]
enum ContractCheckPosition {
    Entry,
    Return,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TailRecursionEligibility {
    Eligible,
    NotRecursive,
    NonTailSelfCall,
    RuntimeReturnContract,
    IndirectValueCall,
}

impl<'a, 'program> FunctionBytecodeEmitter<'a, 'program> {
    fn new(program: &'a ClassfileEmitter<'program>, function: &'a IrFunction) -> Self {
        let mut locals = BTreeMap::new();
        for (index, param) in function.params.iter().enumerate() {
            locals.insert(param.name.clone(), index as u16);
        }
        Self {
            program,
            function,
            locals,
            next_local: function.params.len() as u16,
            max_local: function.params.len() as u16,
            tail_loop_start: None,
        }
    }

    fn emit(&mut self, code: &mut MethodCode) {
        let tail_recursion = classify_tail_recursion(self.function);
        if tail_recursion == TailRecursionEligibility::Eligible {
            let start = code.new_label();
            code.bind(start);
            self.tail_loop_start = Some(start);
        }
        for contract in self
            .function
            .contracts
            .iter()
            .filter(|contract| contract.kind != ContractKind::Ensure)
        {
            self.emit_contract_check(code, contract, ContractCheckPosition::Entry);
        }
        for stmt in &self.function.body {
            self.emit_stmt(code, stmt);
        }
        if !matches!(
            self.function.body.last().map(|stmt| &stmt.kind),
            Some(IrStmtKind::Return { .. })
        ) {
            code.getstatic(
                &self.program.options.runtime_class,
                "UNIT",
                &format!("L{}$Unit;", self.program.options.runtime_class),
            );
            code.op(0xb0);
        }
        code.max_locals = self.max_local.max(1);
    }

    fn emit_stmt(&mut self, code: &mut MethodCode, stmt: &IrStmt) {
        match &stmt.kind {
            IrStmtKind::Let { name, value, .. } => {
                self.emit_expr(code, value);
                let slot = self.bind_local(name);
                code.astore(slot);
            }
            IrStmtKind::Expr { value } => {
                self.emit_expr(code, value);
                code.op(0x57);
            }
            IrStmtKind::Return { value } => {
                if self.emit_tail_expr(code, value) {
                    return;
                }
                if self.has_ensure_contracts() {
                    let result = self.alloc_local();
                    code.astore(result);
                    self.emit_ensure_checks_for_result(code, result);
                    code.aload(result);
                }
                code.op(0xb0);
            }
        }
    }

    fn emit_tail_expr(&mut self, code: &mut MethodCode, expr: &IrExpr) -> bool {
        match &expr.kind {
            IrExprKind::Call {
                target: IrCallTarget::Function(name),
                args,
            } if self.tail_loop_start.is_some() && name == &self.function.name => {
                self.emit_tail_self_call(code, args);
                true
            }
            IrExprKind::Match { scrutinee, arms } => self.emit_tail_match(code, scrutinee, arms),
            _ => {
                self.emit_expr(code, expr);
                false
            }
        }
    }

    fn emit_tail_self_call(&mut self, code: &mut MethodCode, args: &[IrExpr]) {
        let mut temp_slots = Vec::with_capacity(args.len());
        for arg in args {
            self.emit_expr(code, arg);
            let slot = self.alloc_local();
            code.astore(slot);
            temp_slots.push(slot);
        }
        for (index, slot) in temp_slots.into_iter().enumerate() {
            code.aload(slot);
            code.astore(index as u16);
        }
        code.branch_to(
            0xa7,
            self.tail_loop_start
                .expect("tail self call requires a loop start"),
        );
    }

    fn emit_tail_match(
        &mut self,
        code: &mut MethodCode,
        scrutinee: &IrExpr,
        arms: &[IrMatchArm],
    ) -> bool {
        self.emit_expr(code, scrutinee);
        let value_slot = self.alloc_local();
        let result_slot = self.alloc_local();
        code.astore(value_slot);
        let end = code.new_label();
        let saved_locals = self.locals.clone();
        let saved_next = self.next_local;
        let mut has_value_arm = false;
        for arm in arms {
            self.locals = saved_locals.clone();
            self.next_local = saved_next;
            let next = code.new_label();
            self.emit_pattern_condition(code, &arm.pattern, ValueRef::Local(value_slot));
            code.branch_to(0x99, next);
            self.emit_pattern_bindings(code, &arm.pattern, ValueRef::Local(value_slot));
            if !self.emit_tail_expr(code, &arm.value) {
                has_value_arm = true;
                code.astore(result_slot);
                code.branch_to(0xa7, end);
            }
            code.bind(next);
        }
        code.new_class("java/lang/IllegalStateException");
        code.op(0x59);
        code.ldc_string("non-exhaustive match");
        code.invokespecial(
            "java/lang/IllegalStateException",
            "<init>",
            "(Ljava/lang/String;)V",
        );
        code.op(0xbf);
        self.locals = saved_locals;
        self.next_local = self.next_local.max(result_slot + 1);
        if has_value_arm {
            code.bind(end);
            code.aload(result_slot);
        }
        !has_value_arm
    }

    fn emit_expr(&mut self, code: &mut MethodCode, expr: &IrExpr) {
        match &expr.kind {
            IrExprKind::Local(name) => code.aload(self.local_slot(name)),
            IrExprKind::BoolLiteral(value) => {
                code.getstatic(
                    "java/lang/Boolean",
                    if *value { "TRUE" } else { "FALSE" },
                    "Ljava/lang/Boolean;",
                );
            }
            IrExprKind::StringLiteral(value) => code.ldc_string(&veln_string_literal_value(value)),
            IrExprKind::IntLiteral(value) => {
                code.ldc_long(value.parse::<i64>().unwrap_or(0));
                code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
            }
            IrExprKind::FloatLiteral(value) => {
                code.ldc_double(value.parse::<f64>().unwrap_or(0.0));
                code.invokestatic("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;");
            }
            IrExprKind::Unit => {
                code.getstatic(
                    &self.program.options.runtime_class,
                    "UNIT",
                    &format!("L{}$Unit;", self.program.options.runtime_class),
                );
            }
            IrExprKind::FunctionValue(name) => {
                code.new_class(&format!(
                    "{}${}",
                    self.program.options.program_class,
                    self.program.function_name(name)
                ));
                code.op(0x59);
                code.invokespecial(
                    &format!(
                        "{}${}",
                        self.program.options.program_class,
                        self.program.function_name(name)
                    ),
                    "<init>",
                    "()V",
                );
            }
            IrExprKind::ResultOk(value) => self.emit_unary_runtime_with_descriptor(
                code,
                "ok",
                &format!(
                    "(Ljava/lang/Object;)L{}$Result;",
                    self.program.options.runtime_class
                ),
                value,
            ),
            IrExprKind::ResultErr(value) => self.emit_unary_runtime_with_descriptor(
                code,
                "err",
                &format!(
                    "(Ljava/lang/Object;)L{}$Result;",
                    self.program.options.runtime_class
                ),
                value,
            ),
            IrExprKind::OptionSome(value) => self.emit_unary_runtime_with_descriptor(
                code,
                "some",
                &format!(
                    "(Ljava/lang/Object;)L{}$Option;",
                    self.program.options.runtime_class
                ),
                value,
            ),
            IrExprKind::OptionNone => {
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "none",
                    &format!("()L{}$Option;", self.program.options.runtime_class),
                );
            }
            IrExprKind::ListNil => {
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "listNil",
                    "()Ljava/lang/Object;",
                );
            }
            IrExprKind::ListCons { head, tail } => {
                self.emit_expr(code, head);
                self.emit_expr(code, tail);
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "listCons",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                );
            }
            IrExprKind::AdtVariant { name, payloads } => {
                code.ldc_string(&name.join("::"));
                self.emit_object_array(code, payloads.len(), |this, code, index| {
                    this.emit_expr(code, &payloads[index]);
                });
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "adt",
                    "(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/Object;",
                );
            }
            IrExprKind::Call { target, args } => self.emit_call(code, expr, target, args),
            IrExprKind::FieldAccess { base, field } => {
                self.emit_expr(code, base);
                code.ldc_string(field);
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "recordField",
                    "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
                );
            }
            IrExprKind::Try(value) => self.emit_try(code, value),
            IrExprKind::Record(fields) => self.emit_record(code, fields),
            IrExprKind::Dict(entries) => self.emit_dict(code, entries),
            IrExprKind::List(items) => {
                self.emit_object_array(code, items.len(), |this, code, index| {
                    this.emit_expr(code, &items[index]);
                });
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "list",
                    "([Ljava/lang/Object;)Ljava/util/List;",
                );
            }
            IrExprKind::Match { scrutinee, arms } => self.emit_match(code, scrutinee, arms),
            IrExprKind::Prefix { op, expr } => {
                let method = match op {
                    PrefixOp::Not => "not",
                    PrefixOp::Negate => "negate",
                };
                self.emit_unary_runtime(code, method, expr);
            }
            IrExprKind::Binary { op, left, right } => self.emit_binary(code, *op, left, right),
        }
    }

    fn emit_call(
        &mut self,
        code: &mut MethodCode,
        expr: &IrExpr,
        target: &IrCallTarget,
        args: &[IrExpr],
    ) {
        match target {
            IrCallTarget::Function(name) => {
                for arg in args {
                    self.emit_expr(code, arg);
                }
                code.invokestatic(
                    &self.program.options.program_class,
                    &self.program.function_name(name),
                    &object_method_descriptor(args.len()),
                );
            }
            IrCallTarget::StdioBuiltin(name) => {
                for arg in args {
                    self.emit_expr(code, arg);
                }
                code.ldc_string(&expr.node_id.display("call"));
                code.ldc_string(expr.span.file.as_str());
                code.invokestatic(
                    &self.program.options.runtime_class,
                    stdio_method(name),
                    "(Ljava/lang/Object;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;",
                );
            }
            IrCallTarget::ConcurrencyBuiltin(name) => {
                self.emit_runtime_call(code, concurrency_method(name), args);
            }
            IrCallTarget::StandardLibraryBuiltin(name) => {
                self.emit_runtime_call(code, standard_library_method(name), args);
            }
            IrCallTarget::PreludeBuiltin(name) => {
                self.emit_runtime_call(code, prelude_method(name), args);
            }
            IrCallTarget::Value(name) => {
                code.aload(self.local_slot(name));
                self.emit_object_array(code, args.len(), |this, code, index| {
                    this.emit_expr(code, &args[index]);
                });
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "call",
                    "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
                );
            }
        }
    }

    fn emit_runtime_call(&mut self, code: &mut MethodCode, method: &str, args: &[IrExpr]) {
        for arg in args {
            self.emit_expr(code, arg);
        }
        code.invokestatic(
            &self.program.options.runtime_class,
            method,
            &object_method_descriptor(args.len()),
        );
    }

    fn emit_unary_runtime(&mut self, code: &mut MethodCode, method: &str, value: &IrExpr) {
        self.emit_unary_runtime_with_descriptor(
            code,
            method,
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            value,
        );
    }

    fn emit_unary_runtime_with_descriptor(
        &mut self,
        code: &mut MethodCode,
        method: &str,
        descriptor: &str,
        value: &IrExpr,
    ) {
        self.emit_expr(code, value);
        code.invokestatic(&self.program.options.runtime_class, method, descriptor);
    }

    fn emit_binary(&mut self, code: &mut MethodCode, op: BinaryOp, left: &IrExpr, right: &IrExpr) {
        self.emit_expr(code, left);
        self.emit_expr(code, right);
        code.invokestatic(
            &self.program.options.runtime_class,
            binary_method(op),
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    fn emit_record(&mut self, code: &mut MethodCode, fields: &[IrRecordField]) {
        let count = fields.len() * 2;
        self.emit_object_array(code, count, |this, code, index| {
            let field = &fields[index / 2];
            if index % 2 == 0 {
                code.ldc_string(&field.name);
            } else {
                this.emit_expr(code, &field.value);
            }
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "record",
            "([Ljava/lang/Object;)Ljava/util/Map;",
        );
    }

    fn emit_dict(&mut self, code: &mut MethodCode, entries: &[IrDictEntry]) {
        let count = entries.len() * 2;
        self.emit_object_array(code, count, |this, code, index| {
            let entry = &entries[index / 2];
            if index % 2 == 0 {
                this.emit_expr(code, &entry.key);
            } else {
                this.emit_expr(code, &entry.value);
            }
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "dict",
            "([Ljava/lang/Object;)Ljava/util/Map;",
        );
    }

    fn emit_object_array<F>(&mut self, code: &mut MethodCode, len: usize, mut emit: F)
    where
        F: FnMut(&mut Self, &mut MethodCode, usize),
    {
        code.push_i32(len as i32);
        code.anewarray(JAVA_LANG_OBJECT);
        for index in 0..len {
            code.op(0x59);
            code.push_i32(index as i32);
            emit(self, code, index);
            code.op(0x53);
        }
    }

    fn emit_try(&mut self, code: &mut MethodCode, value: &IrExpr) {
        self.emit_expr(code, value);
        let temp = self.alloc_local();
        code.astore(temp);
        code.aload(temp);
        code.invokestatic(
            &self.program.options.runtime_class,
            "isErr",
            "(Ljava/lang/Object;)Z",
        );
        let ok = code.branch(0x99);
        self.emit_ensure_checks_for_result(code, temp);
        code.aload(temp);
        code.op(0xb0);
        code.bind(ok);
        code.aload(temp);
        code.invokestatic(
            &self.program.options.runtime_class,
            "unwrapOk",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    fn emit_match(&mut self, code: &mut MethodCode, scrutinee: &IrExpr, arms: &[IrMatchArm]) {
        self.emit_expr(code, scrutinee);
        let value_slot = self.alloc_local();
        let result_slot = self.alloc_local();
        code.astore(value_slot);
        let end = code.new_label();
        let saved_locals = self.locals.clone();
        let saved_next = self.next_local;
        for arm in arms {
            self.locals = saved_locals.clone();
            self.next_local = saved_next;
            let next = code.new_label();
            self.emit_pattern_condition(code, &arm.pattern, ValueRef::Local(value_slot));
            code.branch_to(0x99, next);
            self.emit_pattern_bindings(code, &arm.pattern, ValueRef::Local(value_slot));
            self.emit_expr(code, &arm.value);
            code.astore(result_slot);
            code.branch_to(0xa7, end);
            code.bind(next);
        }
        code.new_class("java/lang/IllegalStateException");
        code.op(0x59);
        code.ldc_string("non-exhaustive match");
        code.invokespecial(
            "java/lang/IllegalStateException",
            "<init>",
            "(Ljava/lang/String;)V",
        );
        code.op(0xbf);
        code.bind(end);
        code.aload(result_slot);
        self.locals = saved_locals;
        self.next_local = self.next_local.max(result_slot + 1);
    }

    fn emit_pattern_condition(
        &mut self,
        code: &mut MethodCode,
        pattern: &IrPattern,
        value: ValueRef,
    ) {
        match &pattern.kind {
            IrPatternKind::Wildcard | IrPatternKind::Binding(_) => code.push_i32(1),
            IrPatternKind::StringLiteral(text) => {
                value.emit_load(code);
                code.ldc_string(&veln_string_literal_value(text));
                code.invokestatic(
                    "java/util/Objects",
                    "equals",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                );
            }
            IrPatternKind::IntLiteral(text) => {
                value.emit_load(code);
                code.ldc_long(text.parse::<i64>().unwrap_or(0));
                code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
                code.invokestatic(
                    "java/util/Objects",
                    "equals",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                );
            }
            IrPatternKind::FloatLiteral(text) => {
                value.emit_load(code);
                code.ldc_double(text.parse::<f64>().unwrap_or(0.0));
                code.invokestatic("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;");
                code.invokestatic(
                    "java/util/Objects",
                    "equals",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                );
            }
            IrPatternKind::BoolLiteral(value_bool) => {
                value.emit_load(code);
                code.getstatic(
                    "java/lang/Boolean",
                    if *value_bool { "TRUE" } else { "FALSE" },
                    "Ljava/lang/Boolean;",
                );
                code.invokestatic(
                    "java/util/Objects",
                    "equals",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                );
            }
            IrPatternKind::Unit => {
                value.emit_load(code);
                code.getstatic(
                    &self.program.options.runtime_class,
                    "UNIT",
                    &format!("L{}$Unit;", self.program.options.runtime_class),
                );
                code.invokestatic(
                    "java/util/Objects",
                    "equals",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                );
            }
            IrPatternKind::Record(fields) => {
                self.emit_record_pattern_condition(code, fields, value)
            }
            IrPatternKind::Constructor { name, args } => {
                self.emit_constructor_pattern_condition(code, name, args, value);
            }
        }
    }

    fn emit_record_pattern_condition(
        &mut self,
        code: &mut MethodCode,
        fields: &[IrPatternField],
        value: ValueRef,
    ) {
        let fail = code.new_label();
        let done = code.new_label();
        for field in fields {
            value.emit_load(code);
            code.ldc_string(&field.name);
            code.invokestatic(
                &self.program.options.runtime_class,
                "recordHasField",
                "(Ljava/lang/Object;Ljava/lang/String;)Z",
            );
            code.branch_to(0x99, fail);
            let field_value = ValueRef::RecordField {
                base: Box::new(value.clone()),
                field: field.name.clone(),
                runtime: self.program.options.runtime_class.clone(),
            };
            self.emit_pattern_condition(code, &field.pattern, field_value);
            code.branch_to(0x99, fail);
        }
        code.push_i32(1);
        code.branch_to(0xa7, done);
        code.bind(fail);
        code.push_i32(0);
        code.bind(done);
    }

    fn emit_constructor_pattern_condition(
        &mut self,
        code: &mut MethodCode,
        name: &[String],
        args: &[IrPattern],
        value: ValueRef,
    ) {
        let Some(constructor) = name.last().map(String::as_str) else {
            code.push_i32(0);
            return;
        };
        match constructor {
            "None" if args.is_empty() => {
                value.emit_load(code);
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "isNone",
                    "(Ljava/lang/Object;)Z",
                );
            }
            "Some" if args.len() == 1 => {
                self.emit_constructor_inner_condition(
                    code,
                    value,
                    "isSome",
                    "optionValue",
                    &args[0],
                );
            }
            "Ok" if args.len() == 1 => {
                self.emit_constructor_inner_condition(code, value, "isOk", "resultValue", &args[0]);
            }
            "Err" if args.len() == 1 => {
                self.emit_constructor_inner_condition(
                    code,
                    value,
                    "isErr",
                    "resultValue",
                    &args[0],
                );
            }
            "Nil" if args.is_empty() => {
                value.emit_load(code);
                code.invokestatic(
                    &self.program.options.runtime_class,
                    "isNil",
                    "(Ljava/lang/Object;)Z",
                );
            }
            "Cons" if args.len() == 2 => {
                self.emit_constructor_pair_condition(
                    code,
                    value,
                    "isCons",
                    ("listHead", &args[0]),
                    ("listTail", &args[1]),
                );
            }
            _ => self.emit_generic_constructor_condition(code, value, name, args),
        }
    }

    fn emit_generic_constructor_condition(
        &mut self,
        code: &mut MethodCode,
        value: ValueRef,
        name: &[String],
        args: &[IrPattern],
    ) {
        let fail = code.new_label();
        let done = code.new_label();
        value.emit_load(code);
        code.ldc_string(&name.join("::"));
        code.invokestatic(
            &self.program.options.runtime_class,
            "isAdt",
            "(Ljava/lang/Object;Ljava/lang/String;)Z",
        );
        code.branch_to(0x99, fail);
        for (index, pattern) in args.iter().enumerate() {
            let inner_value = ValueRef::AdtPayload {
                base: Box::new(value.clone()),
                index,
                runtime: self.program.options.runtime_class.clone(),
            };
            self.emit_pattern_condition(code, pattern, inner_value);
            code.branch_to(0x99, fail);
        }
        code.push_i32(1);
        code.branch_to(0xa7, done);
        code.bind(fail);
        code.push_i32(0);
        code.bind(done);
    }

    fn emit_constructor_inner_condition(
        &mut self,
        code: &mut MethodCode,
        value: ValueRef,
        test: &str,
        getter: &str,
        inner: &IrPattern,
    ) {
        let fail = code.new_label();
        let done = code.new_label();
        value.emit_load(code);
        code.invokestatic(
            &self.program.options.runtime_class,
            test,
            "(Ljava/lang/Object;)Z",
        );
        code.branch_to(0x99, fail);
        let inner_value = ValueRef::RuntimeUnary {
            base: Box::new(value),
            method: getter.to_string(),
            runtime: self.program.options.runtime_class.clone(),
        };
        self.emit_pattern_condition(code, inner, inner_value);
        code.branch_to(0x99, fail);
        code.push_i32(1);
        code.branch_to(0xa7, done);
        code.bind(fail);
        code.push_i32(0);
        code.bind(done);
    }

    fn emit_constructor_pair_condition(
        &mut self,
        code: &mut MethodCode,
        value: ValueRef,
        test: &str,
        left: (&str, &IrPattern),
        right: (&str, &IrPattern),
    ) {
        let fail = code.new_label();
        let done = code.new_label();
        value.emit_load(code);
        code.invokestatic(
            &self.program.options.runtime_class,
            test,
            "(Ljava/lang/Object;)Z",
        );
        code.branch_to(0x99, fail);
        for (getter, pattern) in [left, right] {
            let inner_value = ValueRef::RuntimeUnary {
                base: Box::new(value.clone()),
                method: getter.to_string(),
                runtime: self.program.options.runtime_class.clone(),
            };
            self.emit_pattern_condition(code, pattern, inner_value);
            code.branch_to(0x99, fail);
        }
        code.push_i32(1);
        code.branch_to(0xa7, done);
        code.bind(fail);
        code.push_i32(0);
        code.bind(done);
    }

    fn emit_pattern_bindings(
        &mut self,
        code: &mut MethodCode,
        pattern: &IrPattern,
        value: ValueRef,
    ) {
        match &pattern.kind {
            IrPatternKind::Binding(name) => {
                value.emit_load(code);
                let slot = self.bind_local(name);
                code.astore(slot);
            }
            IrPatternKind::Record(fields) => {
                for field in fields {
                    self.emit_pattern_bindings(
                        code,
                        &field.pattern,
                        ValueRef::RecordField {
                            base: Box::new(value.clone()),
                            field: field.name.clone(),
                            runtime: self.program.options.runtime_class.clone(),
                        },
                    );
                }
            }
            IrPatternKind::Constructor { name, args } => {
                let Some(constructor) = name.last().map(String::as_str) else {
                    return;
                };
                let getter = match constructor {
                    "Some" => Some("optionValue"),
                    "Ok" | "Err" => Some("resultValue"),
                    _ => None,
                };
                if let (Some(getter), [inner]) = (getter, args.as_slice()) {
                    self.emit_pattern_bindings(
                        code,
                        inner,
                        ValueRef::RuntimeUnary {
                            base: Box::new(value.clone()),
                            method: getter.to_string(),
                            runtime: self.program.options.runtime_class.clone(),
                        },
                    );
                }
                if let ("Cons", [head, tail]) = (constructor, args.as_slice()) {
                    self.emit_pattern_bindings(
                        code,
                        head,
                        ValueRef::RuntimeUnary {
                            base: Box::new(value.clone()),
                            method: "listHead".to_string(),
                            runtime: self.program.options.runtime_class.clone(),
                        },
                    );
                    self.emit_pattern_bindings(
                        code,
                        tail,
                        ValueRef::RuntimeUnary {
                            base: Box::new(value.clone()),
                            method: "listTail".to_string(),
                            runtime: self.program.options.runtime_class.clone(),
                        },
                    );
                }
                if !matches!(constructor, "Some" | "Ok" | "Err" | "Cons") {
                    for (index, pattern) in args.iter().enumerate() {
                        self.emit_pattern_bindings(
                            code,
                            pattern,
                            ValueRef::AdtPayload {
                                base: Box::new(value.clone()),
                                index,
                                runtime: self.program.options.runtime_class.clone(),
                            },
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn emit_contract_check(
        &mut self,
        code: &mut MethodCode,
        contract: &IrContract,
        position: ContractCheckPosition,
    ) {
        if contract.obligation_status != ContractObligationStatus::RuntimeRequired {
            return;
        }
        let blame = match (contract.kind, position) {
            (ContractKind::Require, _) => "caller",
            (ContractKind::Ensure, _) => "implementation",
            (ContractKind::Invariant, ContractCheckPosition::Entry) => "caller",
            (ContractKind::Invariant, ContractCheckPosition::Return) => "implementation",
        };
        self.emit_contract_value(code, &contract.predicate);
        let clause = match contract.kind {
            ContractKind::Require => "require",
            ContractKind::Ensure => "ensure",
            ContractKind::Invariant => "invariant",
        };
        code.ldc_string(clause);
        code.ldc_string(&contract.predicate);
        code.ldc_string(&self.function.name);
        code.ldc_string(blame);
        code.ldc_string(&contract.node_id.display("contract"));
        code.ldc_string(contract.span.file.as_str());
        code.push_i32(contract.span.start.line as i32);
        code.push_i32(contract.span.start.column as i32);
        code.push_i32(contract.span.end.line as i32);
        code.push_i32(contract.span.end.column as i32);
        code.invokestatic(
            &self.program.options.runtime_class,
            "checkContract",
            "(Ljava/lang/Object;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;IIII)V",
        );
    }

    fn emit_contract_value(&mut self, code: &mut MethodCode, text: &str) {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("not ") {
            self.emit_contract_value(code, rest);
            code.invokestatic(
                &self.program.options.runtime_class,
                "not",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
            );
            return;
        }
        for (op_text, op) in [
            ("==", BinaryOp::Equal),
            ("!=", BinaryOp::NotEqual),
            (">=", BinaryOp::GreaterEqual),
            ("<=", BinaryOp::LessEqual),
            (">", BinaryOp::Greater),
            ("<", BinaryOp::Less),
            ("+", BinaryOp::Add),
            ("-", BinaryOp::Subtract),
            ("*", BinaryOp::Multiply),
            ("/", BinaryOp::Divide),
        ] {
            if let Some((left, right)) = split_contract_binary(text, op_text) {
                self.emit_contract_value(code, left);
                self.emit_contract_value(code, right);
                code.invokestatic(
                    &self.program.options.runtime_class,
                    binary_method(op),
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                );
                return;
            }
        }
        if let Some((callee, args)) = parse_contract_call(text) {
            for arg in &args {
                self.emit_contract_value(code, arg);
            }
            let function = callee.rsplit("::").next().unwrap_or(callee);
            code.invokestatic(
                &self.program.options.program_class,
                &self.program.function_name(function),
                &object_method_descriptor(args.len()),
            );
            return;
        }
        if let Some((base, field)) = text.split_once('.') {
            self.emit_contract_value(code, base);
            code.ldc_string(field);
            code.invokestatic(
                &self.program.options.runtime_class,
                "recordField",
                "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
            );
            return;
        }
        if text == "true" {
            code.getstatic("java/lang/Boolean", "TRUE", "Ljava/lang/Boolean;");
        } else if text == "false" {
            code.getstatic("java/lang/Boolean", "FALSE", "Ljava/lang/Boolean;");
        } else if text == "()" {
            code.getstatic(
                &self.program.options.runtime_class,
                "UNIT",
                &format!("L{}$Unit;", self.program.options.runtime_class),
            );
        } else if text.starts_with('"') && text.ends_with('"') {
            code.ldc_string(&veln_string_literal_value(text));
        } else if let Ok(value) = text.parse::<i64>() {
            code.ldc_long(value);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        } else if let Ok(value) = text.parse::<f64>() {
            code.ldc_double(value);
            code.invokestatic("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;");
        } else if self.locals.contains_key(text) {
            code.aload(self.local_slot(text));
        } else {
            code.getstatic("java/lang/Boolean", "FALSE", "Ljava/lang/Boolean;");
        }
    }

    fn has_ensure_contracts(&self) -> bool {
        self.function.contracts.iter().any(|contract| {
            matches!(
                contract.kind,
                ContractKind::Ensure | ContractKind::Invariant
            ) && contract.obligation_status == ContractObligationStatus::RuntimeRequired
        })
    }

    fn emit_ensure_checks_for_result(&mut self, code: &mut MethodCode, result: u16) {
        let previous = self
            .function
            .return_binding
            .as_ref()
            .map(|binding| (binding.clone(), self.locals.insert(binding.clone(), result)));
        for contract in self.function.contracts.iter().filter(|contract| {
            matches!(
                contract.kind,
                ContractKind::Ensure | ContractKind::Invariant
            )
        }) {
            self.emit_contract_check(code, contract, ContractCheckPosition::Return);
        }
        if let Some((binding, old)) = previous {
            if let Some(old) = old {
                self.locals.insert(binding, old);
            } else {
                self.locals.remove(&binding);
            }
        }
    }

    fn bind_local(&mut self, name: &str) -> u16 {
        let slot = self.alloc_local();
        self.locals.insert(name.to_string(), slot);
        slot
    }

    fn alloc_local(&mut self) -> u16 {
        let slot = self.next_local;
        self.next_local += 1;
        self.max_local = self.max_local.max(self.next_local);
        slot
    }

    fn local_slot(&self, name: &str) -> u16 {
        *self
            .locals
            .get(name)
            .unwrap_or_else(|| panic!("missing JVM local `{name}`"))
    }
}

pub(crate) fn classify_tail_recursion(function: &IrFunction) -> TailRecursionEligibility {
    if has_runtime_return_contract(function) {
        return TailRecursionEligibility::RuntimeReturnContract;
    }
    let mut facts = TailRecursionFacts::default();
    for stmt in &function.body {
        scan_stmt_tail_recursion(stmt, &function.name, &mut facts);
    }
    if facts.has_indirect_value_call {
        return TailRecursionEligibility::IndirectValueCall;
    }
    if facts.has_non_tail_self_call {
        return TailRecursionEligibility::NonTailSelfCall;
    }
    if facts.has_tail_self_call {
        TailRecursionEligibility::Eligible
    } else {
        TailRecursionEligibility::NotRecursive
    }
}

fn has_runtime_return_contract(function: &IrFunction) -> bool {
    function.contracts.iter().any(|contract| {
        matches!(
            contract.kind,
            ContractKind::Ensure | ContractKind::Invariant
        ) && contract.obligation_status == ContractObligationStatus::RuntimeRequired
    })
}

#[derive(Default)]
struct TailRecursionFacts {
    has_tail_self_call: bool,
    has_non_tail_self_call: bool,
    has_indirect_value_call: bool,
}

fn scan_stmt_tail_recursion(stmt: &IrStmt, function: &str, facts: &mut TailRecursionFacts) {
    match &stmt.kind {
        IrStmtKind::Let { value, .. } | IrStmtKind::Expr { value } => {
            scan_expr_tail_recursion(value, function, false, facts);
        }
        IrStmtKind::Return { value } => scan_expr_tail_recursion(value, function, true, facts),
    }
}

fn scan_expr_tail_recursion(
    expr: &IrExpr,
    function: &str,
    tail_position: bool,
    facts: &mut TailRecursionFacts,
) {
    match &expr.kind {
        IrExprKind::Call { target, args } => {
            match target {
                IrCallTarget::Function(name) if name == function && tail_position => {
                    facts.has_tail_self_call = true;
                }
                IrCallTarget::Function(name) if name == function => {
                    facts.has_non_tail_self_call = true;
                }
                IrCallTarget::Value(_) => {
                    facts.has_indirect_value_call = true;
                }
                _ => {}
            }
            for arg in args {
                scan_expr_tail_recursion(arg, function, false, facts);
            }
        }
        IrExprKind::Match { scrutinee, arms } => {
            scan_expr_tail_recursion(scrutinee, function, false, facts);
            for arm in arms {
                scan_expr_tail_recursion(&arm.value, function, tail_position, facts);
            }
        }
        IrExprKind::ResultOk(value)
        | IrExprKind::ResultErr(value)
        | IrExprKind::OptionSome(value)
        | IrExprKind::FieldAccess { base: value, .. }
        | IrExprKind::Try(value)
        | IrExprKind::Prefix { expr: value, .. } => {
            scan_expr_tail_recursion(value, function, false, facts);
        }
        IrExprKind::ListCons { head, tail } => {
            scan_expr_tail_recursion(head, function, false, facts);
            scan_expr_tail_recursion(tail, function, false, facts);
        }
        IrExprKind::AdtVariant { payloads, .. } | IrExprKind::List(payloads) => {
            for value in payloads {
                scan_expr_tail_recursion(value, function, false, facts);
            }
        }
        IrExprKind::Record(fields) => {
            for field in fields {
                scan_record_field_tail_recursion(field, function, facts);
            }
        }
        IrExprKind::Dict(entries) => {
            for entry in entries {
                scan_expr_tail_recursion(&entry.key, function, false, facts);
                scan_expr_tail_recursion(&entry.value, function, false, facts);
            }
        }
        IrExprKind::Binary { left, right, .. } => {
            scan_expr_tail_recursion(left, function, false, facts);
            scan_expr_tail_recursion(right, function, false, facts);
        }
        IrExprKind::Local(_)
        | IrExprKind::BoolLiteral(_)
        | IrExprKind::StringLiteral(_)
        | IrExprKind::IntLiteral(_)
        | IrExprKind::FloatLiteral(_)
        | IrExprKind::Unit
        | IrExprKind::FunctionValue(_)
        | IrExprKind::OptionNone
        | IrExprKind::ListNil => {}
    }
}

fn scan_record_field_tail_recursion(
    field: &IrRecordField,
    function: &str,
    facts: &mut TailRecursionFacts,
) {
    scan_expr_tail_recursion(&field.value, function, false, facts);
}

#[derive(Clone)]
enum ValueRef {
    Local(u16),
    RecordField {
        base: Box<ValueRef>,
        field: String,
        runtime: String,
    },
    RuntimeUnary {
        base: Box<ValueRef>,
        method: String,
        runtime: String,
    },
    AdtPayload {
        base: Box<ValueRef>,
        index: usize,
        runtime: String,
    },
}

impl ValueRef {
    fn emit_load(&self, code: &mut MethodCode) {
        match self {
            Self::Local(slot) => code.aload(*slot),
            Self::RecordField {
                base,
                field,
                runtime,
            } => {
                base.emit_load(code);
                code.ldc_string(field);
                code.invokestatic(
                    runtime,
                    "recordField",
                    "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
                );
            }
            Self::RuntimeUnary {
                base,
                method,
                runtime,
            } => {
                base.emit_load(code);
                code.invokestatic(runtime, method, "(Ljava/lang/Object;)Ljava/lang/Object;");
            }
            Self::AdtPayload {
                base,
                index,
                runtime,
            } => {
                base.emit_load(code);
                code.push_i32(*index as i32);
                code.invokestatic(
                    runtime,
                    "adtPayload",
                    "(Ljava/lang/Object;I)Ljava/lang/Object;",
                );
            }
        }
    }
}

fn object_method_descriptor(arg_count: usize) -> String {
    let mut descriptor = "(".to_string();
    for _ in 0..arg_count {
        descriptor.push_str("Ljava/lang/Object;");
    }
    descriptor.push_str(")Ljava/lang/Object;");
    descriptor
}

fn split_contract_binary<'a>(text: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index + op.len() <= text.len() {
        let ch = text[index..].chars().next()?;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && bytes[index..].starts_with(op.as_bytes()) => {
                let left = text[..index].trim();
                let right = text[index + op.len()..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left, right));
                }
            }
            _ => {}
        }
        index += ch.len_utf8();
    }
    None
}

fn parse_contract_call(text: &str) -> Option<(&str, Vec<&str>)> {
    let open = text.find('(')?;
    if !text.ends_with(')') {
        return None;
    }
    let callee = text[..open].trim();
    if callee.is_empty()
        || !callee
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == ':')
    {
        return None;
    }
    let inner = &text[open + 1..text.len() - 1];
    let args = split_contract_args(inner);
    Some((callee, args))
}

fn split_contract_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut start = 0usize;
    let mut index = 0usize;
    while index < text.len() {
        let ch = text[index..]
            .chars()
            .next()
            .expect("index should stay on a character boundary");
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = text[start..index].trim();
                if !arg.is_empty() {
                    args.push(arg);
                }
                start = index + 1;
            }
            _ => {}
        }
        index += ch.len_utf8();
    }
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}

include!(concat!(env!("OUT_DIR"), "/runtime_classes.rs"));

fn runtime_classes() -> Vec<JvmClassFile> {
    RUNTIME_CLASSES
        .iter()
        .map(|(path, contents)| JvmClassFile {
            path: (*path).to_string(),
            contents: contents.to_vec(),
        })
        .collect()
}

struct ClassBuilder {
    name: String,
    access_flags: u16,
    constant_pool: Rc<RefCell<ConstantPool>>,
    methods: Vec<MethodInfo>,
    interfaces: Vec<String>,
}

impl ClassBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            access_flags: 0x0021,
            constant_pool: Rc::new(RefCell::new(ConstantPool::new())),
            methods: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    fn add_default_constructor(&mut self, access_flags: u16) {
        let mut code = MethodCode::new(Rc::clone(&self.constant_pool));
        code.aload(0);
        code.invokespecial(JAVA_LANG_OBJECT, "<init>", "()V");
        code.op(0xb1);
        self.add_method(MethodInfo {
            access_flags,
            name: "<init>".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 1,
            max_locals: 1,
            code: code.code,
            exceptions: Vec::new(),
        });
    }

    fn add_method(&mut self, method: MethodInfo) {
        self.methods.push(method);
    }

    fn finish(self) -> Vec<u8> {
        let this_class = self.constant_pool.borrow_mut().class(&self.name);
        let super_class = self.constant_pool.borrow_mut().class(JAVA_LANG_OBJECT);
        let code_name = self.constant_pool.borrow_mut().utf8("Code");
        let mut methods = Vec::new();
        for method in &self.methods {
            let name = self.constant_pool.borrow_mut().utf8(&method.name);
            let descriptor = self.constant_pool.borrow_mut().utf8(&method.descriptor);
            let mut exceptions = Vec::new();
            for handler in &method.exceptions {
                exceptions.push(SerializedExceptionHandler {
                    start_pc: handler.start_pc,
                    end_pc: handler.end_pc,
                    handler_pc: handler.handler_pc,
                    catch_type: self.constant_pool.borrow_mut().class(&handler.catch_type),
                });
            }
            methods.push(SerializedMethod {
                access_flags: method.access_flags,
                name,
                descriptor,
                code_name,
                max_stack: method.max_stack,
                max_locals: method.max_locals,
                code: method.code.clone(),
                exceptions,
            });
        }
        let mut interfaces = Vec::new();
        for interface in &self.interfaces {
            interfaces.push(self.constant_pool.borrow_mut().class(interface));
        }

        let mut out = Vec::new();
        write_u32(&mut out, 0xcafebabe);
        write_u16(&mut out, 0);
        write_u16(&mut out, PROGRAM_MAJOR_VERSION);
        self.constant_pool.borrow().write(&mut out);
        write_u16(&mut out, self.access_flags);
        write_u16(&mut out, this_class);
        write_u16(&mut out, super_class);
        write_u16(&mut out, interfaces.len() as u16);
        for interface in interfaces {
            write_u16(&mut out, interface);
        }
        write_u16(&mut out, 0);
        write_u16(&mut out, methods.len() as u16);
        for method in methods {
            method.write(&mut out);
        }
        write_u16(&mut out, 0);
        out
    }
}

struct MethodInfo {
    access_flags: u16,
    name: String,
    descriptor: String,
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
    exceptions: Vec<ExceptionHandler>,
}

struct SerializedMethod {
    access_flags: u16,
    name: u16,
    descriptor: u16,
    code_name: u16,
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
    exceptions: Vec<SerializedExceptionHandler>,
}

impl SerializedMethod {
    fn write(&self, out: &mut Vec<u8>) {
        write_u16(out, self.access_flags);
        write_u16(out, self.name);
        write_u16(out, self.descriptor);
        write_u16(out, 1);
        write_u16(out, self.code_name);
        let attribute_length = 12 + self.code.len() + self.exceptions.len() * 8;
        write_u32(out, attribute_length as u32);
        write_u16(out, self.max_stack);
        write_u16(out, self.max_locals);
        write_u32(out, self.code.len() as u32);
        out.extend_from_slice(&self.code);
        write_u16(out, self.exceptions.len() as u16);
        for exception in &self.exceptions {
            write_u16(out, exception.start_pc as u16);
            write_u16(out, exception.end_pc as u16);
            write_u16(out, exception.handler_pc as u16);
            write_u16(out, exception.catch_type);
        }
        write_u16(out, 0);
    }
}

#[derive(Clone)]
struct ExceptionHandler {
    start_pc: usize,
    end_pc: usize,
    handler_pc: usize,
    catch_type: String,
}

struct SerializedExceptionHandler {
    start_pc: usize,
    end_pc: usize,
    handler_pc: usize,
    catch_type: u16,
}

struct MethodCode {
    constant_pool: Rc<RefCell<ConstantPool>>,
    code: Vec<u8>,
    labels: Vec<Option<usize>>,
    patches: Vec<Patch>,
    max_stack: u16,
    max_locals: u16,
    exceptions: Vec<ExceptionHandler>,
}

impl MethodCode {
    fn new(constant_pool: Rc<RefCell<ConstantPool>>) -> Self {
        Self {
            constant_pool,
            code: Vec::new(),
            labels: Vec::new(),
            patches: Vec::new(),
            max_stack: 64,
            max_locals: 0,
            exceptions: Vec::new(),
        }
    }

    fn op(&mut self, op: u8) {
        self.code.push(op);
    }

    fn mark(&self) -> usize {
        self.code.len()
    }

    fn new_label(&mut self) -> usize {
        let id = self.labels.len();
        self.labels.push(None);
        id
    }

    fn bind(&mut self, label: usize) {
        self.labels[label] = Some(self.code.len());
        self.patch_bound_labels();
    }

    fn branch(&mut self, op: u8) -> usize {
        let label = self.new_label();
        self.branch_to(op, label);
        label
    }

    fn branch_to(&mut self, op: u8, label: usize) {
        let pos = self.code.len();
        self.code.push(op);
        self.code.extend_from_slice(&[0, 0]);
        self.patches.push(Patch { pos, label });
    }

    fn patch_bound_labels(&mut self) {
        for patch in &self.patches {
            if let Some(target) = self.labels[patch.label] {
                let offset = target as isize - patch.pos as isize;
                let bytes = (offset as i16).to_be_bytes();
                self.code[patch.pos + 1] = bytes[0];
                self.code[patch.pos + 2] = bytes[1];
            }
        }
    }

    fn aload(&mut self, slot: u16) {
        self.max_locals = self.max_locals.max(slot + 1);
        match slot {
            0..=3 => self.code.push(0x2a + slot as u8),
            _ if slot <= u8::MAX as u16 => {
                self.code.push(0x19);
                self.code.push(slot as u8);
            }
            _ => panic!("too many JVM locals"),
        }
    }

    fn astore(&mut self, slot: u16) {
        self.max_locals = self.max_locals.max(slot + 1);
        match slot {
            0..=3 => self.code.push(0x4b + slot as u8),
            _ if slot <= u8::MAX as u16 => {
                self.code.push(0x3a);
                self.code.push(slot as u8);
            }
            _ => panic!("too many JVM locals"),
        }
    }

    fn push_i32(&mut self, value: i32) {
        match value {
            -1 => self.code.push(0x02),
            0..=5 => self.code.push(0x03 + value as u8),
            -128..=127 => {
                self.code.push(0x10);
                self.code.push(value as i8 as u8);
            }
            -32768..=32767 => {
                self.code.push(0x11);
                self.code.extend_from_slice(&(value as i16).to_be_bytes());
            }
            _ => panic!("integer constant out of JVM push range"),
        }
    }

    fn ldc_string(&mut self, value: &str) {
        let index = self.constant_pool.borrow_mut().string(value);
        self.ldc_index(index);
    }

    fn ldc_long(&mut self, value: i64) {
        let index = self.constant_pool.borrow_mut().long(value);
        self.code.push(0x14);
        write_u16(&mut self.code, index);
    }

    fn ldc_double(&mut self, value: f64) {
        let index = self.constant_pool.borrow_mut().double(value);
        self.code.push(0x14);
        write_u16(&mut self.code, index);
    }

    fn ldc_index(&mut self, index: u16) {
        if index <= u8::MAX as u16 {
            self.code.push(0x12);
            self.code.push(index as u8);
        } else {
            self.code.push(0x13);
            write_u16(&mut self.code, index);
        }
    }

    fn getstatic(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .fieldref(class, name, descriptor);
        self.code.push(0xb2);
        write_u16(&mut self.code, index);
    }

    fn new_class(&mut self, class: &str) {
        let index = self.constant_pool.borrow_mut().class(class);
        self.code.push(0xbb);
        write_u16(&mut self.code, index);
    }

    fn anewarray(&mut self, class: &str) {
        let index = self.constant_pool.borrow_mut().class(class);
        self.code.push(0xbd);
        write_u16(&mut self.code, index);
    }

    fn invokestatic(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .methodref(class, name, descriptor);
        self.code.push(0xb8);
        write_u16(&mut self.code, index);
    }

    fn invokespecial(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .methodref(class, name, descriptor);
        self.code.push(0xb7);
        write_u16(&mut self.code, index);
    }

    fn invokevirtual(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .methodref(class, name, descriptor);
        self.code.push(0xb6);
        write_u16(&mut self.code, index);
    }
}

struct Patch {
    pos: usize,
    label: usize,
}

#[derive(Default)]
struct ConstantPool {
    entries: Vec<CpEntry>,
    indexes: BTreeMap<CpKey, u16>,
}

impl ConstantPool {
    fn new() -> Self {
        Self::default()
    }

    fn utf8(&mut self, value: &str) -> u16 {
        self.intern(CpKey::Utf8(value.to_string()), |key| match key {
            CpKey::Utf8(value) => CpEntry::Utf8(value.clone()),
            _ => unreachable!(),
        })
    }

    fn class(&mut self, name: &str) -> u16 {
        let name_index = self.utf8(name);
        self.intern(CpKey::Class(name.to_string()), |_| {
            CpEntry::Class(name_index)
        })
    }

    fn string(&mut self, value: &str) -> u16 {
        let utf8 = self.utf8(value);
        self.intern(CpKey::String(value.to_string()), |_| CpEntry::String(utf8))
    }

    fn long(&mut self, value: i64) -> u16 {
        self.intern(CpKey::Long(value), |_| CpEntry::Long(value))
    }

    fn double(&mut self, value: f64) -> u16 {
        self.intern(CpKey::Double(value.to_bits()), |_| {
            CpEntry::Double(value.to_bits())
        })
    }

    fn name_and_type(&mut self, name: &str, descriptor: &str) -> u16 {
        let name_index = self.utf8(name);
        let descriptor_index = self.utf8(descriptor);
        self.intern(
            CpKey::NameAndType(name.to_string(), descriptor.to_string()),
            |_| CpEntry::NameAndType {
                name_index,
                descriptor_index,
            },
        )
    }

    fn fieldref(&mut self, class: &str, name: &str, descriptor: &str) -> u16 {
        let class_index = self.class(class);
        let name_and_type = self.name_and_type(name, descriptor);
        self.intern(
            CpKey::Fieldref(class.to_string(), name.to_string(), descriptor.to_string()),
            |_| CpEntry::Fieldref {
                class_index,
                name_and_type,
            },
        )
    }

    fn methodref(&mut self, class: &str, name: &str, descriptor: &str) -> u16 {
        let class_index = self.class(class);
        let name_and_type = self.name_and_type(name, descriptor);
        self.intern(
            CpKey::Methodref(class.to_string(), name.to_string(), descriptor.to_string()),
            |_| CpEntry::Methodref {
                class_index,
                name_and_type,
            },
        )
    }

    fn intern<F>(&mut self, key: CpKey, build: F) -> u16
    where
        F: FnOnce(&CpKey) -> CpEntry,
    {
        if let Some(index) = self.indexes.get(&key) {
            return *index;
        }
        let index = (self.entries.len() + 1) as u16;
        let entry = build(&key);
        self.entries.push(entry);
        if matches!(key, CpKey::Long(_) | CpKey::Double(_)) {
            self.entries.push(CpEntry::Padding);
        }
        self.indexes.insert(key, index);
        index
    }

    fn write(&self, out: &mut Vec<u8>) {
        write_u16(out, (self.entries.len() + 1) as u16);
        for entry in &self.entries {
            match entry {
                CpEntry::Utf8(value) => {
                    out.push(1);
                    write_u16(out, value.len() as u16);
                    out.extend_from_slice(value.as_bytes());
                }
                CpEntry::Class(index) => {
                    out.push(7);
                    write_u16(out, *index);
                }
                CpEntry::String(index) => {
                    out.push(8);
                    write_u16(out, *index);
                }
                CpEntry::Fieldref {
                    class_index,
                    name_and_type,
                } => {
                    out.push(9);
                    write_u16(out, *class_index);
                    write_u16(out, *name_and_type);
                }
                CpEntry::Methodref {
                    class_index,
                    name_and_type,
                } => {
                    out.push(10);
                    write_u16(out, *class_index);
                    write_u16(out, *name_and_type);
                }
                CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                } => {
                    out.push(12);
                    write_u16(out, *name_index);
                    write_u16(out, *descriptor_index);
                }
                CpEntry::Long(value) => {
                    out.push(5);
                    out.extend_from_slice(&value.to_be_bytes());
                }
                CpEntry::Double(bits) => {
                    out.push(6);
                    out.extend_from_slice(&bits.to_be_bytes());
                }
                CpEntry::Padding => {}
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CpKey {
    Utf8(String),
    Class(String),
    String(String),
    Fieldref(String, String, String),
    Methodref(String, String, String),
    NameAndType(String, String),
    Long(i64),
    Double(u64),
}

enum CpEntry {
    Utf8(String),
    Class(u16),
    String(u16),
    Fieldref {
        class_index: u16,
        name_and_type: u16,
    },
    Methodref {
        class_index: u16,
        name_and_type: u16,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    Long(i64),
    Double(u64),
    Padding,
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}
