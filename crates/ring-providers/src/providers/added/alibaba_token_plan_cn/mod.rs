use crate::catalog::{CatalogEntry, ProviderKind};

pub fn catalog_entry() -> CatalogEntry {
    CatalogEntry {
        api_key: Vec::new(),
        name: "Alibaba Token Plan CN".into(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Some("https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1".into()),
        api_key_env: Some("ALIBABA_TOKEN_PLAN_CN_API_KEY".into()),
        default_model: Some("qwen3-max".into()),
        extra_body: None,
    }
}
