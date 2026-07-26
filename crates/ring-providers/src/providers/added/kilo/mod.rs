use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Kilo AI".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.kilo.ai/api/gateway".into()),
        api_key_env: Some("KILO_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
