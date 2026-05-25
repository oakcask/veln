use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn assert_line_col(actual: LineCol, line: usize, column: usize, offset: usize) {
    assert_eq!(
        actual,
        LineCol {
            line,
            column,
            offset
        }
    );
}

#[test]
fn normalizes_source_paths() {
    let path = SourcePath::new("././src\\main.veln");

    assert_eq!(path.as_str(), "src/main.veln");
}

#[test]
fn preserves_internal_relative_segments_when_normalizing_source_paths() {
    let path = SourcePath::new(".\\src\\..\\main.veln");

    assert_eq!(path.as_str(), "src/../main.veln");
}

#[test]
fn normalizes_only_leading_current_directory_segments() {
    let current_dir = SourcePath::new("././");
    let bare_current_dir = SourcePath::new(".");

    assert_eq!(current_dir.as_str(), "");
    assert_eq!(bare_current_dir.as_str(), ".");
}

#[test]
fn maps_offsets_to_one_based_lines_and_columns() {
    let source = SourceFile::new("src/main.veln", "a\nbc\n");

    assert_line_col(source.line_col(0), 1, 1, 0);
    assert_line_col(source.line_col(2), 2, 1, 2);
    assert_line_col(source.line_col(4), 2, 3, 4);
}

#[test]
fn clamps_offsets_to_end_of_file() {
    let source = SourceFile::new("main.veln", "a\n");

    assert_line_col(source.line_col(usize::MAX), 2, 1, 2);
}

#[test]
fn reports_empty_file_start_position() {
    let source = SourceFile::new("empty.veln", "");

    assert!(source.is_empty());
    assert_eq!(source.len(), 0);
    assert_line_col(source.line_col(0), 1, 1, 0);
}

#[test]
fn builds_spans_from_text_ranges() {
    let source = SourceFile::new("main.veln", "alpha\nbeta\ngamma");

    let span = source.span(TextRange::new(3, 8));

    assert_eq!(span.file.as_str(), "main.veln");
    assert_line_col(span.start, 1, 4, 3);
    assert_line_col(span.end, 2, 3, 8);
}

#[test]
fn text_ranges_create_points_and_cover_ranges() {
    let point = TextRange::at(4);

    assert_eq!(point, TextRange::new(4, 4));
    assert_eq!(
        TextRange::new(8, 10).cover(TextRange::new(3, 6)),
        TextRange::new(3, 10)
    );
}

#[test]
fn reads_files_with_project_relative_paths() -> std::io::Result<()> {
    let test_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root =
        std::env::temp_dir().join(format!("veln-source-test-{}-{test_id}", std::process::id()));
    let temp = TempDir { root: root.clone() };
    let src_dir = root.join("src");
    let file_path = src_dir.join("main.veln");
    std::fs::create_dir_all(&src_dir)?;
    std::fs::write(&file_path, "fn main()\nend\n")?;

    let source = SourceFile::read(&root, &file_path)?;

    assert_eq!(source.path().as_str(), "src/main.veln");
    assert_eq!(source.text(), "fn main()\nend\n");

    drop(temp);
    Ok(())
}

struct TempDir {
    root: std::path::PathBuf,
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
