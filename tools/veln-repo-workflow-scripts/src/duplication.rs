use std::cmp::Ordering;

use serde_json::Value;

pub fn render_duplication_summary(
    report: &Value,
    duplicate_limit: usize,
) -> Result<String, String> {
    Ok(DuplicationSummary::parse(report)?.render(duplicate_limit))
}

#[derive(Debug)]
struct DuplicationSummary {
    statistics: DuplicationStatistics,
    rows: Vec<DuplicateRow>,
}

impl DuplicationSummary {
    fn parse(report: &Value) -> Result<Self, String> {
        let duplicates = report
            .get("duplicates")
            .and_then(Value::as_array)
            .ok_or_else(|| "expected a duplicates array".to_owned())?;
        let statistics = DuplicationStatistics::parse(report)?;
        let mut rows = duplicates
            .iter()
            .map(DuplicateRow::parse)
            .collect::<Result<Vec<_>, _>>()?;
        rows.sort_by(DuplicateRow::compare_for_summary);
        Ok(Self { statistics, rows })
    }

    fn render(&self, duplicate_limit: usize) -> String {
        let mut output = self.statistics.render();
        if self.rows.is_empty() {
            output.push_str("No exact clone pairs were detected.\n\n");
            return output;
        }
        output.push_str(
            "| Lines | Tokens | First occurrence | Second occurrence |\n| ---: | ---: | --- | --- |\n",
        );
        for row in self.rows.iter().take(duplicate_limit) {
            row.render_into(&mut output);
        }
        if self.rows.len() > duplicate_limit {
            output.push_str(&format!(
                "\n{} more clone pair(s) omitted from this summary.\n",
                self.rows.len() - duplicate_limit
            ));
        }
        output.push('\n');
        output
    }
}

#[derive(Debug)]
struct DuplicationStatistics {
    sources: u64,
    lines: u64,
    clones: u64,
    duplicated_lines: u64,
    percentage: f64,
}

impl DuplicationStatistics {
    fn parse(report: &Value) -> Result<Self, String> {
        let total = report.pointer("/statistics/total").ok_or_else(|| {
            "expected statistics.total.sources to be a non-negative integer".to_owned()
        })?;
        let sources = non_negative_integer(total, "sources")?;
        let lines = non_negative_integer(total, "lines")?;
        let clones = non_negative_integer(total, "clones")?;
        let duplicated_lines = non_negative_integer(total, "duplicatedLines")?;
        let percentage = total
            .get("percentage")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| {
                "expected statistics.total.percentage to be a non-negative finite number".to_owned()
            })?;
        Ok(Self {
            sources,
            lines,
            clones,
            duplicated_lines,
            percentage,
        })
    }

    fn render(&self) -> String {
        let Self {
            sources,
            lines,
            clones,
            duplicated_lines,
            percentage,
        } = self;
        format!(
            "## Code Duplication Refactor Signals\n\nInspect the largest exact clone pairs when changing either occurrence; consolidating shared behavior can prevent fixes from diverging. This report is advisory because some repetition is intentional.\n\n- Rust files analyzed: {sources}\n- Rust lines analyzed: {lines}\n- Exact clone pairs: {clones}\n- Duplicated lines: {duplicated_lines} ({percentage:.2}%)\n\n### Largest exact clone pairs\n\n"
        )
    }
}

#[derive(Debug)]
struct DuplicateRow {
    lines: u64,
    tokens: u64,
    first_name: String,
    first_start: u64,
    second_name: String,
    second_start: u64,
}

impl DuplicateRow {
    fn parse(value: &Value) -> Result<Self, String> {
        let lines = value.get("lines").and_then(Value::as_u64).ok_or_else(|| {
            "expected each duplicate to have non-negative integer lines and tokens".to_owned()
        })?;
        let tokens = value.get("tokens").and_then(Value::as_u64).ok_or_else(|| {
            "expected each duplicate to have non-negative integer lines and tokens".to_owned()
        })?;
        let (first_name, first_start) = file_location(value.get("firstFile"))?;
        let (second_name, second_start) = file_location(value.get("secondFile"))?;
        Ok(Self {
            lines,
            tokens,
            first_name,
            first_start,
            second_name,
            second_start,
        })
    }

    fn compare_for_summary(left: &Self, right: &Self) -> Ordering {
        right
            .lines
            .cmp(&left.lines)
            .then_with(|| right.tokens.cmp(&left.tokens))
            .then_with(|| left.first_name.cmp(&right.first_name))
            .then_with(|| left.first_start.cmp(&right.first_start))
            .then_with(|| left.second_name.cmp(&right.second_name))
            .then_with(|| left.second_start.cmp(&right.second_start))
    }

    fn render_into(&self, output: &mut String) {
        output.push_str(&format!(
            "| {} | {} | `{}:{}` | `{}:{}` |\n",
            self.lines,
            self.tokens,
            escape_markdown(&self.first_name),
            self.first_start,
            escape_markdown(&self.second_name),
            self.second_start,
        ));
    }
}

fn file_location(value: Option<&Value>) -> Result<(String, u64), String> {
    let value = value.ok_or_else(|| {
        "expected each duplicate occurrence to have a file name and start line".to_owned()
    })?;
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            "expected each duplicate occurrence to have a file name and start line".to_owned()
        })?;
    let start = value.get("start").and_then(Value::as_u64).ok_or_else(|| {
        "expected each duplicate occurrence to have a file name and start line".to_owned()
    })?;
    Ok((name.to_owned(), start))
}

fn non_negative_integer(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("expected statistics.total.{field} to be a non-negative integer"))
}

fn escape_markdown(value: &str) -> String {
    value.replace('|', "\\|").replace('`', "\\`")
}

pub fn escape_annotation_message(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}
