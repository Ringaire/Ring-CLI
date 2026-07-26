use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Ollama Cloud".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://ollama.com/v1".into()),
        api_key_env: Some("OLLAMA_CLOUD_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
