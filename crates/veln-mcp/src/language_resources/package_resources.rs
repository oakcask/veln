use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};
use veln_language_service::{
    NavigationLocation, NavigationSource, PackageDocResult, RenderedPackageDocResource,
};
use veln_repo_language_reference::RenderedResource;

pub(super) const VELN_SOURCE_MEDIA_TYPE: &str = "text/x-veln; charset=utf-8";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RetainedPackageKey {
    pub(super) identity: String,
    pub(super) digest: String,
}

#[derive(Clone, Debug)]
pub(super) enum PackageDocumentation {
    Ready(PackageDocResult),
    CheckedStandardLibrary(Arc<CheckedStandardLibraryDocumentation>),
}

impl PackageDocumentation {
    pub(super) fn search_candidates(
        &self,
        key: &RetainedPackageKey,
    ) -> Vec<PackageSearchCandidate> {
        match self {
            Self::Ready(result) => package_search_candidates(key, result),
            Self::CheckedStandardLibrary(documentation) => documentation.search_candidates.clone(),
        }
    }

    pub(super) fn declaration_uri_for_location(
        &self,
        location: &NavigationLocation,
    ) -> Option<&str> {
        match self {
            Self::Ready(result) => result.declaration_uri_for_location(location),
            Self::CheckedStandardLibrary(documentation) => {
                documentation.declaration_uri_for_location(location)
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct CheckedStandardLibraryDocumentation {
    pub(super) search_candidates: Vec<PackageSearchCandidate>,
    pub(super) declaration_locations: BTreeMap<DeclarationLocationKey, String>,
}

impl CheckedStandardLibraryDocumentation {
    fn declaration_uri_for_location(&self, location: &NavigationLocation) -> Option<&str> {
        let NavigationSource::Package { uri } = &location.source else {
            return None;
        };
        self.declaration_locations
            .get(&DeclarationLocationKey {
                source_uri: uri.clone(),
                line: location.span.start.line,
                column: location.span.start.column,
                offset: location.span.start.offset,
            })
            .map(String::as_str)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct DeclarationLocationKey {
    pub(super) source_uri: String,
    pub(super) line: usize,
    pub(super) column: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublishedResource {
    pub(super) uri: String,
    pub(super) name: String,
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) mime_type: &'static str,
    pub(super) text: String,
    pub(crate) listed: bool,
}

impl PublishedResource {
    pub(super) fn from_rendered(resource: &RenderedResource) -> Self {
        Self {
            uri: resource.uri.clone(),
            name: resource.name.clone(),
            title: resource.title.clone(),
            description: resource.description.clone(),
            mime_type: resource.mime_type,
            text: resource.text.clone(),
            listed: true,
        }
    }

    pub(super) fn from_package_doc(resource: &RenderedPackageDocResource) -> Self {
        Self {
            uri: resource.uri.clone(),
            name: resource.name.clone(),
            title: resource.title.clone(),
            description: resource.description.clone(),
            mime_type: resource.mime_type,
            text: resource.text.clone(),
            listed: resource.listed,
        }
    }

    pub(super) fn from_checked_package_doc(
        resource: veln_repo_mcp_standard_library_docs::CheckedResource,
    ) -> Self {
        Self {
            uri: resource.uri,
            name: resource.name,
            title: resource.title,
            description: resource.description,
            mime_type: veln_language_service::PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
            text: resource.text,
            listed: resource.listed,
        }
    }

    pub(crate) fn metadata(&self) -> Value {
        let mut value = json!({
            "uri": self.uri,
            "name": self.name,
            "title": self.title,
            "mimeType": self.mime_type,
        });
        if let Some(description) = &self.description {
            value["description"] = json!(description);
        }
        value
    }

    pub(super) fn doc_tool_result(&self) -> Value {
        let mut value = json!({
            "uri": self.uri,
            "name": self.name,
            "title": self.title,
            "mimeType": self.mime_type,
            "text": self.text,
        });
        if let Some(description) = &self.description {
            value["description"] = json!(description);
        }
        value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PackageSearchScope {
    StandardLibrary,
    Package,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PackageSearchCandidate {
    pub(crate) scope: PackageSearchScope,
    pub(crate) uri: String,
    pub(crate) identifier: String,
    pub(crate) title: String,
    pub(crate) name: String,
    pub(crate) summary: String,
    pub(crate) keywords: Vec<String>,
    pub(crate) signature: Option<String>,
    pub(crate) documentation: Vec<String>,
}

fn package_search_candidates(
    key: &RetainedPackageKey,
    result: &PackageDocResult,
) -> Vec<PackageSearchCandidate> {
    let scope = if key.identity == "std" {
        PackageSearchScope::StandardLibrary
    } else {
        PackageSearchScope::Package
    };
    result
        .search_candidates()
        .into_iter()
        .map(|candidate| PackageSearchCandidate {
            scope,
            uri: candidate.uri,
            identifier: candidate.identifier,
            title: candidate.title,
            name: candidate.name,
            summary: candidate.summary,
            keywords: candidate.keywords,
            signature: candidate.signature,
            documentation: candidate.documentation,
        })
        .collect()
}
