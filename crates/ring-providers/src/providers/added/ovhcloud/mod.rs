use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "OVHcloud AI".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://oai.endpoints.kepler.ai.cloud.ovh.net/v1".into()),
        api_key_env: Some("OVHCLOUD_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
