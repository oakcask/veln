use super::*;

fn standard_library_builtin_calls(function: &veln_ir::IrFunction) -> Vec<(&'static str, &str)> {
    function
        .body
        .iter()
        .map(|statement| {
            let (statement_kind, value) = match &statement.kind {
                IrStmtKind::Let { value, .. } => ("let", value),
                IrStmtKind::Expr { value } => ("expr", value),
                IrStmtKind::Return { value } => ("return", value),
            };
            let IrExprKind::Call {
                target: IrCallTarget::StandardLibraryBuiltin(symbol),
                ..
            } = &value.kind
            else {
                panic!("expected standard-library builtin call, found {value:#?}");
            };
            (statement_kind, symbol.as_str())
        })
        .collect()
}

mod adapter_arguments;
mod concurrency;
mod filesystem_and_process;
mod lowering_and_types;
mod network;
mod time;
