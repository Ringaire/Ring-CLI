use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "IO.NET".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.io.net/v1".into()),
        api_key_env: Some("IO_NET_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
