use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Nebius Token Factory".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.tokenfactory.nebius.com/v1".into()),
        api_key_env: Some("NEBIUS_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
