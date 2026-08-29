use super::*;

impl<'a, 'program> FunctionBytecodeEmitter<'a, 'program> {
    pub(super) fn emit_binary(
        &mut self,
        code: &mut MethodCode,
        op: BinaryOp,
        left: &IrExpr,
        right: &IrExpr,
    ) {
        self.emit_expr(code, left);
        self.emit_expr(code, right);
        code.invokestatic(
            &self.program.options.runtime_class,
            binary_method(op),
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    pub(super) fn emit_record(&mut self, code: &mut MethodCode, fields: &[IrRecordField]) {
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

    pub(super) fn emit_dict(&mut self, code: &mut MethodCode, entries: &[IrDictEntry]) {
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

    pub(super) fn emit_object_array<F>(&mut self, code: &mut MethodCode, len: usize, mut emit: F)
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

    pub(super) fn emit_try(&mut self, code: &mut MethodCode, value: &IrExpr) {
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
        self.emit_active_handler_cleanup(code);
        code.op(0xb0);
        code.bind(ok);
        code.aload(temp);
        code.invokestatic(
            &self.program.options.runtime_class,
            "unwrapOk",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    pub(super) fn emit_match(
        &mut self,
        code: &mut MethodCode,
        scrutinee: &IrExpr,
        arms: &[IrMatchArm],
    ) {
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

    pub(super) fn emit_pattern_condition(
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
                code.ldc_long(
                    parse_integer_literal(text)
                        .map(|literal| literal.value)
                        .unwrap_or(0),
                );
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

    pub(super) fn emit_record_pattern_condition(
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

    pub(super) fn emit_constructor_pattern_condition(
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

    pub(super) fn emit_generic_constructor_condition(
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

    pub(super) fn emit_constructor_inner_condition(
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

    pub(super) fn emit_constructor_pair_condition(
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

    pub(super) fn emit_pattern_bindings(
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
}
