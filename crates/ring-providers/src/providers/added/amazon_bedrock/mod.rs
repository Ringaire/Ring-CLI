use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Amazon Bedrock".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: None,
        api_key_env: Some("AWS_BEARER_TOKEN_BEDROCK".into()),
        default_model: Some("anthropic.claude-sonnet-4-20250514-v1:0".into()),
        extra_body: None,
    }
}
