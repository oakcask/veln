use super::*;

mod expressions;

pub fn decode_surface_module(bytes: &[u8]) -> Result<SurfaceModule, String> {
    let mut reader = Reader { bytes, position: 0 };
    reader.magic()?;
    let module = reader.surface_module()?;
    reader.eof()?;
    Ok(module)
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

    fn type_path_segments(&mut self) -> Result<TypePathSegments, String> {
        Ok(TypePathSegments {
            segments: self.vec(Self::string)?,
            segment_spans: self.vec(Self::span)?,
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
            invalid_names: self.vec(Self::invalid_name)?,
        })
    }

    fn invalid_name(&mut self) -> Result<InvalidName, String> {
        let name = self.string()?;
        let class = match self.u8()? {
            0 => NameClass::Type,
            1 => NameClass::Constructor,
            2 => NameClass::Module,
            3 => NameClass::Function,
            4 => NameClass::ValueBinding,
            value => return Err(format!("invalid name class tag {value}")),
        };
        let occurrence = match self.u8()? {
            0 => NameOccurrence::Declaration,
            1 => NameOccurrence::Binding,
            2 => NameOccurrence::PatternHead,
            3 => NameOccurrence::AliasTarget,
            4 => NameOccurrence::PathSegment,
            value => return Err(format!("invalid name occurrence tag {value}")),
        };
        Ok(InvalidName {
            name,
            class,
            occurrence,
            span: self.span()?,
            enclosing_function_span: self.option(Self::span)?,
            segment_index: self.option(Self::usize)?,
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
            name_spans: self.vec(Self::span)?,
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
            target_spans: self.vec(Self::span)?,
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
            return_type_paths: self.vec(Self::type_path_segments)?,
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
            ty: self.option(Self::string)?,
            ty_span: self.option(Self::span)?,
            ty_paths: self.vec(Self::type_path_segments)?,
            is_variadic: self.bool()?,
            span: self.span()?,
        })
    }

    fn result_binding(&mut self) -> Result<ResultBinding, String> {
        Ok(ResultBinding {
            node_id: self.node_id()?,
            name: self.string()?,
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
                annotation_paths: self.vec(Self::type_path_segments)?,
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
}
