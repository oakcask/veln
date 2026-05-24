use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, ExitStatus, Output};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) enum JavaRunResult {
    Ran(Output),
    ToolError(String),
}

pub(crate) fn compile_and_run_java(
    build_dir: &Path,
    java: &veln_backend_jvm::JavaProgram,
) -> Result<ExitCode, String> {
    let result = compile_and_run_java_capture(build_dir, java, "veln run")?;
    let output = match result {
        JavaRunResult::Ran(output) => output,
        JavaRunResult::ToolError(message) => {
            eprintln!("{message}");
            return Ok(ExitCode::from(1));
        }
    };
    forward_process_output(&output)?;
    Ok(exit_code_from_status(output.status))
}

pub(crate) fn compile_and_run_java_capture(
    build_dir: &Path,
    java: &veln_backend_jvm::JavaProgram,
    command_name: &str,
) -> Result<JavaRunResult, String> {
    compile_and_run_java_capture_with_env(build_dir, java, command_name, &[])
}

pub(crate) fn compile_and_run_java_capture_with_env(
    build_dir: &Path,
    java: &veln_backend_jvm::JavaProgram,
    command_name: &str,
    java_env: &[(&str, &OsStr)],
) -> Result<JavaRunResult, String> {
    for source in &java.sources {
        fs::write(build_dir.join(&source.path), &source.contents)
            .map_err(|error| error.to_string())?;
    }

    let javac_output = ProcessCommand::new("javac")
        .args(java.sources.iter().map(|source| source.path.as_str()))
        .current_dir(build_dir)
        .output();
    let javac_output = match javac_output {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(JavaRunResult::ToolError(format!(
                "veln: `javac` was not found; install a JDK to use `{command_name}`"
            )));
        }
        Err(error) => return Err(error.to_string()),
    };
    if !javac_output.status.success() {
        return Ok(JavaRunResult::ToolError(format!(
            "veln: javac failed with status {}",
            javac_output.status
        )));
    }

    let mut command = ProcessCommand::new("java");
    command.arg("-cp").arg(build_dir).arg("VelnEntry");
    for (name, value) in java_env {
        command.env(name, value);
    }
    let java_output = command.output();
    let java_output = match java_output {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(JavaRunResult::ToolError(format!(
                "veln: `java` was not found; install a JDK to use `{command_name}`"
            )));
        }
        Err(error) => return Err(error.to_string()),
    };
    Ok(JavaRunResult::Ran(java_output))
}

pub(crate) fn create_build_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    for attempt in 0..100 {
        let candidate = if attempt == 0 {
            base.clone()
        } else {
            env::temp_dir().join(format!("{prefix}-{}-{nanos}-{attempt}", std::process::id()))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate build directory",
    ))
}

fn forward_process_output(output: &Output) -> Result<(), String> {
    io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| error.to_string())?;
    io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn exit_code_from_status(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        Some(_) | None => ExitCode::from(1),
    }
}
