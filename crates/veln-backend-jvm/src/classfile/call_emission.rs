use super::*;

impl<'a, 'program> FunctionBytecodeEmitter<'a, 'program> {
    pub(super) fn emit_call(
        &mut self,
        code: &mut MethodCode,
        expr: &IrExpr,
        target: &IrCallTarget,
        args: &[IrExpr],
    ) {
        match target {
            IrCallTarget::Function(name) => self.emit_program_function_call(code, name, args),
            IrCallTarget::StdioBuiltin(name) => self.emit_stdio_call(code, expr, name, args),
            IrCallTarget::CallbackBoundary { target, callsite } => {
                self.emit_callback_boundary_call(code, target, args, callsite);
            }
            IrCallTarget::Value(name) => self.emit_value_call(code, name, args),
            IrCallTarget::CallsiteValue { name, callsite } => {
                self.emit_callsite_value_call(code, name, args, callsite)
            }
            _ => self.emit_static_runtime_target(code, target, args),
        }
    }

    fn emit_static_runtime_target(
        &mut self,
        code: &mut MethodCode,
        target: &IrCallTarget,
        args: &[IrExpr],
    ) {
        match target {
            IrCallTarget::SchemaDecode(name) => self.emit_schema_decode_call(code, name, args),
            IrCallTarget::SchemaDecodeStep(name) => {
                self.emit_schema_decode_step_call(code, name, args);
            }
            IrCallTarget::SchemaNeutralDecode(name) => {
                self.emit_schema_neutral_decode_call(code, name, args);
            }
            IrCallTarget::SchemaNeutralEncode(name) => {
                self.emit_schema_neutral_encode_call(code, name, args);
            }
            IrCallTarget::SchemaEncode(name) => self.emit_schema_encode_call(code, name, args),
            IrCallTarget::SchemaEncodeStep(name) => {
                self.emit_schema_encode_step_call(code, name, args);
            }
            IrCallTarget::SchemaValidate(name) => self.emit_schema_validate_call(code, name, args),
            IrCallTarget::ConcurrencyBuiltin(name) => {
                self.emit_runtime_call(code, concurrency_method(name), args);
            }
            IrCallTarget::StandardLibraryBuiltin(name) => {
                self.emit_runtime_call(code, standard_library_method(name), args);
            }
            IrCallTarget::PreludeBuiltin(name) => {
                self.emit_runtime_call(code, prelude_method(name), args);
            }
            _ => unreachable!(),
        }
    }

    fn emit_callback_boundary_call(
        &mut self,
        code: &mut MethodCode,
        target: &IrCallbackTarget,
        args: &[IrExpr],
        callsite: &IrExpr,
    ) {
        if let IrCallbackTarget::Function(name) = target {
            self.emit_callback_args(code, args, callsite);
            code.invokestatic(
                &self.program.options.program_class,
                &self.program.function_name(name),
                &object_method_descriptor(args.len()),
            );
            return;
        }
        let method = match target {
            IrCallbackTarget::Function(_) => unreachable!(),
            IrCallbackTarget::ConcurrencyBuiltin(name) => concurrency_method(name),
            IrCallbackTarget::StandardLibraryBuiltin(name) => standard_library_method(name),
            IrCallbackTarget::PreludeBuiltin(name) => prelude_method(name),
        };
        self.emit_runtime_callback_call(code, method, args, callsite);
    }

    fn emit_program_function_call(&mut self, code: &mut MethodCode, name: &str, args: &[IrExpr]) {
        for arg in args {
            self.emit_expr(code, arg);
        }
        code.invokestatic(
            &self.program.options.program_class,
            &self.program.function_name(name),
            &object_method_descriptor(args.len()),
        );
    }

    fn emit_stdio_call(
        &mut self,
        code: &mut MethodCode,
        expr: &IrExpr,
        name: &str,
        args: &[IrExpr],
    ) {
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

    fn emit_value_call(&mut self, code: &mut MethodCode, name: &str, args: &[IrExpr]) {
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

    fn emit_callsite_value_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
        callsite: &IrExpr,
    ) {
        code.aload(self.local_slot(name));
        self.emit_object_array(code, args.len(), |this, code, index| {
            this.emit_expr(code, &args[index]);
        });
        self.emit_expr(code, callsite);
        code.invokestatic(
            &self.program.options.runtime_class,
            "callAt",
            "(Ljava/lang/Object;[Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    pub(super) fn emit_perform(
        &mut self,
        code: &mut MethodCode,
        effect: &str,
        operation: &str,
        args: &[IrExpr],
    ) {
        code.ldc_string(effect);
        code.ldc_string(operation);
        self.emit_object_array(code, args.len(), |this, code, index| {
            this.emit_expr(code, &args[index]);
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "perform",
            "(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/Object;",
        );
    }

    pub(super) fn emit_handle(
        &mut self,
        code: &mut MethodCode,
        effect: &str,
        providers: &[IrHandlerProvider],
        context_args: &[IrExpr],
        body: &IrExpr,
    ) {
        self.emit_handler_registration(code, effect, providers, context_args);
        self.emit_handled_body(code, body);
    }

    fn emit_handler_registration(
        &mut self,
        code: &mut MethodCode,
        effect: &str,
        providers: &[IrHandlerProvider],
        context_args: &[IrExpr],
    ) {
        code.ldc_string(effect);
        self.emit_object_array(code, providers.len(), |_, code, index| {
            code.ldc_string(&providers[index].operation);
        });
        self.emit_object_array(code, providers.len(), |this, code, index| {
            let name = &providers[index].function;
            this.emit_function_value(code, name, this.program.function_callsite(name));
        });
        self.emit_object_array(code, context_args.len(), |this, code, index| {
            this.emit_expr(code, &context_args[index]);
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "pushHandler",
            "(Ljava/lang/String;[Ljava/lang/Object;[Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
        );
        code.op(0x57);
    }

    fn emit_handled_body(&mut self, code: &mut MethodCode, body: &IrExpr) {
        let try_start = code.mark();
        let parent_unwind = self.push_handler_unwind(code);
        self.emit_expr(code, body);
        self.restore_unwind(parent_unwind);
        let try_end = code.mark();
        let result_slot = self.alloc_local();
        code.astore(result_slot);
        self.emit_pop_handler(code);
        let done = code.new_label();
        code.branch_to(0xa7, done);
        self.emit_handler_unwind(code, try_start, try_end);
        code.bind(done);
        code.aload(result_slot);
    }

    fn emit_handler_unwind(&mut self, code: &mut MethodCode, try_start: usize, try_end: usize) {
        let handler_pc = code.mark();
        let throwable_slot = self.alloc_local();
        code.astore(throwable_slot);
        let cleanup_start = code.mark();
        self.emit_pop_handler(code);
        let cleanup_end = code.mark();
        let rethrow = code.new_label();
        code.branch_wide_to(rethrow);
        let cleanup_failure_handler = code.mark();
        let cleanup_failure_slot = self.alloc_local();
        code.astore(cleanup_failure_slot);
        self.emit_attach_cleanup_failure(code, throwable_slot, cleanup_failure_slot);
        code.bind(rethrow);
        code.aload(throwable_slot);
        code.op(0xbf);
        code.exceptions.push(ExceptionHandler {
            start_pc: try_start,
            end_pc: try_end,
            handler_pc,
            catch_type: "java/lang/Throwable".to_string(),
        });
        code.exceptions.push(ExceptionHandler {
            start_pc: cleanup_start,
            end_pc: cleanup_end,
            handler_pc: cleanup_failure_handler,
            catch_type: "java/lang/Throwable".to_string(),
        });
    }

    pub(super) fn emit_pop_handler(&mut self, code: &mut MethodCode) {
        code.invokestatic(
            &self.program.options.runtime_class,
            "popHandler",
            "()Ljava/lang/Object;",
        );
        code.op(0x57);
    }

    pub(super) fn emit_runtime_call(
        &mut self,
        code: &mut MethodCode,
        method: &str,
        args: &[IrExpr],
    ) {
        for arg in args {
            self.emit_expr(code, arg);
        }
        code.invokestatic(
            &self.program.options.runtime_class,
            method,
            &object_method_descriptor(args.len()),
        );
    }

    fn emit_runtime_callback_call(
        &mut self,
        code: &mut MethodCode,
        method: &str,
        args: &[IrExpr],
        callsite: &IrExpr,
    ) {
        self.emit_callback_args(code, args, callsite);
        code.invokestatic(
            &self.program.options.runtime_class,
            method,
            &object_method_descriptor(args.len()),
        );
    }

    fn emit_callback_args(&mut self, code: &mut MethodCode, args: &[IrExpr], callsite: &IrExpr) {
        for arg in args {
            self.emit_callback_arg(code, arg, callsite);
        }
    }

    pub(super) fn emit_callback_arg(
        &mut self,
        code: &mut MethodCode,
        arg: &IrExpr,
        callsite: &IrExpr,
    ) {
        self.emit_expr(code, arg);
        if matches!(arg.ty, CoreType::Function { .. }) {
            self.emit_expr(code, callsite);
            code.invokestatic(
                &self.program.options.runtime_class,
                "bindCallsite",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            );
        }
    }

    pub(super) fn emit_unary_runtime(
        &mut self,
        code: &mut MethodCode,
        method: &str,
        value: &IrExpr,
    ) {
        self.emit_unary_runtime_with_descriptor(
            code,
            method,
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            value,
        );
    }

    pub(super) fn emit_unary_runtime_with_descriptor(
        &mut self,
        code: &mut MethodCode,
        method: &str,
        descriptor: &str,
        value: &IrExpr,
    ) {
        self.emit_expr(code, value);
        code.invokestatic(&self.program.options.runtime_class, method, descriptor);
    }
}
