use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Azure OpenAI".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: None,
        api_key_env: Some("AZURE_API_KEY".into()),
        default_model: Some("gpt-4o".into()),
        extra_body: None,
    }
}
