use veln_ast::Expr;

use crate::types::CallOrigin;

pub(crate) fn stdio_signature(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    let [module, name] = segments else {
        return None;
    };
    if module != "stdio" || !matches!(name.as_str(), "print" | "println" | "eprint" | "eprintln") {
        return None;
    }
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol: format!("{module}::{name}"),
        effects: vec!["stdio".to_string()],
    })
}
