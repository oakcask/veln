use super::*;

pub(super) struct FunctionBytecodeEmitter<'a, 'program> {
    pub(super) program: &'a ClassfileEmitter<'program>,
    pub(super) function: &'a IrFunction,
    pub(super) locals: BTreeMap<String, u16>,
    pub(super) next_local: u16,
    pub(super) max_local: u16,
    pub(super) tail_loop_start: Option<usize>,
    pub(super) active_handler_frames: usize,
}

#[derive(Clone, Copy)]
pub(super) enum ContractCheckPosition {
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
    pub(super) fn new(program: &'a ClassfileEmitter<'program>, function: &'a IrFunction) -> Self {
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
            active_handler_frames: 0,
        }
    }

    pub(super) fn emit(&mut self, code: &mut MethodCode) {
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

    pub(super) fn emit_stmt(&mut self, code: &mut MethodCode, stmt: &IrStmt) {
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

    pub(super) fn emit_tail_expr(&mut self, code: &mut MethodCode, expr: &IrExpr) -> bool {
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

    pub(super) fn emit_tail_self_call(&mut self, code: &mut MethodCode, args: &[IrExpr]) {
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

    pub(super) fn emit_tail_match(
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

    pub(super) fn emit_expr(&mut self, code: &mut MethodCode, expr: &IrExpr) {
        match &expr.kind {
            IrExprKind::Local(name) => self.emit_local(code, name),
            IrExprKind::BoolLiteral(value) => self.emit_bool_literal(code, *value),
            IrExprKind::StringLiteral(value) => self.emit_string_literal(code, value),
            IrExprKind::IntLiteral(value) => self.emit_int_literal(code, value),
            IrExprKind::FloatLiteral(value) => self.emit_float_literal(code, value),
            IrExprKind::Unit => self.emit_unit(code),
            IrExprKind::FunctionValue(name) => self.emit_function_value(code, name),
            IrExprKind::ResultOk(value) => self.emit_result_constructor(code, "ok", value),
            IrExprKind::ResultErr(value) => self.emit_result_constructor(code, "err", value),
            IrExprKind::OptionSome(value) => self.emit_option_some(code, value),
            IrExprKind::OptionNone => self.emit_option_none(code),
            IrExprKind::ListNil => self.emit_list_nil(code),
            IrExprKind::ListCons { head, tail } => self.emit_list_cons(code, head, tail),
            IrExprKind::AdtVariant { name, payloads } => {
                self.emit_adt_variant(code, name, payloads)
            }
            IrExprKind::Call { target, args } => self.emit_call(code, expr, target, args),
            IrExprKind::FieldAccess { base, field } => self.emit_field_access(code, base, field),
            IrExprKind::Perform {
                effect,
                operation,
                args,
            } => self.emit_perform(code, effect, operation, args),
            IrExprKind::Handle {
                effect,
                providers,
                context_args,
                body,
            } => self.emit_handle(code, effect, providers, context_args, body),
            IrExprKind::Try(value) => self.emit_try(code, value),
            IrExprKind::Record(fields) => self.emit_record(code, fields),
            IrExprKind::Dict(entries) => self.emit_dict(code, entries),
            IrExprKind::List(items) => self.emit_list(code, items),
            IrExprKind::Match { scrutinee, arms } => self.emit_match(code, scrutinee, arms),
            IrExprKind::Prefix { op, expr } => self.emit_prefix(code, *op, expr),
            IrExprKind::Binary { op, left, right } => self.emit_binary(code, *op, left, right),
        }
    }

    pub(super) fn emit_local(&mut self, code: &mut MethodCode, name: &str) {
        code.aload(self.local_slot(name));
    }

    pub(super) fn emit_bool_literal(&mut self, code: &mut MethodCode, value: bool) {
        code.getstatic(
            "java/lang/Boolean",
            if value { "TRUE" } else { "FALSE" },
            "Ljava/lang/Boolean;",
        );
    }

    pub(super) fn emit_string_literal(&mut self, code: &mut MethodCode, value: &str) {
        code.ldc_string(&veln_string_literal_value(value));
    }

    pub(super) fn emit_int_literal(&mut self, code: &mut MethodCode, value: &str) {
        code.ldc_long(
            parse_integer_literal(value)
                .map(|literal| literal.value)
                .unwrap_or(0),
        );
        code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
    }

    pub(super) fn emit_float_literal(&mut self, code: &mut MethodCode, value: &str) {
        code.ldc_double(value.parse::<f64>().unwrap_or(0.0));
        code.invokestatic("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;");
    }

    pub(super) fn emit_unit(&mut self, code: &mut MethodCode) {
        code.getstatic(
            &self.program.options.runtime_class,
            "UNIT",
            &format!("L{}$Unit;", self.program.options.runtime_class),
        );
    }

    pub(super) fn emit_function_value(&mut self, code: &mut MethodCode, name: &str) {
        let class_name = format!(
            "{}${}",
            self.program.options.program_class,
            self.program.function_name(name)
        );
        code.new_class(&class_name);
        code.op(0x59);
        code.invokespecial(&class_name, "<init>", "()V");
    }

    pub(super) fn emit_result_constructor(
        &mut self,
        code: &mut MethodCode,
        method: &str,
        value: &IrExpr,
    ) {
        self.emit_unary_runtime_with_descriptor(
            code,
            method,
            &format!(
                "(Ljava/lang/Object;)L{}$Result;",
                self.program.options.runtime_class
            ),
            value,
        );
    }

    pub(super) fn emit_option_some(&mut self, code: &mut MethodCode, value: &IrExpr) {
        self.emit_unary_runtime_with_descriptor(
            code,
            "some",
            &format!(
                "(Ljava/lang/Object;)L{}$Option;",
                self.program.options.runtime_class
            ),
            value,
        );
    }

    pub(super) fn emit_option_none(&mut self, code: &mut MethodCode) {
        code.invokestatic(
            &self.program.options.runtime_class,
            "none",
            &format!("()L{}$Option;", self.program.options.runtime_class),
        );
    }

    pub(super) fn emit_list_nil(&mut self, code: &mut MethodCode) {
        code.invokestatic(
            &self.program.options.runtime_class,
            "listNil",
            "()Ljava/lang/Object;",
        );
    }

    pub(super) fn emit_list_cons(&mut self, code: &mut MethodCode, head: &IrExpr, tail: &IrExpr) {
        self.emit_expr(code, head);
        self.emit_expr(code, tail);
        code.invokestatic(
            &self.program.options.runtime_class,
            "listCons",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    pub(super) fn emit_adt_variant(
        &mut self,
        code: &mut MethodCode,
        name: &[String],
        payloads: &[IrExpr],
    ) {
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

    pub(super) fn emit_field_access(&mut self, code: &mut MethodCode, base: &IrExpr, field: &str) {
        self.emit_expr(code, base);
        code.ldc_string(field);
        code.invokestatic(
            &self.program.options.runtime_class,
            "recordField",
            "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
        );
    }

    pub(super) fn emit_list(&mut self, code: &mut MethodCode, items: &[IrExpr]) {
        self.emit_object_array(code, items.len(), |this, code, index| {
            this.emit_expr(code, &items[index]);
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_prefix(&mut self, code: &mut MethodCode, op: PrefixOp, expr: &IrExpr) {
        let method = match op {
            PrefixOp::Not => "not",
            PrefixOp::Negate => "negate",
            PrefixOp::BitwiseNot => "bitwiseNot",
        };
        self.emit_unary_runtime(code, method, expr);
    }
}
