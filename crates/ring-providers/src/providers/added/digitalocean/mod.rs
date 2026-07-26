use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "DigitalOcean".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://inference.do-ai.run/v1".into()),
        api_key_env: Some("DIGITALOCEAN_ACCESS_TOKEN".into()),
        default_model: None,
        extra_body: None,
    }
}
