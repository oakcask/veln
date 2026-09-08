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
            IrCallTarget::CodecDecode { function, codec } => {
                self.emit_codec_decode_call(code, function, codec, args)
            }
            IrCallTarget::SchemaDecode(name) => {
                self.emit_schema_decode_call(code, name, args);
            }
            IrCallTarget::SchemaDecodeStep(name) => {
                self.emit_schema_decode_step_call(code, name, args);
            }
            IrCallTarget::SchemaNeutralDecode(name) => {
                self.emit_schema_neutral_decode_call(code, name, args);
            }
            IrCallTarget::SchemaNeutralEncode(name) => {
                self.emit_schema_neutral_encode_call(code, name, args);
            }
            IrCallTarget::SchemaEncode(name) => {
                self.emit_schema_encode_call(code, name, args);
            }
            IrCallTarget::SchemaEncodeStep(name) => {
                self.emit_schema_encode_step_call(code, name, args);
            }
            IrCallTarget::SchemaValidate(name) => {
                self.emit_schema_validate_call(code, name, args);
            }
            IrCallTarget::StdioBuiltin(name) => self.emit_stdio_call(code, expr, name, args),
            IrCallTarget::ConcurrencyBuiltin(name) => {
                self.emit_runtime_call(code, concurrency_method(name), args);
            }
            IrCallTarget::StandardLibraryBuiltin(name) => {
                self.emit_runtime_call(code, standard_library_method(name), args);
            }
            IrCallTarget::PreludeBuiltin(name) => {
                self.emit_runtime_call(code, prelude_method(name), args);
            }
            IrCallTarget::Value(name) => self.emit_value_call(code, name, args),
        }
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

    fn emit_codec_decode_call(
        &mut self,
        code: &mut MethodCode,
        function: &str,
        codec: &str,
        args: &[IrExpr],
    ) {
        let [view, base_offset] = args else {
            panic!("codec decode boundary call should receive ByteView and ByteOffset arguments");
        };
        self.emit_expr(code, view);
        self.emit_expr(code, base_offset);
        self.emit_program_function_call(code, function, args);
        code.ldc_string(codec);
        code.invokestatic(
            &self.program.options.runtime_class,
            "validateCodecDecodeStep",
            "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
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
        code.ldc_string(effect);
        self.emit_object_array(code, providers.len(), |_, code, index| {
            code.ldc_string(&providers[index].operation);
        });
        self.emit_object_array(code, providers.len(), |this, code, index| {
            this.emit_function_value(code, &providers[index].function);
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
        let try_start = code.mark();
        self.active_handler_frames += 1;
        self.emit_expr(code, body);
        self.active_handler_frames -= 1;
        let try_end = code.mark();
        let result_slot = self.alloc_local();
        code.astore(result_slot);
        self.emit_pop_handler(code);
        let done = code.new_label();
        code.branch_to(0xa7, done);
        let handler_pc = code.mark();
        let throwable_slot = self.alloc_local();
        code.astore(throwable_slot);
        self.emit_pop_handler(code);
        code.aload(throwable_slot);
        code.op(0xbf);
        code.exceptions.push(ExceptionHandler {
            start_pc: try_start,
            end_pc: try_end,
            handler_pc,
            catch_type: "java/lang/Throwable".to_string(),
        });
        code.bind(done);
        code.aload(result_slot);
    }

    pub(super) fn emit_pop_handler(&mut self, code: &mut MethodCode) {
        code.invokestatic(
            &self.program.options.runtime_class,
            "popHandler",
            "()Ljava/lang/Object;",
        );
        code.op(0x57);
    }

    pub(super) fn emit_active_handler_cleanup(&mut self, code: &mut MethodCode) {
        for _ in 0..self.active_handler_frames {
            self.emit_pop_handler(code);
        }
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
