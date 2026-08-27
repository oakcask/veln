use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceLessNameClass {
    Module,
    Function,
    Type,
    Constructor,
}

impl SourceLessNameClass {
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

    fn accepts(self, name: &str) -> bool {
        let Some(initial) = name.as_bytes().first() else {
            return false;
        };
        match self {
            Self::Module | Self::Function => initial.is_ascii_lowercase(),
            Self::Type | Self::Constructor => initial.is_ascii_uppercase(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InvalidStandardSymbolCase {
    pub(crate) provider: &'static str,
    pub(crate) name: String,
    pub(crate) name_class: SourceLessNameClass,
    pub(crate) reason: InvalidStandardSymbolReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InvalidStandardSymbolReason {
    InvalidCase,
    InvalidLookupClass,
    InvalidLookupKey,
    DuplicateLookupKey,
}

impl InvalidStandardSymbolCase {
    pub(crate) fn code(&self) -> &'static str {
        "toolchain.invalid_symbol_case"
    }

    pub(crate) fn required_initial(&self) -> &'static str {
        self.name_class.required_initial()
    }

    pub(crate) fn diagnostic(&self) -> Diagnostic {
        let message = match self.reason {
            InvalidStandardSymbolReason::InvalidCase => format!(
                "compiler-provided {} `{}` from `{}` must start with an {} letter",
                self.name_class.as_str(),
                self.name,
                self.provider,
                self.name_class.required_initial_description()
            ),
            InvalidStandardSymbolReason::InvalidLookupClass => format!(
                "compiler-provided {} lookup descriptor `{}` from `{}` declares a non-{} name class",
                self.name_class.as_str(),
                self.name,
                self.provider,
                self.name_class.as_str()
            ),
            InvalidStandardSymbolReason::InvalidLookupKey => format!(
                "compiler-provided {} `{}` from `{}` has an invalid source lookup key",
                self.name_class.as_str(),
                self.name,
                self.provider
            ),
            InvalidStandardSymbolReason::DuplicateLookupKey => format!(
                "compiler-provided {} lookup key `{}` from `{}` is duplicated",
                self.name_class.as_str(),
                self.name,
                self.provider
            ),
        };
        Diagnostic::new(
            self.code(),
            Severity::Error,
            DiagnosticKind::Toolchain,
            message,
            None,
            JsonValue::object([
                ("provider", JsonValue::string(self.provider)),
                ("name", JsonValue::string(self.name.clone())),
                ("name_class", JsonValue::string(self.name_class.as_str())),
                (
                    "required_initial",
                    JsonValue::string(self.required_initial()),
                ),
            ]),
        )
    }
}

pub(crate) fn validate_source_less_name(
    provider: &'static str,
    name: &str,
    name_class: SourceLessNameClass,
) -> Result<(), InvalidStandardSymbolCase> {
    if name_class.accepts(name) {
        Ok(())
    } else {
        Err(InvalidStandardSymbolCase {
            provider,
            name: name.to_string(),
            name_class,
            reason: InvalidStandardSymbolReason::InvalidCase,
        })
    }
}
