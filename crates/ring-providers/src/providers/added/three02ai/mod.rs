use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "302.AI".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.302.ai/v1".into()),
        api_key_env: Some("THREE02AI_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
