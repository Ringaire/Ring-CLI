use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Alibaba".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://dashscope-intl.aliyuncs.com/compatible-mode/v1".into()),
        api_key_env: Some("ALIBABA_API_KEY".into()),
        default_model: Some("qwen-plus".into()),
        extra_body: None,
    }
}
