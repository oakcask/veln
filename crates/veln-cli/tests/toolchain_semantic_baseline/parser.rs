use super::*;

#[derive(Default)]
pub(super) struct BaselineParser {
    schema: Option<String>,
    source_git_tree: Option<String>,
    roots: Vec<String>,
    declared_count: Option<usize>,
    declared_aggregate: Option<String>,
    cases: BTreeMap<String, BTreeMap<String, String>>,
    current: Option<(String, BTreeMap<String, String>)>,
}

impl BaselineParser {
    pub(super) fn parse(mut self, text: &str) -> Result<Inventory, String> {
        for (index, raw_line) in text.lines().enumerate() {
            self.parse_line(index + 1, raw_line)?;
        }
        self.finish_open_case(text.lines().count() + 1, None)?;
        self.into_inventory()
    }

    fn parse_line(&mut self, line_number: usize, raw_line: &str) -> Result<(), String> {
        let mut parts = raw_line.splitn(3, '\t');
        let kind = parts.next().unwrap_or_default();
        let first = parts
            .next()
            .ok_or_else(|| format!("baseline line {line_number} has no value"))?;
        match kind {
            "schema" => self.schema = Some(parse_json_string(first, line_number)?),
            "source_git_tree" => {
                self.source_git_tree = Some(parse_json_string(first, line_number)?);
            }
            "root" => self.roots.push(parse_json_string(first, line_number)?),
            "case_count" => self.declared_count = Some(parse_count(first, line_number)?),
            "case" => self.start_case(first, line_number)?,
            "field" => self.insert_field(first, parts.next(), line_number)?,
            "case_digest" => self.finish_case_digest(first, line_number)?,
            "aggregate_digest" => self.finish_aggregate(first, line_number)?,
            _ => {
                return Err(format!(
                    "baseline line {line_number} has unknown record `{kind}`"
                ));
            }
        }
        Ok(())
    }

    fn start_case(&mut self, value: &str, line_number: usize) -> Result<(), String> {
        self.finish_open_case(line_number, None)?;
        self.current = Some((parse_json_string(value, line_number)?, BTreeMap::new()));
        Ok(())
    }

    fn insert_field(
        &mut self,
        path: &str,
        value: Option<&str>,
        line_number: usize,
    ) -> Result<(), String> {
        let value =
            value.ok_or_else(|| format!("baseline line {line_number} has no field value"))?;
        let (_, fields) = self
            .current
            .as_mut()
            .ok_or_else(|| format!("baseline line {line_number} has a field outside a case"))?;
        if fields.insert(path.to_string(), value.to_string()).is_some() {
            return Err(format!(
                "baseline line {line_number} repeats field `{path}`"
            ));
        }
        Ok(())
    }

    fn finish_case_digest(&mut self, value: &str, line_number: usize) -> Result<(), String> {
        let digest = parse_json_string(value, line_number)?;
        self.finish_open_case(line_number, Some(&digest))
    }

    fn finish_aggregate(&mut self, value: &str, line_number: usize) -> Result<(), String> {
        self.finish_open_case(line_number, None)?;
        self.declared_aggregate = Some(parse_json_string(value, line_number)?);
        Ok(())
    }

    fn finish_open_case(
        &mut self,
        line_number: usize,
        declared_digest: Option<&str>,
    ) -> Result<(), String> {
        finish_case(
            &mut self.current,
            &mut self.cases,
            declared_digest,
            line_number,
        )
    }

    fn into_inventory(self) -> Result<Inventory, String> {
        let schema = self
            .schema
            .ok_or_else(|| "baseline is missing schema".to_string())?;
        validate_schema(&schema)?;
        validate_case_count(self.declared_count, self.cases.len())?;
        validate_aggregate(self.declared_aggregate.as_deref(), &self.cases)?;
        Ok(Inventory {
            schema,
            roots: self.roots,
            source_git_tree: self
                .source_git_tree
                .ok_or_else(|| "baseline is missing source Git tree".to_string())?,
            cases: self.cases,
        })
    }
}

fn parse_count(value: &str, line_number: usize) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("baseline line {line_number} has invalid case count: {error}"))
}

fn validate_schema(schema: &str) -> Result<(), String> {
    if schema == SCHEMA {
        Ok(())
    } else {
        Err(format!("unsupported baseline schema `{schema}`"))
    }
}

fn validate_case_count(declared_count: Option<usize>, actual_count: usize) -> Result<(), String> {
    if declared_count == Some(actual_count) {
        Ok(())
    } else {
        Err(format!(
            "baseline case count mismatch: declared {declared_count:?}, found {actual_count}"
        ))
    }
}

fn validate_aggregate(
    declared_aggregate: Option<&str>,
    cases: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(), String> {
    let actual_aggregate = aggregate_digest(cases);
    if declared_aggregate == Some(actual_aggregate.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "baseline aggregate digest mismatch: declared {declared_aggregate:?}, computed {actual_aggregate}"
        ))
    }
}

fn finish_case(
    current: &mut Option<(String, BTreeMap<String, String>)>,
    cases: &mut BTreeMap<String, BTreeMap<String, String>>,
    declared_digest: Option<&str>,
    line_number: usize,
) -> Result<(), String> {
    let Some((id, fields)) = current.take() else {
        if declared_digest.is_some() {
            return Err(format!(
                "baseline line {line_number} has a digest outside a case"
            ));
        }
        return Ok(());
    };
    if let Some(declared_digest) = declared_digest {
        let actual = fields_digest(&fields);
        if declared_digest != actual {
            return Err(format!(
                "baseline case `{id}` digest mismatch: declared {declared_digest}, computed {actual}"
            ));
        }
    }
    if cases.insert(id.clone(), fields).is_some() {
        return Err(format!("baseline repeats case `{id}`"));
    }
    Ok(())
}
