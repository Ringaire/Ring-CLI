use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Google Vertex AI".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://aiplatform.googleapis.com/v1".into()),
        api_key_env: Some("GOOGLE_VERTEX_API_KEY".into()),
        default_model: Some("gemini-2.5-pro".into()),
        extra_body: None,
    }
}
