use super::*;

impl<'a, 'program> FunctionBytecodeEmitter<'a, 'program> {
    pub(super) fn emit_contract_check(
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
        let mut callsite_calls = HashMap::with_capacity(contract.callsite_calls.len());
        for call in &contract.callsite_calls {
            if let Some(existing) = callsite_calls.insert(call.callee.as_str(), call) {
                debug_assert_eq!(existing, call, "one callee must have one contract-call ABI");
            }
        }
        #[cfg(test)]
        {
            code.contract_call_metadata_work += contract.callsite_calls.len();
        }
        self.emit_contract_value(code, &contract.predicate, &callsite_calls, 0);
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

    pub(super) fn emit_contract_value(
        &mut self,
        code: &mut MethodCode,
        text: &str,
        callsite_calls: &HashMap<&str, &IrContractCall>,
        operand_depth: usize,
    ) {
        match parse_contract_value(text) {
            ContractValue::Not(value) => {
                self.emit_contract_unary(code, value, "not", callsite_calls, operand_depth)
            }
            ContractValue::Binary { left, right, op } => {
                self.emit_contract_binary(code, left, right, op, callsite_calls, operand_depth)
            }
            ContractValue::BitwiseNot(value) => {
                self.emit_contract_unary(code, value, "bitwiseNot", callsite_calls, operand_depth)
            }
            ContractValue::Call { callee, args } => {
                self.emit_contract_call(code, callee, &args, callsite_calls, operand_depth)
            }
            ContractValue::Field { base, field } => {
                self.emit_contract_field(code, base, field, callsite_calls, operand_depth)
            }
            ContractValue::Scalar(value) => self.emit_contract_scalar(code, value, operand_depth),
        }
    }

    fn emit_contract_unary(
        &mut self,
        code: &mut MethodCode,
        value: &str,
        method: &str,
        callsite_calls: &HashMap<&str, &IrContractCall>,
        operand_depth: usize,
    ) {
        self.emit_contract_value(code, value, callsite_calls, operand_depth);
        code.invokestatic(
            &self.program.options.runtime_class,
            method,
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    fn emit_contract_binary(
        &mut self,
        code: &mut MethodCode,
        left: &str,
        right: &str,
        op: BinaryOp,
        callsite_calls: &HashMap<&str, &IrContractCall>,
        operand_depth: usize,
    ) {
        self.emit_contract_value(code, left, callsite_calls, operand_depth);
        self.emit_contract_value(code, right, callsite_calls, operand_depth + 1);
        code.invokestatic(
            &self.program.options.runtime_class,
            binary_method(op),
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    fn emit_contract_call(
        &mut self,
        code: &mut MethodCode,
        callee: &str,
        args: &[&str],
        callsite_calls: &HashMap<&str, &IrContractCall>,
        operand_depth: usize,
    ) {
        #[cfg(test)]
        {
            code.contract_call_metadata_work += 1;
        }
        let callsite_call = callsite_calls.get(callee).copied();
        let fixed_arg_count = callsite_call.map_or(args.len(), |call| call.fixed_arg_count);
        let abi_arg_count = fixed_arg_count
            + usize::from(callsite_call.is_some_and(|call| call.variadic))
            + usize::from(callsite_call.is_some());
        record_contract_stack(code, operand_depth + abi_arg_count.max(1));
        for (index, arg) in args[..fixed_arg_count].iter().enumerate() {
            self.emit_contract_value(code, arg, callsite_calls, operand_depth + index);
        }
        if callsite_call.is_some_and(|call| call.variadic) {
            self.emit_contract_variadic_tail(
                code,
                &args[fixed_arg_count..],
                callsite_calls,
                operand_depth + fixed_arg_count,
            );
        }
        if callsite_call.is_some() {
            code.aload(self.local_slot("callsite"));
        }
        let source_function = callee.rsplit("::").next().unwrap_or(callee);
        let function = callsite_call.map_or(source_function, |call| call.target.as_str());
        code.invokestatic(
            &self.program.options.program_class,
            &self.program.function_name(function),
            &object_method_descriptor(abi_arg_count),
        );
    }

    fn emit_contract_variadic_tail(
        &mut self,
        code: &mut MethodCode,
        args: &[&str],
        callsite_calls: &HashMap<&str, &IrContractCall>,
        operand_depth: usize,
    ) {
        let saved_next = self.next_local;
        let tail_slot = self.alloc_local();
        record_contract_stack(code, operand_depth + 1);
        self.emit_list_nil(code);
        code.astore(tail_slot);
        for arg in args {
            self.emit_contract_value(code, arg, callsite_calls, operand_depth);
            record_contract_stack(code, operand_depth + 2);
            code.aload(tail_slot);
            code.invokestatic(
                &self.program.options.runtime_class,
                "listCons",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            );
            code.astore(tail_slot);
        }
        code.aload(tail_slot);
        code.invokestatic(
            &self.program.options.runtime_class,
            "listReverse",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        );
        self.next_local = saved_next;
    }

    fn emit_contract_field(
        &mut self,
        code: &mut MethodCode,
        base: &str,
        field: &str,
        callsite_calls: &HashMap<&str, &IrContractCall>,
        operand_depth: usize,
    ) {
        self.emit_contract_value(code, base, callsite_calls, operand_depth);
        record_contract_stack(code, operand_depth + 2);
        code.ldc_string(field);
        code.invokestatic(
            &self.program.options.runtime_class,
            "recordField",
            "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
        );
    }

    fn emit_contract_scalar(
        &mut self,
        code: &mut MethodCode,
        value: ContractScalar<'_>,
        operand_depth: usize,
    ) {
        let value_slots = match &value {
            ContractScalar::Integer(_) | ContractScalar::Float(_) => 2,
            _ => 1,
        };
        record_contract_stack(code, operand_depth + value_slots);
        match value {
            ContractScalar::Bool(value) => code.getstatic(
                "java/lang/Boolean",
                if value { "TRUE" } else { "FALSE" },
                "Ljava/lang/Boolean;",
            ),
            ContractScalar::Unit => code.getstatic(
                &self.program.options.runtime_class,
                "UNIT",
                &format!("L{}$Unit;", self.program.options.runtime_class),
            ),
            ContractScalar::String(value) => {
                code.ldc_string(&veln_string_literal_value(value));
            }
            ContractScalar::Integer(value) => {
                code.ldc_long(value);
                code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
            }
            ContractScalar::Float(value) => {
                code.ldc_double(value);
                code.invokestatic("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;");
            }
            ContractScalar::Symbol(name) => self.emit_contract_symbol(code, name),
        }
    }

    fn emit_contract_symbol(&self, code: &mut MethodCode, name: &str) {
        if self.locals.contains_key(name) {
            code.aload(self.local_slot(name));
        } else {
            code.getstatic("java/lang/Boolean", "FALSE", "Ljava/lang/Boolean;");
        }
    }

    pub(super) fn has_ensure_contracts(&self) -> bool {
        self.function.contracts.iter().any(|contract| {
            matches!(
                contract.kind,
                ContractKind::Ensure | ContractKind::Invariant
            ) && contract.obligation_status == ContractObligationStatus::RuntimeRequired
        })
    }

    pub(super) fn emit_ensure_checks_for_result(&mut self, code: &mut MethodCode, result: u16) {
        let locals_mark = self.locals.mark();
        if let Some(binding) = &self.function.return_binding {
            self.locals.insert(binding.clone(), result);
        }
        for contract in self.function.contracts.iter().filter(|contract| {
            matches!(
                contract.kind,
                ContractKind::Ensure | ContractKind::Invariant
            )
        }) {
            self.emit_contract_check(code, contract, ContractCheckPosition::Return);
        }
        self.locals.rollback(locals_mark);
    }

    pub(super) fn bind_local(&mut self, name: &str) -> u16 {
        let slot = self.alloc_local();
        self.locals.insert(name.to_string(), slot);
        slot
    }

    pub(super) fn alloc_local(&mut self) -> u16 {
        let slot = self.next_local;
        self.next_local += 1;
        self.max_local = self.max_local.max(self.next_local);
        slot
    }

    pub(super) fn local_slot(&self, name: &str) -> u16 {
        *self
            .locals
            .get(name)
            .unwrap_or_else(|| panic!("missing JVM local `{name}`"))
    }
}

fn record_contract_stack(code: &mut MethodCode, operand_depth: usize) {
    code.max_stack = code.max_stack.max(
        u16::try_from(operand_depth)
            .expect("contract expression JVM operand stack requirement exceeds classfile limit"),
    );
}
