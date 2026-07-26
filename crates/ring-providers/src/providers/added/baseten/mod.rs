use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Baseten".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://inference.baseten.co/v1".into()),
        api_key_env: Some("BASETEN_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
