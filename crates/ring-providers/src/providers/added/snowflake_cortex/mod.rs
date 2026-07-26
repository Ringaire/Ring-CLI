use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Snowflake Cortex".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: None,
        api_key_env: Some("SNOWFLAKE_API_KEY".into()),
        default_model: None,
        extra_body: None,
    }
}
