use super::*;

mod expressions;

pub fn encode_surface_module(module: &SurfaceModule) -> Vec<u8> {
    let mut writer = Writer { bytes: Vec::new() };
    writer.bytes.extend_from_slice(MAGIC);
    writer.surface_module(module);
    writer.bytes
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn string(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn option<T>(&mut self, value: &Option<T>, mut write: impl FnMut(&mut Self, &T)) {
        match value {
            Some(value) => {
                self.bool(true);
                write(self, value);
            }
            None => self.bool(false),
        }
    }

    fn vec<T>(&mut self, values: &[T], mut write: impl FnMut(&mut Self, &T)) {
        self.u32(values.len() as u32);
        for value in values {
            write(self, value);
        }
    }

    fn node_id(&mut self, value: NodeId) {
        self.u32(value.as_u32());
    }

    fn line_col(&mut self, value: &LineCol) {
        self.usize(value.line);
        self.usize(value.column);
        self.usize(value.offset);
    }

    fn span(&mut self, value: &SourceSpan) {
        self.string(value.file.as_str());
        self.line_col(&value.start);
        self.line_col(&value.end);
    }

    fn type_path_segments(&mut self, value: &TypePathSegments) {
        self.vec(&value.segments, |writer, value| writer.string(value));
        self.vec(&value.segment_spans, Self::span);
    }

    fn surface_module(&mut self, module: &SurfaceModule) {
        self.option(&module.module, Self::module_header);
        self.vec(&module.uses, Self::use_decl);
        self.vec(&module.aliases, Self::public_alias);
        self.vec(&module.effects, Self::effect_decl);
        self.vec(&module.handlers, Self::handler_decl);
        self.vec(&module.types, Self::type_decl);
        self.vec(&module.schemas, Self::schema_decl);
        self.vec(&module.codecs, Self::codec_decl);
        self.vec(&module.functions, Self::function);
        self.vec(&module.invalid_names, Self::invalid_name);
    }

    fn invalid_name(&mut self, value: &InvalidName) {
        self.string(&value.name);
        self.u8(match value.class {
            NameClass::Type => 0,
            NameClass::Constructor => 1,
            NameClass::Module => 2,
            NameClass::Function => 3,
            NameClass::ValueBinding => 4,
        });
        self.u8(match value.occurrence {
            NameOccurrence::Declaration => 0,
            NameOccurrence::Binding => 1,
            NameOccurrence::PatternHead => 2,
            NameOccurrence::AliasTarget => 3,
            NameOccurrence::PathSegment => 4,
        });
        self.span(&value.span);
        self.option(&value.enclosing_function_span, Self::span);
        self.option(&value.segment_index, |writer, value| writer.usize(*value));
    }

    fn module_header(&mut self, value: &ModuleHeader) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.span(&value.span);
    }

    fn use_decl(&mut self, value: &UseDecl) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.string(&value.name);
        self.string(&value.alias);
        self.vec(&value.name_spans, Self::span);
        self.option(&value.package, |writer, value| writer.string(value));
        self.option(&value.package_span, Self::span);
        self.span(&value.span);
        self.use_origin(value.origin);
    }

    fn use_origin(&mut self, value: UseOrigin) {
        self.u8(match value {
            UseOrigin::Source => 0,
            UseOrigin::ImplicitStandardPrelude => 1,
        });
    }

    fn public_alias(&mut self, value: &PublicAlias) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.public_alias_kind(value.kind);
        self.option(&value.name, |writer, value| writer.string(value));
        self.vec(&value.target, |writer, value| writer.string(value));
        self.vec(&value.target_spans, Self::span);
        self.span(&value.span);
    }

    fn public_alias_kind(&mut self, value: PublicAliasKind) {
        self.u8(match value {
            PublicAliasKind::Function => 0,
            PublicAliasKind::Type => 1,
            PublicAliasKind::Schema => 2,
        });
    }

    fn visibility(&mut self, value: Visibility) {
        self.u8(match value {
            Visibility::Public => 0,
            Visibility::Private => 1,
        });
    }

    fn effect_decl(&mut self, value: &EffectDecl) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.visibility(value.visibility);
        self.option(&value.name, |writer, value| writer.string(value));
        self.vec(&value.operations, Self::effect_operation);
        self.span(&value.span);
    }

    fn effect_operation(&mut self, value: &EffectOperationDecl) {
        self.node_id(value.node_id);
        self.option(&value.name, |writer, value| writer.string(value));
        self.span(&value.name_span);
        self.vec(&value.params, Self::param);
        self.option(&value.return_type, |writer, value| writer.string(value));
        self.vec(&value.return_type_paths, Self::type_path_segments);
        self.span(&value.span);
    }

    fn handler_decl(&mut self, value: &HandlerDecl) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.visibility(value.visibility);
        self.option(&value.name, |writer, value| writer.string(value));
        self.vec(&value.params, Self::param);
        self.vec(&value.effect, |writer, value| writer.string(value));
        self.span(&value.effect_span);
        self.option(&value.effects, |writer, values| {
            writer.vec(values, |writer, value| writer.string(value));
        });
        self.option(&value.effect_spans, |writer, values| {
            writer.vec(values, Self::span);
        });
        self.vec(&value.operation_clauses, Self::handler_operation);
        self.span(&value.span);
    }

    fn handler_operation(&mut self, value: &HandlerOperationClauseDecl) {
        self.node_id(value.node_id);
        self.option(&value.operation, |writer, value| writer.string(value));
        self.span(&value.operation_span);
        self.vec(&value.params, Self::param);
        self.expr(&value.body);
        self.span(&value.span);
    }

    fn type_decl(&mut self, value: &TypeDecl) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.visibility(value.visibility);
        self.option(&value.name, |writer, value| writer.string(value));
        self.vec(&value.params, |writer, value| writer.string(value));
        self.vec(&value.variants, Self::type_variant);
        self.span(&value.span);
    }

    fn type_variant(&mut self, value: &TypeVariantDecl) {
        self.node_id(value.node_id);
        self.visibility(value.visibility);
        self.option(&value.name, |writer, value| writer.string(value));
        self.vec(&value.fields, Self::type_variant_field);
        self.span(&value.span);
    }

    fn type_variant_field(&mut self, value: &TypeVariantField) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.string(&value.ty);
        self.vec(&value.ty_paths, Self::type_path_segments);
        self.span(&value.span);
    }

    fn schema_decl(&mut self, value: &SchemaDecl) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.visibility(value.visibility);
        self.option(&value.name, |writer, value| writer.string(value));
        self.option(&value.format, Self::schema_format);
        self.vec(&value.fields, Self::schema_field);
        self.vec(&value.validations, Self::schema_validation);
        self.span(&value.span);
    }

    fn schema_format(&mut self, value: &SchemaFormatClause) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.span(&value.span);
    }

    fn schema_field(&mut self, value: &SchemaField) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.string(&value.ty);
        self.vec(&value.ty_paths, Self::type_path_segments);
        self.option(&value.where_clause, Self::schema_field_where);
        self.span(&value.span);
    }

    fn schema_field_where(&mut self, value: &SchemaFieldWhereClause) {
        self.node_id(value.node_id);
        self.string(&value.predicate);
        self.span(&value.span);
    }

    fn schema_validation(&mut self, value: &SchemaValidationClause) {
        self.node_id(value.node_id);
        self.string(&value.predicate);
        self.span(&value.span);
    }

    fn codec_decl(&mut self, value: &CodecDecl) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.visibility(value.visibility);
        self.option(&value.name, |writer, value| writer.string(value));
        self.option(&value.schema, |writer, value| writer.string(value));
        self.vec(&value.directions, |writer, value| {
            writer.codec_direction(*value)
        });
        self.vec(&value.implementations, Self::codec_implementation);
        self.span(&value.span);
    }

    fn codec_direction(&mut self, value: CodecDirection) {
        self.u8(match value {
            CodecDirection::Decode => 0,
            CodecDirection::Encode => 1,
        });
    }

    fn codec_implementation(&mut self, value: &CodecImplementationClause) {
        self.node_id(value.node_id);
        self.codec_direction(value.direction);
        self.codec_implementation_kind(&value.kind);
        self.span(&value.span);
    }

    fn codec_implementation_kind(&mut self, value: &CodecImplementationKind) {
        match value {
            CodecImplementationKind::Derive => self.u8(0),
            CodecImplementationKind::With { function } => {
                self.u8(1);
                self.option(function, |writer, value| writer.string(value));
            }
        }
    }

    fn function(&mut self, value: &Function) {
        self.node_id(value.node_id);
        self.option(&value.module_name, |writer, value| writer.string(value));
        self.function_kind(value.kind);
        self.visibility(value.visibility);
        self.option(&value.name, |writer, value| writer.string(value));
        self.option(&value.effect_binder, Self::effect_binder);
        self.vec(&value.params, Self::param);
        self.option(&value.return_binding, Self::result_binding);
        self.option(&value.return_type, |writer, value| writer.string(value));
        self.option(&value.return_type_span, Self::span);
        self.vec(&value.return_type_paths, Self::type_path_segments);
        self.option(&value.effects, |writer, values| {
            writer.vec(values, |writer, value| writer.string(value));
        });
        self.option(&value.effect_spans, |writer, values| {
            writer.vec(values, Self::span);
        });
        self.vec(&value.contracts, Self::contract);
        self.vec(&value.body, Self::body_line);
        self.span(&value.span);
    }

    fn function_kind(&mut self, value: FunctionKind) {
        self.u8(match value {
            FunctionKind::Function => 0,
            FunctionKind::Test => 1,
        });
    }

    fn effect_binder(&mut self, value: &EffectBinder) {
        self.string(&value.name);
        self.span(&value.span);
    }

    fn param(&mut self, value: &Param) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.option(&value.ty, |writer, value| writer.string(value));
        self.option(&value.ty_span, Self::span);
        self.vec(&value.ty_paths, Self::type_path_segments);
        self.bool(value.is_variadic);
        self.span(&value.span);
    }

    fn result_binding(&mut self, value: &ResultBinding) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.span(&value.span);
    }

    fn contract(&mut self, value: &Contract) {
        self.node_id(value.node_id);
        self.contract_kind(value.kind);
        self.string(&value.text);
        self.span(&value.span);
    }

    fn contract_kind(&mut self, value: ContractKind) {
        self.u8(match value {
            ContractKind::Require => 0,
            ContractKind::Ensure => 1,
            ContractKind::Invariant => 2,
        });
    }

    fn body_line(&mut self, value: &BodyLine) {
        self.node_id(value.node_id);
        match &value.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                annotation_paths,
                expr,
            } => {
                self.u8(0);
                self.pattern(pattern);
                self.option(annotation, |writer, value| writer.string(value));
                self.vec(annotation_paths, Self::type_path_segments);
                self.expr(expr);
            }
            BodyLineKind::Expr { expr } => {
                self.u8(1);
                self.expr(expr);
            }
        }
        self.span(&value.span);
    }
}
