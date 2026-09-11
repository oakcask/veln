use std::collections::BTreeMap;

use serde_json::Value;
use veln_repo_language_reference::RenderedResource;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LanguageTopic {
    pub(crate) uri: String,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) keywords: Vec<String>,
    pub(crate) body: String,
}

pub(super) fn language_topics(
    resources: &[RenderedResource],
) -> Result<Vec<LanguageTopic>, String> {
    let catalog = checked_language_catalog()?;
    let topics = checked_language_topics(&catalog)?;
    let uri_by_name = resources
        .iter()
        .map(|resource| (resource.name.as_str(), resource.uri.as_str()))
        .collect::<BTreeMap<_, _>>();
    topics
        .iter()
        .map(|topic| language_topic(topic, &uri_by_name))
        .collect()
}

fn checked_language_catalog() -> Result<Value, String> {
    serde_json::from_str(veln_repo_language_reference::checked_catalog_bytes())
        .map_err(|error| format!("parse checked language-reference catalog: {error}"))
}

fn checked_language_topics(catalog: &Value) -> Result<&Vec<Value>, String> {
    catalog
        .get("topics")
        .and_then(Value::as_array)
        .ok_or_else(|| "checked language-reference catalog must contain topics".to_string())
}

fn language_topic(
    topic: &Value,
    uri_by_name: &BTreeMap<&str, &str>,
) -> Result<LanguageTopic, String> {
    let id = string_field(topic, "id")?;
    Ok(LanguageTopic {
        uri: uri_by_name
            .get(id)
            .ok_or("checked topic resource must exist")?
            .to_string(),
        id: id.to_string(),
        title: string_field(topic, "title")?.to_string(),
        summary: string_field(topic, "summary")?.to_string(),
        keywords: string_array_field(topic, "keywords")?
            .into_iter()
            .map(str::to_string)
            .collect(),
        body: string_array_field(topic, "body")?.join("\n\n"),
    })
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("checked language-reference topic must contain `{field}`"))
}

fn string_array_field<'a>(value: &'a Value, field: &str) -> Result<Vec<&'a str>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("checked language-reference topic must contain `{field}`"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .ok_or_else(|| format!("checked language-reference `{field}` must contain strings"))
        })
        .collect()
}
