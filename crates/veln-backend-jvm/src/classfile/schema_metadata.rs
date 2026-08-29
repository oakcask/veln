use super::*;

impl<'a, 'program> FunctionBytecodeEmitter<'a, 'program> {
    pub(super) fn emit_schema_validate_call(
        &mut self,
        code: &mut MethodCode,
        name: &str,
        args: &[IrExpr],
    ) {
        let schema = self
            .program
            .program
            .schema_decoders
            .iter()
            .find(|schema| schema.schema_name == name)
            .unwrap_or_else(|| panic!("missing schema validation spec `{name}`"));
        let [value] = args else {
            panic!("schema validation call should receive one record argument");
        };
        self.emit_expr(code, value);
        code.ldc_string(&schema.schema_name);
        self.emit_schema_field_names(code, schema);
        self.emit_schema_field_predicates(code, schema);
        self.emit_schema_validation(code, schema);
        code.invokestatic(
            &self.program.options.runtime_class,
            "validateDeclaredSchemaValue",
            &object_method_descriptor(5),
        );
    }

    pub(super) fn emit_schema_field_names(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_string(&schema.fields[index].name);
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_field_widths(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_long(schema.fields[index].width as i64);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_field_max_values(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_long(schema.fields[index].max_value);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_field_little_endian_values(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            if schema.fields[index].little_endian {
                code.getstatic("java/lang/Boolean", "TRUE", "Ljava/lang/Boolean;");
            } else {
                code.getstatic("java/lang/Boolean", "FALSE", "Ljava/lang/Boolean;");
            }
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_repeat_count_fields(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_string(
                schema.fields[index]
                    .repeat
                    .as_ref()
                    .map(|repeat| repeat.count_field.as_str())
                    .unwrap_or(""),
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_repeat_widths(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            let width = schema.fields[index]
                .repeat
                .as_ref()
                .map(|repeat| repeat.width as i64)
                .unwrap_or(0);
            code.ldc_long(width);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_repeat_max_values(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            let max_value = schema.fields[index]
                .repeat
                .as_ref()
                .map(|repeat| repeat.max_value)
                .unwrap_or(0);
            code.ldc_long(max_value);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_repeat_little_endian_values(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            if schema.fields[index]
                .repeat
                .as_ref()
                .is_some_and(|repeat| repeat.little_endian)
            {
                code.getstatic("java/lang/Boolean", "TRUE", "Ljava/lang/Boolean;");
            } else {
                code.getstatic("java/lang/Boolean", "FALSE", "Ljava/lang/Boolean;");
            }
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_repeat_reserved_values(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            let value = schema.fields[index]
                .repeat
                .as_ref()
                .and_then(|repeat| repeat.reserved_bits.as_ref())
                .map(|reserved| reserved.expected_value)
                .unwrap_or(-1);
            code.ldc_long(value);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_repeat_byte_view_length_fields(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_string(
                schema.fields[index]
                    .repeat
                    .as_ref()
                    .and_then(|repeat| repeat.byte_view_length_field.as_deref())
                    .unwrap_or(""),
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_repeat_schema_specs(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |this, code, index| {
            if let Some(payload_schema) = schema.fields[index]
                .repeat
                .as_ref()
                .and_then(|repeat| repeat.payload_schema.as_ref())
                .or(schema.fields[index].payload_schema.as_ref())
            {
                this.emit_schema_metadata(code, payload_schema);
            } else {
                code.ldc_string("");
            }
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_reserved_bit_widths(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            let bit_width = schema.fields[index]
                .reserved_bits
                .as_ref()
                .map(|reserved| reserved.bit_width as i64)
                .unwrap_or(0);
            code.ldc_long(bit_width);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_reserved_values(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            let expected_value = schema.fields[index]
                .reserved_bits
                .as_ref()
                .map(|reserved| reserved.expected_value)
                .unwrap_or(0);
            code.ldc_long(expected_value);
            code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_field_predicates(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_string(schema.fields[index].predicate.as_deref().unwrap_or(""));
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_byte_view_multiples(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_string(
                schema.fields[index]
                    .length_multiple
                    .as_deref()
                    .unwrap_or(""),
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_validation(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        code.ldc_string(schema.validation.as_deref().unwrap_or(""));
    }

    pub(super) fn emit_schema_dispatch_tag_fields(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            code.ldc_string(
                schema.fields[index]
                    .dispatch
                    .as_ref()
                    .map(|dispatch| dispatch.tag_field.as_str())
                    .unwrap_or(""),
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_dispatch_case_tags(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |this, code, index| {
            let cases = schema.fields[index]
                .dispatch
                .as_ref()
                .map(|dispatch| dispatch.cases.as_slice())
                .unwrap_or(&[]);
            this.emit_object_array(code, cases.len(), |_, code, case_index| {
                code.ldc_long(cases[case_index].tag);
                code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
            });
            code.invokestatic(
                &this.program.options.runtime_class,
                "list",
                "([Ljava/lang/Object;)Ljava/util/List;",
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_dispatch_length_fields(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |_, code, index| {
            let length_field = schema.fields[index]
                .dispatch
                .as_ref()
                .and_then(|dispatch| {
                    dispatch.length_field.as_ref().map(|length_field| {
                        if dispatch.preserves_unknown {
                            length_field.clone()
                        } else {
                            format!("closed:{length_field}")
                        }
                    })
                })
                .or_else(|| schema.fields[index].length_field.clone())
                .unwrap_or_default();
            code.ldc_string(&length_field);
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_dispatch_case_widths(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |this, code, index| {
            let cases = schema.fields[index]
                .dispatch
                .as_ref()
                .map(|dispatch| dispatch.cases.as_slice())
                .unwrap_or(&[]);
            this.emit_object_array(code, cases.len(), |_, code, case_index| {
                code.ldc_long(cases[case_index].width as i64);
                code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
            });
            code.invokestatic(
                &this.program.options.runtime_class,
                "list",
                "([Ljava/lang/Object;)Ljava/util/List;",
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_dispatch_case_little_endian_values(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |this, code, index| {
            let cases = schema.fields[index]
                .dispatch
                .as_ref()
                .map(|dispatch| dispatch.cases.as_slice())
                .unwrap_or(&[]);
            this.emit_object_array(code, cases.len(), |_, code, case_index| {
                if cases[case_index].little_endian {
                    code.getstatic("java/lang/Boolean", "TRUE", "Ljava/lang/Boolean;");
                } else {
                    code.getstatic("java/lang/Boolean", "FALSE", "Ljava/lang/Boolean;");
                }
            });
            code.invokestatic(
                &this.program.options.runtime_class,
                "list",
                "([Ljava/lang/Object;)Ljava/util/List;",
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_dispatch_case_schema_specs(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, schema.fields.len(), |this, code, index| {
            let cases = schema.fields[index]
                .dispatch
                .as_ref()
                .map(|dispatch| dispatch.cases.as_slice())
                .unwrap_or(&[]);
            this.emit_object_array(code, cases.len(), |this, code, case_index| {
                if let Some(payload_schema) = cases[case_index].payload_schema.as_ref() {
                    this.emit_schema_metadata(code, payload_schema);
                } else if let Some(reserved_bits) = cases[case_index].reserved_bits.as_ref() {
                    code.ldc_string(&format!("reserved:{}", reserved_bits.expected_value));
                } else if let Some(payload_schema_name) =
                    cases[case_index].payload_schema_name.as_ref()
                {
                    code.ldc_string(payload_schema_name);
                } else {
                    code.ldc_string("");
                }
            });
            code.invokestatic(
                &this.program.options.runtime_class,
                "list",
                "([Ljava/lang/Object;)Ljava/util/List;",
            );
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_metadata(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, 23, |this, code, index| match index {
            0 => code.ldc_string(&schema.schema_name),
            1 => this.emit_schema_field_names(code, schema),
            2 => this.emit_schema_field_widths(code, schema),
            3 => this.emit_schema_field_max_values(code, schema),
            4 => this.emit_schema_field_little_endian_values(code, schema),
            5 => this.emit_schema_repeat_count_fields(code, schema),
            6 => this.emit_schema_repeat_widths(code, schema),
            7 => this.emit_schema_repeat_max_values(code, schema),
            8 => this.emit_schema_repeat_little_endian_values(code, schema),
            9 => this.emit_schema_repeat_reserved_values(code, schema),
            10 => this.emit_schema_repeat_byte_view_length_fields(code, schema),
            11 => this.emit_schema_repeat_schema_specs(code, schema),
            12 => this.emit_schema_reserved_bit_widths(code, schema),
            13 => this.emit_schema_reserved_values(code, schema),
            14 => this.emit_schema_field_predicates(code, schema),
            15 => this.emit_schema_byte_view_multiples(code, schema),
            16 => this.emit_schema_validation(code, schema),
            17 => this.emit_schema_dispatch_tag_fields(code, schema),
            18 => this.emit_schema_dispatch_length_fields(code, schema),
            19 => this.emit_schema_dispatch_case_tags(code, schema),
            20 => this.emit_schema_dispatch_case_widths(code, schema),
            21 => this.emit_schema_dispatch_case_little_endian_values(code, schema),
            22 => this.emit_schema_dispatch_case_schema_specs(code, schema),
            _ => unreachable!(),
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }

    pub(super) fn emit_schema_validation_metadata(
        &mut self,
        code: &mut MethodCode,
        schema: &IrSchemaDecodeSpec,
    ) {
        self.emit_object_array(code, 5, |this, code, index| match index {
            0 => code.ldc_string(&schema.schema_name),
            1 => this.emit_schema_field_names(code, schema),
            2 => this.emit_schema_field_predicates(code, schema),
            3 => this.emit_schema_validation(code, schema),
            4 => {
                this.emit_object_array(code, schema.fields.len(), |this, code, field_index| {
                    if let Some(payload_schema) = schema.fields[field_index].payload_schema.as_ref()
                    {
                        this.emit_schema_validation_metadata(code, payload_schema);
                    } else {
                        code.ldc_string("");
                    }
                });
                code.invokestatic(
                    &this.program.options.runtime_class,
                    "list",
                    "([Ljava/lang/Object;)Ljava/util/List;",
                );
            }
            _ => unreachable!(),
        });
        code.invokestatic(
            &self.program.options.runtime_class,
            "list",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
    }
}
