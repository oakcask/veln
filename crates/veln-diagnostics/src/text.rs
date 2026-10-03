use std::fmt;
use std::sync::{Arc, OnceLock};

#[derive(Clone)]
pub struct DiagnosticText(Arc<DiagnosticTextInner>);

struct DiagnosticTextInner {
    value: DiagnosticTextValue,
    flattened: OnceLock<Arc<str>>,
}

#[derive(Clone)]
enum DiagnosticTextValue {
    Plain(Arc<str>),
    Parts(Arc<[DiagnosticText]>),
}

impl DiagnosticText {
    pub fn parts(parts: impl IntoIterator<Item = DiagnosticText>) -> Self {
        Self(Arc::new(DiagnosticTextInner {
            value: DiagnosticTextValue::Parts(parts.into_iter().collect()),
            flattened: OnceLock::new(),
        }))
    }

    pub fn write_to(&self, out: &mut String) {
        match &self.0.value {
            DiagnosticTextValue::Plain(value) => out.push_str(value),
            DiagnosticTextValue::Parts(parts) => {
                for part in parts.iter() {
                    part.write_to(out);
                }
            }
        }
    }

    pub fn to_owned_string(&self) -> String {
        self.as_str().to_string()
    }

    pub fn as_str(&self) -> &str {
        match &self.0.value {
            DiagnosticTextValue::Plain(value) => value,
            DiagnosticTextValue::Parts(_) => self
                .0
                .flattened
                .get_or_init(|| {
                    let mut text = String::new();
                    self.write_to(&mut text);
                    Arc::from(text)
                })
                .as_ref(),
        }
    }

    pub fn contains(&self, pattern: &str) -> bool {
        self.as_str().contains(pattern)
    }

    pub fn starts_with(&self, pattern: &str) -> bool {
        self.as_str().starts_with(pattern)
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl From<String> for DiagnosticText {
    fn from(value: String) -> Self {
        Self(Arc::new(DiagnosticTextInner {
            value: DiagnosticTextValue::Plain(value.into()),
            flattened: OnceLock::new(),
        }))
    }
}

impl From<&str> for DiagnosticText {
    fn from(value: &str) -> Self {
        Self(Arc::new(DiagnosticTextInner {
            value: DiagnosticTextValue::Plain(Arc::from(value)),
            flattened: OnceLock::new(),
        }))
    }
}

impl fmt::Display for DiagnosticText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.value {
            DiagnosticTextValue::Plain(value) => formatter.write_str(value),
            DiagnosticTextValue::Parts(parts) => {
                for part in parts.iter() {
                    fmt::Display::fmt(part, formatter)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for DiagnosticText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), formatter)
    }
}

impl PartialEq for DiagnosticText {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for DiagnosticText {}

impl PartialEq<str> for DiagnosticText {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for DiagnosticText {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<String> for DiagnosticText {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}
