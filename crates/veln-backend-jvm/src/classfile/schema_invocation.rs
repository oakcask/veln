use super::*;

impl<'a, 'program> FunctionBytecodeEmitter<'a, 'program> {
    pub(super) fn emit_schema_decode_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        let schema = declared_schema(self.program.program, name, "missing schema decoder spec");
        let [view] = args else {
            panic!("schema decoder call should receive one ByteView argument");
        };
        self.emit_expr(code, view);
        self.emit_schema_decoding_metadata(code, schema);
        code.invokestatic(
            &self.program.options.runtime_class,
            "byteDecodeDeclaredBinarySchema",
            &object_method_descriptor(24),
        );
    }

    pub(super) fn emit_schema_decode_step_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        let schema = declared_schema(self.program.program, name, "missing schema decoder spec");
        let [view, base_offset] = args else {
            panic!("schema decode-step call should receive ByteView and ByteOffset arguments");
        };
        self.emit_expr(code, view);
        self.emit_expr(code, base_offset);
        self.emit_schema_decoding_metadata(code, schema);
        code.invokestatic(
            &self.program.options.runtime_class,
            "byteDecodeStepDeclaredBinarySchema",
            &object_method_descriptor(25),
        );
    }

    fn emit_schema_decoding_metadata(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_schema_binary_metadata(code, schema);
        self.emit_schema_validation(code, schema);
        self.emit_schema_dispatch_metadata(code, schema);
    }

    fn emit_schema_binary_metadata(&mut self, code: &mut MethodCode, schema: &IrSchemaDecodeSpec) {
        code.ldc_string(&schema.schema_name);
        self.emit_schema_field_names(code, schema);
        self.emit_schema_field_widths(code, schema);
        self.emit_schema_field_max_values(code, schema);
        self.emit_schema_field_little_endian_values(code, schema);
        self.emit_schema_repeat_count_fields(code, schema);
        self.emit_schema_repeat_widths(code, schema);
        self.emit_schema_repeat_max_values(code, schema);
        self.emit_schema_repeat_little_endian_values(code, schema);
        self.emit_schema_repeat_reserved_values(code, schema);
        self.emit_schema_repeat_byte_view_length_fields(code, schema);
        self.emit_schema_repeat_schema_specs(code, schema);
        self.emit_schema_reserved_bit_widths(code, schema);
        self.emit_schema_reserved_values(code, schema);
        self.emit_schema_field_predicates(code, schema);
        self.emit_schema_byte_view_multiples(code, schema);
    }

    fn emit_schema_dispatch_metadata(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_schema_dispatch_tag_fields(code, schema);
        self.emit_schema_dispatch_length_fields(code, schema);
        self.emit_schema_dispatch_case_tags(code, schema);
        self.emit_schema_dispatch_case_widths(code, schema);
        self.emit_schema_dispatch_case_little_endian_values(code, schema);
        self.emit_schema_dispatch_case_schema_specs(code, schema);
    }

    pub(super) fn emit_schema_encode_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        let schema = declared_schema(self.program.program, name, "missing schema encoder spec");
        let [value] = args else {
            panic!("schema encoder call should receive one record argument");
        };
        self.emit_expr(code, value);
        self.emit_schema_encoding_metadata(code, schema);
        code.invokestatic(
            &self.program.options.runtime_class,
            "byteEncodeDeclaredBinarySchema",
            &object_method_descriptor(23),
        );
    }

    pub(super) fn emit_schema_neutral_decode_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        self.emit_schema_neutral_composition_call(code, name, args);
    }

    pub(super) fn emit_schema_neutral_encode_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        self.emit_schema_neutral_composition_call(code, name, args);
    }

    pub(super) fn emit_schema_neutral_composition_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        let schema = declared_schema(self.program.program, name, "missing schema validation spec");
        let [value] = args else {
            panic!("format-neutral schema call should receive one record argument");
        };
        self.emit_expr(code, value);
        self.emit_schema_validation_metadata(code, schema);
        code.invokestatic(
            &self.program.options.runtime_class,
            "validateDeclaredSchemaCompositionValue",
            &object_method_descriptor(2),
        );
    }

    pub(super) fn emit_schema_encode_step_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        let schema = declared_schema(self.program.program, name, "missing schema encoder spec");
        let (value, budget, runtime_method, argument_count) = match args {
            [value] => (value, None, "byteEncodeStepDeclaredBinarySchema", 23),
            [value, budget] => (
                value,
                Some(budget),
                "byteEncodeStepDeclaredBinarySchemaBudgeted",
                24,
            ),
            _ => panic!(
                "schema encode-step call should receive one record argument or value plus budget"
            ),
        };
        self.emit_expr(code, value);
        if let Some(budget) = budget {
            self.emit_expr(code, budget);
        }
        self.emit_schema_encoding_metadata(code, schema);
        code.invokestatic(
            &self.program.options.runtime_class,
            runtime_method,
            &object_method_descriptor(argument_count),
        );
    }

    pub(super) fn emit_schema_encoding_metadata(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_schema_binary_metadata(code, schema);
        self.emit_schema_dispatch_metadata(code, schema);
    }
}

fn declared_schema<'a>(
    program: &'a TypedProgram,
    name: &str,
    missing_spec: &str,
) -> &'a IrSchemaDecodeSpec {
    program
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == name)
        .unwrap_or_else(|| panic!("{missing_spec} `{name}`"))
}
