use std::collections::BTreeMap;

use serde_json::{Value, json};
use veln_repo_language_reference::{RenderedResource, render_checked_language_reference};

#[derive(Clone)]
pub(crate) struct LanguageResources {
    resources: Vec<RenderedResource>,
    by_uri: BTreeMap<String, RenderedResource>,
    topics: Vec<LanguageTopic>,
}

impl LanguageResources {
    pub(crate) fn checked() -> Result<Self, String> {
        let rendered = render_checked_language_reference()?;
        let topics = language_topics(&rendered.resources)?;
        let by_uri = rendered
            .resources
            .iter()
            .cloned()
            .map(|resource| (resource.uri.clone(), resource))
            .collect();
        Ok(Self {
            resources: rendered.resources,
            by_uri,
            topics,
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test(resources: Vec<RenderedResource>, topics: Vec<LanguageTopic>) -> Self {
        let by_uri = resources
            .iter()
            .cloned()
            .map(|resource| (resource.uri.clone(), resource))
            .collect();
        Self {
            resources,
            by_uri,
            topics,
        }
    }

    pub(crate) fn list_result(&self) -> Value {
        json!({
            "resources": self.resources.iter().map(resource_metadata).collect::<Vec<_>>()
        })
    }

    pub(crate) fn read_result(&self, uri: &str) -> Option<Value> {
        self.by_uri.get(uri).map(|resource| {
            json!({
                "contents": [{
                    "uri": resource.uri,
                    "mimeType": resource.mime_type,
                    "text": resource.text,
                }]
            })
        })
    }

    pub(crate) fn read_doc_result(&self, uri: &str) -> Option<Value> {
        self.by_uri.get(uri).map(|resource| {
            let mut value = json!({
                "uri": resource.uri,
                "name": resource.name,
                "title": resource.title,
                "mimeType": resource.mime_type,
                "text": resource.text,
            });
            if let Some(description) = &resource.description {
                value["description"] = json!(description);
            }
            value
        })
    }

    pub(crate) fn topics(&self) -> &[LanguageTopic] {
        &self.topics
    }
}

fn resource_metadata(resource: &RenderedResource) -> Value {
    let mut value = json!({
        "uri": resource.uri,
        "name": resource.name,
        "title": resource.title,
        "mimeType": resource.mime_type,
    });
    if let Some(description) = &resource.description {
        value["description"] = json!(description);
    }
    value
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LanguageTopic {
    pub(crate) uri: String,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) keywords: Vec<String>,
    pub(crate) body: String,
}

fn language_topics(resources: &[RenderedResource]) -> Result<Vec<LanguageTopic>, String> {
    let catalog: Value =
        serde_json::from_str(veln_repo_language_reference::checked_catalog_bytes())
            .map_err(|error| format!("parse checked language-reference catalog: {error}"))?;
    let topics = catalog
        .get("topics")
        .and_then(Value::as_array)
        .ok_or("checked language-reference catalog must contain topics")?;
    let uri_by_name = resources
        .iter()
        .map(|resource| (resource.name.as_str(), resource.uri.as_str()))
        .collect::<BTreeMap<_, _>>();
    topics
        .iter()
        .map(|topic| {
            let id = string_field(topic, "id")?;
            let body = string_array_field(topic, "body")?.join("\n\n");
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
                body,
            })
        })
        .collect()
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
