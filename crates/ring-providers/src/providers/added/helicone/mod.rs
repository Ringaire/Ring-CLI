use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Helicone".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://ai-gateway.helicone.ai/v1".into()),
        api_key_env: Some("HELICONE_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
