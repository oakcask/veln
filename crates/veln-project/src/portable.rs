use std::error::Error;
use std::fmt;

use unicode_normalization::UnicodeNormalization;

/// Unicode data version used by portable package identities and source paths.
pub const PORTABLE_UNICODE_VERSION: (u8, u8, u8) = unicode_normalization::UNICODE_VERSION;

/// Schema-ready spelling of [`PORTABLE_UNICODE_VERSION`].
pub const PORTABLE_UNICODE_VERSION_STRING: &str = "17.0.0";

/// A validated transport-independent package identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageIdentity(String);

impl PackageIdentity {
    /// Validates an ordinary root-package name or dependency-table key.
    pub fn new(identity: impl Into<String>) -> Result<Self, PackageIdentityError> {
        let identity = identity.into();
        validate_package_identity(&identity)?;
        if identity == "std" {
            return Err(PackageIdentityError::ReservedStandard);
        }
        Ok(Self(identity))
    }

    /// Returns the identity reserved for the embedded standard package.
    pub fn embedded_standard() -> Self {
        Self("std".to_string())
    }

    /// Returns the exact validated identity spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PackageIdentity {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for PackageIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A package identity is outside the portable package-snapshot domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageIdentityError {
    Empty,
    TooLong { scalar_count: usize },
    NotNfc,
    EmptySegment { segment_index: usize },
    Whitespace { segment_index: usize },
    ReservedStandard,
}

impl fmt::Display for PackageIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("package identity is empty"),
            Self::TooLong { scalar_count } => write!(
                formatter,
                "package identity has {scalar_count} Unicode scalars; the maximum is 255"
            ),
            Self::NotNfc => formatter.write_str("package identity is not Unicode NFC"),
            Self::EmptySegment { segment_index } => {
                write!(
                    formatter,
                    "package identity segment {segment_index} is empty"
                )
            }
            Self::Whitespace { segment_index } => write!(
                formatter,
                "package identity segment {segment_index} contains whitespace"
            ),
            Self::ReservedStandard => {
                formatter.write_str("package identity `std` is reserved for the embedded package")
            }
        }
    }
}

impl Error for PackageIdentityError {}

fn validate_package_identity(identity: &str) -> Result<(), PackageIdentityError> {
    let scalar_count = identity.chars().count();
    if scalar_count == 0 {
        return Err(PackageIdentityError::Empty);
    }
    if scalar_count > 255 {
        return Err(PackageIdentityError::TooLong { scalar_count });
    }
    if !is_nfc(identity) {
        return Err(PackageIdentityError::NotNfc);
    }
    for (segment_index, segment) in identity.split('/').enumerate() {
        if segment.is_empty() {
            return Err(PackageIdentityError::EmptySegment { segment_index });
        }
        if segment.chars().any(char::is_whitespace) {
            return Err(PackageIdentityError::Whitespace { segment_index });
        }
    }
    Ok(())
}

/// A UTF-8 source path is outside the portable package-snapshot domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortableSourcePathError {
    NotNfc,
    EmptySegment {
        segment_index: usize,
    },
    DotSegment {
        segment_index: usize,
    },
    Control {
        segment_index: usize,
        character: char,
    },
    ForbiddenCharacter {
        segment_index: usize,
        character: char,
    },
    TrailingSpace {
        segment_index: usize,
    },
    TrailingDot {
        segment_index: usize,
    },
    ReservedDevice {
        segment_index: usize,
    },
}

impl fmt::Display for PortableSourcePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotNfc => formatter.write_str("source path is not Unicode NFC"),
            Self::EmptySegment { segment_index } => {
                write!(formatter, "source path segment {segment_index} is empty")
            }
            Self::DotSegment { segment_index } => write!(
                formatter,
                "source path segment {segment_index} is `.` or `..`"
            ),
            Self::Control {
                segment_index,
                character,
            } => write!(
                formatter,
                "source path segment {segment_index} contains control U+{:04X}",
                *character as u32
            ),
            Self::ForbiddenCharacter {
                segment_index,
                character,
            } => write!(
                formatter,
                "source path segment {segment_index} contains forbidden `{character}`"
            ),
            Self::TrailingSpace { segment_index } => {
                write!(
                    formatter,
                    "source path segment {segment_index} ends in a space"
                )
            }
            Self::TrailingDot { segment_index } => {
                write!(
                    formatter,
                    "source path segment {segment_index} ends in a dot"
                )
            }
            Self::ReservedDevice { segment_index } => write!(
                formatter,
                "source path segment {segment_index} uses a platform-reserved device name"
            ),
        }
    }
}

impl Error for PortableSourcePathError {}

pub(crate) fn validate_source_path(path: &str) -> Result<(), PortableSourcePathError> {
    if !is_nfc(path) {
        return Err(PortableSourcePathError::NotNfc);
    }
    for (segment_index, segment) in path.split('/').enumerate() {
        if segment.is_empty() {
            return Err(PortableSourcePathError::EmptySegment { segment_index });
        }
        if matches!(segment, "." | "..") {
            return Err(PortableSourcePathError::DotSegment { segment_index });
        }
        if let Some(character) = segment.chars().find(|character| character.is_control()) {
            return Err(PortableSourcePathError::Control {
                segment_index,
                character,
            });
        }
        if let Some(character) = segment
            .chars()
            .find(|character| matches!(character, '\\' | ':'))
        {
            return Err(PortableSourcePathError::ForbiddenCharacter {
                segment_index,
                character,
            });
        }
        if segment.ends_with(' ') {
            return Err(PortableSourcePathError::TrailingSpace { segment_index });
        }
        if segment.ends_with('.') {
            return Err(PortableSourcePathError::TrailingDot { segment_index });
        }
        if is_reserved_device(segment) {
            return Err(PortableSourcePathError::ReservedDevice { segment_index });
        }
    }
    Ok(())
}

pub(crate) fn default_case_fold(path: &str) -> String {
    unicase::UniCase::unicode(path).to_folded_case()
}

fn is_nfc(value: &str) -> bool {
    value.nfc().eq(value.chars())
}

fn is_reserved_device(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or(segment);
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || reserved_numbered_device(&upper, "COM")
        || reserved_numbered_device(&upper, "LPT")
}

fn reserved_numbered_device(stem: &str, prefix: &str) -> bool {
    let Some(number) = stem.strip_prefix(prefix) else {
        return false;
    };
    matches!(
        number,
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_identity_enforces_scalar_and_segment_boundaries() {
        let max = "λ".repeat(255);
        assert_eq!(PackageIdentity::new(&max).unwrap().as_str(), max);
        assert_eq!(PackageIdentity::new("x"), Ok(PackageIdentity("x".into())));
        assert_eq!(PackageIdentity::new(""), Err(PackageIdentityError::Empty));
        assert_eq!(
            PackageIdentity::new("x".repeat(256)),
            Err(PackageIdentityError::TooLong { scalar_count: 256 })
        );
        assert_eq!(
            PackageIdentity::new("owner//package"),
            Err(PackageIdentityError::EmptySegment { segment_index: 1 })
        );
        assert_eq!(
            PackageIdentity::new("/package"),
            Err(PackageIdentityError::EmptySegment { segment_index: 0 })
        );
        assert_eq!(
            PackageIdentity::new("owner/"),
            Err(PackageIdentityError::EmptySegment { segment_index: 1 })
        );
        assert_eq!(
            PackageIdentity::new("owner/package "),
            Err(PackageIdentityError::Whitespace { segment_index: 1 })
        );
        assert_eq!(
            PackageIdentity::new("owner/pack\u{2003}age"),
            Err(PackageIdentityError::Whitespace { segment_index: 1 })
        );
    }

    #[test]
    fn package_identity_rejects_non_nfc_and_reserves_standard_identity() {
        assert_eq!(
            PackageIdentity::new("cafe\u{301}"),
            Err(PackageIdentityError::NotNfc)
        );
        assert_eq!(
            PackageIdentity::new("std"),
            Err(PackageIdentityError::ReservedStandard)
        );
        assert_eq!(PackageIdentity::embedded_standard().as_str(), "std");
    }

    #[test]
    fn source_path_validation_covers_portable_segment_rules() {
        let cases = [
            (
                "",
                PortableSourcePathError::EmptySegment { segment_index: 0 },
            ),
            (
                "a//b.veln",
                PortableSourcePathError::EmptySegment { segment_index: 1 },
            ),
            (
                "./a.veln",
                PortableSourcePathError::DotSegment { segment_index: 0 },
            ),
            (
                "a/../b.veln",
                PortableSourcePathError::DotSegment { segment_index: 1 },
            ),
            (
                "a/line\nb.veln",
                PortableSourcePathError::Control {
                    segment_index: 1,
                    character: '\n',
                },
            ),
            (
                "a/nul\0b.veln",
                PortableSourcePathError::Control {
                    segment_index: 1,
                    character: '\0',
                },
            ),
            (
                "a\\b.veln",
                PortableSourcePathError::ForbiddenCharacter {
                    segment_index: 0,
                    character: '\\',
                },
            ),
            (
                "a:b.veln",
                PortableSourcePathError::ForbiddenCharacter {
                    segment_index: 0,
                    character: ':',
                },
            ),
            (
                "a /b.veln",
                PortableSourcePathError::TrailingSpace { segment_index: 0 },
            ),
            (
                "a./b.veln",
                PortableSourcePathError::TrailingDot { segment_index: 0 },
            ),
            (
                "nul.veln",
                PortableSourcePathError::ReservedDevice { segment_index: 0 },
            ),
            (
                "COM1.txt",
                PortableSourcePathError::ReservedDevice { segment_index: 0 },
            ),
            (
                "lpt\u{b2}.veln",
                PortableSourcePathError::ReservedDevice { segment_index: 0 },
            ),
        ];
        for (path, expected) in cases {
            assert_eq!(validate_source_path(path), Err(expected), "{path:?}");
        }
        assert_eq!(validate_source_path("src/caf\u{e9}.veln"), Ok(()));
        assert_eq!(
            validate_source_path("src/cafe\u{301}.veln"),
            Err(PortableSourcePathError::NotNfc)
        );
    }

    #[test]
    fn unicode_contract_uses_full_default_case_folding() {
        assert_eq!(PORTABLE_UNICODE_VERSION, (17, 0, 0));
        assert_eq!(PORTABLE_UNICODE_VERSION_STRING, "17.0.0");
        assert_eq!(default_case_fold("Straße.veln"), "strasse.veln");
        assert_eq!(default_case_fold("στιγμας.veln"), "στιγμασ.veln");
    }
}
