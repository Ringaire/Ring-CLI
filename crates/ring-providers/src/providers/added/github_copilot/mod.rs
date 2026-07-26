use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "GitHub Copilot".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://api.githubcopilot.com/v1".into()),
        api_key_env: Some("GITHUB_COPILOT_TOKEN".into()),
        default_model: Some("gpt-4o".into()),
        extra_body: None,
    }
}
