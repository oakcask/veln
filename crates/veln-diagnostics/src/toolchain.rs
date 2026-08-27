use crate::{Diagnostic, DiagnosticKind, JsonValue, Severity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolchainSymbolNameClass {
    Module,
    Function,
    Type,
    Constructor,
}

impl ToolchainSymbolNameClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Function => "function",
            Self::Type => "type",
            Self::Constructor => "constructor",
        }
    }

    fn required_initial(self) -> &'static str {
        match self {
            Self::Module | Self::Function => "ascii_lowercase",
            Self::Type | Self::Constructor => "ascii_uppercase",
        }
    }

    fn required_initial_description(self) -> &'static str {
        match self {
            Self::Module | Self::Function => "ASCII lowercase",
            Self::Type | Self::Constructor => "ASCII uppercase",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolchainSymbolNameFailureReason {
    InvalidCase,
    InvalidLookupClass,
    InvalidLookupKey,
    DuplicateLookupKey,
}

pub fn toolchain_invalid_symbol_case_diagnostic(
    provider: &'static str,
    name: impl Into<String>,
    name_class: ToolchainSymbolNameClass,
    reason: ToolchainSymbolNameFailureReason,
) -> Diagnostic {
    let name = name.into();
    let message = match reason {
        ToolchainSymbolNameFailureReason::InvalidCase => format!(
            "compiler-provided {} `{}` from `{}` must start with an {} letter",
            name_class.as_str(),
            name,
            provider,
            name_class.required_initial_description()
        ),
        ToolchainSymbolNameFailureReason::InvalidLookupClass => format!(
            "compiler-provided {} lookup descriptor `{}` from `{}` declares a non-{} name class",
            name_class.as_str(),
            name,
            provider,
            name_class.as_str()
        ),
        ToolchainSymbolNameFailureReason::InvalidLookupKey => format!(
            "compiler-provided {} `{}` from `{}` has an invalid source lookup key",
            name_class.as_str(),
            name,
            provider
        ),
        ToolchainSymbolNameFailureReason::DuplicateLookupKey => format!(
            "compiler-provided {} lookup key `{}` from `{}` is duplicated",
            name_class.as_str(),
            name,
            provider
        ),
    };
    Diagnostic::new(
        "toolchain.invalid_symbol_case",
        Severity::Error,
        DiagnosticKind::Toolchain,
        message,
        None,
        JsonValue::object([
            ("provider", JsonValue::string(provider)),
            ("name", JsonValue::string(name)),
            ("name_class", JsonValue::string(name_class.as_str())),
            (
                "required_initial",
                JsonValue::string(name_class.required_initial()),
            ),
        ]),
    )
}
