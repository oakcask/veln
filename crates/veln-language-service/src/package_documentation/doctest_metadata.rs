use super::*;

pub(super) struct PackageDocVisibleDoctests {
    pub(super) doctests: Vec<PackageDocDoctest>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub(super) struct GeneratedDoctestSource {
    pub(super) source: SourceFile,
    pub(super) line_origins: BTreeMap<usize, DoctestSourceLineOrigin>,
}

#[derive(Clone, Debug)]
pub(super) struct DoctestSourceLineOrigin {
    pub(super) original_span: SourceSpan,
    pub(super) generated_content_column: usize,
}

pub(super) fn visible_doctests_for(
    source: &SourceFile,
    target_line: usize,
) -> PackageDocVisibleDoctests {
    let source = SourceFile::new(
        source.path().as_str(),
        doctest_doc_block_before(source, target_line),
    );
    let extracted = visible_doctests(&source);
    PackageDocVisibleDoctests {
        doctests: extracted
            .doctests
            .into_iter()
            .map(|doctest| PackageDocDoctest {
                kind: "veln".to_string(),
                code: doctest.code,
                expected_error: doctest.expected_error,
                should_fail: doctest.should_fail,
                expected_output: doctest
                    .expected_output
                    .map(expected_outputs)
                    .unwrap_or_default(),
            })
            .collect(),
        diagnostics: extracted.diagnostics,
    }
}

pub(super) fn doctest_doc_block_before(source: &SourceFile, target_line: usize) -> String {
    if target_line <= 1 {
        return String::new();
    }
    let lines = source.text().lines().collect::<Vec<_>>();
    let mut index = target_line - 2;
    let mut docs = Vec::new();
    while let Some(line) = lines.get(index) {
        if line.trim_start().strip_prefix("##").is_some() {
            docs.push(*line);
        } else {
            break;
        }
        if index == 0 {
            break;
        }
        index -= 1;
    }
    docs.reverse();
    if documentation_lines_are_adr_lite(docs.iter().copied()) {
        return String::new();
    }
    docs.join("\n")
}

pub(super) fn expected_outputs(output: ExpectedOutput) -> Vec<PackageDocExpectedOutput> {
    let mut outputs = Vec::new();
    if let Some(stdout) = output.stdout {
        outputs.push(PackageDocExpectedOutput {
            stream: "stdout".to_string(),
            lines: output_lines(&stdout),
        });
    }
    if let Some(stderr) = output.stderr {
        outputs.push(PackageDocExpectedOutput {
            stream: "stderr".to_string(),
            lines: output_lines(&stderr),
        });
    }
    outputs
}

pub(super) fn output_lines(output: &str) -> Vec<String> {
    if output.is_empty() {
        Vec::new()
    } else {
        output.split('\n').map(ToString::to_string).collect()
    }
}
