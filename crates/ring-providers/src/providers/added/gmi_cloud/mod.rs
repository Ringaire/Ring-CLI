use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "GMI Cloud".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.gmicloud.ai/v1".into()),
        api_key_env: Some("GMI_CLOUD_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
