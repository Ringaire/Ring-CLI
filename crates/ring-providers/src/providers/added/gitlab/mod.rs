use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "GitLab Duo".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://gitlab.com/api/v4/duo".into()),
        api_key_env: Some("GITLAB_TOKEN".into()),
        default_model: Some("duo-chat-sonnet-4-5".into()),
        extra_body: None,
    }
}
