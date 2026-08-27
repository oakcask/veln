use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::semantic_model::Type;
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

fn with_builtin_type_syntax_registry<R>(
    lookup: impl FnOnce(&BuiltinTypeSyntaxRegistry) -> R,
) -> Result<R, InvalidStandardSymbolCase> {
    #[cfg(test)]
    {
        let mut lookup = Some(lookup);
        if let Some(result) = with_test_builtin_type_syntax_registry(|registry| {
            lookup
                .take()
                .expect("builtin type syntax lookup closure is called once")(registry)
        }) {
            return result;
        }
        let lookup = lookup.expect("builtin type syntax lookup closure has not been called");
        production_builtin_type_syntax_registry().map(lookup)
    }

    #[cfg(not(test))]
    {
        production_builtin_type_syntax_registry().map(lookup)
    }
}

fn production_builtin_type_syntax_registry()
-> Result<&'static BuiltinTypeSyntaxRegistry, InvalidStandardSymbolCase> {
    static REGISTRY: OnceLock<Result<BuiltinTypeSyntaxRegistry, InvalidStandardSymbolCase>> =
        OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            BuiltinTypeSyntaxRegistry::from_validated_source_less_descriptors(
                BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            )
        })
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
thread_local! {
    static TEST_BUILTIN_TYPE_SYNTAX_DESCRIPTORS:
        std::cell::RefCell<Option<&'static [BuiltinTypeSyntaxDescriptor]>> =
            const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn with_builtin_type_syntax_descriptors_for_test<R>(
    descriptors: &'static [BuiltinTypeSyntaxDescriptor],
    test: impl FnOnce() -> R,
) -> R {
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    TEST_BUILTIN_TYPE_SYNTAX_DESCRIPTORS.with(|current| {
        let previous = current.replace(Some(descriptors));
        let result = catch_unwind(AssertUnwindSafe(test));
        current.replace(previous);
        match result {
            Ok(result) => result,
            Err(payload) => resume_unwind(payload),
        }
    })
}

#[cfg(test)]
fn with_test_builtin_type_syntax_registry<R>(
    lookup: impl FnOnce(&BuiltinTypeSyntaxRegistry) -> R,
) -> Option<Result<R, InvalidStandardSymbolCase>> {
    TEST_BUILTIN_TYPE_SYNTAX_DESCRIPTORS.with(|current| {
        current.borrow().map(|descriptors| {
            let registry =
                BuiltinTypeSyntaxRegistry::from_validated_source_less_descriptors(descriptors)?;
            Ok(lookup(&registry))
        })
    })
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

pub fn type_annotation_reference_names(text: &str) -> Result<Vec<String>, String> {
    Ok(type_annotation_reference_paths(text)?
        .into_iter()
        .flatten()
        .collect())
}

pub fn type_annotation_reference_paths(text: &str) -> Result<Vec<Vec<String>>, String> {
    let ty = parse_type_annotation(text)?;
    let mut paths = Vec::new();
    collect_type_reference_paths(&ty, &mut paths);
    Ok(paths)
}

pub(crate) fn parse_type_or_unknown(text: Option<&str>) -> Type {
    text.and_then(|text| parse_type_annotation(text).ok())
        .unwrap_or(Type::Unknown)
}

pub(crate) fn parse_type_annotation(text: &str) -> Result<Type, String> {
    let mut parser = TypeParser::new(text);
    let ty = parser.parse_type()?;
    parser.skip_ws();
    if parser.at_end() {
        Ok(ty)
    } else {
        Err(format!("unexpected `{}`", &parser.text[parser.cursor..]))
    }
}

fn collect_type_reference_paths(ty: &Type, paths: &mut Vec<Vec<String>>) {
    match ty {
        Type::Named { name, args } => {
            paths.push(name.split("::").map(str::to_string).collect());
            for arg in args {
                collect_type_reference_paths(arg, paths);
            }
        }
        Type::Record(fields) => {
            for (_, field_type) in fields {
                collect_type_reference_paths(field_type, paths);
            }
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            for param in params {
                collect_type_reference_paths(param, paths);
            }
            if let Some(variadic) = variadic {
                collect_type_reference_paths(variadic, paths);
            }
            collect_type_reference_paths(return_type, paths);
        }
        Type::Unknown => {}
    }
}

struct TypeParser<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> TypeParser<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, cursor: 0 }
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        self.skip_ws();
        if self.eat('{') {
            return self.parse_record_type();
        }
        if self.eat('(') {
            self.skip_ws();
            if self.eat(')') {
                return Ok(Type::unit());
            }
            return Err("expected `)` for unit type `()`".to_string());
        }
        if self.eat_keyword("fn") {
            return self.parse_function_type();
        }

        let Some(name) = self.parse_ident() else {
            return Err("expected type".to_string());
        };
        self.skip_ws();
        let args = if self.eat('<') {
            let args = self.parse_type_list('>')?;
            self.expect('>')?;
            args
        } else if self.at('(') {
            return Err(format!("unexpected `{}`", &self.text[self.cursor..]));
        } else {
            Vec::new()
        };
        self.validate_named_type(name, args)
    }

    fn parse_record_type(&mut self) -> Result<Type, String> {
        let mut fields = Vec::new();
        while !self.at_end() && !self.at('}') {
            let Some(name) = self.parse_ident() else {
                return Err("expected record field name".to_string());
            };
            if fields
                .iter()
                .any(|(field_name, _): &(String, Type)| field_name == &name)
            {
                return Err(format!("duplicate record field `{name}`"));
            }
            self.expect(':')?;
            let ty = self.parse_type()?;
            fields.push((name, ty));
            self.skip_ws();
            if !self.eat(',') {
                break;
            }
            self.skip_ws();
            if self.at('}') {
                break;
            }
        }
        self.expect('}')?;
        Ok(Type::Record(fields))
    }

    fn parse_function_type(&mut self) -> Result<Type, String> {
        self.expect('(')?;
        let (params, variadic) = self.parse_function_param_type_list()?;
        self.expect(')')?;
        self.skip_ws();
        if !self.eat_str("->") {
            return Err("expected `->` in function type".to_string());
        }
        let return_type = self.parse_type()?;
        let effects = self.parse_function_effect_set()?;
        Ok(Type::Function {
            params,
            variadic: variadic.map(Box::new),
            return_type: Box::new(return_type),
            effects,
        })
    }

    fn parse_function_effect_set(&mut self) -> Result<Vec<String>, String> {
        if !self.eat_keyword("effects") {
            return Ok(Vec::new());
        }
        self.expect('[')?;
        let mut effects = Vec::new();
        while !self.at_end() && !self.at(']') {
            let effect = self.parse_effect_entry()?;
            reject_duplicate_effect_row_tail(&effects, &effect)?;
            effects.push(effect);
            self.skip_ws();
            let has_more = self.eat(',');
            reject_non_final_effect_row_tail(&effects, has_more, self.at(']'))?;
            if !has_more {
                break;
            }
        }
        self.expect(']')?;
        Ok(effects)
    }

    fn parse_effect_entry(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.eat_str("...") {
            let Some(row) = self.parse_ident() else {
                return Err("expected effect row variable".to_string());
            };
            return Ok(format!("...{row}"));
        }
        self.parse_ident()
            .ok_or_else(|| "expected effect name".to_string())
    }

    fn parse_function_param_type_list(&mut self) -> Result<(Vec<Type>, Option<Type>), String> {
        let mut params = Vec::new();
        let mut variadic = None;
        self.skip_ws();
        while !self.at_end() && !self.at(')') {
            let is_variadic = self.eat_str("...");
            let ty = self.parse_function_param_type(is_variadic)?;
            self.skip_ws();
            let has_more = self.eat(',');
            push_function_param(&mut params, &mut variadic, is_variadic, has_more, ty)?;
            self.skip_ws();
            if !has_more {
                break;
            }
            if self.at(')') {
                break;
            }
        }
        Ok((params, variadic))
    }

    fn parse_function_param_type(&mut self, is_variadic: bool) -> Result<Type, String> {
        if !is_variadic {
            return self.parse_type();
        }
        self.skip_ws();
        if self.at(')') || self.at(',') {
            return Ok(Type::Unknown);
        }
        let ty = self
            .parse_type()
            .map_err(|_| "expected type after variadic marker".to_string())?;
        Ok(normalize_variadic_type(ty))
    }

    fn parse_type_list(&mut self, end: char) -> Result<Vec<Type>, String> {
        let mut args = Vec::new();
        self.skip_ws();
        while !self.at_end() && !self.at(end) {
            args.push(self.parse_type()?);
            self.skip_ws();
            if !self.eat(',') {
                break;
            }
            self.skip_ws();
            if self.at(end) {
                break;
            }
        }
        Ok(args)
    }

    fn validate_named_type(&self, name: String, args: Vec<Type>) -> Result<Type, String> {
        let expected_arity = with_builtin_type_syntax_registry(|registry| registry.arity(&name))
            .expect("source-less type syntax registry is valid");
        if let Some(expected) = expected_arity
            && args.len() != expected
        {
            return Err(format!(
                "`{name}` expects {expected} type argument(s), found {}",
                args.len()
            ));
        }
        if name == "Dict" && args.len() == 2 {
            Ok(Type::dict(args[0].clone(), args[1].clone()))
        } else {
            Ok(Type::named(name, args))
        }
    }

    fn parse_ident(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.cursor;
        while let Some(ch) = self.current() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        while self.text[self.cursor..].starts_with("::") {
            self.cursor += 2;
            let segment_start = self.cursor;
            while let Some(ch) = self.current() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    self.cursor += ch.len_utf8();
                } else {
                    break;
                }
            }
            if self.cursor == segment_start {
                self.cursor = start;
                return None;
            }
        }
        (self.cursor > start).then(|| self.text[start..self.cursor].to_string())
    }

    fn skip_ws(&mut self) {
        while self.current().is_some_and(char::is_whitespace) {
            self.cursor += 1;
        }
    }

    fn eat(&mut self, expected: char) -> bool {
        self.skip_ws();
        if self.at(expected) {
            self.cursor += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn at(&self, expected: char) -> bool {
        self.current() == Some(expected)
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.eat(expected) {
            Ok(())
        } else {
            Err(format!("expected `{expected}`"))
        }
    }

    fn eat_keyword(&mut self, keyword: &str) -> bool {
        self.skip_ws();
        if self.text[self.cursor..].starts_with(keyword)
            && self.text[self.cursor + keyword.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
        {
            self.cursor += keyword.len();
            true
        } else {
            false
        }
    }

    fn eat_str(&mut self, expected: &str) -> bool {
        self.skip_ws();
        if self.text[self.cursor..].starts_with(expected) {
            self.cursor += expected.len();
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.text.len()
    }

    fn current(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }
}

fn normalize_variadic_type(ty: Type) -> Type {
    match ty {
        Type::Named { name, args } if name == "unknown" && args.is_empty() => Type::Unknown,
        ty => ty,
    }
}

fn push_function_param(
    params: &mut Vec<Type>,
    variadic: &mut Option<Type>,
    is_variadic: bool,
    has_more: bool,
    ty: Type,
) -> Result<(), String> {
    if !is_variadic {
        params.push(ty);
        return Ok(());
    }
    if variadic.is_some() {
        return Err("function type has more than one variadic parameter".to_string());
    }
    if has_more {
        return Err("variadic function type parameter must be the final parameter".to_string());
    }
    *variadic = Some(ty);
    Ok(())
}

fn reject_duplicate_effect_row_tail(effects: &[String], effect: &str) -> Result<(), String> {
    if effect.starts_with("...")
        && effects
            .iter()
            .any(|entry: &String| entry.starts_with("..."))
    {
        Err("function type effect set has more than one row tail".to_string())
    } else {
        Ok(())
    }
}

fn reject_non_final_effect_row_tail(
    effects: &[String],
    has_more: bool,
    at_end: bool,
) -> Result<(), String> {
    if effects.last().is_some_and(|entry| entry.starts_with("...")) && has_more && !at_end {
        Err("effect row tail must be the final effect".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_annotation_reference_names_ignore_record_fields() {
        let names =
            type_annotation_reference_names("{item: Int, payload: module::Packet}").unwrap();

        assert_eq!(names, vec!["Int", "module", "Packet"]);
    }

    #[test]
    fn type_annotation_reference_names_collect_nested_function_types() {
        let names = type_annotation_reference_names(
            "fn({input: Request}, ...stream::Chunk) -> Result<Response, error::AppError>",
        )
        .unwrap();

        assert_eq!(
            names,
            vec![
                "Request", "stream", "Chunk", "Result", "Response", "error", "AppError"
            ]
        );
    }

    #[test]
    fn type_annotation_reference_paths_preserve_qualified_names() {
        let paths = type_annotation_reference_paths(
            "fn({input: Request}, ...stream::Chunk) -> Result<Response, error::AppError>",
        )
        .unwrap();

        assert_eq!(
            paths,
            vec![
                vec!["Request"],
                vec!["stream", "Chunk"],
                vec!["Result"],
                vec!["Response"],
                vec!["error", "AppError"]
            ]
        );
    }
}
