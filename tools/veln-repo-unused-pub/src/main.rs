use std::path::PathBuf;

use serde_json::json;

fn main() {
    if let Err(error) = run() {
        eprintln!("unused-public-item scan failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut roots = Vec::new();
    let mut json_output = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "usage: veln-repo-unused-pub [--format human|json] [ROOT ...]\n\
                    Reports suspected dead public Rust items; findings do not fail the command.\n\
                    Defaults to crates tools. References are matched by name across all roots.\n\
                    Review cfg, macros, external callers, and same-name collisions before removing code."
                );
                return Ok(());
            }
            "--format" => match args.next().as_deref() {
                Some("json") => json_output = true,
                Some("human") => json_output = false,
                _ => return Err("--format requires human or json".into()),
            },
            _ if arg.starts_with('-') => return Err(format!("unknown option {arg}")),
            _ => roots.push(PathBuf::from(arg)),
        }
    }
    if roots.is_empty() {
        roots = vec!["crates".into(), "tools".into()];
    }
    let report = veln_repo_unused_pub::scan(&roots)?;
    if json_output {
        let findings: Vec<_> = report
            .findings
            .iter()
            .map(|finding| {
                json!({
                    "file": finding.file, "line": finding.line, "kind": finding.kind,
                    "name": finding.name, "evidence": finding.evidence,
                })
            })
            .collect();
        println!(
            "{}",
            json!({
                "schema_version": 1, "status": "advisory", "files": report.files,
                "public_definitions": report.definitions, "findings": findings,
                "limitations": ["name matching without Rust name resolution or reachability",
                    "imports and macro tokens count as references; same names can hide unused items",
                "cfg is not evaluated; only cfg(test), test attributes and test file paths mark test references",
                "cfg(test) on out-of-line module declarations is not propagated to their files",
                    "no macro expansion, string interpolation, external callers, or generated code analysis",
                    "fields, enum variants, foreign items, associated constants and types are not candidates"]
            })
        );
    } else {
        println!(
            "Suspected dead public items: {} ({} public definitions, {} Rust files)",
            report.findings.len(),
            report.definitions,
            report.files
        );
        for finding in &report.findings {
            println!(
                "{}:{}: {} {} [{}]",
                finding.file.display(),
                finding.line,
                finding.kind,
                finding.name,
                finding.evidence
            );
        }
        println!(
            "Review references, cfg, macros, and external callers before removing code.\n\
            This is an advisory name-based scan; imports and same-name items can hide unused code."
        );
    }
    Ok(())
}
