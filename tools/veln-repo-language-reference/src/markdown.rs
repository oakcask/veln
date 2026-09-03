use serde_json::Value;
use sha2::{Digest, Sha256};

pub const LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE: &str = "text/markdown; charset=utf-8";
pub const LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT: usize = 262_144;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedLanguageReference {
    pub resources: Vec<RenderedResource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedResource {
    pub uri: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub mime_type: &'static str,
    pub text: String,
}

pub fn render_checked_language_reference() -> Result<RenderedLanguageReference, String> {
    render_language_reference(
        crate::checked_catalog_bytes(),
        crate::checked_catalog_digest(),
    )
}

pub fn render_language_reference(
    catalog_bytes: &str,
    digest: &str,
) -> Result<RenderedLanguageReference, String> {
    let catalog: Value = serde_json::from_str(catalog_bytes).map_err(|error| {
        format!("parse checked language-reference catalog before rendering: {error}")
    })?;
    let topics = catalog
        .get("topics")
        .and_then(Value::as_array)
        .ok_or("render language-reference catalog with a topics array")?;
    let base = format!("veln-doc:///language/snapshot/{digest}");
    let index_uri = format!("{base}/index");
    let mut resources = Vec::with_capacity(topics.len() + 1);
    resources.push(RenderedResource {
        uri: index_uri,
        name: "language-index".to_string(),
        title: "Veln Language Reference".to_string(),
        description: None,
        mime_type: LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
        text: render_index(&base, topics)?,
    });
    for topic in topics {
        let id = string_field(topic, "id")?;
        let title = string_field(topic, "title")?;
        let summary = string_field(topic, "summary")?;
        resources.push(RenderedResource {
            uri: topic_uri(&base, id),
            name: id.to_string(),
            title: title.to_string(),
            description: Some(summary.to_string()),
            mime_type: LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
            text: render_topic(&base, topic)?,
        });
    }
    resources.sort_by(|left, right| left.uri.as_bytes().cmp(right.uri.as_bytes()));
    for resource in &resources {
        if resource.text.len() > LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT {
            return Err(format!(
                "language-reference resource `{}` renders to {} bytes, above the {} byte limit",
                resource.uri,
                resource.text.len(),
                LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT
            ));
        }
    }
    Ok(RenderedLanguageReference { resources })
}

pub fn rendered_language_reference_digest(rendered: &RenderedLanguageReference) -> String {
    let mut digest = Sha256::new();
    digest.update(crate::RENDERED_DIGEST_DOMAIN);
    for resource in &rendered.resources {
        update_digest_field(&mut digest, &resource.uri);
        update_digest_field(&mut digest, &resource.name);
        update_digest_field(&mut digest, &resource.title);
        update_digest_field(&mut digest, resource.description.as_deref().unwrap_or(""));
        update_digest_field(&mut digest, resource.mime_type);
        update_digest_field(&mut digest, &resource.text);
    }
    crate::hex_lower(&digest.finalize())
}

fn update_digest_field(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn render_index(base: &str, topics: &[Value]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("# Veln Language Reference\n\n");
    for topic in topics {
        let id = string_field(topic, "id")?;
        let title = string_field(topic, "title")?;
        let summary = string_field(topic, "summary")?;
        out.push_str("- [");
        out.push_str(title);
        out.push_str("](");
        out.push_str(&topic_uri(base, id));
        out.push_str(") - ");
        out.push_str(summary);
        out.push('\n');
    }
    Ok(out)
}

fn render_topic(base: &str, topic: &Value) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(string_field(topic, "title")?);
    out.push_str("\n\n");
    out.push_str(string_field(topic, "summary")?);
    out.push_str("\n\n");
    for paragraph in string_array_field(topic, "body")? {
        out.push_str(paragraph);
        out.push_str("\n\n");
    }
    out.push_str("## Grammar\n\n");
    for grammar in array_field(topic, "grammar")? {
        out.push_str("### ");
        out.push_str(string_field(grammar, "name")?);
        out.push_str("\n\n```ebnf\n");
        out.push_str(string_field(grammar, "text")?);
        out.push_str("\n```\n\n");
    }
    out.push_str("## Examples\n\n");
    for example in array_field(topic, "examples")? {
        out.push_str("### ");
        out.push_str(string_field(example, "display_name")?);
        out.push_str("\n\n");
        for file in array_field(example, "files")? {
            out.push_str("#### ");
            out.push_str(string_field(file, "path")?);
            out.push_str("\n\n```veln\n");
            out.push_str(string_field(file, "source")?);
            out.push_str("\n```\n\n");
        }
    }
    out.push_str("## Keywords\n\n");
    for keyword in string_array_field(topic, "keywords")? {
        out.push_str("- ");
        out.push_str(keyword);
        out.push('\n');
    }
    out.push_str("\n## Related Topics\n\n");
    for related in string_array_field(topic, "related")? {
        out.push_str("- [");
        out.push_str(related);
        out.push_str("](");
        out.push_str(&topic_uri(base, related));
        out.push_str(")\n");
    }
    Ok(out)
}

fn topic_uri(base: &str, topic_id: &str) -> String {
    format!("{base}/topic/{topic_id}")
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("render language-reference catalog with string field `{field}`"))
}

fn array_field<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("render language-reference catalog with array field `{field}`"))
}

fn string_array_field<'a>(value: &'a Value, field: &str) -> Result<Vec<&'a str>, String> {
    array_field(value, field)?
        .iter()
        .map(|entry| {
            entry.as_str().ok_or_else(|| {
                format!("render language-reference catalog with string values in `{field}`")
            })
        })
        .collect()
}
