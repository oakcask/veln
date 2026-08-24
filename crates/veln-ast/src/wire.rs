use veln_source::{LineCol, SourcePath, SourceSpan};

use crate::{
    BinaryOp, BodyLine, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationClause,
    CodecImplementationKind, Contract, ContractKind, DictEntry, EffectBinder, EffectDecl,
    EffectOperationDecl, Expr, ExprKind, Function, FunctionKind, HandlerDecl,
    HandlerOperationClauseDecl, IfBranch, MatchArm, ModuleHeader, NodeId, Param, Pattern,
    PatternField, PatternKind, PrefixOp, PublicAlias, PublicAliasKind, RecordField, ResultBinding,
    SatisfyClause, SchemaDecl, SchemaField, SchemaFieldWhereClause, SchemaFormatClause,
    SchemaValidationClause, SurfaceModule, TypeDecl, TypeVariantDecl, TypeVariantField, UseDecl,
    UseOrigin, Visibility,
};

const MAGIC: &[u8; 8] = b"VLNAST1\n";

pub fn encode_surface_module(module: &SurfaceModule) -> Vec<u8> {
    let mut writer = Writer { bytes: Vec::new() };
    writer.bytes.extend_from_slice(MAGIC);
    writer.surface_module(module);
    writer.bytes
}

pub fn decode_surface_module(bytes: &[u8]) -> Result<SurfaceModule, String> {
    let mut reader = Reader { bytes, position: 0 };
    reader.magic()?;
    let module = reader.surface_module()?;
    reader.eof()?;
    Ok(module)
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
        self.span(&value.name_span);
        self.option(&value.ty, |writer, value| writer.string(value));
        self.option(&value.ty_span, Self::span);
        self.bool(value.is_variadic);
        self.span(&value.span);
    }

    fn result_binding(&mut self, value: &ResultBinding) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.span(&value.name_span);
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
                expr,
            } => {
                self.u8(0);
                self.pattern(pattern);
                self.option(annotation, |writer, value| writer.string(value));
                self.expr(expr);
            }
            BodyLineKind::Expr { expr } => {
                self.u8(1);
                self.expr(expr);
            }
        }
        self.span(&value.span);
    }

    fn expr(&mut self, value: &Expr) {
        self.node_id(value.node_id);
        self.expr_kind(&value.kind);
        self.span(&value.span);
    }

    fn expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => self.scalar_expr_kind(value),
            ExprKind::TypeApply { .. } | ExprKind::Call { .. } => {
                self.invocation_expr_kind(value);
            }
            ExprKind::Perform { .. } | ExprKind::Handle { .. } => self.effect_expr_kind(value),
            ExprKind::SchemaDecode { .. }
            | ExprKind::SchemaEncode { .. }
            | ExprKind::FieldAccess { .. }
            | ExprKind::Try(_) => self.schema_and_access_expr_kind(value),
            ExprKind::Record(_)
            | ExprKind::Dict(_)
            | ExprKind::List(_)
            | ExprKind::Match { .. }
            | ExprKind::If { .. } => self.aggregate_expr_kind(value),
            ExprKind::Prefix { .. } | ExprKind::Binary { .. } => self.operator_expr_kind(value),
        }
    }

    fn scalar_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Missing => self.u8(0),
            ExprKind::Hole { name, satisfy } => {
                self.u8(1);
                self.option(name, |writer, value| writer.string(value));
                self.option(satisfy, Self::satisfy);
            }
            ExprKind::NamePath(path) => {
                self.u8(2);
                self.vec(path, |writer, value| writer.string(value));
            }
            ExprKind::StringLiteral(value) => {
                self.u8(3);
                self.string(value);
            }
            ExprKind::IntLiteral(value) => {
                self.u8(4);
                self.string(value);
            }
            ExprKind::FloatLiteral(value) => {
                self.u8(5);
                self.string(value);
            }
            ExprKind::BoolLiteral(value) => {
                self.u8(6);
                self.bool(*value);
            }
            ExprKind::Unit => self.u8(7),
            _ => unreachable!("non-scalar expression passed to scalar wire encoder"),
        }
    }

    fn invocation_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::TypeApply { callee, type_args } => {
                self.u8(8);
                self.expr(callee);
                self.vec(type_args, |writer, value| writer.string(value));
            }
            ExprKind::Call { callee, args } => {
                self.u8(9);
                self.expr(callee);
                self.vec(args, Self::expr);
            }
            _ => unreachable!("non-invocation expression passed to invocation wire encoder"),
        }
    }

    fn effect_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Perform {
                effect,
                effect_span,
                operation,
                operation_span,
                args,
            } => {
                self.u8(10);
                self.vec(effect, |writer, value| writer.string(value));
                self.span(effect_span);
                self.string(operation);
                self.span(operation_span);
                self.vec(args, Self::expr);
            }
            ExprKind::Handle {
                body,
                handler,
                handler_span,
                args,
            } => {
                self.u8(11);
                self.expr(body);
                self.vec(handler, |writer, value| writer.string(value));
                self.span(handler_span);
                self.vec(args, Self::expr);
            }
            _ => unreachable!("non-effect expression passed to effect wire encoder"),
        }
    }

    fn schema_and_access_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::SchemaDecode {
                schema,
                input,
                base,
            } => {
                self.u8(12);
                self.vec(schema, |writer, value| writer.string(value));
                self.expr(input);
                self.expr(base);
            }
            ExprKind::SchemaEncode { schema, value } => {
                self.u8(13);
                self.vec(schema, |writer, value| writer.string(value));
                self.expr(value);
            }
            ExprKind::FieldAccess {
                base,
                field,
                field_span,
            } => {
                self.u8(14);
                self.expr(base);
                self.string(field);
                self.span(field_span);
            }
            ExprKind::Try(expr) => {
                self.u8(15);
                self.expr(expr);
            }
            _ => unreachable!("non-schema or access expression passed to schema wire encoder"),
        }
    }

    fn aggregate_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Record(fields) => {
                self.u8(16);
                self.vec(fields, Self::record_field);
            }
            ExprKind::Dict(entries) => {
                self.u8(17);
                self.vec(entries, Self::dict_entry);
            }
            ExprKind::List(items) => {
                self.u8(18);
                self.vec(items, Self::expr);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.u8(19);
                self.expr(scrutinee);
                self.vec(arms, Self::match_arm);
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.u8(20);
                self.expr(condition);
                self.expr(then_branch);
                self.vec(else_if_branches, Self::if_branch);
                self.expr(else_branch);
            }
            _ => unreachable!("non-aggregate expression passed to aggregate wire encoder"),
        }
    }

    fn operator_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Prefix { op, expr } => {
                self.u8(21);
                self.prefix_op(*op);
                self.expr(expr);
            }
            ExprKind::Binary { op, left, right } => {
                self.u8(22);
                self.binary_op(*op);
                self.expr(left);
                self.expr(right);
            }
            _ => unreachable!("non-operator expression passed to operator wire encoder"),
        }
    }

    fn satisfy(&mut self, value: &SatisfyClause) {
        self.option(&value.candidate, |writer, value| writer.string(value));
        self.option(&value.candidate_span, Self::span);
        self.string(&value.predicate);
        self.span(&value.span);
    }

    fn record_field(&mut self, value: &RecordField) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.expr(&value.expr);
        self.span(&value.span);
    }

    fn dict_entry(&mut self, value: &DictEntry) {
        self.node_id(value.node_id);
        self.expr(&value.key);
        self.expr(&value.value);
        self.span(&value.span);
    }

    fn match_arm(&mut self, value: &MatchArm) {
        self.node_id(value.node_id);
        self.pattern(&value.pattern);
        self.expr(&value.expr);
        self.span(&value.span);
    }

    fn if_branch(&mut self, value: &IfBranch) {
        self.node_id(value.node_id);
        self.expr(&value.condition);
        self.expr(&value.expr);
        self.span(&value.span);
    }

    fn prefix_op(&mut self, value: PrefixOp) {
        self.u8(match value {
            PrefixOp::Not => 0,
            PrefixOp::Negate => 1,
            PrefixOp::BitwiseNot => 2,
        });
    }

    fn binary_op(&mut self, value: BinaryOp) {
        self.u8(match value {
            BinaryOp::PipeGreater => 0,
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::BitwiseOr => 3,
            BinaryOp::BitwiseXor => 4,
            BinaryOp::BitwiseAnd => 5,
            BinaryOp::Equal => 6,
            BinaryOp::NotEqual => 7,
            BinaryOp::Less => 8,
            BinaryOp::LessEqual => 9,
            BinaryOp::Greater => 10,
            BinaryOp::GreaterEqual => 11,
            BinaryOp::ShiftLeft => 12,
            BinaryOp::ShiftRight => 13,
            BinaryOp::ShiftRightLogical => 14,
            BinaryOp::Add => 15,
            BinaryOp::Subtract => 16,
            BinaryOp::Multiply => 17,
            BinaryOp::Divide => 18,
        });
    }

    fn pattern(&mut self, value: &Pattern) {
        self.node_id(value.node_id);
        self.pattern_kind(&value.kind);
        self.span(&value.span);
    }

    fn pattern_kind(&mut self, value: &PatternKind) {
        match value {
            PatternKind::Wildcard => self.u8(0),
            PatternKind::Binding(value) => {
                self.u8(1);
                self.string(value);
            }
            PatternKind::StringLiteral(value) => {
                self.u8(2);
                self.string(value);
            }
            PatternKind::IntLiteral(value) => {
                self.u8(3);
                self.string(value);
            }
            PatternKind::FloatLiteral(value) => {
                self.u8(4);
                self.string(value);
            }
            PatternKind::BoolLiteral(value) => {
                self.u8(5);
                self.bool(*value);
            }
            PatternKind::Unit => self.u8(6),
            PatternKind::Record(fields) => {
                self.u8(7);
                self.vec(fields, Self::pattern_field);
            }
            PatternKind::Constructor { name, args } => {
                self.u8(8);
                self.vec(name, |writer, value| writer.string(value));
                self.vec(args, Self::pattern);
            }
        }
    }

    fn pattern_field(&mut self, value: &PatternField) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.pattern(&value.pattern);
        self.span(&value.span);
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn magic(&mut self) -> Result<(), String> {
        let magic = self.take(MAGIC.len())?;
        if magic == MAGIC {
            Ok(())
        } else {
            Err("invalid surface module wire header".to_string())
        }
    }

    fn eof(&self) -> Result<(), String> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err("trailing surface module wire data".to_string())
        }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| "surface module wire offset overflow".to_string())?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| "truncated surface module wire data".to_string())?;
        self.position = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, String> {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(bytes))
    }

    fn usize(&mut self) -> Result<usize, String> {
        usize::try_from(self.u64()?).map_err(|_| "surface module usize overflow".to_string())
    }

    fn bool(&mut self) -> Result<bool, String> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(format!("invalid surface module bool tag {value}")),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        let len = self.u32()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| "invalid UTF-8 string".to_string())
    }

    fn option<T>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        if self.bool()? {
            Ok(Some(read(self)?))
        } else {
            Ok(None)
        }
    }

    fn vec<T>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, String>,
    ) -> Result<Vec<T>, String> {
        let len = self.u32()? as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(read(self)?);
        }
        Ok(values)
    }

    fn node_id(&mut self) -> Result<NodeId, String> {
        Ok(NodeId::new(self.u32()?))
    }

    fn line_col(&mut self) -> Result<LineCol, String> {
        Ok(LineCol {
            line: self.usize()?,
            column: self.usize()?,
            offset: self.usize()?,
        })
    }

    fn span(&mut self) -> Result<SourceSpan, String> {
        Ok(SourceSpan {
            file: SourcePath::new(self.string()?),
            start: self.line_col()?,
            end: self.line_col()?,
        })
    }

    fn surface_module(&mut self) -> Result<SurfaceModule, String> {
        Ok(SurfaceModule {
            module: self.option(Self::module_header)?,
            uses: self.vec(Self::use_decl)?,
            aliases: self.vec(Self::public_alias)?,
            effects: self.vec(Self::effect_decl)?,
            handlers: self.vec(Self::handler_decl)?,
            types: self.vec(Self::type_decl)?,
            schemas: self.vec(Self::schema_decl)?,
            codecs: self.vec(Self::codec_decl)?,
            functions: self.vec(Self::function)?,
        })
    }

    fn module_header(&mut self) -> Result<ModuleHeader, String> {
        Ok(ModuleHeader {
            node_id: self.node_id()?,
            name: self.string()?,
            span: self.span()?,
        })
    }

    fn use_decl(&mut self) -> Result<UseDecl, String> {
        Ok(UseDecl {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            name: self.string()?,
            alias: self.string()?,
            package: self.option(Self::string)?,
            package_span: self.option(Self::span)?,
            span: self.span()?,
            origin: self.use_origin()?,
        })
    }

    fn use_origin(&mut self) -> Result<UseOrigin, String> {
        match self.u8()? {
            0 => Ok(UseOrigin::Source),
            1 => Ok(UseOrigin::ImplicitStandardPrelude),
            value => Err(format!("invalid use origin tag {value}")),
        }
    }

    fn public_alias(&mut self) -> Result<PublicAlias, String> {
        Ok(PublicAlias {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            kind: self.public_alias_kind()?,
            name: self.option(Self::string)?,
            target: self.vec(Self::string)?,
            span: self.span()?,
        })
    }

    fn public_alias_kind(&mut self) -> Result<PublicAliasKind, String> {
        match self.u8()? {
            0 => Ok(PublicAliasKind::Function),
            1 => Ok(PublicAliasKind::Type),
            2 => Ok(PublicAliasKind::Schema),
            value => Err(format!("invalid public alias kind tag {value}")),
        }
    }

    fn visibility(&mut self) -> Result<Visibility, String> {
        match self.u8()? {
            0 => Ok(Visibility::Public),
            1 => Ok(Visibility::Private),
            value => Err(format!("invalid visibility tag {value}")),
        }
    }

    fn effect_decl(&mut self) -> Result<EffectDecl, String> {
        Ok(EffectDecl {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            operations: self.vec(Self::effect_operation)?,
            span: self.span()?,
        })
    }

    fn effect_operation(&mut self) -> Result<EffectOperationDecl, String> {
        Ok(EffectOperationDecl {
            node_id: self.node_id()?,
            name: self.option(Self::string)?,
            name_span: self.span()?,
            params: self.vec(Self::param)?,
            return_type: self.option(Self::string)?,
            span: self.span()?,
        })
    }

    fn handler_decl(&mut self) -> Result<HandlerDecl, String> {
        Ok(HandlerDecl {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            params: self.vec(Self::param)?,
            effect: self.vec(Self::string)?,
            effect_span: self.span()?,
            effects: self.option(|reader| reader.vec(Self::string))?,
            effect_spans: self.option(|reader| reader.vec(Self::span))?,
            operation_clauses: self.vec(Self::handler_operation)?,
            span: self.span()?,
        })
    }

    fn handler_operation(&mut self) -> Result<HandlerOperationClauseDecl, String> {
        Ok(HandlerOperationClauseDecl {
            node_id: self.node_id()?,
            operation: self.option(Self::string)?,
            operation_span: self.span()?,
            params: self.vec(Self::param)?,
            body: self.expr()?,
            span: self.span()?,
        })
    }

    fn type_decl(&mut self) -> Result<TypeDecl, String> {
        Ok(TypeDecl {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            params: self.vec(Self::string)?,
            variants: self.vec(Self::type_variant)?,
            span: self.span()?,
        })
    }

    fn type_variant(&mut self) -> Result<TypeVariantDecl, String> {
        Ok(TypeVariantDecl {
            node_id: self.node_id()?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            fields: self.vec(Self::type_variant_field)?,
            span: self.span()?,
        })
    }

    fn type_variant_field(&mut self) -> Result<TypeVariantField, String> {
        Ok(TypeVariantField {
            node_id: self.node_id()?,
            name: self.string()?,
            ty: self.string()?,
            span: self.span()?,
        })
    }

    fn schema_decl(&mut self) -> Result<SchemaDecl, String> {
        Ok(SchemaDecl {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            format: self.option(Self::schema_format)?,
            fields: self.vec(Self::schema_field)?,
            validations: self.vec(Self::schema_validation)?,
            span: self.span()?,
        })
    }

    fn schema_format(&mut self) -> Result<SchemaFormatClause, String> {
        Ok(SchemaFormatClause {
            node_id: self.node_id()?,
            name: self.string()?,
            span: self.span()?,
        })
    }

    fn schema_field(&mut self) -> Result<SchemaField, String> {
        Ok(SchemaField {
            node_id: self.node_id()?,
            name: self.string()?,
            ty: self.string()?,
            where_clause: self.option(Self::schema_field_where)?,
            span: self.span()?,
        })
    }

    fn schema_field_where(&mut self) -> Result<SchemaFieldWhereClause, String> {
        Ok(SchemaFieldWhereClause {
            node_id: self.node_id()?,
            predicate: self.string()?,
            span: self.span()?,
        })
    }

    fn schema_validation(&mut self) -> Result<SchemaValidationClause, String> {
        Ok(SchemaValidationClause {
            node_id: self.node_id()?,
            predicate: self.string()?,
            span: self.span()?,
        })
    }

    fn codec_decl(&mut self) -> Result<CodecDecl, String> {
        Ok(CodecDecl {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            schema: self.option(Self::string)?,
            directions: self.vec(Self::codec_direction)?,
            implementations: self.vec(Self::codec_implementation)?,
            span: self.span()?,
        })
    }

    fn codec_direction(&mut self) -> Result<CodecDirection, String> {
        match self.u8()? {
            0 => Ok(CodecDirection::Decode),
            1 => Ok(CodecDirection::Encode),
            value => Err(format!("invalid codec direction tag {value}")),
        }
    }

    fn codec_implementation(&mut self) -> Result<CodecImplementationClause, String> {
        Ok(CodecImplementationClause {
            node_id: self.node_id()?,
            direction: self.codec_direction()?,
            kind: self.codec_implementation_kind()?,
            span: self.span()?,
        })
    }

    fn codec_implementation_kind(&mut self) -> Result<CodecImplementationKind, String> {
        match self.u8()? {
            0 => Ok(CodecImplementationKind::Derive),
            1 => Ok(CodecImplementationKind::With {
                function: self.option(Self::string)?,
            }),
            value => Err(format!("invalid codec implementation tag {value}")),
        }
    }

    fn function(&mut self) -> Result<Function, String> {
        Ok(Function {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            kind: self.function_kind()?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            effect_binder: self.option(Self::effect_binder)?,
            params: self.vec(Self::param)?,
            return_binding: self.option(Self::result_binding)?,
            return_type: self.option(Self::string)?,
            return_type_span: self.option(Self::span)?,
            effects: self.option(|reader| reader.vec(Self::string))?,
            effect_spans: self.option(|reader| reader.vec(Self::span))?,
            contracts: self.vec(Self::contract)?,
            body: self.vec(Self::body_line)?,
            span: self.span()?,
        })
    }

    fn function_kind(&mut self) -> Result<FunctionKind, String> {
        match self.u8()? {
            0 => Ok(FunctionKind::Function),
            1 => Ok(FunctionKind::Test),
            value => Err(format!("invalid function kind tag {value}")),
        }
    }

    fn effect_binder(&mut self) -> Result<EffectBinder, String> {
        Ok(EffectBinder {
            name: self.string()?,
            span: self.span()?,
        })
    }

    fn param(&mut self) -> Result<Param, String> {
        Ok(Param {
            node_id: self.node_id()?,
            name: self.string()?,
            name_span: self.span()?,
            ty: self.option(Self::string)?,
            ty_span: self.option(Self::span)?,
            is_variadic: self.bool()?,
            span: self.span()?,
        })
    }

    fn result_binding(&mut self) -> Result<ResultBinding, String> {
        Ok(ResultBinding {
            node_id: self.node_id()?,
            name: self.string()?,
            name_span: self.span()?,
            span: self.span()?,
        })
    }

    fn contract(&mut self) -> Result<Contract, String> {
        Ok(Contract {
            node_id: self.node_id()?,
            kind: self.contract_kind()?,
            text: self.string()?,
            span: self.span()?,
        })
    }

    fn contract_kind(&mut self) -> Result<ContractKind, String> {
        match self.u8()? {
            0 => Ok(ContractKind::Require),
            1 => Ok(ContractKind::Ensure),
            2 => Ok(ContractKind::Invariant),
            value => Err(format!("invalid contract kind tag {value}")),
        }
    }

    fn body_line(&mut self) -> Result<BodyLine, String> {
        let node_id = self.node_id()?;
        let kind = match self.u8()? {
            0 => BodyLineKind::Let {
                pattern: self.pattern()?,
                annotation: self.option(Self::string)?,
                expr: self.expr()?,
            },
            1 => BodyLineKind::Expr { expr: self.expr()? },
            value => return Err(format!("invalid body line kind tag {value}")),
        };
        Ok(BodyLine {
            node_id,
            kind,
            span: self.span()?,
        })
    }

    fn expr(&mut self) -> Result<Expr, String> {
        Ok(Expr {
            node_id: self.node_id()?,
            kind: self.expr_kind()?,
            span: self.span()?,
        })
    }

    fn expr_kind(&mut self) -> Result<ExprKind, String> {
        let tag = self.u8()?;
        match tag {
            0..=7 => self.scalar_expr_kind(tag),
            8..=9 => self.invocation_expr_kind(tag),
            10..=11 => self.effect_expr_kind(tag),
            12..=15 => self.schema_and_access_expr_kind(tag),
            16..=20 => self.aggregate_expr_kind(tag),
            21..=22 => self.operator_expr_kind(tag),
            value => Err(format!("invalid expr kind tag {value}")),
        }
    }

    fn scalar_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            0 => Ok(ExprKind::Missing),
            1 => Ok(ExprKind::Hole {
                name: self.option(Self::string)?,
                satisfy: self.option(Self::satisfy)?,
            }),
            2 => Ok(ExprKind::NamePath(self.vec(Self::string)?)),
            3 => Ok(ExprKind::StringLiteral(self.string()?)),
            4 => Ok(ExprKind::IntLiteral(self.string()?)),
            5 => Ok(ExprKind::FloatLiteral(self.string()?)),
            6 => Ok(ExprKind::BoolLiteral(self.bool()?)),
            7 => Ok(ExprKind::Unit),
            _ => unreachable!("non-scalar tag passed to scalar wire decoder"),
        }
    }

    fn invocation_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            8 => Ok(ExprKind::TypeApply {
                callee: Box::new(self.expr()?),
                type_args: self.vec(Self::string)?,
            }),
            9 => Ok(ExprKind::Call {
                callee: Box::new(self.expr()?),
                args: self.vec(Self::expr)?,
            }),
            _ => unreachable!("non-invocation tag passed to invocation wire decoder"),
        }
    }

    fn effect_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            10 => Ok(ExprKind::Perform {
                effect: self.vec(Self::string)?,
                effect_span: self.span()?,
                operation: self.string()?,
                operation_span: self.span()?,
                args: self.vec(Self::expr)?,
            }),
            11 => Ok(ExprKind::Handle {
                body: Box::new(self.expr()?),
                handler: self.vec(Self::string)?,
                handler_span: self.span()?,
                args: self.vec(Self::expr)?,
            }),
            _ => unreachable!("non-effect tag passed to effect wire decoder"),
        }
    }

    fn schema_and_access_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            12 => Ok(ExprKind::SchemaDecode {
                schema: self.vec(Self::string)?,
                input: Box::new(self.expr()?),
                base: Box::new(self.expr()?),
            }),
            13 => Ok(ExprKind::SchemaEncode {
                schema: self.vec(Self::string)?,
                value: Box::new(self.expr()?),
            }),
            14 => Ok(ExprKind::FieldAccess {
                base: Box::new(self.expr()?),
                field: self.string()?,
                field_span: self.span()?,
            }),
            15 => Ok(ExprKind::Try(Box::new(self.expr()?))),
            _ => unreachable!("non-schema or access tag passed to schema wire decoder"),
        }
    }

    fn aggregate_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            16 => Ok(ExprKind::Record(self.vec(Self::record_field)?)),
            17 => Ok(ExprKind::Dict(self.vec(Self::dict_entry)?)),
            18 => Ok(ExprKind::List(self.vec(Self::expr)?)),
            19 => Ok(ExprKind::Match {
                scrutinee: Box::new(self.expr()?),
                arms: self.vec(Self::match_arm)?,
            }),
            20 => Ok(ExprKind::If {
                condition: Box::new(self.expr()?),
                then_branch: Box::new(self.expr()?),
                else_if_branches: self.vec(Self::if_branch)?,
                else_branch: Box::new(self.expr()?),
            }),
            _ => unreachable!("non-aggregate tag passed to aggregate wire decoder"),
        }
    }

    fn operator_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            21 => Ok(ExprKind::Prefix {
                op: self.prefix_op()?,
                expr: Box::new(self.expr()?),
            }),
            22 => Ok(ExprKind::Binary {
                op: self.binary_op()?,
                left: Box::new(self.expr()?),
                right: Box::new(self.expr()?),
            }),
            _ => unreachable!("non-operator tag passed to operator wire decoder"),
        }
    }

    fn satisfy(&mut self) -> Result<SatisfyClause, String> {
        Ok(SatisfyClause {
            candidate: self.option(Self::string)?,
            candidate_span: self.option(Self::span)?,
            predicate: self.string()?,
            span: self.span()?,
        })
    }

    fn record_field(&mut self) -> Result<RecordField, String> {
        Ok(RecordField {
            node_id: self.node_id()?,
            name: self.string()?,
            expr: self.expr()?,
            span: self.span()?,
        })
    }

    fn dict_entry(&mut self) -> Result<DictEntry, String> {
        Ok(DictEntry {
            node_id: self.node_id()?,
            key: self.expr()?,
            value: self.expr()?,
            span: self.span()?,
        })
    }

    fn match_arm(&mut self) -> Result<MatchArm, String> {
        Ok(MatchArm {
            node_id: self.node_id()?,
            pattern: self.pattern()?,
            expr: self.expr()?,
            span: self.span()?,
        })
    }

    fn if_branch(&mut self) -> Result<IfBranch, String> {
        Ok(IfBranch {
            node_id: self.node_id()?,
            condition: self.expr()?,
            expr: self.expr()?,
            span: self.span()?,
        })
    }

    fn prefix_op(&mut self) -> Result<PrefixOp, String> {
        match self.u8()? {
            0 => Ok(PrefixOp::Not),
            1 => Ok(PrefixOp::Negate),
            2 => Ok(PrefixOp::BitwiseNot),
            value => Err(format!("invalid prefix op tag {value}")),
        }
    }

    fn binary_op(&mut self) -> Result<BinaryOp, String> {
        match self.u8()? {
            0 => Ok(BinaryOp::PipeGreater),
            1 => Ok(BinaryOp::Or),
            2 => Ok(BinaryOp::And),
            3 => Ok(BinaryOp::BitwiseOr),
            4 => Ok(BinaryOp::BitwiseXor),
            5 => Ok(BinaryOp::BitwiseAnd),
            6 => Ok(BinaryOp::Equal),
            7 => Ok(BinaryOp::NotEqual),
            8 => Ok(BinaryOp::Less),
            9 => Ok(BinaryOp::LessEqual),
            10 => Ok(BinaryOp::Greater),
            11 => Ok(BinaryOp::GreaterEqual),
            12 => Ok(BinaryOp::ShiftLeft),
            13 => Ok(BinaryOp::ShiftRight),
            14 => Ok(BinaryOp::ShiftRightLogical),
            15 => Ok(BinaryOp::Add),
            16 => Ok(BinaryOp::Subtract),
            17 => Ok(BinaryOp::Multiply),
            18 => Ok(BinaryOp::Divide),
            value => Err(format!("invalid binary op tag {value}")),
        }
    }

    fn pattern(&mut self) -> Result<Pattern, String> {
        Ok(Pattern {
            node_id: self.node_id()?,
            kind: self.pattern_kind()?,
            span: self.span()?,
        })
    }

    fn pattern_kind(&mut self) -> Result<PatternKind, String> {
        match self.u8()? {
            0 => Ok(PatternKind::Wildcard),
            1 => Ok(PatternKind::Binding(self.string()?)),
            2 => Ok(PatternKind::StringLiteral(self.string()?)),
            3 => Ok(PatternKind::IntLiteral(self.string()?)),
            4 => Ok(PatternKind::FloatLiteral(self.string()?)),
            5 => Ok(PatternKind::BoolLiteral(self.bool()?)),
            6 => Ok(PatternKind::Unit),
            7 => Ok(PatternKind::Record(self.vec(Self::pattern_field)?)),
            8 => Ok(PatternKind::Constructor {
                name: self.vec(Self::string)?,
                args: self.vec(Self::pattern)?,
            }),
            value => Err(format!("invalid pattern kind tag {value}")),
        }
    }

    fn pattern_field(&mut self) -> Result<PatternField, String> {
        Ok(PatternField {
            node_id: self.node_id()?,
            name: self.string()?,
            pattern: self.pattern()?,
            span: self.span()?,
        })
    }
}
