pub const REQUIRED_JAVA_MAJOR: u32 = 25;

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

pub fn parse_java_major(output: &str) -> Option<u32> {
    let mut detected = None;
    for line in output.lines() {
        let line = line.trim_start();
        let version = if let Some(rest) = strip_words(line, &["javac"]) {
            Some(unquoted_version(rest))
        } else if let Some(rest) = strip_words(line, &["java", "version"]) {
            Some(quoted_version(rest))
        } else {
            strip_words(line, &["openjdk", "version"]).map(quoted_version)
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
    if version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-+".contains(&byte))
    {
        return None;
    }
    let first_end = version
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(version.len());
    let first = version[..first_end].parse::<u32>().ok()?;
    let suffix = &version[first_end..];
    if !suffix.is_empty() {
        let mut previous_was_separator = false;
        for ch in suffix.chars() {
            let is_separator = matches!(ch, '.' | '_' | '-' | '+');
            if is_separator && previous_was_separator {
                return None;
            }
            previous_was_separator = is_separator;
        }
        if previous_was_separator {
            return None;
        }
    }
    if first == 1 {
        let legacy = suffix.strip_prefix('.')?;
        let component_end = legacy
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(legacy.len());
        legacy[..component_end].parse::<u32>().ok()
    } else {
        suffix
            .is_empty()
            .then_some(first)
            .or_else(|| suffix.starts_with(['.', '-', '+']).then_some(first))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_legacy_and_early_access_version_output() {
        assert_eq!(parse_java_major("javac 25.0.1"), Some(25));
        assert_eq!(
            parse_java_major("openjdk version \"26.0.1\" 2027-04-20"),
            Some(26)
        );
        assert_eq!(parse_java_major("java version \"1.8.0_472\""), Some(8));
        assert_eq!(parse_java_major("openjdk version \"25-ea\""), Some(25));
    }

    #[test]
    fn ignores_leading_wrapper_output_with_unrelated_numbers() {
        assert_eq!(
            parse_java_major("notice 25\nopenjdk version \"24.0.2\" 2025-07-15"),
            Some(24)
        );
        assert_eq!(
            parse_java_major("notice 24\nopenjdk version \"25.0.1\" 2025-10-21"),
            Some(25)
        );
    }

    #[test]
    fn rejects_output_without_a_recognized_java_version_line() {
        assert_eq!(parse_java_major("notice 25"), None);
        assert_eq!(parse_java_major("javac version unavailable"), None);
        assert_eq!(parse_java_major(""), None);
    }

    #[test]
    fn rejects_malformed_or_conflicting_recognized_version_lines() {
        assert_eq!(parse_java_major("javac"), None);
        assert_eq!(parse_java_major("openjdk version 25.0.1"), None);
        assert_eq!(parse_java_major("java version \"25.0.1"), None);
        assert_eq!(parse_java_major("javac 25."), None);
        assert_eq!(
            parse_java_major("javac 25.0.1\nopenjdk version \"24.0.2\""),
            None
        );
        assert_eq!(
            parse_java_major("java version \"25.0.1\"\njavac unavailable"),
            None
        );
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
