pub const REQUIRED_JAVA_MAJOR: u32 = 25;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum JavaTool {
    Compiler,
    Runtime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaMajorTooOld {
    pub required: u32,
    pub detected: u32,
}

pub fn require_java_major(detected: u32) -> Result<(), JavaMajorTooOld> {
    if detected < REQUIRED_JAVA_MAJOR {
        Err(JavaMajorTooOld {
            required: REQUIRED_JAVA_MAJOR,
            detected,
        })
    } else {
        Ok(())
    }
}

pub fn parse_java_major(tool: JavaTool, outputs: &[&str]) -> Option<u32> {
    let mut detected = None;
    for output in outputs {
        for line in output.lines() {
            let line = line.trim_start();
            let version = match tool {
                JavaTool::Compiler => strip_words(line, &["javac"]).map(unquoted_version),
                JavaTool::Runtime => strip_words(line, &["java", "version"])
                    .or_else(|| strip_words(line, &["openjdk", "version"]))
                    .map(quoted_version),
            };
            let Some(version) = version else {
                continue;
            };
            let major = parse_version_major(version?)?;
            match detected {
                Some(previous) if previous != major => return None,
                Some(_) => {}
                None => detected = Some(major),
            }
        }
    }
    detected
}

fn strip_words<'a>(mut input: &'a str, words: &[&str]) -> Option<&'a str> {
    for word in words {
        input = input.strip_prefix(word)?;
        if !input.is_empty() && !input.starts_with(char::is_whitespace) {
            return None;
        }
        input = input.trim_start();
    }
    Some(input)
}

fn unquoted_version(input: &str) -> Option<&str> {
    let version = input.split_whitespace().next()?;
    (!matches!(version.chars().next(), Some('"' | '\''))).then_some(version)
}

fn quoted_version(input: &str) -> Option<&str> {
    let input = input.strip_prefix('"')?;
    let closing_quote = input.find('"')?;
    let version = &input[..closing_quote];
    let remainder = &input[closing_quote + 1..];
    (remainder.is_empty() || remainder.starts_with(char::is_whitespace)).then_some(version)
}

fn parse_version_major(version: &str) -> Option<u32> {
    let bytes = version.as_bytes();
    let mut cursor = take_digits(bytes, 0)?;
    let first = version[..cursor].parse::<u32>().ok()?;
    let mut legacy_major = None;

    while cursor < bytes.len() && matches!(bytes[cursor], b'.' | b'_') {
        let separator = bytes[cursor];
        let start = cursor + 1;
        cursor = take_digits(bytes, start)?;
        if first == 1 && separator == b'.' && legacy_major.is_none() {
            legacy_major = Some(version[start..cursor].parse::<u32>().ok()?);
        }
    }
    if cursor < bytes.len() && bytes[cursor] == b'-' {
        cursor = take_alphanumeric(bytes, cursor + 1)?;
    }
    if cursor < bytes.len() && bytes[cursor] == b'+' {
        cursor = take_digits(bytes, cursor + 1)?;
        if cursor < bytes.len() && bytes[cursor] == b'-' {
            cursor = take_alphanumeric(bytes, cursor + 1)?;
        }
    }
    if cursor != bytes.len() {
        return None;
    }
    if first == 1 {
        legacy_major
    } else {
        Some(first)
    }
}

fn take_digits(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;
    while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
    }
    (cursor > start).then_some(cursor)
}

fn take_alphanumeric(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;
    while bytes.get(cursor).is_some_and(u8::is_ascii_alphanumeric) {
        cursor += 1;
    }
    (cursor > start).then_some(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_legacy_and_early_access_version_output() {
        assert_eq!(
            parse_java_major(JavaTool::Compiler, &["javac 25.0.1"]),
            Some(25)
        );
        assert_eq!(
            parse_java_major(JavaTool::Compiler, &["javac 1.8.0_472-b08"]),
            Some(8)
        );
        assert_eq!(
            parse_java_major(JavaTool::Compiler, &["javac 25-ea+1"]),
            Some(25)
        );
        assert_eq!(
            parse_java_major(JavaTool::Runtime, &["openjdk version \"26.0.1\""]),
            Some(26)
        );
        assert_eq!(
            parse_java_major(JavaTool::Runtime, &["java version \"1.8.0_472-b08\""]),
            Some(8)
        );
        assert_eq!(
            parse_java_major(JavaTool::Runtime, &["openjdk version \"25-ea+1\""]),
            Some(25)
        );
        assert_eq!(
            parse_java_major(
                JavaTool::Runtime,
                &["java version \"25.0.1\"", "openjdk version \"25\""]
            ),
            Some(25)
        );
    }

    #[test]
    fn ignores_leading_wrapper_output_with_unrelated_numbers() {
        assert_eq!(
            parse_java_major(
                JavaTool::Runtime,
                &["notice 25\nopenjdk version \"24.0.2\""]
            ),
            Some(24)
        );
        assert_eq!(
            parse_java_major(
                JavaTool::Runtime,
                &["notice 24\nopenjdk version \"25.0.1\""]
            ),
            Some(25)
        );
    }

    #[test]
    fn rejects_output_without_a_recognized_java_version_line() {
        assert_eq!(parse_java_major(JavaTool::Runtime, &["notice 25"]), None);
        assert_eq!(
            parse_java_major(JavaTool::Compiler, &["javac version unavailable"]),
            None
        );
        assert_eq!(parse_java_major(JavaTool::Runtime, &[""]), None);
    }

    #[test]
    fn rejects_malformed_or_conflicting_recognized_version_lines() {
        assert_eq!(parse_java_major(JavaTool::Compiler, &["javac"]), None);
        assert_eq!(
            parse_java_major(JavaTool::Runtime, &["openjdk version 25.0.1"]),
            None
        );
        assert_eq!(
            parse_java_major(JavaTool::Runtime, &["java version \"25.0.1"]),
            None
        );
        assert_eq!(parse_java_major(JavaTool::Compiler, &["javac 25."]), None);
        assert_eq!(
            parse_java_major(JavaTool::Runtime, &["openjdk version \"25.not-a-version\""]),
            None
        );
        assert_eq!(
            parse_java_major(JavaTool::Compiler, &["javac 25.0.1", "javac 24.0.2"]),
            None
        );
        assert_eq!(
            parse_java_major(
                JavaTool::Runtime,
                &["java version \"25.0.1\"", "openjdk version \"24.0.2\""]
            ),
            None
        );
    }

    #[test]
    fn requires_the_invoked_tool_banner() {
        assert_eq!(
            parse_java_major(JavaTool::Compiler, &["openjdk version \"25.0.1\""]),
            None
        );
        assert_eq!(parse_java_major(JavaTool::Runtime, &["javac 25.0.1"]), None);
    }

    #[test]
    fn rejects_older_major_and_accepts_required_or_newer_major() {
        assert_eq!(
            require_java_major(24),
            Err(JavaMajorTooOld {
                required: 25,
                detected: 24
            })
        );
        assert_eq!(require_java_major(25), Ok(()));
        assert_eq!(require_java_major(26), Ok(()));
    }
}
