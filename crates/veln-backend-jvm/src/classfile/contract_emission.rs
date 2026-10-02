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
        self.emit_contract_value(code, &contract.predicate, &contract.callsite_callees);
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
        callsite_callees: &[String],
    ) {
        match parse_contract_value(text) {
            ContractValue::Not(value) => {
                self.emit_contract_unary(code, value, "not", callsite_callees)
            }
            ContractValue::Binary { left, right, op } => {
                self.emit_contract_binary(code, left, right, op, callsite_callees)
            }
            ContractValue::BitwiseNot(value) => {
                self.emit_contract_unary(code, value, "bitwiseNot", callsite_callees)
            }
            ContractValue::Call { callee, args } => {
                self.emit_contract_call(code, callee, &args, callsite_callees)
            }
            ContractValue::Field { base, field } => {
                self.emit_contract_field(code, base, field, callsite_callees)
            }
            ContractValue::Scalar(value) => self.emit_contract_scalar(code, value),
        }
    }

    fn emit_contract_unary(
        &mut self,
        code: &mut MethodCode,
        value: &str,
        method: &str,
        callsite_callees: &[String],
    ) {
        self.emit_contract_value(code, value, callsite_callees);
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
        callsite_callees: &[String],
    ) {
        self.emit_contract_value(code, left, callsite_callees);
        self.emit_contract_value(code, right, callsite_callees);
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
        callsite_callees: &[String],
    ) {
        for arg in args {
            self.emit_contract_value(code, arg, callsite_callees);
        }
        let requires_callsite = callsite_callees.iter().any(|candidate| candidate == callee);
        if requires_callsite {
            code.aload(self.local_slot("callsite"));
        }
        let function = callee.rsplit("::").next().unwrap_or(callee);
        code.invokestatic(
            &self.program.options.program_class,
            &self.program.function_name(function),
            &object_method_descriptor(args.len() + usize::from(requires_callsite)),
        );
    }

    fn emit_contract_field(
        &mut self,
        code: &mut MethodCode,
        base: &str,
        field: &str,
        callsite_callees: &[String],
    ) {
        self.emit_contract_value(code, base, callsite_callees);
        code.ldc_string(field);
        code.invokestatic(
            &self.program.options.runtime_class,
            "recordField",
            "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
        );
    }

    fn emit_contract_scalar(&mut self, code: &mut MethodCode, value: ContractScalar<'_>) {
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
