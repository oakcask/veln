#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    Parse,
    Module,
    Name,
    Type,
    Contract,
    Effect,
    Lint,
    Hole,
    Doc,
}

impl DiagnosticKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Module => "module",
            Self::Name => "name",
            Self::Type => "type",
            Self::Contract => "contract",
            Self::Effect => "effect",
            Self::Lint => "lint",
            Self::Hole => "hole",
            Self::Doc => "doc",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    Error,
    Partial,
}

impl CheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Partial => "partial",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_strings_are_stable() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Info.as_str(), "info");
        assert_eq!(Severity::Hint.as_str(), "hint");
    }

    #[test]
    fn diagnostic_kind_strings_are_stable() {
        assert_eq!(DiagnosticKind::Parse.as_str(), "parse");
        assert_eq!(DiagnosticKind::Module.as_str(), "module");
        assert_eq!(DiagnosticKind::Name.as_str(), "name");
        assert_eq!(DiagnosticKind::Type.as_str(), "type");
        assert_eq!(DiagnosticKind::Contract.as_str(), "contract");
        assert_eq!(DiagnosticKind::Effect.as_str(), "effect");
        assert_eq!(DiagnosticKind::Lint.as_str(), "lint");
        assert_eq!(DiagnosticKind::Hole.as_str(), "hole");
        assert_eq!(DiagnosticKind::Doc.as_str(), "doc");
    }

    #[test]
    fn check_status_strings_are_stable() {
        assert_eq!(CheckStatus::Ok.as_str(), "ok");
        assert_eq!(CheckStatus::Error.as_str(), "error");
        assert_eq!(CheckStatus::Partial.as_str(), "partial");
    }
}
