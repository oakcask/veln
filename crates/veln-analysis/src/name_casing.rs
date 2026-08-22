use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;
use veln_syntax::{
    BodyLine, Expr, ExprKind, Pattern, PatternKind, PublicAliasKind, SyntaxItem, SyntaxTree,
};

#[derive(Clone, Copy)]
enum NameClass {
    Type,
    Constructor,
    Function,
    ValueBinding,
}

impl NameClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Constructor => "constructor",
            Self::Function => "function",
            Self::ValueBinding => "value_binding",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Type => "type name",
            Self::Constructor => "constructor name",
            Self::Function => "function name",
            Self::ValueBinding => "binding name",
        }
    }

    fn required_initial(self) -> &'static str {
        match self {
            Self::Type | Self::Constructor => "ascii_uppercase",
            Self::Function | Self::ValueBinding => "ascii_lowercase",
        }
    }

    fn accepts(self, initial: ObservedInitial) -> bool {
        match self {
            Self::Type | Self::Constructor => initial == ObservedInitial::AsciiUppercase,
            Self::Function | Self::ValueBinding => initial == ObservedInitial::AsciiLowercase,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ObservedInitial {
    AsciiUppercase,
    AsciiLowercase,
    Underscore,
    Other,
}

impl ObservedInitial {
    fn classify(name: &str) -> Self {
        match name.as_bytes().first() {
            Some(b'A'..=b'Z') => Self::AsciiUppercase,
            Some(b'a'..=b'z') => Self::AsciiLowercase,
            Some(b'_') => Self::Underscore,
            _ => Self::Other,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::AsciiUppercase => "ascii_uppercase",
            Self::AsciiLowercase => "ascii_lowercase",
            Self::Underscore => "underscore",
            Self::Other => "other",
        }
    }
}

pub(crate) fn validate_identifier_casing(tree: &SyntaxTree) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for item in &tree.items {
        match item {
            SyntaxItem::Type(type_decl) => {
                validate_optional_name(
                    &mut diagnostics,
                    type_decl.name.as_deref(),
                    type_decl.name_span.as_ref(),
                    NameClass::Type,
                    "declaration",
                );
                for variant in &type_decl.variants {
                    validate_optional_name(
                        &mut diagnostics,
                        variant.name.as_deref(),
                        variant.name_span.as_ref(),
                        NameClass::Constructor,
                        "declaration",
                    );
                }
            }
            SyntaxItem::Function(function) => {
                validate_optional_name(
                    &mut diagnostics,
                    function.name.as_deref(),
                    function.name_span.as_ref(),
                    NameClass::Function,
                    "declaration",
                );
                validate_params(&mut diagnostics, &function.params);
                if let Some(binding) = &function.return_binding {
                    validate_name(
                        &mut diagnostics,
                        &binding.name,
                        &binding.span,
                        NameClass::ValueBinding,
                        "binding",
                    );
                }
                for line in &function.body {
                    validate_body_line(&mut diagnostics, line);
                }
            }
            SyntaxItem::Handler(handler) => {
                validate_params(&mut diagnostics, &handler.params);
                for clause in &handler.operation_clauses {
                    validate_params(&mut diagnostics, &clause.params);
                    validate_expr(&mut diagnostics, &clause.body);
                }
            }
            SyntaxItem::PublicAlias(alias) => {
                let class = match alias.kind {
                    PublicAliasKind::Function => Some(NameClass::Function),
                    PublicAliasKind::Type => Some(NameClass::Type),
                    PublicAliasKind::Schema => None,
                };
                if let Some(class) = class {
                    validate_optional_name(
                        &mut diagnostics,
                        alias.name.as_deref(),
                        alias.name_span.as_ref(),
                        class,
                        "declaration",
                    );
                }
            }
            SyntaxItem::Effect(_) | SyntaxItem::Schema(_) | SyntaxItem::Codec(_) => {}
        }
    }
    diagnostics.sort_by(|left, right| {
        left.span
            .as_ref()
            .map(|span| (span.file.as_str(), span.start.offset, span.end.offset))
            .cmp(
                &right
                    .span
                    .as_ref()
                    .map(|span| (span.file.as_str(), span.start.offset, span.end.offset)),
            )
    });
    diagnostics
}

fn validate_params(diagnostics: &mut Vec<Diagnostic>, params: &[veln_syntax::Param]) {
    for param in params {
        validate_name(
            diagnostics,
            &param.name,
            &param.name_span,
            NameClass::ValueBinding,
            "binding",
        );
    }
}

fn validate_body_line(diagnostics: &mut Vec<Diagnostic>, line: &BodyLine) {
    match line {
        BodyLine::Let { pattern, expr, .. } => {
            validate_pattern(diagnostics, pattern);
            validate_expr(diagnostics, expr);
        }
        BodyLine::Expr { expr, .. } => validate_expr(diagnostics, expr),
    }
}

fn validate_expr(diagnostics: &mut Vec<Diagnostic>, expr: &Expr) {
    match &expr.kind {
        ExprKind::Hole { satisfy, .. } => {
            if let Some(satisfy) = satisfy
                && let (Some(candidate), Some(span)) = (
                    satisfy.candidate.as_deref(),
                    satisfy.candidate_span.as_ref(),
                )
            {
                validate_name(
                    diagnostics,
                    candidate,
                    span,
                    NameClass::ValueBinding,
                    "binding",
                );
            }
        }
        ExprKind::TypeApply { callee, .. } => validate_expr(diagnostics, callee),
        ExprKind::Call { callee, args } => {
            validate_expr(diagnostics, callee);
            for arg in args {
                validate_expr(diagnostics, arg);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                validate_expr(diagnostics, arg);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            validate_expr(diagnostics, body);
            for arg in args {
                validate_expr(diagnostics, arg);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            validate_expr(diagnostics, input);
            validate_expr(diagnostics, base);
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => validate_expr(diagnostics, value),
        ExprKind::Record(fields) => {
            for field in fields {
                validate_expr(diagnostics, &field.expr);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                validate_expr(diagnostics, &entry.key);
                validate_expr(diagnostics, &entry.value);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                validate_expr(diagnostics, item);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            validate_expr(diagnostics, scrutinee);
            for arm in arms {
                validate_pattern(diagnostics, &arm.pattern);
                validate_expr(diagnostics, &arm.expr);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            validate_expr(diagnostics, condition);
            validate_expr(diagnostics, then_branch);
            for branch in else_if_branches {
                validate_expr(diagnostics, &branch.condition);
                validate_expr(diagnostics, &branch.expr);
            }
            validate_expr(diagnostics, else_branch);
        }
        ExprKind::Binary { left, right, .. } => {
            validate_expr(diagnostics, left);
            validate_expr(diagnostics, right);
        }
        ExprKind::Missing
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn validate_pattern(diagnostics: &mut Vec<Diagnostic>, pattern: &Pattern) {
    match &pattern.kind {
        PatternKind::Binding(name) => validate_name(
            diagnostics,
            name,
            &pattern.span,
            NameClass::ValueBinding,
            "binding",
        ),
        PatternKind::Record(fields) => {
            for field in fields {
                validate_pattern(diagnostics, &field.pattern);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                validate_pattern(diagnostics, arg);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn validate_optional_name(
    diagnostics: &mut Vec<Diagnostic>,
    name: Option<&str>,
    span: Option<&SourceSpan>,
    class: NameClass,
    occurrence: &'static str,
) {
    if let (Some(name), Some(span)) = (name, span) {
        validate_name(diagnostics, name, span, class, occurrence);
    }
}

fn validate_name(
    diagnostics: &mut Vec<Diagnostic>,
    name: &str,
    span: &SourceSpan,
    class: NameClass,
    occurrence: &'static str,
) {
    if name.is_empty() {
        return;
    }
    let observed = ObservedInitial::classify(name);
    if class.accepts(observed) {
        return;
    }
    let required_label = match class.required_initial() {
        "ascii_uppercase" => "an ASCII uppercase letter",
        _ => "an ASCII lowercase letter",
    };
    diagnostics.push(Diagnostic::new(
        "name.invalid_case",
        Severity::Error,
        DiagnosticKind::Name,
        format!("{} must start with {required_label}", class.label()),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("origin", JsonValue::string("source")),
            ("occurrence", JsonValue::string(occurrence)),
            ("name", JsonValue::string(name)),
            ("name_class", JsonValue::string(class.as_str())),
            (
                "required_initial",
                JsonValue::string(class.required_initial()),
            ),
            ("observed_initial", JsonValue::string(observed.as_str())),
        ]),
    ));
}

#[cfg(test)]
mod tests {
    use veln_source::SourceFile;
    use veln_syntax::{ExprKind, SyntaxItem, parse};

    use super::*;

    fn casing_diagnostics(text: &str) -> Vec<Diagnostic> {
        let parsed = parse(&SourceFile::new("main.veln", text));
        assert!(
            parsed.diagnostics.is_empty(),
            "source should parse without recovery diagnostics: {:?}",
            parsed.diagnostics
        );
        validate_identifier_casing(&parsed.tree)
    }

    #[test]
    fn accepts_each_declaration_and_binding_class() {
        let diagnostics = casing_diagnostics(concat!(
            "type Option\n",
            "  None\n",
            "  Some(value: Int)\n",
            "end\n",
            "pub type ExportedOption = Option\n",
            "pub fn exported = identity\n",
            "fn identity(value: Int) -> result: Int\n",
            "  let {item: bound} = {item: value}\n",
            "  match bound\n",
            "    matched => matched\n",
            "  end\n",
            "end\n",
            "test identity_test(input: Int)\n",
            "  input\n",
            "end\n",
            "effect Ask\n",
            "  value(input: Int) -> Int\n",
            "end\n",
            "handler ask(context: Int) handles Ask\n",
            "  value(operation_value) => operation_value\n",
            "end\n",
        ));

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn rejects_every_covered_declaration_and_binding_position() {
        let source_text = concat!(
            "type option\n",
            "  some\n",
            "end\n",
            "pub type alias = Option\n",
            "pub fn Export = identity\n",
            "fn Build(Argument: Int) -> Result: Int\n",
            "  let _Local = Argument\n",
            "  let {item: _nested} = {item: Argument}\n",
            "  match Argument\n",
            "    _arm => Argument\n",
            "  end\n",
            "  _hole satisfy Candidate => true\n",
            "end\n",
            "test Check(TestArgument: Int)\n",
            "  TestArgument\n",
            "end\n",
            "effect Ask\n",
            "  value(input: Int) -> Int\n",
            "end\n",
            "handler ask(Context: Int) handles Ask\n",
            "  value(OperationArgument) => OperationArgument\n",
            "end\n",
        );
        let diagnostics = casing_diagnostics(source_text);
        let names = diagnostics
            .iter()
            .map(|diagnostic| {
                let json = diagnostic.details.to_json();
                let marker = "\"name\":\"";
                let start = json.find(marker).expect("name detail") + marker.len();
                json[start..]
                    .split('"')
                    .next()
                    .expect("name detail value")
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            [
                "option",
                "some",
                "alias",
                "Export",
                "Build",
                "Argument",
                "Result",
                "_Local",
                "_nested",
                "_arm",
                "Candidate",
                "Check",
                "TestArgument",
                "Context",
                "OperationArgument",
            ]
        );
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.kind == DiagnosticKind::Name
                && diagnostic.span.as_ref().is_some_and(|span| {
                    let name = &source_text[span.start.offset..span.end.offset];
                    names.contains(&name.to_string())
                })
        }));
    }

    #[test]
    fn rejects_public_and_private_payload_and_nullary_constructors() {
        let text = concat!(
            "pub type result\n",
            "  private_nullary\n",
            "  private_payload(Int)\n",
            "  pub public_nullary\n",
            "  pub public_payload(Int)\n",
            "end\n",
        );
        let diagnostics = casing_diagnostics(text);
        let spans = diagnostics
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.as_ref().expect("name span");
                &text[span.start.offset..span.end.offset]
            })
            .collect::<Vec<_>>();

        assert_eq!(
            spans,
            [
                "result",
                "private_nullary",
                "private_payload",
                "public_nullary",
                "public_payload",
            ]
        );
    }

    #[test]
    fn underscore_recovery_preserves_expression_holes() {
        let source = SourceFile::new(
            "main.veln",
            "fn _Build(_Argument: Int) -> _Result: Int\n  let _local = _expression\n  _local\nend\n",
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let diagnostics = validate_identifier_casing(&parsed.tree);
        assert_eq!(diagnostics.len(), 4);
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic
                .details
                .to_json()
                .contains("\"observed_initial\":\"underscore\"")
        }));

        let SyntaxItem::Function(function) = &parsed.tree.items[0] else {
            panic!("expected function")
        };
        let BodyLine::Let { expr, .. } = &function.body[0] else {
            panic!("expected let")
        };
        assert!(
            matches!(expr.kind, ExprKind::Hole { ref name, .. } if name.as_deref() == Some("expression"))
        );
    }

    #[test]
    fn underscore_led_names_recover_in_every_covered_position() {
        let text = concat!(
            "type _Type\n",
            "  _Variant\n",
            "end\n",
            "pub type _TypeAlias = Option\n",
            "pub fn _function_alias = identity\n",
            "fn _function(_parameter: Int) -> _result: Int\n",
            "  let _local = _parameter\n",
            "  let {item: _nested} = {item: _parameter}\n",
            "  match _parameter\n",
            "    _matched => _parameter\n",
            "  end\n",
            "  _hole satisfy _candidate => true\n",
            "end\n",
            "test _test() -> ()\n",
            "  ()\n",
            "end\n",
            "effect Ask\n",
            "  value(input: Int) -> Int\n",
            "end\n",
            "handler ask(_context: Int) handles Ask\n",
            "  value(_operation_parameter) => _operation_parameter\n",
            "end\n",
        );
        let diagnostics = casing_diagnostics(text);
        let spans = diagnostics
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.as_ref().expect("name span");
                &text[span.start.offset..span.end.offset]
            })
            .collect::<Vec<_>>();

        assert_eq!(
            spans,
            [
                "_Type",
                "_Variant",
                "_TypeAlias",
                "_function_alias",
                "_function",
                "_parameter",
                "_result",
                "_local",
                "_nested",
                "_matched",
                "_candidate",
                "_test",
                "_context",
                "_operation_parameter",
            ]
        );
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic
                .details
                .to_json()
                .contains("\"observed_initial\":\"underscore\"")
        }));

        let standalone = parse(&SourceFile::new("main.veln", "fn _() -> ()\n  ()\nend\n"));
        assert!(
            standalone
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.expected_identifier")
        );
        assert!(validate_identifier_casing(&standalone.tree).is_empty());
    }

    #[test]
    fn exact_spans_account_for_unicode_and_crlf() {
        let source = SourceFile::new("unicode.veln", "# λ\r\nfn Build()\r\n  0\r\nend\r\n");
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let diagnostics = validate_identifier_casing(&parsed.tree);

        assert_eq!(diagnostics.len(), 1);
        let span = diagnostics[0].span.as_ref().expect("name span");
        assert_eq!(span.start.line, 2);
        assert_eq!(span.start.column, 4);
        assert_eq!(span.start.offset, 9);
        assert_eq!(span.end.offset, 14);
        assert_eq!(&source.text()[span.start.offset..span.end.offset], "Build");
    }
}
