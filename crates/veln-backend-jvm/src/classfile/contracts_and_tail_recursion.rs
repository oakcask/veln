use super::*;

pub(super) fn contract_integer_value(text: &str) -> Option<i64> {
    if let Some(magnitude) = text.strip_prefix('-') {
        return parse_integer_literal(magnitude.trim())
            .ok()
            .and_then(|literal| literal.value.checked_neg());
    }
    parse_integer_literal(text)
        .ok()
        .map(|literal| literal.value)
}

pub(crate) fn classify_tail_recursion(function: &IrFunction) -> TailRecursionEligibility {
    if has_runtime_return_contract(function) {
        return TailRecursionEligibility::RuntimeReturnContract;
    }
    let mut facts = TailRecursionFacts::default();
    for stmt in &function.body {
        scan_stmt_tail_recursion(stmt, &function.name, &mut facts);
    }
    if facts.has_indirect_value_call {
        return TailRecursionEligibility::IndirectValueCall;
    }
    if facts.has_non_tail_self_call {
        return TailRecursionEligibility::NonTailSelfCall;
    }
    if facts.has_tail_self_call {
        TailRecursionEligibility::Eligible
    } else {
        TailRecursionEligibility::NotRecursive
    }
}

fn has_runtime_return_contract(function: &IrFunction) -> bool {
    function.contracts.iter().any(|contract| {
        matches!(
            contract.kind,
            ContractKind::Ensure | ContractKind::Invariant
        ) && contract.obligation_status == ContractObligationStatus::RuntimeRequired
    })
}

#[derive(Default)]
struct TailRecursionFacts {
    has_tail_self_call: bool,
    has_non_tail_self_call: bool,
    has_indirect_value_call: bool,
}

fn scan_stmt_tail_recursion(stmt: &IrStmt, function: &str, facts: &mut TailRecursionFacts) {
    match &stmt.kind {
        IrStmtKind::Let { value, .. } | IrStmtKind::Expr { value } => {
            scan_expr_tail_recursion(value, function, false, facts);
        }
        IrStmtKind::Return { value } => scan_expr_tail_recursion(value, function, true, facts),
    }
}

fn scan_expr_tail_recursion(
    expr: &IrExpr,
    function: &str,
    tail_position: bool,
    facts: &mut TailRecursionFacts,
) {
    match &expr.kind {
        IrExprKind::Call { target, args } => {
            match target {
                IrCallTarget::Function(name) if name == function && tail_position => {
                    facts.has_tail_self_call = true;
                }
                IrCallTarget::Function(name) if name == function => {
                    facts.has_non_tail_self_call = true;
                }
                IrCallTarget::CodecDecode { function: name, .. } if name == function => {
                    facts.has_non_tail_self_call = true;
                }
                IrCallTarget::Value(_) => {
                    facts.has_indirect_value_call = true;
                }
                _ => {}
            }
            for arg in args {
                scan_expr_tail_recursion(arg, function, false, facts);
            }
        }
        IrExprKind::Match { scrutinee, arms } => {
            scan_expr_tail_recursion(scrutinee, function, false, facts);
            for arm in arms {
                scan_expr_tail_recursion(&arm.value, function, tail_position, facts);
            }
        }
        IrExprKind::Perform { args, .. } => {
            for arg in args {
                scan_expr_tail_recursion(arg, function, false, facts);
            }
        }
        IrExprKind::Handle {
            context_args, body, ..
        } => {
            for arg in context_args {
                scan_expr_tail_recursion(arg, function, false, facts);
            }
            scan_expr_tail_recursion(body, function, tail_position, facts);
        }
        IrExprKind::ResultOk(value)
        | IrExprKind::ResultErr(value)
        | IrExprKind::OptionSome(value)
        | IrExprKind::FieldAccess { base: value, .. }
        | IrExprKind::Try(value)
        | IrExprKind::Prefix { expr: value, .. } => {
            scan_expr_tail_recursion(value, function, false, facts);
        }
        IrExprKind::ListCons { head, tail } => {
            scan_expr_tail_recursion(head, function, false, facts);
            scan_expr_tail_recursion(tail, function, false, facts);
        }
        IrExprKind::AdtVariant { payloads, .. } | IrExprKind::List(payloads) => {
            for value in payloads {
                scan_expr_tail_recursion(value, function, false, facts);
            }
        }
        IrExprKind::Record(fields) => {
            for field in fields {
                scan_record_field_tail_recursion(field, function, facts);
            }
        }
        IrExprKind::Dict(entries) => {
            for entry in entries {
                scan_expr_tail_recursion(&entry.key, function, false, facts);
                scan_expr_tail_recursion(&entry.value, function, false, facts);
            }
        }
        IrExprKind::Binary { left, right, .. } => {
            scan_expr_tail_recursion(left, function, false, facts);
            scan_expr_tail_recursion(right, function, false, facts);
        }
        IrExprKind::Local(_)
        | IrExprKind::BoolLiteral(_)
        | IrExprKind::StringLiteral(_)
        | IrExprKind::IntLiteral(_)
        | IrExprKind::FloatLiteral(_)
        | IrExprKind::Unit
        | IrExprKind::FunctionValue(_)
        | IrExprKind::OptionNone
        | IrExprKind::ListNil => {}
    }
}

fn scan_record_field_tail_recursion(
    field: &IrRecordField,
    function: &str,
    facts: &mut TailRecursionFacts,
) {
    scan_expr_tail_recursion(&field.value, function, false, facts);
}

#[derive(Clone)]
pub(super) enum ValueRef {
    Local(u16),
    RecordField {
        base: Box<ValueRef>,
        field: String,
        runtime: String,
    },
    RuntimeUnary {
        base: Box<ValueRef>,
        method: String,
        runtime: String,
    },
    AdtPayload {
        base: Box<ValueRef>,
        index: usize,
        runtime: String,
    },
}

impl ValueRef {
    pub(super) fn emit_load(&self, code: &mut MethodCode) {
        match self {
            Self::Local(slot) => code.aload(*slot),
            Self::RecordField {
                base,
                field,
                runtime,
            } => {
                base.emit_load(code);
                code.ldc_string(field);
                code.invokestatic(
                    runtime,
                    "recordField",
                    "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
                );
            }
            Self::RuntimeUnary {
                base,
                method,
                runtime,
            } => {
                base.emit_load(code);
                code.invokestatic(runtime, method, "(Ljava/lang/Object;)Ljava/lang/Object;");
            }
            Self::AdtPayload {
                base,
                index,
                runtime,
            } => {
                base.emit_load(code);
                code.push_i32(*index as i32);
                code.invokestatic(
                    runtime,
                    "adtPayload",
                    "(Ljava/lang/Object;I)Ljava/lang/Object;",
                );
            }
        }
    }
}

pub(super) fn object_method_descriptor(arg_count: usize) -> String {
    let mut descriptor = "(".to_string();
    for _ in 0..arg_count {
        descriptor.push_str("Ljava/lang/Object;");
    }
    descriptor.push_str(")Ljava/lang/Object;");
    descriptor
}

pub(crate) fn split_contract_binary<'a>(text: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut split = None;
    while index + op.len() <= text.len() {
        let ch = text[index..].chars().next()?;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && contract_operator_at(bytes, index, op.as_bytes()) => {
                let left = text[..index].trim();
                let right = text[index + op.len()..].trim();
                if !left.is_empty() && !right.is_empty() {
                    split = Some((left, right));
                }
            }
            _ => {}
        }
        index += ch.len_utf8();
    }
    split
}

fn contract_operator_at(text: &[u8], index: usize, operator: &[u8]) -> bool {
    if !text[index..].starts_with(operator) {
        return false;
    }
    let previous = index
        .checked_sub(1)
        .and_then(|index| text.get(index))
        .copied();
    let next = text.get(index + operator.len()).copied();
    match operator {
        b">" => previous != Some(b'>') && !matches!(next, Some(b'>' | b'=')),
        b">>" | b">>>" => previous != Some(b'>') && next != Some(b'>'),
        b"<" => previous != Some(b'<') && !matches!(next, Some(b'<' | b'=')),
        b"<<" => previous != Some(b'<') && next != Some(b'<'),
        b"|" => next != Some(b'>'),
        _ => true,
    }
}

pub(super) fn strip_contract_outer_parens(mut text: &str) -> &str {
    loop {
        let Some(inner) = text
            .strip_prefix('(')
            .and_then(|text| text.strip_suffix(')'))
        else {
            return text;
        };
        let mut depth = 0usize;
        let mut closes_at_end = false;
        for (index, ch) in text.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        closes_at_end = index + ch.len_utf8() == text.len();
                        if !closes_at_end {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        if !closes_at_end {
            return text;
        }
        text = inner.trim();
    }
}

pub(super) fn parse_contract_call(text: &str) -> Option<(&str, Vec<&str>)> {
    let open = text.find('(')?;
    if !text.ends_with(')') {
        return None;
    }
    let callee = text[..open].trim();
    if callee.is_empty()
        || !callee
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == ':')
    {
        return None;
    }
    let inner = &text[open + 1..text.len() - 1];
    let args = split_contract_args(inner);
    Some((callee, args))
}

fn split_contract_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut start = 0usize;
    let mut index = 0usize;
    while index < text.len() {
        let ch = text[index..]
            .chars()
            .next()
            .expect("index should stay on a character boundary");
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = text[start..index].trim();
                if !arg.is_empty() {
                    args.push(arg);
                }
                start = index + 1;
            }
            _ => {}
        }
        index += ch.len_utf8();
    }
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}

include!(concat!(env!("OUT_DIR"), "/runtime_classes.rs"));

pub(super) fn runtime_classes() -> Vec<JvmClassFile> {
    RUNTIME_CLASSES
        .iter()
        .map(|(path, contents)| JvmClassFile {
            path: (*path).to_string(),
            contents: contents.to_vec(),
        })
        .collect()
}
