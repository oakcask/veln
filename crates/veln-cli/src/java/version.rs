use std::io;
use std::path::Path;
use std::process::Command as ProcessCommand;

use veln_backend_jvm::{JavaTool, REQUIRED_JAVA_MAJOR, parse_java_major, require_java_major};

pub(super) fn java_version_error(java_launcher: &Path, command_name: &str) -> Option<String> {
    let output = match ProcessCommand::new(java_launcher).arg("-version").output() {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Some(missing_java_message(command_name));
        }
        Err(_) => {
            return Some(format!(
                "veln: selected `java` could not report its major version; Java {REQUIRED_JAVA_MAJOR} or newer is required for `{command_name}`"
            ));
        }
    };
    if !output.status.success() {
        return Some(format!(
            "veln: selected `java` could not report its major version; Java {REQUIRED_JAVA_MAJOR} or newer is required for `{command_name}`"
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let Some(detected_major) = parse_java_major(JavaTool::Runtime, &[&stdout, &stderr]) else {
        return Some(format!(
            "veln: could not determine the selected `java` major version; Java {REQUIRED_JAVA_MAJOR} or newer is required for `{command_name}`"
        ));
    };
    if let Err(version) = require_java_major(detected_major) {
        return Some(format!(
            "veln: Java {} or newer is required for `{command_name}`; selected `java` has major version {}",
            version.required, version.detected
        ));
    }
    None
}

pub(super) fn missing_java_message(command_name: &str) -> String {
    format!("veln: `java` was not found; install JDK 25 or newer to use `{command_name}`")
}
