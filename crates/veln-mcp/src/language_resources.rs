use std::collections::BTreeMap;

use serde_json::{Value, json};
use veln_repo_language_reference::{RenderedResource, render_checked_language_reference};

#[derive(Clone)]
pub(crate) struct LanguageResources {
    resources: Vec<RenderedResource>,
    by_uri: BTreeMap<String, RenderedResource>,
}

impl LanguageResources {
    pub(crate) fn checked() -> Result<Self, String> {
        let rendered = render_checked_language_reference()?;
        let by_uri = rendered
            .resources
            .iter()
            .cloned()
            .map(|resource| (resource.uri.clone(), resource))
            .collect();
        Ok(Self {
            resources: rendered.resources,
            by_uri,
        })
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
