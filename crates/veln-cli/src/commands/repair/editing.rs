use std::fs;
use std::path::{Path, PathBuf};

use veln_source::LineCol;

use super::candidates::{RepairCandidate, RepairEdit};
use super::outcome::AppliedEdit;

#[derive(Debug)]
pub(super) struct FileEdit {
    pub(super) path: PathBuf,
    pub(super) original: String,
    pub(super) repaired: String,
}

#[derive(Debug)]
pub(super) struct EditPlan {
    pub(super) files: Vec<FileEdit>,
    pub(super) applied_edits: Vec<AppliedEdit>,
}

#[derive(Debug)]
struct ResolvedEdit {
    start: usize,
    end: usize,
    applied: AppliedEdit,
}

struct PendingFileEdit {
    file: String,
    path: PathBuf,
    original: String,
    edits: Vec<ResolvedEdit>,
}

pub(super) fn build_edit_plan(
    root: &Path,
    candidate: &RepairCandidate,
) -> Result<EditPlan, String> {
    if candidate.edits.is_empty() {
        return Err("unsupported edit shape".to_string());
    }

    let mut pending_files = Vec::<PendingFileEdit>::new();
    for edit in &candidate.edits {
        let pending_index = if let Some(index) = pending_files
            .iter()
            .position(|pending| pending.file == edit.file)
        {
            index
        } else {
            let path = source_path(root, &edit.file)?;
            let original = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            pending_files.push(PendingFileEdit {
                file: edit.file.clone(),
                path,
                original,
                edits: Vec::new(),
            });
            pending_files.len() - 1
        };
        let has_explicit_followup = candidate
            .edits
            .iter()
            .any(|other| other.file == edit.file && other.start.offset == edit.end.offset);
        let resolved = resolve_edit_span(
            &pending_files[pending_index].original,
            edit,
            has_explicit_followup,
        )?;
        pending_files[pending_index].edits.push(resolved);
    }

    let mut files = Vec::new();
    let mut applied_edits = Vec::new();
    for mut pending in pending_files {
        pending.edits.sort_by_key(|edit| edit.start);
        for pair in pending.edits.windows(2) {
            if pair[0].end > pair[1].start {
                return Err("repair edits overlap".to_string());
            }
        }

        let mut repaired = pending.original.clone();
        for edit in pending.edits.iter().rev() {
            repaired.replace_range(edit.start..edit.end, &edit.applied.edit.replacement);
        }
        applied_edits.extend(pending.edits.into_iter().map(|edit| edit.applied));
        files.push(FileEdit {
            path: pending.path,
            original: pending.original,
            repaired,
        });
    }

    Ok(EditPlan {
        files,
        applied_edits,
    })
}

fn source_path(root: &Path, file: &str) -> Result<PathBuf, String> {
    let path = Path::new(file);
    if path.is_absolute() {
        return Err("repair target must be project-relative".to_string());
    }
    Ok(root.join(path))
}

fn resolve_edit_span(
    source: &str,
    edit: &RepairEdit,
    has_explicit_followup: bool,
) -> Result<ResolvedEdit, String> {
    let start = edit.start.offset;
    let mut end = edit.end.offset;
    if start >= end || end > source.len() {
        return Err("repair target span is stale".to_string());
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err("repair target span is not on character boundaries".to_string());
    }
    let target = &source[start..end];
    if target.trim_start().starts_with('_') {
        if !has_explicit_followup && source[end..].starts_with(" satisfy ") {
            end += source[end..]
                .find('\n')
                .unwrap_or_else(|| source[end..].len());
        }
    } else if !(edit.is_satisfy_suffix_deletion() && is_satisfy_suffix_text(target)) {
        return Err("repair target no longer names a hole".to_string());
    }

    Ok(ResolvedEdit {
        start,
        end,
        applied: AppliedEdit {
            edit: RepairEdit {
                file: edit.file.clone(),
                start: edit.start,
                end: line_col_at(source, end),
                replacement: edit.replacement.clone(),
            },
        },
    })
}

fn is_satisfy_suffix_text(text: &str) -> bool {
    text.starts_with(" satisfy ") || text.starts_with("satisfy ")
}

fn line_col_at(source: &str, offset: usize) -> LineCol {
    let offset = offset.min(source.len());
    let mut line = 1;
    let mut line_start = 0;
    for (index, byte) in source.bytes().enumerate() {
        if index >= offset {
            break;
        }
        if byte == b'\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    let column = source[line_start..offset].chars().count() + 1;
    LineCol {
        line,
        column,
        offset,
    }
}

#[cfg(test)]
fn replace_span(source: &str, candidate: &RepairCandidate) -> Result<LegacyResolvedEdit, String> {
    let edit = candidate
        .edits
        .first()
        .ok_or_else(|| "unsupported edit shape".to_string())?;
    let resolved = resolve_edit_span(source, edit, false)?;
    let mut repaired = source.to_string();
    repaired.replace_range(
        resolved.start..resolved.end,
        &resolved.applied.edit.replacement,
    );
    Ok(LegacyResolvedEdit {
        repaired,
        end: resolved.applied.edit.end,
    })
}

#[cfg(test)]
#[derive(Debug)]
struct LegacyResolvedEdit {
    repaired: String,
    end: LineCol,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::repair::test_candidate;

    #[test]
    fn replace_span_refuses_stale_targets() {
        let error =
            replace_span("value\n", &test_candidate(0, 5)).expect_err("target is not a hole");

        assert_eq!(error, "repair target no longer names a hole");
    }

    #[test]
    fn replace_span_refuses_out_of_bounds_targets() {
        let error = replace_span("_hole\n", &test_candidate(0, 20)).expect_err("target is stale");

        assert_eq!(error, "repair target span is stale");
    }

    #[test]
    fn replace_span_refuses_saved_parse_delimiter_targets() {
        let mut candidate = test_candidate(8, 9);
        candidate.source_candidate_id = "parse.type_parameter_delimiters".to_string();
        candidate.edits[0].replacement = "<".to_string();

        let error =
            replace_span("type Box(A)\n", &candidate).expect_err("delimiter target is not a hole");

        assert_eq!(error, "repair target no longer names a hole");
    }

    #[test]
    fn replace_span_removes_satisfy_suffix() {
        let edit = replace_span(
            "fn main(order: Int) -> Int\n  _value satisfy candidate => candidate == order\nend\n",
            &test_candidate(29, 35),
        )
        .expect("satisfy suffix should be included in the edit");

        assert_eq!(edit.repaired, "fn main(order: Int) -> Int\n  value\nend\n");
        assert_eq!(edit.end.line, 2);
        assert_eq!(edit.end.column, 49);
    }

    #[test]
    fn build_edit_plan_refuses_overlapping_edits() {
        let root = std::env::temp_dir().join(format!("veln-repair-overlap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(root.join("main.veln"), "__hole\n").expect("source should be written");

        let mut candidate = test_candidate(0, 6);
        candidate.edits.push(RepairEdit {
            file: "main.veln".to_string(),
            start: LineCol {
                line: 1,
                column: 2,
                offset: 1,
            },
            end: LineCol {
                line: 1,
                column: 7,
                offset: 6,
            },
            replacement: "other".to_string(),
        });

        let error = build_edit_plan(&root, &candidate).expect_err("overlap should refuse");

        assert_eq!(error, "repair edits overlap");
        let _ = fs::remove_dir_all(&root);
    }
}
