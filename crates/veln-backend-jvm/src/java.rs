use std::collections::BTreeSet;

pub(crate) struct JavaIdentifierAllocator {
    used_names: BTreeSet<String>,
    next_suffix: usize,
    #[cfg(test)]
    allocation_work: usize,
}

impl JavaIdentifierAllocator {
    pub(crate) fn new() -> Self {
        Self {
            used_names: BTreeSet::new(),
            next_suffix: 1,
            #[cfg(test)]
            allocation_work: 0,
        }
    }

    pub(crate) fn allocate(&mut self, base: &str) -> String {
        let mut candidate = normalized_java_identifier(base);
        #[cfg(test)]
        {
            self.allocation_work += 1;
        }
        if self.used_names.insert(candidate.clone()) {
            return candidate;
        }

        let original = candidate;
        loop {
            candidate = format!("{original}_{}", self.next_suffix);
            self.next_suffix += 1;
            #[cfg(test)]
            {
                self.allocation_work += 1;
            }
            if self.used_names.insert(candidate.clone()) {
                return candidate;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn allocation_work(&self) -> usize {
        self.allocation_work
    }
}

pub(crate) fn java_type_identifier(name: &str) -> String {
    let sanitized = sanitize_identifier_text(name);
    if sanitized.is_empty() || is_java_keyword(&sanitized) {
        "VelnGenerated".to_string()
    } else {
        sanitized
    }
}

pub(crate) fn sanitize_identifier_text(text: &str) -> String {
    let mut output = String::new();
    for (index, character) in text.chars().enumerate() {
        let valid = character == '_' || character == '$' || character.is_ascii_alphanumeric();
        if !valid {
            output.push('_');
            continue;
        }
        if index == 0 && character.is_ascii_digit() {
            output.push('_');
        }
        output.push(character);
    }
    output
}

fn normalized_java_identifier(base: &str) -> String {
    let mut candidate = if base.is_empty() || is_java_keyword(base) {
        format!("_{base}")
    } else {
        base.to_string()
    };
    if candidate == "_" {
        candidate = "_value".to_string();
    }
    candidate
}

fn is_java_keyword(value: &str) -> bool {
    java_keywords().contains(&value)
}

fn java_keywords() -> &'static [&'static str] {
    &[
        "abstract",
        "assert",
        "boolean",
        "break",
        "byte",
        "case",
        "catch",
        "char",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "float",
        "for",
        "goto",
        "if",
        "implements",
        "import",
        "instanceof",
        "int",
        "interface",
        "long",
        "native",
        "new",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "short",
        "static",
        "strictfp",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "try",
        "void",
        "volatile",
        "while",
    ]
}

pub(crate) fn veln_string_literal_value(raw: &str) -> String {
    let Some(inner) = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return raw.to_string();
    };
    let mut output = String::new();
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('"') => output.push('"'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}
