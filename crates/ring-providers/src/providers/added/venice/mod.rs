use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Venice AI".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.venice.ai/api/v1".into()),
        api_key_env: Some("VENICE_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
