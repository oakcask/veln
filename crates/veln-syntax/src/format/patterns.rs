use crate::{Pattern, PatternKind};

pub(super) fn format_pattern(pattern: &Pattern) -> String {
    match &pattern.kind {
        PatternKind::Wildcard => "_".to_string(),
        PatternKind::Binding(name) => name.clone(),
        PatternKind::StringLiteral(value)
        | PatternKind::IntLiteral(value)
        | PatternKind::FloatLiteral(value) => value.clone(),
        PatternKind::BoolLiteral(true) => "true".to_string(),
        PatternKind::BoolLiteral(false) => "false".to_string(),
        PatternKind::Unit => "()".to_string(),
        PatternKind::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, format_pattern(&field.pattern)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        PatternKind::Constructor { name, args, .. } => {
            if args.is_empty() {
                name.join("::")
            } else {
                let args = args
                    .iter()
                    .map(format_pattern)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({args})", name.join("::"))
            }
        }
    }
}
