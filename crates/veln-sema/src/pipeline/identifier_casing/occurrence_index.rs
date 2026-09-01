use super::*;

pub(super) fn enclosing_function_span_for_segment(
    module: &SurfaceModule,
    segment: &QualifiedPathSegment,
) -> Option<veln_source::SourceSpan> {
    module
        .invalid_names
        .iter()
        .find(|invalid| {
            invalid.occurrence == NameOccurrence::PathSegment
                && invalid.segment_index == Some(segment.segment_index)
                && invalid.span.file == segment.span.file
                && invalid.span.start.offset == segment.span.start.offset
                && invalid.span.end.offset == segment.span.end.offset
        })
        .and_then(|invalid| invalid.enclosing_function_span.clone())
        .or_else(|| function_span_for_segment(module, &segment.span))
}

fn function_span_for_segment(
    module: &SurfaceModule,
    span: &veln_source::SourceSpan,
) -> Option<veln_source::SourceSpan> {
    module
        .functions
        .iter()
        .find(|function| {
            function.span.file == span.file
                && function.span.start.offset <= span.start.offset
                && function.span.end.offset >= span.end.offset
        })
        .map(|function| function.span.clone())
}

pub(super) fn classified_invalid_path_segment(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Option<QualifiedPathSegment> {
    if invalid_name_repeats_quarantined_import_alias(invalid, module) {
        return None;
    }
    match invalid.class {
        NameClass::Module => {
            if invalid_segment_is_constructor_type_qualifier(invalid, occurrences, environment) {
                return Some(classified_path_segment(
                    invalid,
                    NameClass::Type,
                    QualifiedPathSegmentEvidence::Resolved,
                ));
            }
            if invalid_value_segment_lacks_value_role(invalid, occurrences, environment) {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Module,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::Type => {
            if invalid_type_segment_has_module_role(invalid, occurrences, environment) {
                return Some(classified_path_segment(
                    invalid,
                    NameClass::Module,
                    QualifiedPathSegmentEvidence::Resolved,
                ));
            }
            if invalid_type_segment_lacks_constructor_role(invalid, occurrences, environment) {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Type,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::Constructor => {
            if invalid_constructor_segment_has_function_role(invalid, occurrences, environment) {
                return Some(classified_path_segment(
                    invalid,
                    NameClass::Function,
                    QualifiedPathSegmentEvidence::UniqueRecovery,
                ));
            }
            if invalid_constructor_segment_lacks_constructor_role(invalid, occurrences, environment)
            {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Constructor,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::Function => {
            if invalid_function_segment_lacks_function_role(invalid, occurrences, environment)
                || invalid_constructor_segment_lacks_constructor_role(
                    invalid,
                    occurrences,
                    environment,
                )
            {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Function,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::ValueBinding => {
            if invalid_value_segment_lacks_value_role(invalid, occurrences, environment) {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::ValueBinding,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
    }
}

#[derive(Clone)]
pub(super) struct QualifiedPathOccurrence {
    pub(super) segments: Vec<String>,
    pub(super) segment_spans: Vec<veln_source::SourceSpan>,
    pub(super) current_module: Option<String>,
    pub(super) call_role: bool,
    pub(super) pattern_role: bool,
}

#[derive(Default)]
pub(super) struct QualifiedPathOccurrenceIndex {
    by_segment: BTreeMap<(String, usize, usize, usize), Vec<QualifiedPathOccurrence>>,
}

impl QualifiedPathOccurrenceIndex {
    pub(super) fn new(module: &SurfaceModule) -> Self {
        let mut index = Self::default();
        for function in &module.functions {
            for line in &function.body {
                index.collect_body_line(line, function.module_name.as_deref());
            }
        }
        for handler in &module.handlers {
            for clause in &handler.operation_clauses {
                index.collect_expr(&clause.body, handler.module_name.as_deref(), false);
            }
        }
        index
    }

    pub(super) fn occurrences_for(&self, invalid: &InvalidName) -> &[QualifiedPathOccurrence] {
        let Some(segment_index) = invalid.segment_index else {
            return &[];
        };
        self.by_segment
            .get(&(
                invalid.span.file.as_str().to_string(),
                invalid.span.start.offset,
                invalid.span.end.offset,
                segment_index,
            ))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn collect_body_line(&mut self, line: &veln_ast::BodyLine, current_module: Option<&str>) {
        match &line.kind {
            veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
                self.collect_pattern(pattern, current_module);
                self.collect_expr(expr, current_module, false);
            }
            veln_ast::BodyLineKind::Expr { expr } => self.collect_expr(expr, current_module, false),
        }
    }

    fn collect_expr(
        &mut self,
        expr: &veln_ast::Expr,
        current_module: Option<&str>,
        call_role: bool,
    ) {
        match &expr.kind {
            veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } => self.insert(segments, segment_spans, current_module, call_role, false),
            veln_ast::ExprKind::Call { callee, args } => {
                self.collect_expr(callee, current_module, true);
                for arg in args {
                    self.collect_expr(arg, current_module, false);
                }
            }
            veln_ast::ExprKind::TypeApply { callee, .. }
            | veln_ast::ExprKind::FieldAccess { base: callee, .. }
            | veln_ast::ExprKind::Try(callee)
            | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
                self.collect_expr(callee, current_module, call_role);
            }
            veln_ast::ExprKind::Binary { left, right, .. } => {
                self.collect_expr(left, current_module, false);
                self.collect_expr(right, current_module, false);
            }
            veln_ast::ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_expr(condition, current_module, false);
                self.collect_expr(then_branch, current_module, false);
                for branch in else_if_branches {
                    self.collect_expr(&branch.condition, current_module, false);
                    self.collect_expr(&branch.expr, current_module, false);
                }
                self.collect_expr(else_branch, current_module, false);
            }
            veln_ast::ExprKind::Record(fields) => {
                for field in fields {
                    self.collect_expr(&field.expr, current_module, false);
                }
            }
            veln_ast::ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_expr(&entry.key, current_module, false);
                    self.collect_expr(&entry.value, current_module, false);
                }
            }
            veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
                for item in items {
                    self.collect_expr(item, current_module, false);
                }
            }
            veln_ast::ExprKind::Handle { body, args, .. } => {
                self.collect_expr(body, current_module, false);
                for arg in args {
                    self.collect_expr(arg, current_module, false);
                }
            }
            veln_ast::ExprKind::SchemaDecode { input, base, .. } => {
                self.collect_expr(input, current_module, false);
                self.collect_expr(base, current_module, false);
            }
            veln_ast::ExprKind::SchemaEncode { value, .. } => {
                self.collect_expr(value, current_module, false);
            }
            veln_ast::ExprKind::Match { scrutinee, arms } => {
                self.collect_expr(scrutinee, current_module, false);
                for arm in arms {
                    self.collect_pattern(&arm.pattern, current_module);
                    self.collect_expr(&arm.expr, current_module, false);
                }
            }
            _ => {}
        }
    }

    fn collect_pattern(&mut self, pattern: &veln_ast::Pattern, current_module: Option<&str>) {
        match &pattern.kind {
            veln_ast::PatternKind::Constructor {
                name,
                name_spans,
                args,
            } => {
                self.insert(name, name_spans, current_module, false, true);
                for arg in args {
                    self.collect_pattern(arg, current_module);
                }
            }
            veln_ast::PatternKind::Record(fields) => {
                for field in fields {
                    self.collect_pattern(&field.pattern, current_module);
                }
            }
            _ => {}
        }
    }

    fn insert(
        &mut self,
        segments: &[String],
        segment_spans: &[veln_source::SourceSpan],
        current_module: Option<&str>,
        call_role: bool,
        pattern_role: bool,
    ) {
        if segments.len() < 2 {
            return;
        }
        let occurrence = QualifiedPathOccurrence {
            segments: segments.to_vec(),
            segment_spans: segment_spans.to_vec(),
            current_module: current_module.map(str::to_string),
            call_role,
            pattern_role,
        };
        for (index, span) in segment_spans.iter().enumerate() {
            self.by_segment
                .entry((
                    span.file.as_str().to_string(),
                    span.start.offset,
                    span.end.offset,
                    index,
                ))
                .or_default()
                .push(occurrence.clone());
        }
    }
}

pub(super) fn name_satisfies_class(name: &str, class: NameClass) -> bool {
    match class {
        NameClass::Type | NameClass::Constructor => {
            name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        }
        NameClass::Module | NameClass::Function | NameClass::ValueBinding => {
            name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        }
    }
}

fn classified_path_segment(
    invalid: &InvalidName,
    role: NameClass,
    evidence: QualifiedPathSegmentEvidence,
) -> QualifiedPathSegment {
    QualifiedPathSegment {
        name: invalid.name.clone(),
        role,
        occurrence: invalid.occurrence,
        span: invalid.span.clone(),
        segment_index: invalid
            .segment_index
            .expect("classified path segment has segment index"),
        evidence,
    }
}

fn invalid_segment_is_constructor_type_qualifier(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    occurrences
        .occurrences_for(invalid)
        .iter()
        .any(|occurrence| {
            !occurrence.pattern_role
                && invalid.segment_index.is_some_and(|index| {
                    index + 2 == occurrence.segments.len()
                        && type_qualified_constructor_path(
                            invalid,
                            &occurrence.segments,
                            occurrence.current_module.as_deref(),
                            environment,
                        )
                })
        })
}

fn invalid_value_segment_lacks_value_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if !matches!(invalid.class, NameClass::Module | NameClass::ValueBinding)
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .any(|occurrence| {
            !occurrence.pattern_role
                && invalid_value_segment_lacks_value_role_for_path(
                    invalid,
                    &occurrence.segments,
                    &occurrence.segment_spans,
                    occurrence.current_module.as_deref(),
                    environment,
                )
        })
}
