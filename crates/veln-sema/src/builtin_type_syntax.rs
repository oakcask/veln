use std::collections::BTreeSet;

use crate::source_less_names::{
    InvalidStandardSymbolCase, InvalidStandardSymbolReason, SourceLessNameClass,
    validate_source_less_name,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BuiltinTypeSyntaxDescriptor {
    pub(crate) name: &'static str,
    pub(crate) name_class: SourceLessNameClass,
    pub(crate) arity: usize,
}

#[derive(Debug)]
pub(crate) struct BuiltinTypeSyntaxRegistry {
    descriptors: Vec<&'static BuiltinTypeSyntaxDescriptor>,
}

impl BuiltinTypeSyntaxRegistry {
    pub(crate) fn from_validated_source_less_descriptors(
        descriptors: &'static [BuiltinTypeSyntaxDescriptor],
    ) -> Result<Self, InvalidStandardSymbolCase> {
        let mut lookup_keys = BTreeSet::new();
        for descriptor in descriptors {
            if descriptor.name_class != SourceLessNameClass::Type {
                return Err(InvalidStandardSymbolCase {
                    provider: "type_syntax",
                    name: descriptor.name.to_string(),
                    name_class: SourceLessNameClass::Type,
                    reason: InvalidStandardSymbolReason::InvalidLookupClass,
                });
            }
            validate_source_less_name("type_syntax", descriptor.name, descriptor.name_class)?;
            if !lookup_keys.insert(descriptor.name) {
                return Err(InvalidStandardSymbolCase {
                    provider: "type_syntax",
                    name: descriptor.name.to_string(),
                    name_class: SourceLessNameClass::Type,
                    reason: InvalidStandardSymbolReason::DuplicateLookupKey,
                });
            }
        }
        Ok(Self {
            descriptors: descriptors.iter().collect(),
        })
    }

    pub(crate) fn descriptors(&self) -> &[&'static BuiltinTypeSyntaxDescriptor] {
        &self.descriptors
    }

    pub(crate) fn arity(&self, name: &str) -> Option<usize> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.name == name)
            .map(|descriptor| descriptor.arity)
    }
}

pub(crate) const BUILTIN_TYPE_SYNTAX_DESCRIPTORS: &[BuiltinTypeSyntaxDescriptor] = &[
    builtin_type_syntax_descriptor("Bool", 0),
    builtin_type_syntax_descriptor("Int", 0),
    builtin_type_syntax_descriptor("Float", 0),
    builtin_type_syntax_descriptor("String", 0),
    builtin_type_syntax_descriptor("Unit", 0),
    builtin_type_syntax_descriptor("Option", 1),
    builtin_type_syntax_descriptor("Vec", 1),
    builtin_type_syntax_descriptor("Result", 2),
    builtin_type_syntax_descriptor("Dict", 2),
];

const fn builtin_type_syntax_descriptor(
    name: &'static str,
    arity: usize,
) -> BuiltinTypeSyntaxDescriptor {
    BuiltinTypeSyntaxDescriptor {
        name,
        name_class: SourceLessNameClass::Type,
        arity,
    }
}
