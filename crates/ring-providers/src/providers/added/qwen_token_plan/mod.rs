use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Qwen Token Plan".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1".into()),
        api_key_env: Some("QWEN_TOKEN_PLAN_API_KEY".into()),
        default_model: Some("qwen3-max".into()),
        extra_body: None,
    }
}
