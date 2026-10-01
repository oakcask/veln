use super::*;

pub(super) struct FunctionBytecodeEmitter<'a, 'program> {
    pub(super) program: &'a ClassfileEmitter<'program>,
    pub(super) function: &'a IrFunction,
    pub(super) locals: LocalBindings,
    pub(super) next_local: u16,
    pub(super) max_local: u16,
    pub(super) tail_loop_start: Option<usize>,
    active_cleanup_regions: Vec<Vec<RegisteredCleanup>>,
    active_unwind: Option<usize>,
    unwind_nodes: Vec<UnwindNode>,
    unwind_result: u16,
    unwind_return_label: Option<usize>,
    exception_unwind_used: bool,
}

pub(super) struct LocalBindings {
    values: BTreeMap<String, u16>,
    changes: Vec<LocalBindingChange>,
    #[cfg(test)]
    peak_retained_entry_count: usize,
}

struct LocalBindingChange {
    name: String,
    previous: Option<u16>,
}

impl LocalBindings {
    fn new(values: BTreeMap<String, u16>) -> Self {
        #[cfg(test)]
        let peak_retained_entry_count = values.len();
        Self {
            values,
            changes: Vec::new(),
            #[cfg(test)]
            peak_retained_entry_count,
        }
    }

    pub(super) fn mark(&self) -> usize {
        self.changes.len()
    }

    pub(super) fn insert(&mut self, name: String, slot: u16) {
        let previous = self.values.insert(name.clone(), slot);
        self.changes.push(LocalBindingChange { name, previous });
        #[cfg(test)]
        {
            self.peak_retained_entry_count = self
                .peak_retained_entry_count
                .max(self.values.len() + self.changes.len());
        }
    }

    pub(super) fn get(&self, name: &str) -> Option<&u16> {
        self.values.get(name)
    }

    pub(super) fn contains_key(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    pub(super) fn rollback(&mut self, mark: usize) {
        while self.changes.len() > mark {
            let change = self.changes.pop().expect("local binding change");
            if let Some(previous) = change.previous {
                self.values.insert(change.name, previous);
            } else {
                self.values.remove(&change.name);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn peak_retained_entry_count(&self) -> usize {
        self.peak_retained_entry_count
    }
}

#[derive(Clone)]
struct RegisteredCleanup {
    block: IrDeferredBlock,
    captures: BTreeMap<String, u16>,
}

#[derive(Clone)]
struct UnwindNode {
    label: usize,
    exception_entry_label: usize,
    exception_action_label: usize,
    parent: Option<usize>,
    action: UnwindAction,
}

#[derive(Clone)]
enum UnwindAction {
    Cleanup(RegisteredCleanup),
    PopHandler,
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
        let unwind_result = function.params.len() as u16;
        Self {
            program,
            function,
            locals: LocalBindings::new(locals),
            next_local: unwind_result + 1,
            max_local: unwind_result + 1,
            tail_loop_start: None,
            active_cleanup_regions: Vec::new(),
            active_unwind: None,
            unwind_nodes: Vec::new(),
            unwind_result,
            unwind_return_label: None,
            exception_unwind_used: false,
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
        if !self.emit_region(code, &self.function.body, true) {
            code.getstatic(
                &self.program.options.runtime_class,
                "UNIT",
                &format!("L{}$Unit;", self.program.options.runtime_class),
            );
            code.op(0xb0);
        }
        self.emit_unwind_blocks(code);
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
            IrStmtKind::Defer(_) => {
                unreachable!("defer registration is emitted by its cleanup region")
            }
        }
    }

    fn emit_region(&mut self, code: &mut MethodCode, body: &[IrStmt], function: bool) -> bool {
        let parent_unwind = self.active_unwind;
        self.active_cleanup_regions.push(Vec::new());
        let mut returns = false;
        for stmt in body {
            match &stmt.kind {
                IrStmtKind::Defer(block) => {
                    let cleanup = self.register_cleanup(code, block);
                    self.active_cleanup_regions
                        .last_mut()
                        .expect("active cleanup region")
                        .push(cleanup.clone());
                    self.push_unwind_node(code, UnwindAction::Cleanup(cleanup));
                }
                IrStmtKind::Return { value } => {
                    let exception_start = code.mark();
                    let unwind = self.active_unwind;
                    let has_cleanups = self
                        .active_cleanup_regions
                        .last()
                        .is_some_and(|cleanups| !cleanups.is_empty());
                    if function && !has_cleanups {
                        if self.emit_tail_expr(code, value) {
                            let exception_end = code.mark();
                            self.protect_exception_range(
                                code,
                                exception_start,
                                exception_end,
                                unwind,
                            );
                            returns = true;
                            break;
                        }
                    } else {
                        self.emit_expr(code, value);
                    }
                    let exception_end = code.mark();
                    self.protect_exception_range(code, exception_start, exception_end, unwind);
                    let result = self.alloc_local();
                    code.astore(result);
                    let cleanups = self
                        .active_cleanup_regions
                        .last()
                        .expect("active cleanup region")
                        .clone();
                    self.emit_cleanup_sequence(code, cleanups.iter().rev());
                    if function && self.has_ensure_contracts() {
                        self.emit_ensure_checks_for_result(code, result);
                    }
                    code.aload(result);
                    if function {
                        code.op(0xb0);
                    }
                    returns = true;
                    break;
                }
                _ => {
                    let exception_start = code.mark();
                    let unwind = self.active_unwind;
                    self.emit_stmt(code, stmt);
                    let exception_end = code.mark();
                    self.protect_exception_range(code, exception_start, exception_end, unwind);
                }
            }
        }
        self.active_cleanup_regions
            .pop()
            .expect("active cleanup region");
        self.active_unwind = parent_unwind;
        returns
    }

    fn register_cleanup(
        &mut self,
        code: &mut MethodCode,
        block: &IrDeferredBlock,
    ) -> RegisteredCleanup {
        let mut captures = BTreeMap::new();
        for capture in &block.captures {
            self.emit_local(code, &capture.name);
            let slot = self.alloc_local();
            code.astore(slot);
            captures.insert(capture.name.clone(), slot);
        }
        RegisteredCleanup {
            block: block.clone(),
            captures,
        }
    }

    fn emit_registered_cleanup(&mut self, code: &mut MethodCode, cleanup: &RegisteredCleanup) {
        let locals_mark = self.locals.mark();
        let saved_next = self.next_local;
        for (name, slot) in &cleanup.captures {
            self.locals.insert(name.clone(), *slot);
        }
        for stmt in &cleanup.block.body {
            match &stmt.kind {
                IrStmtKind::Return { value } => {
                    self.emit_expr(code, value);
                    code.op(0x57);
                }
                IrStmtKind::Defer(_) => {
                    unreachable!("nested deferred registration is rejected before backend lowering")
                }
                _ => self.emit_stmt(code, stmt),
            }
        }
        self.locals.rollback(locals_mark);
        self.next_local = saved_next;
    }

    fn emit_cleanup_sequence<'cleanup>(
        &mut self,
        code: &mut MethodCode,
        cleanups: impl Iterator<Item = &'cleanup RegisteredCleanup>,
    ) {
        let mut cleanups = cleanups.peekable();
        if cleanups.peek().is_none() {
            return;
        }
        let primary_failure = self.alloc_local();
        let cleanup_failure = self.alloc_local();
        code.op(0x01);
        code.astore(primary_failure);
        for cleanup in cleanups {
            let cleanup_start = code.mark();
            self.emit_registered_cleanup(code, cleanup);
            let cleanup_end = code.mark();
            let next_cleanup = code.new_label();
            let failure_handler = code.new_label();
            code.branch_wide_to(next_cleanup);
            code.bind(failure_handler);
            code.astore(cleanup_failure);
            code.aload(primary_failure);
            let has_primary_failure = code.branch(0xc7);
            code.aload(cleanup_failure);
            code.astore(primary_failure);
            code.branch_wide_to(next_cleanup);
            code.bind(has_primary_failure);
            self.emit_attach_cleanup_failure(code, primary_failure, cleanup_failure);
            code.bind(next_cleanup);
            code.add_exception_handler_to_label(cleanup_start, cleanup_end, failure_handler);
        }
        code.aload(primary_failure);
        let completed = code.branch(0xc6);
        code.aload(primary_failure);
        code.op(0xbf);
        code.bind(completed);
    }

    fn emit_attach_cleanup_failure(
        &self,
        code: &mut MethodCode,
        primary_failure: u16,
        cleanup_failure: u16,
    ) {
        code.aload(primary_failure);
        code.aload(cleanup_failure);
        code.invokestatic(
            &self.program.options.runtime_class,
            "attachCleanupFailure",
            "(Ljava/lang/Throwable;Ljava/lang/Throwable;)V",
        );
    }

    pub(super) fn push_handler_unwind(&mut self, code: &mut MethodCode) -> Option<usize> {
        let parent = self.active_unwind;
        self.push_unwind_node(code, UnwindAction::PopHandler);
        parent
    }

    pub(super) fn restore_unwind(&mut self, unwind: Option<usize>) {
        self.active_unwind = unwind;
    }

    pub(super) fn emit_try_unwind(&mut self, code: &mut MethodCode, result: u16) {
        let unwind_result = self.unwind_result;
        code.aload(result);
        code.astore(unwind_result);
        self.exception_unwind_used = true;
        self.unwind_return_label(code);
        let target = match self.active_unwind {
            Some(node) => self.unwind_nodes[node].label,
            None => self.unwind_return_label.expect("unwind return label"),
        };
        code.branch_wide_from_any_stack_to(target);
    }

    fn push_unwind_node(&mut self, code: &mut MethodCode, action: UnwindAction) {
        let node = UnwindNode {
            label: code.new_label(),
            exception_entry_label: code.new_label(),
            exception_action_label: code.new_label(),
            parent: self.active_unwind,
            action,
        };
        self.active_unwind = Some(self.unwind_nodes.len());
        self.unwind_nodes.push(node);
    }

    fn protect_exception_range(
        &mut self,
        code: &mut MethodCode,
        start_pc: usize,
        end_pc: usize,
        unwind: Option<usize>,
    ) {
        let Some(unwind) = unwind else {
            return;
        };
        if start_pc == end_pc {
            return;
        }
        self.exception_unwind_used = true;
        code.add_exception_handler_to_label(
            start_pc,
            end_pc,
            self.unwind_nodes[unwind].exception_entry_label,
        );
    }

    fn unwind_return_label(&mut self, code: &mut MethodCode) -> usize {
        match self.unwind_return_label {
            Some(label) => label,
            None => {
                let label = code.new_label();
                self.unwind_return_label = Some(label);
                label
            }
        }
    }

    fn emit_unwind_blocks(&mut self, code: &mut MethodCode) {
        if self.unwind_return_label.is_none() && !self.exception_unwind_used {
            return;
        }
        let scratch_base = self.max_local;
        let throw_label = self.exception_unwind_used.then(|| code.new_label());
        if let Some(return_label) = self.unwind_return_label {
            let mut index = 0;
            while index < self.unwind_nodes.len() {
                let node = self.unwind_nodes[index].clone();
                code.bind(node.label);
                self.active_unwind = node.parent;
                self.next_local = scratch_base;
                match node.action {
                    UnwindAction::Cleanup(cleanup) => {
                        let cleanup_start = code.mark();
                        self.emit_registered_cleanup(code, &cleanup);
                        let cleanup_end = code.mark();
                        let cleanup_failure_handler = code.new_label();
                        code.add_exception_handler_to_label(
                            cleanup_start,
                            cleanup_end,
                            cleanup_failure_handler,
                        );
                        let target = match node.parent {
                            Some(parent) => self.unwind_nodes[parent].label,
                            None => return_label,
                        };
                        code.branch_wide_to(target);
                        code.bind(cleanup_failure_handler);
                        code.astore(self.unwind_result);
                        let failure_target = match node.parent {
                            Some(parent) => self.unwind_nodes[parent].exception_action_label,
                            None => throw_label.expect("exception unwind label"),
                        };
                        code.branch_wide_to(failure_target);
                        index += 1;
                        continue;
                    }
                    UnwindAction::PopHandler => self.emit_pop_handler(code),
                }
                let target = match node.parent {
                    Some(parent) => self.unwind_nodes[parent].label,
                    None => return_label,
                };
                code.branch_wide_to(target);
                index += 1;
            }
            code.bind(return_label);
            let result = self.unwind_result;
            self.emit_ensure_checks_for_result(code, result);
            code.aload(result);
            code.op(0xb0);
        }
        if self.exception_unwind_used {
            let throw_label = throw_label.expect("exception unwind label");
            let mut index = 0;
            while index < self.unwind_nodes.len() {
                let node = self.unwind_nodes[index].clone();
                code.bind(node.exception_entry_label);
                code.astore(self.unwind_result);
                code.bind(node.exception_action_label);
                self.active_unwind = node.parent;
                self.next_local = scratch_base;
                match node.action {
                    UnwindAction::Cleanup(cleanup) => {
                        let cleanup_start = code.mark();
                        self.emit_registered_cleanup(code, &cleanup);
                        let cleanup_end = code.mark();
                        let cleanup_failure_handler = code.new_label();
                        code.add_exception_handler_to_label(
                            cleanup_start,
                            cleanup_end,
                            cleanup_failure_handler,
                        );
                        let target = match node.parent {
                            Some(parent) => self.unwind_nodes[parent].exception_action_label,
                            None => throw_label,
                        };
                        code.branch_wide_to(target);
                        code.bind(cleanup_failure_handler);
                        let cleanup_failure = self.alloc_local();
                        code.astore(cleanup_failure);
                        self.emit_attach_cleanup_failure(code, self.unwind_result, cleanup_failure);
                        code.branch_wide_to(target);
                        index += 1;
                        continue;
                    }
                    UnwindAction::PopHandler => self.emit_pop_handler(code),
                }
                let target = match node.parent {
                    Some(parent) => self.unwind_nodes[parent].exception_action_label,
                    None => throw_label,
                };
                code.branch_wide_to(target);
                index += 1;
            }
            code.bind(throw_label);
            code.aload(self.unwind_result);
            code.op(0xbf);
        }
        self.active_unwind = None;
    }

    fn emit_cleanup_region(&mut self, code: &mut MethodCode, body: &[IrStmt]) {
        let locals_mark = self.locals.mark();
        let saved_next = self.next_local;
        if !self.emit_region(code, body, false) {
            self.emit_unit(code);
        }
        self.locals.rollback(locals_mark);
        self.next_local = saved_next;
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
        let locals_mark = self.locals.mark();
        let saved_next = self.next_local;
        let mut has_value_arm = false;
        for arm in arms {
            self.locals.rollback(locals_mark);
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
        self.locals.rollback(locals_mark);
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
            IrExprKind::CleanupRegion { region } => self.emit_cleanup_region(code, region),
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
