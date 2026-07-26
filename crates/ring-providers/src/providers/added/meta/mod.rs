use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Meta Llama".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.meta.ai/v1".into()),
        api_key_env: Some("META_API_KEY".into()),
        default_model: Some("llama-4-maverick".into()),
        extra_body: None,
    }
}
