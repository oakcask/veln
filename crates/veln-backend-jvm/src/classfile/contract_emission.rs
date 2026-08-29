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

    pub(super) fn emit_contract_value(&mut self, code: &mut MethodCode, text: &str) {
        let text = strip_contract_outer_parens(text.trim());
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
            ("|", BinaryOp::BitwiseOr),
            ("^", BinaryOp::BitwiseXor),
            ("&", BinaryOp::BitwiseAnd),
            ("==", BinaryOp::Equal),
            ("!=", BinaryOp::NotEqual),
            (">=", BinaryOp::GreaterEqual),
            ("<=", BinaryOp::LessEqual),
            (">", BinaryOp::Greater),
            ("<", BinaryOp::Less),
            (">>>", BinaryOp::ShiftRightLogical),
            (">>", BinaryOp::ShiftRight),
            ("<<", BinaryOp::ShiftLeft),
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
        if let Some(rest) = text.strip_prefix('~') {
            self.emit_contract_value(code, rest);
            code.invokestatic(
                &self.program.options.runtime_class,
                "bitwiseNot",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
            );
            return;
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
        } else if let Some(value) = contract_integer_value(text) {
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

    pub(super) fn has_ensure_contracts(&self) -> bool {
        self.function.contracts.iter().any(|contract| {
            matches!(
                contract.kind,
                ContractKind::Ensure | ContractKind::Invariant
            ) && contract.obligation_status == ContractObligationStatus::RuntimeRequired
        })
    }

    pub(super) fn emit_ensure_checks_for_result(&mut self, code: &mut MethodCode, result: u16) {
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
