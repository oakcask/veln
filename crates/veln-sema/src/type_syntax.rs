use crate::semantic_model::Type;

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
        let effects = if self.eat_keyword("effects") {
            self.expect('[')?;
            let mut effects = Vec::new();
            while !self.at_end() && !self.at(']') {
                let effect = self.parse_effect_entry()?;
                if effect.starts_with("...")
                    && effects
                        .iter()
                        .any(|entry: &String| entry.starts_with("..."))
                {
                    return Err("function type effect set has more than one row tail".to_string());
                }
                effects.push(effect);
                self.skip_ws();
                let has_more = self.eat(',');
                if effects.last().is_some_and(|entry| entry.starts_with("..."))
                    && has_more
                    && !self.at(']')
                {
                    return Err("effect row tail must be the final effect".to_string());
                }
                if !has_more {
                    break;
                }
            }
            self.expect(']')?;
            effects
        } else {
            Vec::new()
        };
        Ok(Type::Function {
            params,
            variadic: variadic.map(Box::new),
            return_type: Box::new(return_type),
            effects,
        })
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
            let ty = if is_variadic {
                self.skip_ws();
                if self.at(')') || self.at(',') {
                    Type::Unknown
                } else {
                    let ty = self
                        .parse_type()
                        .map_err(|_| "expected type after variadic marker".to_string())?;
                    match ty {
                        Type::Named { name, args } if name == "unknown" && args.is_empty() => {
                            Type::Unknown
                        }
                        ty => ty,
                    }
                }
            } else {
                self.parse_type()?
            };
            self.skip_ws();
            let has_more = self.eat(',');
            if is_variadic {
                if variadic.is_some() {
                    return Err("function type has more than one variadic parameter".to_string());
                }
                if has_more {
                    return Err(
                        "variadic function type parameter must be the final parameter".to_string(),
                    );
                }
                variadic = Some(ty);
            } else {
                params.push(ty);
            }
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
        let expected_arity = match name.as_str() {
            "Bool" | "Int" | "Float" | "String" | "Unit" => Some(0),
            "Option" | "Vec" => Some(1),
            "Result" | "Dict" => Some(2),
            _ => None,
        };
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
