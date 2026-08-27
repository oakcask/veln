use veln_diagnostics::{
    Diagnostic, ToolchainSymbolNameClass, ToolchainSymbolNameFailureReason,
    toolchain_invalid_symbol_case_diagnostic,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SourceLessNameClass {
    Module,
    Function,
    Type,
    Constructor,
}

impl SourceLessNameClass {
    fn toolchain_class(self) -> ToolchainSymbolNameClass {
        match self {
            Self::Module => ToolchainSymbolNameClass::Module,
            Self::Function => ToolchainSymbolNameClass::Function,
            Self::Type => ToolchainSymbolNameClass::Type,
            Self::Constructor => ToolchainSymbolNameClass::Constructor,
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
    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        "toolchain.invalid_symbol_case"
    }

    #[cfg(test)]
    pub(crate) fn required_initial(&self) -> &'static str {
        match self.name_class {
            SourceLessNameClass::Module | SourceLessNameClass::Function => "ascii_lowercase",
            SourceLessNameClass::Type | SourceLessNameClass::Constructor => "ascii_uppercase",
        }
    }

    pub(crate) fn diagnostic(&self) -> Diagnostic {
        toolchain_invalid_symbol_case_diagnostic(
            self.provider,
            self.name.clone(),
            self.name_class.toolchain_class(),
            self.reason.toolchain_reason(),
        )
    }
}

impl InvalidStandardSymbolReason {
    fn toolchain_reason(self) -> ToolchainSymbolNameFailureReason {
        match self {
            Self::InvalidCase => ToolchainSymbolNameFailureReason::InvalidCase,
            Self::InvalidLookupClass => ToolchainSymbolNameFailureReason::InvalidLookupClass,
            Self::InvalidLookupKey => ToolchainSymbolNameFailureReason::InvalidLookupKey,
            Self::DuplicateLookupKey => ToolchainSymbolNameFailureReason::DuplicateLookupKey,
        }
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
