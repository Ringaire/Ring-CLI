use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "ZenMux".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://zenmux.ai/api/v1".into()),
        api_key_env: Some("ZENMUX_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
