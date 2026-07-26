use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "STACKIT".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://ai.stackit.cloud/v1".into()),
        api_key_env: Some("STACKIT_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
