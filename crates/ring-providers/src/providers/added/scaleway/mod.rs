use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Scaleway".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.scaleway.com/v1".into()),
        api_key_env: Some("SCALEWAY_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
