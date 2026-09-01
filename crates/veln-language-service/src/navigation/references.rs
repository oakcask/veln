fn call_references(file: &IndexedFile, name: &str) -> Vec<SourceSpan> {
    let tokens = lex(&file.source).tokens;
    let scopes = function_scopes(&tokens);
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == name
                && is_identifier(&token.text)
                && previous_non_layout_token(&tokens, *index)
                    .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
                && !is_field_name(&tokens, *index)
                && !is_function_declaration_name(&tokens, *index)
                && !is_parameter_name(&tokens, *index)
                && !is_local_binding_name(&tokens, *index)
                && !is_handler_operation_clause_operation_name(&tokens, *index)
                && (token_scope(&scopes, token.range.start)
                    .is_some_and(|scope| !scope.shadows(name, &tokens, *index))
                    || handler_function_reference_is_unshadowed(file, &tokens, *index, name)
                    || is_function_alias_target_reference(&tokens, *index, name)
                    || is_codec_implementation_function_reference(&tokens, *index, name))
        })
        .map(|(_, token)| file.source.span(token.range))
        .collect()
}

impl IndexedFile {
    fn type_reference_spans(&self, name: &str) -> Vec<(usize, SourceSpan)> {
        if !name
            .chars()
            .next()
            .is_some_and(|initial| initial.is_ascii_uppercase())
        {
            return Vec::new();
        }
        self.type_reference_spans_named(name)
    }

    fn type_reference_spans_named(&self, name: &str) -> Vec<(usize, SourceSpan)> {
        self.type_reference_locations()
            .iter()
            .filter(|(candidate, _, _)| candidate == name)
            .map(|(_, token_index, span)| (*token_index, span.clone()))
            .collect()
    }

    fn type_reference_locations(&self) -> &TypeReferenceLocations {
        self.type_reference_locations.get_or_init(|| {
            #[cfg(test)]
            record_type_reference_collection();
            collect_type_reference_locations(&self.source, &self.tokens)
        })
    }
}

fn collect_type_reference_locations(
    source: &SourceFile,
    tokens: &[Token],
) -> TypeReferenceLocations {
    let parsed = parse(source);
    let mut spans: TypeReferenceLocations = Vec::new();
    for item in &parsed.tree.items {
        match item {
            SyntaxItem::Function(function) => {
                spans.extend(type_references_in_params(
                    source,
                    tokens,
                    &function.params,
                ));
                if let Some(span) = &function.return_type_span {
                    spans.extend(type_reference_tokens_in_span(source, tokens, span));
                }
                spans.extend(type_references_in_body_lines(
                    source,
                    tokens,
                    &function.body,
                ));
            }
            SyntaxItem::Handler(handler) => {
                spans.extend(type_references_in_params(
                    source,
                    tokens,
                    &handler.params,
                ));
            }
            SyntaxItem::Effect(effect) => {
                for operation in &effect.operations {
                    spans.extend(type_references_in_params(
                        source,
                        tokens,
                        &operation.params,
                    ));
                    spans.extend(type_references_after_token_in_span(
                        source,
                        tokens,
                        &operation.span,
                        TokenKind::Arrow,
                    ));
                }
            }
            SyntaxItem::Type(type_decl) => {
                for variant in &type_decl.variants {
                    for field in &variant.fields {
                        spans.extend(type_references_in_variant_field(
                            source,
                            tokens,
                            &field.span,
                        ));
                    }
                }
            }
            SyntaxItem::PublicAlias(alias) if alias.kind == PublicAliasKind::Type => {
                spans.extend(
                    alias
                        .target_spans
                        .iter()
                        .flat_map(|span| type_reference_tokens_in_span(source, tokens, span)),
                );
            }
            _ => {}
        }
    }
    spans.sort_by(|left, right| {
        left.2
            .file
            .as_str()
            .cmp(right.2.file.as_str())
            .then(left.2.start.offset.cmp(&right.2.start.offset))
            .then(left.2.end.offset.cmp(&right.2.end.offset))
    });
    spans.dedup_by(|left, right| {
        left.0 == right.0
            && left.2.file == right.2.file
            && left.2.start.offset == right.2.start.offset
            && left.2.end.offset == right.2.end.offset
    });
    spans
}

fn is_type_reference_token(file: &IndexedFile, name: &str, selection: &SourceSpan) -> bool {
    file.type_reference_spans(name)
        .into_iter()
        .any(|(_, span)| {
            span.file == selection.file
                && span.start.offset == selection.start.offset
                && span.end.offset == selection.end.offset
        })
}

fn is_type_reference_token_named(file: &IndexedFile, name: &str, selection: &SourceSpan) -> bool {
    file.type_reference_spans_named(name)
        .into_iter()
        .any(|(_, span)| {
            span.file == selection.file
                && span.start.offset == selection.start.offset
                && span.end.offset == selection.end.offset
        })
}

fn type_references_in_params(
    source: &SourceFile,
    tokens: &[Token],
    params: &[veln_syntax::Param],
) -> TypeReferenceLocations {
    params
        .iter()
        .filter_map(|param| param.ty_span.as_ref())
        .flat_map(|span| type_reference_tokens_in_span(source, tokens, span))
        .collect()
}

fn type_references_in_body_lines(
    source: &SourceFile,
    tokens: &[Token],
    body: &[BodyLine],
) -> TypeReferenceLocations {
    body.iter()
        .flat_map(|line| match line {
            BodyLine::Let {
                annotation: Some(_),
                span,
                ..
            } => type_references_after_token_until_token_in_span(
                source,
                tokens,
                span,
                TokenKind::Colon,
                TokenKind::Equal,
            ),
            _ => Vec::new(),
        })
        .collect()
}

fn type_references_in_variant_field(
    source: &SourceFile,
    tokens: &[Token],
    field_span: &SourceSpan,
) -> TypeReferenceLocations {
    let colon_offset = tokens
        .iter()
        .find(|token| {
            token.kind == TokenKind::Colon
                && token.range.start >= field_span.start.offset
                && token.range.end <= field_span.end.offset
        })
        .map(|token| token.range.end)
        .unwrap_or(field_span.start.offset);
    type_reference_tokens_in_range(source, tokens, colon_offset, field_span.end.offset)
}

fn type_references_after_token_in_span(
    source: &SourceFile,
    tokens: &[Token],
    span: &SourceSpan,
    start_kind: TokenKind,
) -> TypeReferenceLocations {
    let start_offset = tokens
        .iter()
        .find(|token| {
            token.kind == start_kind
                && token.range.start >= span.start.offset
                && token.range.end <= span.end.offset
        })
        .map(|token| token.range.end)
        .unwrap_or(span.end.offset);
    type_reference_tokens_in_range(source, tokens, start_offset, span.end.offset)
}

fn type_references_after_token_until_token_in_span(
    source: &SourceFile,
    tokens: &[Token],
    span: &SourceSpan,
    start_kind: TokenKind,
    end_kind: TokenKind,
) -> TypeReferenceLocations {
    let Some(start_index) = tokens.iter().position(|token| {
        token.kind == start_kind
            && token.range.start >= span.start.offset
            && token.range.end <= span.end.offset
    }) else {
        return Vec::new();
    };
    let start_offset = tokens[start_index].range.end;
    let end_offset = tokens[start_index + 1..]
        .iter()
        .find(|token| token.kind == end_kind && token.range.end <= span.end.offset)
        .map(|token| token.range.start)
        .unwrap_or(span.end.offset);
    type_reference_tokens_in_range(source, tokens, start_offset, end_offset)
}

fn type_reference_tokens_in_span(
    source: &SourceFile,
    tokens: &[Token],
    span: &SourceSpan,
) -> TypeReferenceLocations {
    type_reference_tokens_in_range(source, tokens, span.start.offset, span.end.offset)
}

fn type_reference_tokens_in_range(
    source: &SourceFile,
    tokens: &[Token],
    start_offset: usize,
    end_offset: usize,
) -> TypeReferenceLocations {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            token.range.start >= start_offset
                && token.range.end <= end_offset
                && token.kind == TokenKind::Ident
        })
        .map(|(index, token)| {
            (
                token.text.clone(),
                index,
                source.span(token.range),
            )
        })
        .collect()
}
