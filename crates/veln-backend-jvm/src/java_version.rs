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
    let version = output.split_whitespace().find_map(|part| {
        let candidate = part.trim_matches(|ch| ch == '"' || ch == '\'');
        candidate
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
            .then_some(candidate)
    })?;
    let mut components = version.split(['.', '-']);
    let first = components.next()?.parse::<u32>().ok()?;
    if first == 1 {
        components.next()?.parse::<u32>().ok()
    } else {
        Some(first)
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
    fn rejects_output_without_a_java_version() {
        assert_eq!(parse_java_major("javac version unavailable"), None);
        assert_eq!(parse_java_major(""), None);
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
