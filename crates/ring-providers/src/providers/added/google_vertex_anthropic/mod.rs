use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Google Vertex Anthropic".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://aiplatform.googleapis.com/v1".into()),
        api_key_env: Some("GOOGLE_VERTEX_API_KEY".into()),
        default_model: Some("claude-sonnet-4-20250514".into()),
        extra_body: None,
    }
}
