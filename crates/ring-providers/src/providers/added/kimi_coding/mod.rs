use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Kimi For Coding".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.kimi.com/coding".into()),
        api_key_env: Some("KIMI_API_KEY".into()),
        default_model: Some("kimi-k2".into()),
        extra_body: None,
    }
}
